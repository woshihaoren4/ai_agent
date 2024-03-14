use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::task::Waker;

mod runtime;
mod rwmap_node_loader;
mod default_callback_set;

pub use runtime::*;
pub use rwmap_node_loader::*;
pub use default_callback_set::*;

#[derive(Debug,Default)]
pub struct TaskInput{
    args:HashMap<String,Box<dyn Any+Send+Sync+'static>>
}
impl TaskInput{
    pub fn args_len(&self)->usize{
        self.args.len()
    }
    pub fn set_value<T:Any+Send+Sync+'static>(&mut self,val:T){
        self.args.insert("default".into(),Box::new(val));
    }
    pub fn get_value<T:Any>(&mut self)->Option<T>{
        let mut key = String::new();
        for (k,_) in self.args.iter(){
            key  = k.to_string();
        }
        if let Some(s) = self.args.get(key.as_str()){
            if s.downcast_ref::<T>().is_none(){
                return None
            }
        }else {
            return None
        }
        if let Some(s) = self.args.remove(key.as_str()){
            unsafe {
                let ptr = Box::into_raw(s) as *mut T;
                let t = Box::from_raw(ptr);
                return Some(*t)
            }
        }
        return None
    }
    pub fn ref_value<T:Any>(&self)->Option<&T>{
        let mut key = String::new();
        for (k,_) in self.args.iter(){
            key  = k.to_string();
        }
        if let Some(s) = self.args.get(key.as_str()){
            return s.downcast_ref()
        }
        return None
    }
    pub fn new<T:Any+Send+Sync+'static>(id:String,val:T)->Self{
        let mut args:HashMap<String,Box<dyn Any+Send+Sync+'static>> = HashMap::new();
        args.insert(id,Box::new(val));
        Self{args}
    }
    pub fn from_box_value(id:String,val:Box<dyn Any+Send+Sync+'static>)->Self{
        let mut args:HashMap<String,Box<dyn Any+Send+Sync+'static>> = HashMap::new();
        args.insert(id,val);
        Self{args}
    }
    pub fn from_value<T:Any+Send+Sync+'static>(val:T)->Self{
        Self::new("".into(),val)
    }


    fn append(&mut self,ti:TaskInput){
        self.args.extend(ti.args);
    }

}
#[derive(Debug)]
pub struct TaskOutput{
    pub over: bool,
    pub result:HashMap<String,Box<dyn Any+Send+Sync+'static>>,

    error: Option<anyhow::Error>,
    ctx : Arc<Context>,
}
impl Default for TaskOutput{
    fn default() -> Self {
        Self{
            over:false,
            error:None,
            result:HashMap::new(),
            ctx: Arc::new(Context::default()),
        }
    }
}
impl TaskOutput {
    pub fn from_value<T:Any+Send+Sync+'static>(val:T)->Self{
        Self::new("".into(),val)
    }
    pub fn new<T:Any+Send+Sync+'static>(next_id:String,val:T)->Self{
        let mut op = Self::default();
        op.result.insert(next_id,Box::new(val));
        op
    }
    pub fn over(mut self)->Self{
        self.over = true;self
    }
    pub fn get_value<T:Any>(&mut self)->Option<T>{
        let mut key = String::new();
        for (k,_) in self.result.iter(){
            key  = k.to_string();
        }
        if let Some(s) = self.result.get(key.as_str()){
            if s.downcast_ref::<T>().is_none(){
                return None
            }
        }else {
            return None
        }
        if let Some(s) = self.result.remove(key.as_str()){
            unsafe {
                let ptr = Box::into_raw(s) as *mut T;
                let t = Box::from_raw(ptr);
                return Some(*t)
            }
        }
        return None
    }
    pub fn get_context(&self)->Arc<Context>{
        self.ctx.clone()
    }
    fn into_output(self){
        let output = self.ctx.output.clone();
        let mut lock = output.lock().unwrap();
        (*lock.deref_mut()) = Some(self)
    }
    fn error(e:anyhow::Error)->Self{
        let mut to = TaskOutput::default();
        to.error = Some(e);
        to
    }
    fn set_ctx(&mut self,ctx:Arc<Context>){
        self.ctx = ctx;
    }
}

