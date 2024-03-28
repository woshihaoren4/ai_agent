use crate::default_callback_set::DefaultCallbackSet;
use crate::rwmap_node_loader::RWMapNodeLoader;
use crate::{CallBack, CallBackSet, Context, Node, NodeLoader, Task, TaskInput, TaskOutput};
use async_channel::{Receiver, Sender};
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::Pin;
use std::ptr::replace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use wd_tools::{PFArc, PFErr};

pub struct Runtime {
    status: Arc<AtomicUsize>, // 1:await 2:running 3:quiting 4:dead
    nodes: Arc<dyn NodeLoader>,
    task_chan: Option<Sender<Task>>,
    result_chan: Option<Receiver<TaskOutput>>,
    callback: Arc<dyn CallBackSet>,
    // receiver_msg:
}
pub struct RuntimeWait {
    code: String,
    callback: Arc<dyn CallBackSet>,
    output: Arc<Mutex<Option<TaskOutput>>>,
}

impl RuntimeWait {
    pub fn new(
        code: String,
        callback: Arc<dyn CallBackSet>,
        output: Arc<Mutex<Option<TaskOutput>>>,
    ) -> Self {
        RuntimeWait {
            code,
            callback,
            output,
        }
    }
}

impl Future for RuntimeWait {
    type Output = TaskOutput;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut lock = self.output.lock().unwrap();
        if lock.is_none() {
            let waker = cx.waker().clone();
            self.callback.push(self.code.clone(), CallBack::new(waker));
            return Poll::Pending;
        }
        let opt = unsafe { replace(lock.deref_mut(), None) };
        let output = opt.unwrap();
        return Poll::Ready(output);
    }
}
impl Clone for Runtime {
    fn clone(&self) -> Self {
        let task_chan = if let Some(ref s) = self.task_chan {
            Some(s.clone())
        } else {
            None
        };
        let result_chan = if let Some(ref s) = self.result_chan {
            Some(s.clone())
        } else {
            None
        };
        Runtime {
            status: self.status.clone(),
            nodes: self.nodes.clone(),
            task_chan,
            result_chan,
            callback: self.callback.clone(),
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        let status = AtomicUsize::new(1).arc();
        let nodes = RWMapNodeLoader::default().arc();
        let task_chan = None;
        let result_chan = None;
        let callback = DefaultCallbackSet::default().arc();
        Self {
            status,
            nodes,
            task_chan,
            result_chan,
            callback,
        }
    }
    pub fn upsert_node<N: Node + 'static>(&mut self, id: String, n: N) {
        self.nodes.set(vec![(id, Arc::new(n))]);
    }

    pub fn launch(&mut self) {
        let rt = self.clone();
        let (task_sender, task_receiver) = async_channel::bounded(1024);
        let (result_sender, result_receiver) = async_channel::bounded(1024);

        self.task_chan = Some(task_sender.clone());
        self.result_chan = Some(result_receiver.clone());
        tokio::spawn(async {
            rt.start_work(task_receiver, task_sender, result_sender)
                .await;
        });
        let rt = self.clone();
        rt.start_result_consumer();
        self.status.store(1, Ordering::Relaxed);
    }
    pub async fn raw_run(
        &self,
        mut ctx: Context,
        first_node_id: String,
        input: TaskInput,
    ) -> anyhow::Result<TaskOutput> {
        let status = self.status.load(Ordering::Relaxed);
        if self.status.load(Ordering::Relaxed) != 1 {
            return anyhow::anyhow!("Runtime status[{}] not running", status).err();
        }
        let output = Arc::new(Mutex::new(None));
        ctx.set_output(output.clone());
        let ctx = Arc::new(ctx);
        let code = ctx.code.clone();
        let task = Task::new(ctx, "".into(), first_node_id, Box::new(Some(true))).set_input(input);
        let rtw = RuntimeWait::new(code, self.callback.clone(), output);

        if let Some(ref s) = self.task_chan {
            s.send(task).await?;
        } else {
            return anyhow::anyhow!("task chan is null").err();
        };

        let result = rtw.await;
        Ok(result)
    }
    pub async fn call<
        S: Into<String>,
        F: Into<String>,
        In: Any + Send + Sync + 'static,
        Out: Any + Send + Sync + 'static,
        CH: FnOnce(Context)->Context,
    >(
        &self,
        task_code: S,
        first_node_id: F,
        input: In,
        ctx_handle:CH,
    ) -> anyhow::Result<Out> {
        let input = TaskInput::from_value(input);
        let ctx = Context::new(task_code.into());
        let ctx = ctx_handle(ctx);

        let mut output = self.raw_run(ctx, first_node_id.into(), input).await?;

        if let Some(e) = output.error {
            return anyhow::anyhow!("run error:{}", e).err();
        }

        match output.get_value() {
            Some(out) => Ok(out),
            None => anyhow::anyhow!("output type reflect failed or not result").err(),
        }
    }
    pub async fn run<
        S: Into<String>,
        F: Into<String>,
        In: Any + Send + Sync + 'static,
        Out: Any + Send + Sync + 'static,
    >(
        &self,
        task_code: S,
        first_node_id: F,
        input: In,
    ) -> anyhow::Result<Out> {
        self.call(task_code,first_node_id,input,|x|x).await
    }

