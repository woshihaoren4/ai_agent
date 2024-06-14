use crate::context::Context;
use crate::{Flow, Node, Output, PlanNode};
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::task::Waker;

pub const START_NODE_CODE: &'static str = "start";
pub const END_NODE_CODE: &'static str = "end";
pub const END_RESULT_ERROR: &'static str = "xxx_rt_end_result_error";
pub const END_ABNORMAL_END: &'static str = "xxx_rt_end_abnormal_end";

#[async_trait::async_trait]
pub trait Service: Send + Sync {
    async fn call(&self, flow: Flow) -> anyhow::Result<Output>;
}

pub trait ServiceLoader: Send + Sync {
    fn get(&self, ids: &str) -> Option<Arc<dyn Service>>;
    fn set(&self, nodes: Vec<(String, Arc<dyn Service>)>);
}

pub struct WakerCallBack {
    pub waker: Waker,
}

pub trait WakerWaitPool: Send + Sync {
    fn push(&self, code: String, waker: WakerCallBack);
    fn remove(&self, code: &str) -> Option<WakerCallBack>;
}

#[derive(Debug)]
pub enum NextNodeResult {
    Over,             //没有下一个节点了
    Wait,             //下一个节点未就绪
    Error(String),    //只需等待即可
    Nodes(Vec<Node>), //向下一个分支走
}
pub trait Plan: Send + Sync {
    fn next(&self, ctx: Arc<Context>, node_id: &str) -> NextNodeResult;
    fn set(&self, nodes: Vec<PlanNode>);
    fn update(&self,node_code:&str,update:Box<dyn FnOnce(Option<&mut PlanNode>)>);
}

#[derive(Debug)]
pub struct ServiceFn<F> {
    function: F,
    // _p:PhantomData<Fut>,
}
impl<T> ServiceFn<T> {
    pub fn new(function: T) -> ServiceFn<T> {
        Self { function }
        // Self{function,_p:PhantomData::default()}
    }
}

#[async_trait::async_trait]
impl<F, Fut> Service for ServiceFn<F>
where
    F: Fn(Flow) -> Fut + Send + Sync,
    Fut: Future<Output = anyhow::Result<Output>> + Send,
{
    async fn call(&self, flow: Flow) -> anyhow::Result<Output> {
        (self.function)(flow).await
    }
}
