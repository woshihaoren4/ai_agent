use std::any::Any;
use std::collections::{ VecDeque};
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;
use wd_tools::{PFArc, PFErr};
use crate::{Context, CtxStatus, END_ABNORMAL_END, END_NODE_CODE, Flow, NextNodeResult, Output, Plan, RTError, Service, ServiceFn, ServiceLoader, START_NODE_CODE, WakerCallBack, WakerWaitPool};
use crate::default_node_loader::DefaultNodeLoader;
use crate::default_waker_pool::DefaultWakerPool;

#[derive(Clone)]
pub struct Runtime{
    pub(crate) status: Arc<AtomicUsize>, // 1:await 2:running 3:quiting 4:dead
    //先注册的先执行
    pub(crate) middle:VecDeque<Arc<dyn Service>>,
    pub(crate) nodes:Arc<dyn ServiceLoader>,
    pub(crate) waker:Arc<dyn WakerWaitPool>,
}

impl Runtime {
    pub fn new<SL:ServiceLoader+ 'static,W:WakerWaitPool+ 'static>(sl:SL,waker:W)->Self{
        let status = AtomicUsize::new(1).arc();
        let middle = VecDeque::default();
        let nodes = Arc::new(sl);
        let waker = Arc::new(waker);
        Self{status,middle,nodes,waker}
    }
    pub fn register_middle<Mid:Service+ 'static>(mut self,service:Mid)->Self{
        self.middle.push_back(service.arc());self
    }
    pub fn register_middle_fn<T:Future<Output=anyhow::Result<Output>> + Send + 'static,F:Fn(Flow)->T+Send+Sync+ 'static>(self,service:F)->Self{
        self.register_middle(ServiceFn::new(service))
    }
    pub fn register_service<ID:Into<String>,S:Service+ 'static>(self,id:ID,service:S)->Self{
        self.nodes.set(vec![(id.into(),Arc::new(service))]);self
    }
    pub fn register_service_fn<ID:Into<String>,T:Future<Output=anyhow::Result<Output>> + Send + Sync+ 'static,F:Fn(Flow)->T+Send+Sync+ 'static>(self,id:ID,service:F)->Self{
        self.register_service(id.into(),ServiceFn::new(service))
    }
    pub fn launch(self)->Arc<Self>{
        self.status.store(2,Ordering::Relaxed);self.arc()
    }
    pub fn stop(&self){
        self.status.store(3,Ordering::Relaxed);
    }
    pub fn is_running(&self)->bool{
        self.status.load(Ordering::Relaxed) == 2
    }

    pub fn check(&self,ctx:&Context)->anyhow::Result<()>{
        //检查状态
        if !self.is_running(){
            return RTError::RuntimeDisable.anyhow()
        }
        if ctx.status() != CtxStatus::INIT {
            return RTError::ContextStatusAbnormal("ctx status is not init".into()).anyhow()
        }
        //todo 任务统计
        Ok(())
    }
    pub fn spawn(&self,ctx:Arc<Context>)->anyhow::Result<()>{
        self.check(&ctx)?;
        //修改ctx状态
        ctx.set_status(CtxStatus::RUNNING);
        //执行
        Runtime::exec_next_node(ctx, START_NODE_CODE);
        Ok(())
    }
    pub async fn block_on<Out:Any>(&self,ctx:Arc<Context>)->anyhow::Result<Out>{
        self.check(&ctx)?;
        let rt_wait = RuntimeWait::<Out>{
            ctx,
            _out: Default::default(),
        };
        rt_wait.await
    }
    fn exec_next_node(ctx:Arc<Context>, node_code:&str){
        let result = ctx.plan.next(ctx.clone(), node_code);
        let nodes = match result {
            NextNodeResult::Over | NextNodeResult::Wait => {
                return
            }
            NextNodeResult::Error(e) => {
                ctx.error_over(RTError::UNKNOWN(e));
                return
            }
            NextNodeResult::Nodes(s) => s,
        };
        for i in nodes{
            let mut middle = ctx.runtime.middle.clone();
            match ctx.runtime.nodes.get(i.node_type_id.as_str()) {
                None => {
                    let err =  RTError::UnknownNodeId(i.node_type_id);
                    ctx.error_over(err);
                    return;
                },
                Some(n) => middle.push_back(n),
            };

            let flow = Flow::new(i, ctx.clone(), middle);
            let this_node_code = node_code.to_string();

            tokio::spawn(async move{
                let code = flow.code.clone();
                let ctx = flow.ctx.clone();


                let parent_ctx_code = ctx.parent_code.clone().unwrap_or("".into());
                ctx.push_stack_info(parent_ctx_code,this_node_code,flow.code.clone());

                if let Err(e) = flow.call().await {
                    //检查是否强制终止
                    if let Some(e) = e.downcast_ref::<RTError>(){
                        if *e == RTError::ContextAbort{
                            return;
                        }
                    }
                    //否则为异常错误
                    wd_log::log_error_ln!("Runtime.exec_next_node:Unanticipated errors:{}",e);
                    ctx.error_over(e.deref());
                }else{
                    Runtime::exec_next_node(ctx,code.as_str());
                }
            });
        };
    }
    pub fn ctx<C:Into<String>,P: Plan + 'static>(self:&Arc<Self>, code:C, plan:P) ->Context{
        Context::new(code,plan,self.clone())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        let sl = DefaultNodeLoader::default();
        let wwp = DefaultWakerPool::default();
        Runtime::new(sl,wwp).register_default_middle_handles()
    }
}

#[derive(Clone)]
pub struct RuntimeWait<O>{
    ctx:Arc<Context>,
    _out:PhantomData<O>,
}

impl<O:Any> Future for RuntimeWait<O> {
    type Output = anyhow::Result<O>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.ctx.status() {
            CtxStatus::INIT => {
                //先添加回调任务
                let waker = cx.waker().clone();
                let code = self.ctx.code.clone();
                let waker = WakerCallBack{waker};
                self.ctx.runtime.waker.push(code,waker);
                //再执行任务
                self.ctx.set_status(CtxStatus::RUNNING);
                Runtime::exec_next_node(self.ctx.clone(),START_NODE_CODE);

                return Poll::Pending
            }
            CtxStatus::RUNNING => {
                let err_info = "running status can not to RuntimeWait.poll";
                wd_log::log_warn_ln!("{}",err_info);
                return Poll::Ready(anyhow::Error::msg(err_info).err())
            }
            CtxStatus::SUCCESS |CtxStatus::ERROR => {
                let result = self.ctx.end_output::<O>();
                return Poll::Ready(result);
            }
        }
    }
}