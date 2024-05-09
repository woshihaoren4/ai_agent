use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;
use wd_tools::{PFArc, PFErr};
use crate::{Context, CtxStatus, Flow, Input, NextNodeResult, Output, Plan, RTError, Service, ServiceFn, ServiceLoader, START_NODE_CODE, WakerWaitPool};
use crate::default_node_loader::DefaultNodeLoader;
use crate::default_waker_pool::DefaultWakerPool;

#[derive(Clone)]
pub struct Runtime{
    status: Arc<AtomicUsize>, // 1:await 2:running 3:quiting 4:dead
    //先注册的先执行
    middle:VecDeque<Arc<dyn Service>>,
    nodes:Arc<dyn ServiceLoader>,
    waker:Arc<dyn WakerWaitPool>,
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
    pub fn register_middle_fn<T:Future<Output=anyhow::Result<Output>> + Send + Sync+ 'static,F:Fn(Flow)->T+Send+Sync+ 'static>(mut self,service:F)->Self{
        self.register_middle(ServiceFn::new(service))
    }
    pub fn register_service<ID:Into<String>,S:Service+ 'static>(mut self,id:ID,service:S)->Self{
        self.nodes.set(vec![(id.into(),Arc::new(service))]);self
    }
    pub fn register_service_fn<ID:Into<String>,T:Future<Output=anyhow::Result<Output>> + Send + Sync+ 'static,F:Fn(Flow)->T+Send+Sync+ 'static>(mut self,id:ID,service:F)->Self{
        self.register_service(id.into(),ServiceFn::new(service))
    }
    pub fn launch(self)->Self{
        self.status.store(2,Ordering::Relaxed);self
    }
    pub fn stop(&self){
        self.status.store(3,Ordering::Relaxed);
    }
    pub fn is_running(&self)->bool{
        self.status.load(Ordering::Relaxed) == 2
    }
    pub async fn spawn(&self,ctx:Arc<Context>)->anyhow::Result<()>{
        //检查状态
        if !self.is_running(){
            return RTError::RuntimeDisable.anyhow()
        }
        if ctx.meta.status() != CtxStatus::INIT {
            return RTError::ContextStatusAbnormal("ctx status is not init".into()).anyhow()
        }
        //任务统计

        //修改ctx状态
        ctx.meta.set_status(CtxStatus::RUNNING);
        //执行
        Runtime::exec_next_node(ctx, START_NODE_CODE);
        Ok(())
    }
    pub fn block_on(&self,ctx:Arc<Context>)->anyhow::Result<()>{

    }
    fn exec_next_node(ctx:Arc<Context>,node_id:&str){
        let result = ctx.plan.next(ctx.clone(), node_id);
        let nodes = match result {
            NextNodeResult::Over => {
                return
            }
            NextNodeResult::Wait => {
                return
            }
            NextNodeResult::Nodes(s) => s,
        };
        for i in nodes{
            let mut middle = ctx.middle.clone();
            match ctx.nodes.get(i.node_type_id.as_str()) {
                None => {
                    let err =  RTError::UnknownNodeId(i.node_type_id);
                    ctx.error_over(err);
                    return;
                },
                Some(n) => middle.push_back(n),
            };
            let flow = Flow::new(i, ctx.clone(), middle);
            tokio::spawn(async move{
                let code = flow.code.clone();
                let ctx = flow.ctx.clone();
                if let Err(e) = flow.call().await {
                    wd_log::log_error_ln!("Runtime.exec_next_node:Unanticipated errors:{}",e);
                    ctx.error_over(e.deref());
                }else{
                    Runtime::exec_next_node(ctx,code.as_str());
                }
            });
        };
    }
    pub fn ctx<C:Into<String>,P:Plan+ 'static>(&self,code:C,plan:P)->Context{
        Context{
            code: code.into(),
            meta: Arc::new(Default::default()),
            plan: Arc::new(plan),
            extend: Mutex::new(Default::default()),
            over_callback: vec![],
            middle: self.middle.clone(),
            nodes: self.nodes.clone(),
            waker: self.waker.clone(),
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        let sl = DefaultNodeLoader::default();
        let wwp = DefaultWakerPool::default();
        Runtime::new(sl,wwp)
    }
}

#[derive(Clone)]
pub struct RuntimeWait{
    ctx:Arc<Context>,
    waker:Arc<dyn WakerWaitPool>,
}

impl Future for RuntimeWait {
    type Output = anyhow::Result<Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.ctx.meta.status() {
            CtxStatus::INIT => {
                let waker = cx.waker().clone();
                let code = self.ctx.code.clone();
                self.waker.push(code,waker)
                return Poll::Pending
            }
            CtxStatus::RUNNING => {}
            CtxStatus::SUCCESS => {}
            CtxStatus::ERROR => {}
        }
    }
}