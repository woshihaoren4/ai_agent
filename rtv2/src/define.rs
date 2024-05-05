use std::fmt::Debug;
use std::sync::Arc;
use std::task::Waker;
use serde_json::Value;
use crate::context::Context;
use crate::{Flow, Input, Output};

pub const START_NODE_CODE:&'static str = "start";
pub const END_RESULT_ERROR:&'static str = "end_result_error";

#[async_trait::async_trait]
pub trait Service: Debug + Send + Sync {
    async fn call(&self, ctx: Flow) -> anyhow::Result<Output>;
}

pub trait ServiceLoader:Debug + Send + Sync {
    fn get(&self, ids: &str) -> Option<Arc<dyn Service>>;
    fn set(&self, nodes: Vec<(String, Arc<dyn Service>)>);
}

pub struct CallBack {
    waker: Waker,
}

pub trait WakerWaitPool:Debug + Send + Sync {
    fn push(&self, code: String, waker: CallBack);
    fn remove(&self, code: &str) -> Option<CallBack>;
}

pub struct Node{
    pub code:String,       //当前节点的编码
    pub ready:Vec<Node>,   //他的上一个节点
    pub go:Vec<Node>,      //他要去的下一个节点

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
pub trait Plan:Debug+ Send + Sync{
    fn next(&self,ctx: Arc<Context>,node_id:&str)->NextNodeResult;
}