    fn start_result_consumer(self) {
        let Runtime {
            // status,
            // nodes,
            // task_chan,
            result_chan,
            // callback,
            ..
        } = self;
        tokio::spawn(async move {
            let result_chan = result_chan.unwrap();
            while let Ok(output) = result_chan.recv().await {
                let code = output.ctx.clone();
                output.into_output();
                if let Some(call) = self.callback.remove(code.code.as_str()) {
                    call.waker.wake_by_ref();
                } else {
                    wd_log::log_field("error", "not find task waker")
                        .field("task_flow_code", code.code.as_str())
                        .info("not find");
                }
            }
        });
    }

    async fn start_work(
        self,
        receiver: Receiver<Task>,
        task_sender: Sender<Task>,
        result_chan: Sender<TaskOutput>,
    ) {
        let mut task_set = HashMap::<String, Task>::new(); //fixme 需要做超时处理
        let node_loader = self.nodes.clone();
        while let Ok(s) = receiver.recv().await {
            //先看是否已经有节点执行过task了
            let task = if let Some(mut t) = task_set.remove(s.node_id.as_str()) {
                t.input.append(s.input);
                t
            } else {
                s
            };

            let id = task.node_id.clone();
            let node = match node_loader.get(id.as_str()) {
                Ok(o) => o,
                Err(e) => {
                    let mut output = TaskOutput::error(e);
                    output.set_ctx(task.ctx);
                    if let Err(e) = result_chan.send(output).await {
                        wd_log::log_field("error", e)
                            .error("get node failed,send to result_chan channel failed");
                    }
                    continue;
                }
            };

            if !node.ready(task.ctx.clone(), &task.input) {
                task_set.insert(task.node_id.clone(), task);
                continue;
            }
            self.async_run_task(task, node, task_sender.clone(), result_chan.clone());
        }
    }
    fn async_run_task(
        &self,
        task: Task,
        node: Arc<dyn Node>,
        task_chan: Sender<Task>,
        result_chan: Sender<TaskOutput>,
    ) {
        let Task {
            ctx,
            node_id,
            input,
        } = task;
        tokio::spawn(async move {
            let result = node.go(ctx.clone(), input).await;
            let mut output = match result {
                Ok(o) => o,
                Err(e) => {
                    let mut output = TaskOutput::error(e);
                    output.set_ctx(ctx);
                    if let Err(e) = result_chan.send(output).await {
                        wd_log::log_field("error", e)
                            .error("node run failed, send to result_chan channel failed");
                    }
                    return;
                }
            };
            if output.over || output.error.is_some() {
                output.set_ctx(ctx.clone());
                if let Err(e) = result_chan.send(output).await {
                    wd_log::log_field("error", e)
                        .error("node over, send to result_chan channel failed");
                }
                return;
            }
            for (id, val) in output.result {
                ctx.add_task_to_chain(node_id.as_str(), id.as_str());
                let task = Task::new(ctx.clone(), node_id.clone(), id, val);
                if let Err(e) = task_chan.send(task).await {
                    wd_log::log_field("error", e)
                        .error("run next node, send to task_chan channel failed");
                }
                return;
            }
        });
    }
}
