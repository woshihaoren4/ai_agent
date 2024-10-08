use std::collections::VecDeque;
use std::sync::Arc;
use wd_tools::Ctx;
use crate::Context;

pub struct ContextImpl{

}

pub struct Input{

}
pub struct Output{

}

pub struct Node{
    pub name : String,
    pub service_name: String,

    pub(crate) middle_index : usize,
    pub(crate) service : Arc<dyn Service>,
}

pub enum PlanResult{
    Nodes(Vec<Node>),
    End,
    Wait,
}

pub trait Plan : Send{
    fn string(&self)->String{
        "".into()
    }
    fn next(&mut self,name:&str)->anyhow::Result<PlanResult>;
    fn remove(&mut self,name:&str)->Option<Node>;
    fn insert(&mut self,name:&str,node:Node);
}

#[async_trait::async_trait]
pub trait Service: Send {
    async fn call(&self, ctx: Arc<Context>, node:Node) -> anyhow::Result<Output>;
}

#[async_trait::async_trait]
pub trait ServiceLoader {
    async fn load(&self, name:&str)->Option<Arc<dyn Service>>;
}

#[async_trait::async_trait]
pub trait ServiceMiddle: Send {
    // true: Continue to execute
    // false: Skip the middle
    fn filter(&self,_node:&Node)->bool{
        true
    }

    async fn call(&self, ctx: Arc<Context>, node:Node) -> anyhow::Result<Output>;
}