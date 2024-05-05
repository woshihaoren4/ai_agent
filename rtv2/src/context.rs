use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use crate::{END_RESULT_ERROR, Input, Node, Output, Plan, RTError, Service, ServiceLoader, WakerWaitPool};


#[derive(Debug)]
pub struct Context {
    //任务流名称
    pub code: String,
    //堆栈信息
    pub meta: Arc<Meta>,
    //执行计划
    pub plan:Arc<dyn Plan>,
    //全局扩展字段
    pub extend: Mutex<HashMap<String, Box<dyn Any + Send + Sync + 'static>>>,
    //结束时回调
    pub over_callback: Vec<Box<dyn Fn(Arc<Context>)>>,
    //可能存在父亲流程
    // pub(crate) parent_ctx:Option<Arc<Context>>,
    pub(crate) middle:VecDeque<Arc<dyn Service>>,
    pub(crate) nodes:Arc<dyn ServiceLoader>,
    pub(crate) wakers:Arc<dyn WakerWaitPool>
}

#[derive(Debug)]
pub struct Flow{
    pub ctx:Arc<Context>,
    pub code:String,
    pub node_type_id:String, //类型节点id
    pub node_config:String,  //类型节点配置
    //中间流程
    pub(crate) middle:VecDeque<Arc<dyn Service>>,
}


#[derive(Debug, Default)]
pub struct Meta{
    pub status: AtomicU8, //0: running,1:success,2:error
    pub stack: Arc<Mutex<ContextStack>>,
}
#[derive(Debug, Default)]
pub struct ContextStack{
    round: usize,
    //round,parent_id,node_id -> node_id
    stack: Vec<(usize,String,String,String)>,
}
impl Context{
    pub fn set<S:Into<String>,V:Any + Send + Sync>(&self,key:S,value:V){
        let mut lock = self.extend.lock().unwrap();
        lock.insert(key.into(),Box::new(value));
    }
    pub fn error_over(&self,err:impl Error){
        let err = format!("{}",err);
        //fixme cas
        self.meta.status.store(2u8,Ordering::Relaxed);
        self.set(END_RESULT_ERROR,err);
    }
}

impl Flow{
    pub fn new(node:Node,ctx:Arc<Context>,middle:VecDeque<Arc<dyn Service>>)->Self{
        let Node{ code, node_type_id, node_config,.. } = node;
        Self{ctx,code,node_type_id,node_config,middle}
    }
    pub async fn next(mut self)-> anyhow::Result<Output>{
        let opt = self.middle.pop_front();
        let n = match opt {
            None => return RTError::FlowLastNodeNil.into().err(),
            Some(n) => n,
        };
        n.next().await
    }
}