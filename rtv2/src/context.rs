use std::any::{Any};
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use wd_tools::PFErr;
use crate::{END_NODE_CODE, END_RESULT_ERROR, Node, Output, Plan, RTError, Service, ServiceLoader, WakerWaitPool};


pub struct Context {
    pub parent_code:Option<String>,
    //任务流名称
    pub code: String,
    //堆栈信息
    pub meta: Arc<Meta>,
    //执行计划
    pub plan:Arc<dyn Plan>,
    //全局扩展字段
    pub extend: Mutex<HashMap<String, Box<dyn Any + Send + Sync + 'static>>>,
    //结束时回调
    pub over_callback: Option<Mutex<Vec<Box<dyn FnOnce(Arc<Context>)+Send+Sync+'static>>>> ,
    //可能存在父亲流程
    // pub(crate) parent_ctx:Option<Arc<Context>>,
    pub(crate) middle:VecDeque<Arc<dyn Service>>,
    pub(crate) nodes:Arc<dyn ServiceLoader>,
    pub(crate) waker:Arc<dyn WakerWaitPool>
}

impl Debug for Context{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",format!("code:{},meta:{:?}",self.code,self.meta))
    }
}

pub struct Flow{
    pub ctx:Arc<Context>,
    pub code:String,
    pub node_type_id:String, //类型节点id
    pub node_config:String,  //类型节点配置
    //中间流程
    pub(crate) middle:VecDeque<Arc<dyn Service>>,
}

impl Debug for Flow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"ctx:{:?},code:{},node_type_id:{},node_config:{}",self.ctx,self.code,self.node_type_id,self.node_config)
    }
}


#[derive(Debug, Default)]
pub struct Meta{
    pub status: AtomicU8, //0:init 1:running, 2:success, 3:error
    pub stack: Arc<Mutex<ContextStack>>,
}

#[derive(Debug,Eq, PartialEq)]
pub enum CtxStatus{
    INIT,RUNNING,SUCCESS,ERROR
}
impl From<CtxStatus> for u8{
    fn from(value: CtxStatus) -> Self {
        match value {
            CtxStatus::INIT => 0u8,
            CtxStatus::RUNNING => 1u8,
            CtxStatus::SUCCESS => 2u8,
            CtxStatus::ERROR => 3u8,
        }
    }
}
impl From<u8> for CtxStatus{
    fn from(value: u8) -> Self {
        match value {
            0=>CtxStatus::INIT,
            1=>CtxStatus::RUNNING,
            2=>CtxStatus::SUCCESS,
            _=>CtxStatus::ERROR,
        }
    }
}

#[derive(Debug, Default)]
pub struct ContextStack{
    round: usize,
    //round,parent_id,node_id -> node_id
    stack: Vec<(usize,String,String,String)>,
}

impl Context{
    pub fn new<C:Into<String>,P:Plan + 'static>(code:C,plan:P,middle: VecDeque<Arc<dyn Service>>, nodes: Arc<dyn ServiceLoader>, waker: Arc<dyn WakerWaitPool>)->Self{
        Self{
            parent_code: None,
            code: code.into(),
            meta: Arc::new(Default::default()),
            plan: Arc::new(plan),
            extend: Mutex::new(Default::default()),
            over_callback: None,
            middle,
            nodes,
            waker,
        }
    }
    pub fn set<S:Into<String>,V:Any + Send + Sync>(&self,key:S,value:V){
        let mut lock = self.extend.lock().unwrap();
        lock.insert(key.into(),Box::new(value));
    }
    pub fn set_box<S:Into<String>>(&self,key:S,value:Box<dyn Any + Send + Sync + 'static>){
        let mut lock = self.extend.lock().unwrap();
        lock.insert(key.into(),value);
    }
    pub fn remove<V:Any>(&self,key:&str)->Option<V>{
        let mut lock = self.extend.lock().unwrap();
        let val = lock.get(key)?;
        let opt = val.downcast_ref::<V>();
        if opt.is_none() {
            return None
        }
        let val = lock.remove(key).unwrap();
        let box_val:Box<V> = val.downcast().unwrap();
        return Some(*box_val)
    }
    pub fn push_callback(mut self,function:impl FnOnce(Arc<Context>)+Send+Sync+'static)->Self{
        if self.over_callback.is_none(){
            self.over_callback = Some(Mutex::new(vec![]));
        }
        if let Some(ref lock) = self.over_callback{
            let mut lock = lock.lock().unwrap();
            lock.push(Box::new(function));
        }
        self
    }
    pub fn error_over(&self,err:impl Error){
        let err = format!("{}",err);
        self.set(END_RESULT_ERROR,err);
        //fixme cas
        self.meta.set_status(CtxStatus::ERROR);
    }
    pub fn end_over<V:Any + Send + Sync>(&self,val:Option<V>){
        if let Some(val) = val{
            self.set(END_NODE_CODE,val);
        }
        //fixme cas
        self.meta.set_status(CtxStatus::SUCCESS);
    }
    pub fn end_output<V:Any>(&self)->anyhow::Result<V>{
        let status = self.meta.status();
        return match status {
            CtxStatus::INIT | CtxStatus::RUNNING => {
                anyhow::anyhow!("context is not over").err()
            }
            CtxStatus::SUCCESS => {
                if let Some(s) = self.remove::<V>(END_NODE_CODE) {
                    Ok(s)
                } else {
                    anyhow::anyhow!("end output type abnormal").err()
                }
            }
            CtxStatus::ERROR => {
                let err: String = self.remove(END_RESULT_ERROR).unwrap_or("nil error".to_string());
                anyhow::Error::msg(err).err()
            }
        }
    }
}

impl Flow{
    pub fn new(node:Node,ctx:Arc<Context>,middle:VecDeque<Arc<dyn Service>>)->Self{
        let Node{ code, node_type_id, node_config,.. } = node;
        Self{ctx,code,node_type_id,node_config,middle}
    }
    pub async fn call(mut self) -> anyhow::Result<Output>{
        let opt = self.middle.pop_front();
        let n = match opt {
            None => return RTError::FlowLastNodeNil.anyhow(),
            Some(n) => n,
        };
        n.call(self).await
    }
}

impl Meta{
    pub fn set_status(&self,status:CtxStatus){
        //fixme cas
        self.status.store(status.into(),Ordering::Relaxed)
    }
    pub fn status(&self)->CtxStatus{
        let status = self.status.load(Ordering::Relaxed);
        status.into()
    }
    pub fn push_stack_info<T:Into<String>,P:Into<String>,C:Into<String>,>(&self,parent_ctx_code:T,prev:P,next:C){
        let mut lock = self.stack.lock().unwrap();
        lock.round += 1;
        let round = lock.round;
        lock.stack.push((round,parent_ctx_code.into(),prev.into(),next.into()));
    }
}

// impl ContextStack {
//     pub fn push_stack_info<T:Into<String>,P:Into<String>,C:Into<String>>(&mut self,parent_ctx_code:T,prev:P,next:C){
//         self.round+=1;
//         self.stack.push((lock.round,parent_ctx_code.into(),prev.into(),next.into()));
//     }
// }