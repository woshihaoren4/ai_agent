use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::Waker;
use serde_json::Value;
use crate::context::Context;
use crate::{Flow, Input, Output};

pub const START_NODE_CODE:&'static str = "start";
pub const END_RESULT_ERROR:&'static str = "end_result_error";

#[async_trait::async_trait]
pub trait Service: Send + Sync {
    async fn call(&self, flow: Flow) -> anyhow::Result<Output>;
}

pub trait ServiceLoader:Send + Sync {
    fn get(&self, ids: &str) -> Option<Arc<dyn Service>>;
    fn set(&self, nodes: Vec<(String, Arc<dyn Service>)>);
}

pub struct WakerCallBack {
    waker: Waker,
}

pub trait WakerWaitPool: Send + Sync {
    fn push(&self, code: String, waker: WakerCallBack);
    fn remove(&self, code: &str) -> Option<WakerCallBack>;
}

pub struct Node{
    pub code:String,       //当前节点的编码
    // pub ready:Vec<Node>,   //他的上一个节点
    // pub go:Vec<Node>,      //他要去的下一个节点

    pub node_type_id:String, //类型节点id
    pub node_config:String,  //类型节点配置

    // pub sub_plan:Option<Box<dyn Plan>>,
    // pub sub_input_call:Option<Box<dyn Fn(&Flow)->anyhow::Result<Input>>>,
}

pub enum NextNodeResult{
    Over, //流程结束
    Wait, //只需等待即可
    Nodes(Vec<Node>), //向下一个分支走
}
pub trait Plan: Send + Sync{
    fn next(&self,ctx: Arc<Context>,node_id:&str)->NextNodeResult;
}

#[derive(Debug)]
pub struct ServiceFn<F,Fut>{
    function:F,
    _p:PhantomData<Fut>,
}
impl<T,F> ServiceFn<T,F>{
    pub fn new(function:T)->ServiceFn<T,F>{
        Self{function,_p:PhantomData::default()}
    }
}

#[async_trait::async_trait]
impl<F,Fut> Service for ServiceFn<F,Fut>
where F:Fn(Flow)->Fut + Send + Sync,
    Fut:Future<Output=anyhow::Result<Output>> + Send + Sync
{
    async fn call(&self, flow: Flow) -> anyhow::Result<Output> {
        (self.function)(flow).await
    }
}