use crate::context::{ContextEntity, CtxStatus};
use crate::{
    Context, Node, Output, Plan, PlanResult, ProgramPool, ProgramPoolImpl, ServiceFn,
    ServiceLoader, ServiceLoaderImpl, ServiceMiddle, TaskFlowHook,
};
use pin_project_lite::pin_project;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use wd_tools::PFErr;

pub type RuntimeBuilder = RuntimeEntity;
pub struct RuntimeEntity {
    pub services: Box<dyn ServiceLoader + Sync + 'static>,
    pub service_middles: Vec<Arc<dyn ServiceMiddle + Sync + 'static>>,
    pub thread_pool: Box<dyn ProgramPool + Sync + 'static>,
    pub task_flow_middles_start: Vec<Box<dyn TaskFlowHook + Sync + 'static>>,
    pub task_flow_middles_end: Vec<Box<dyn TaskFlowHook + Sync + 'static>>,
}

pub struct Runtime {
    pub(crate) entity: Arc<RuntimeEntity>,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        let services = Box::new(ServiceLoaderImpl::default());
        let service_middles = Vec::new();
        let thread_pool = Box::new(ProgramPoolImpl);
        let task_flow_middles_start = Vec::new();
        let task_flow_middles_end = Vec::new();
        Self {
            services,
            service_middles,
            thread_pool,
            task_flow_middles_start,
            task_flow_middles_end,
        }
        .register_service_middle_fn(Runtime::input_output_middle)
    }
}
impl RuntimeBuilder {
    pub fn set_service_loader<T: ServiceLoader + Sync + 'static>(mut self, loader: T) -> Self {
        self.services = Box::new(loader);
        self
    }
    pub fn register_service_middle<F: ServiceMiddle + Sync + 'static>(mut self, middle: F) -> Self {
        self.service_middles.push(Arc::new(middle));
        self
    }
    pub fn register_service_middle_fn<F, Fut>(mut self, middle_fn: F) -> Self
    where
        F: Fn(Context, Node) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<Output>> + Send,
    {
        self.service_middles
            .push(Arc::new(ServiceFn::new(middle_fn)));
        self
    }
    pub fn register_task_start_hook<F: TaskFlowHook + Sync + 'static>(mut self, hook: F) -> Self {
        self.task_flow_middles_start.push(Box::new(hook));
        self
    }
    pub fn register_task_start_hook_fn<F, Fut>(mut self, hook_fn: F) -> Self
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send,
    {
        self.task_flow_middles_start
            .push(Box::new(ServiceFn::new(hook_fn)));
        self
    }
    pub fn register_task_end_hook<F: TaskFlowHook + Sync + 'static>(mut self, hook: F) -> Self {
        self.task_flow_middles_end.push(Box::new(hook));
        self
    }
    pub fn register_task_end_hook_fn<F, Fut>(mut self, hook_fn: F) -> Self
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send,
    {
        self.task_flow_middles_end
            .push(Box::new(ServiceFn::new(hook_fn)));
        self
    }
    pub fn build(self) -> Runtime {
        Runtime {
            entity: Arc::new(self),
        }
    }
}

impl Clone for Runtime {
    fn clone(&self) -> Self {
        Runtime {
            entity: self.entity.clone(),
        }
    }
}

impl Runtime {
    pub fn context<P: Plan + Sync + 'static>(&self, p: P) -> Context {
        ContextEntity::new(self.clone(), p).build()
    }
}

impl Runtime {
    pub async fn go<In: Any, Out: Any>(mut ctx: Context, input: In) -> anyhow::Result<Out> {
        let start_node_name = ctx.lock(|c| c.plan.start_node_name().to_string());
        ctx = ctx.set(start_node_name, input);
        Self::start_and_wait_task_over(ctx.clone()).await;
        let end_node_name = ctx.lock(|c| c.plan.end_node_name().to_string());
        match ctx.into_status() {
            CtxStatus::SUCCESS => ctx.remove(end_node_name.as_str()).await,
            CtxStatus::Error(e) => Err(e),
            ca => anyhow::anyhow!("Runtime.go Abnormal state[{}]", ca).err(),
        }
    }
    pub async fn start_and_wait_task_over(mut ctx: Context) {
        let (nodes, rt) = match ctx
            .ctx(|c| {
                let start_node_name = c.plan.start_node_name().to_string();
                if let PlanResult::Nodes(nodes) = c.plan.next(start_node_name.as_str())? {
                    Ok((nodes, c.rt.clone()))
                } else {
                    anyhow::anyhow!("plan start node result is not PlanResult::Nodes").err()
                }
            })
            .await
        {
            Ok(o) => o,
            Err(e) => {
                ctx.error(e);
                return;
            }
        };
        //执行前置任务
        for i in rt.entity.task_flow_middles_start.iter() {
            if let Err(e) = i.call(ctx.clone()).await {
                ctx.error(e);
                return;
            }
        }
        let task = TaskFlowWait::from(ctx.clone());
        let wait = TaskFlowPush::from(ctx.clone());
        let art = rt.clone();
        let actx = ctx.clone();
        tokio::spawn(async move {
            wait.await;
            for mut i in nodes {
                let service = art.entity.services.load(i.service_name.as_str()).await;
                if let Some(s) = service {
                    i = i.set_service_are(s)
                } else {
                    actx.error(anyhow::anyhow!(
                        "Node[{}].service[{}] not found",
                        i.name,
                        i.service_name
                    ));
                    return;
                }
                if let Err(e) = art
                    .entity
                    .thread_pool
                    .push(Box::pin(actx.clone().next(i)))
                    .await
                {
                    actx.error(e);
                    return;
                }
            }
        });
        if let Err(e) = task.await {
            ctx.error(e);
        }
        //执行后置任务
        for i in rt.entity.task_flow_middles_end.iter() {
            if let Err(e) = i.call(ctx.clone()).await {
                ctx.error(e);
                return;
            }
        }
    }
}

pin_project! {
    struct TaskFlowWait{
    ctx:Context
}
}

impl From<Context> for TaskFlowWait {
    fn from(ctx: Context) -> Self {
        Self { ctx }
    }
}
impl Future for TaskFlowWait {
    type Output = anyhow::Result<()>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        this.ctx.lock(|c| {
            return match c.status {
                CtxStatus::New => {
                    let mut status = CtxStatus::RUNNING(cx.waker().clone());
                    unsafe {
                        std::ptr::swap(&mut c.status, &mut status);
                    }
                    Poll::Pending
                }
                CtxStatus::RUNNING(_) => Poll::Ready(
                    anyhow::anyhow!("TaskFlowWait.status[RUNNING]: Abnormal wake up").err(),
                ),
                CtxStatus::SUCCESS => Poll::Ready(Ok(())),
                CtxStatus::Error(ref _e) => Poll::Ready(Ok(())),
                CtxStatus::Over => Poll::Ready(
                    anyhow::anyhow!("TaskFlowWait.status[Over]: Abnormal wake up").err(),
                ),
            };
        })
    }
}

struct TaskFlowPush {
    ctx: Context,
}
impl From<Context> for TaskFlowPush {
    fn from(ctx: Context) -> Self {
        Self { ctx }
    }
}
impl Future for TaskFlowPush {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if self.ctx.is_running() {
            return Poll::Ready(());
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