#[derive(Debug,Default)]
pub struct Context{
    // over: AtomicBool,
    round: usize,
    code: String, //任务流的唯一标识
    flow:Mutex<Vec<String>>,  //format: round:task_id->task_id
    output:Arc<Mutex<Option<TaskOutput>>>,
    map:Mutex<HashMap<String,Box<dyn Any+Send+Sync+'static>>>,
}
impl Context{
    pub fn new<S:Into<String>>(code:S)->Self{
        let mut ctx = Context::default();
        ctx.code = code.into();
        ctx
    }
    pub fn set<S:Into<String>,V:Any+Send+Sync+'static>(&self,key:S,val:V){
        let mut lock = self.map.lock().unwrap();
        lock.insert(key.into(),Box::new(val));
    }
    pub fn get<I:Any,O,F:FnOnce(Option<&mut I>)->O>(&self,key:&str,function:F)->O{
        let mut lock = self.map.lock().unwrap();
        let opt = lock.get_mut(key);
        if opt.is_none(){
            return function(None)
        }
        let input = opt.unwrap().downcast_mut::<I>();
        function(input)
    }
    pub fn remove<V:Any>(&self,key:&str)->Option<V>{
        let mut lock = self.map.lock().unwrap();
        let val = lock.remove(key)?;
        if let Some(_) = val.downcast_ref::<V>(){
            let ptr = Box::into_raw(val) as *mut V;
            unsafe {
                let t = Box::from_raw(ptr);
                return Some(*t)
            }
        }
        return None

    }
    pub fn get_round(&self)->usize{
        let _lock = self.flow.lock().unwrap();
        self.round
    }
    pub fn get_flow_stack(&self)->Vec<String>{
        let mut lock = self.flow.lock().unwrap();
        return lock.deref().clone()
    }
    pub fn flow_key_analyze(key:&str)->(String,String,String){
        let v: Vec<&str> = key.split(&[':', '-', '>'][..]).collect();
        if v.len() < 3{
            return (v[0].to_string(),String::new(),v[2].to_string())
        }
        return (v[0].to_string(),v[1].to_string(),v[2].to_string())
    }
    fn add_task_to_chain(&self,prev:&str,next:&str){
        let mut lock = self.flow.lock().unwrap();
        #[allow(invalid_reference_casting)]
        unsafe {
            *((&self.round) as *const usize as *mut usize) += 1;
        }
        lock.push(format!("{}:{}->{}",self.round,prev,next));
    }
    fn set_output(&mut self,output:Arc<Mutex<Option<TaskOutput>>>){
        self.output = output;
    }
}
#[derive(Debug,Default)]
pub struct Task{
    ctx:Arc<Context>,
    node_id:String,
    input:TaskInput,
}

impl Task {
    pub fn new(ctx:Arc<Context>,prev:String,next:String,input:Box<dyn Any+Send+Sync+'static>)->Self{
        let input = TaskInput::from_box_value(prev,input);
        Self{
            ctx,
            input,
            node_id:next
        }
    }
    pub fn set_input(mut self,input:TaskInput)->Self{
        self.input = input;self
    }
}

#[async_trait::async_trait]
pub trait Node:Send+Sync {
    fn id(&self)->String;
    // ready: 只能判断流程节点，不能用来做参数校验
    fn ready(&self,_ctx:Arc<Context>,_args:&TaskInput)->bool{
        true
    }
    async fn go(&self, ctx:Arc<Context>, args:TaskInput) ->anyhow::Result<TaskOutput>;
}
pub trait NodeLoader:Send+Sync{
    fn get(&self,ids:&str)->anyhow::Result<Arc<dyn Node>>;
    fn set(&self,nodes:Vec<(String,Arc<dyn Node>)>);
}
pub struct CallBack{
    waker:Waker,
}

impl CallBack {
    pub fn new(waker:Waker)->Self{
        CallBack{
            waker,
        }
    }
}

pub trait CallBackSet:Send+Sync{
    fn push(&self,code:String,waker:CallBack);
    fn remove(&self,code:&str)->Option<CallBack>;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use serde_json::Value;
    use crate::{Context, Node, TaskInput, TaskOutput};
    use crate::runtime::Runtime;

    struct NodeEntity(String, bool);
    #[async_trait::async_trait]
    impl Node for NodeEntity {
        fn id(&self) -> String {
            self.0.clone()
        }

        fn ready(&self, _ctx: Arc<Context>, _args: &TaskInput) -> bool {
            true
        }

        async fn go(&self, _ctx: Arc<Context>, _args: TaskInput) -> anyhow::Result<TaskOutput> {
            // println!("run --->{}",self.0);
            if self.1 {
                Ok(TaskOutput::from_value("success".to_string()).over())
            }else{
                Ok(TaskOutput::new("2".into(),Value::Null))
            }
        }
    }

    #[tokio::test]
    async fn test_runtime(){
        let mut rt = Runtime::new();
        rt.upsert_node("1".into(), NodeEntity("1".into(), false));
        rt.upsert_node("2".into(), NodeEntity("2".into(), true));
        rt.launch();

        let start_time = std::time::Instant::now();
        for _i in 0..10000{
            let mut ctx = Context::default();
            ctx.code = "1->2".into();
            let mut output = rt.raw_run(ctx, "1".into(), TaskInput::new("".into(), Value::Null)).await.unwrap();
            assert_eq!("success".to_string(),output.get_value::<String>().unwrap());
        }
        let ms = start_time.elapsed().as_millis();
        println!("user time :{}ms",ms);
    }
    #[tokio::test]
    async fn test_run(){
        let mut rt = Runtime::new();
        rt.upsert_node("1".into(), NodeEntity("1".into(), false));
        rt.upsert_node("2".into(), NodeEntity("2".into(), true));
        rt.launch();

        let result:String = rt.run("1->2", "1", "hello world").await.unwrap();
        assert_eq!(result.as_str(),"success")
    }
}
