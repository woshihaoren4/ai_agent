use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use serde_json::Value;

mod runtime;
mod rwmap_node_loader;
mod default_callback_set;

#[derive(Debug,Default)]
pub struct TaskInput{
    args:HashMap<String,Value>
}
impl TaskInput{
    pub fn set_value(&mut self,val:Value){
        self.args.insert("default".into(),val);
    }
    pub fn get_value(&mut self)->Value{
        let mut key = String::new();
        for (k,_) in self.args.iter(){
            key  = k.to_string();
        }
        if let Some(s) = self.args.remove(key.as_str()){
            return s
        }
        return Value::Null
    }

    fn append(&mut self,ti:TaskInput){
        self.args.extend(ti.args);
    }
    pub fn new(id:String,val:Value)->Self{
        let args = HashMap::from([(id,val)]);
        Self{args}
    }
}
#[derive(Debug)]
pub struct TaskOutput{
    pub over: bool,
    pub result:HashMap<String,Value>,

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
    pub fn value(val:Value)->Self{
        Self::new("".into(),val)
    }
    pub fn new(next_id:String,val:Value)->Self{
        let mut op = Self::default();
        op.result.insert(next_id,val);
        op
    }
    pub fn over(mut self)->Self{
        self.over = true;self
    }
    pub fn get_value(&mut self)->Value{
        let mut key = String::new();
        for (k,_) in self.result.iter(){
            key  = k.to_string();
        }
        if let Some(s) = self.result.remove(key.as_str()){
            return s
        }
        return Value::Null
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
    map:Mutex<HashMap<String,Value>>,
}
impl Context{
    pub fn new<S:Into<String>>(code:S)->Self{
        let mut ctx = Context::default();
        ctx.code = code.into();
        ctx
    }
    pub fn set<S:Into<String>>(&self,key:S,val:Value){
        let mut lock = self.map.lock().unwrap();
        lock.insert(key.into(),val);
    }
    pub fn get<O,F:FnOnce(Option<&Value>)->O>(&self,key:&str,function:F)->O{
        let lock = self.map.lock().unwrap();
        let opt = lock.get(key);
        function(opt)
    }
    pub fn remove(&self,key:&str)->Option<Value>{
        let mut lock = self.map.lock().unwrap();
        lock.remove(key)
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
    pub fn new(ctx:Arc<Context>,prev:String,next:String,input:Value)->Self{
        let input = TaskInput::new(prev,input);
        Self{
            ctx,
            input,
            node_id:next
        }
    }
}

#[async_trait::async_trait]
pub trait Node:Send+Sync {
    fn id(&self)->String;
    fn ready(&self,ctx:Arc<Context>,args:&TaskInput)->bool;
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
                Ok(TaskOutput::new("".into(),Value::String("success".into())).over())
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
            assert_eq!(Value::String("success".into()),output.get_value());
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
