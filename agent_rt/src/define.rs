use crate::{Context, END_NODE_NAME, START_NODE_NAME};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use serde_json::Value;

pub struct ContextImpl {}
pub struct Output {
    inner: Box<dyn Any + Send + Sync + 'static>,
}
impl Output {
    pub fn new<T: Any + Send + Sync + 'static>(t: T) -> Output {
        let inner = Box::new(t);
        Self { inner }
    }
}

impl Output {
    pub fn into_box(self) -> Box<dyn Any + Send + Sync + 'static> {
        self.inner
    }
}

#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub service_name: String,
    pub config: Value,

    pub(crate) middle_index: usize,
    pub(crate) service: Arc<dyn Service + Sync + 'static>,
}
impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self)
    }
}
impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"agent_rt::Node[name:{},service_name:{}]",self.name,self.service_name)
    }
}
impl PartialEq for Node{
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.service_name == other.service_name
    }
}
impl Node {
    pub fn new<N: Into<String>>(name: N) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            service_name: name,
            config: Value::Null,
            middle_index: 0,
            service: Arc::new(()),
        }
    }
    pub fn set_service_name<S: Into<String>>(mut self, name: S) -> Self {
        self.service_name = name.into();
        self
    }
    pub fn set_service_are(mut self, service: Arc<dyn Service + Sync + 'static>) -> Self {
        self.service = service;
        self
    }
    pub fn set_value(mut self,val:Value)->Self{
        self.config = val;self
    }
}

#[derive(Clone,Debug,PartialEq)]
pub enum PlanResult {
    Nodes(Vec<Node>),
    End,
    Wait,
}

pub trait Plan: Send {
    fn string(&self) -> String {
        "".into()
    }
    fn start_node_name(&self) -> &str {
        START_NODE_NAME
    }
    fn end_node_name(&self) -> &str {
        END_NODE_NAME
    }
    fn get(&mut self,name:&str)->Option<&Node>;
    fn next(&mut self, name: &str) -> anyhow::Result<PlanResult>;
    fn remove(&mut self, name: &str) -> Option<Node>;
    fn insert(&mut self, node: Node);
}

#[async_trait::async_trait]
pub trait Service: Send {
    async fn call(&self, ctx: Context, node: Node) -> anyhow::Result<Output>;
}

#[async_trait::async_trait]
pub trait ServiceLoader: Send {
    async fn load(&self, name: &str) -> Option<Arc<dyn Service + Sync + 'static>>;
}

#[async_trait::async_trait]
pub trait ServiceMiddle: Send {
    // true: Continue to execute
    // false: Skip the middle
    fn filter(&self, _node: &Node) -> bool {
        true
    }
    async fn call(&self, ctx: Context, node: Node) -> anyhow::Result<Output>;
}

#[async_trait::async_trait]
pub trait TaskFlowHook: Send {
    async fn call(&self, ctx: Context) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait ProgramPool: Send {
    //不能阻塞执行
    async fn push(
        &self,
        fut: Pin<Box<dyn Future<Output = anyhow::Result<Output>> + Send>>,
    ) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct ServiceLoaderImpl {
    map: HashMap<String, Arc<dyn Service + Sync + 'static>>,
}
impl ServiceLoaderImpl {
    pub fn register<K: Into<String>, S: Service + Sync + 'static>(mut self, key: K, s: S) -> Self {
        self.map.insert(key.into(), Arc::new(s));
        self
    }
    pub fn register_fn<K, F, Fut>(mut self, key: K, s: F) -> Self
    where
        K: Into<String>,
        F: Fn(Context, Node) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<Output>> + Send,
    {
        self.map.insert(key.into(), Arc::new(ServiceFn::new(s)));
        self
    }
}
#[async_trait::async_trait]
impl ServiceLoader for ServiceLoaderImpl {
    async fn load(&self, name: &str) -> Option<Arc<dyn Service + Sync + 'static>> {
        self.map.get(name).map(|x| x.clone())
    }
}
#[derive(Debug)]
pub struct ServiceFn<F> {
    function: F,
}
impl<T> ServiceFn<T> {
    pub fn new(function: T) -> ServiceFn<T> {
        Self { function }
    }
}

#[async_trait::async_trait]
impl<F, Fut> Service for ServiceFn<F>
where
    F: Fn(Context, Node) -> Fut + Send + Sync,
    Fut: Future<Output = anyhow::Result<Output>> + Send,
{
    async fn call(&self, ctx: Context, node: Node) -> anyhow::Result<Output> {
        (self.function)(ctx, node).await
    }
}

#[async_trait::async_trait]
impl Service for () {
    async fn call(&self, _ctx: Context, _node: Node) -> anyhow::Result<Output> {
        Ok(Output::new(()))
    }
}

#[async_trait::async_trait]
impl<F, Fut> ServiceMiddle for ServiceFn<F>
where
    F: Fn(Context, Node) -> Fut + Send + Sync,
    Fut: Future<Output = anyhow::Result<Output>> + Send,
{
    async fn call(&self, ctx: Context, node: Node) -> anyhow::Result<Output> {
        (self.function)(ctx, node).await
    }
}

#[async_trait::async_trait]
impl<F, Fut> TaskFlowHook for ServiceFn<F>
where
    F: Fn(Context) -> Fut + Send + Sync,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    async fn call(&self, ctx: Context) -> anyhow::Result<()> {
        (self.function)(ctx).await
    }
}

pub struct ProgramPoolImpl;

#[async_trait::async_trait]
impl ProgramPool for ProgramPoolImpl {
    async fn push(
        &self,
        fut: Pin<Box<dyn Future<Output = anyhow::Result<Output>> + Send>>,
    ) -> anyhow::Result<()> {
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                wd_log::log_field("error", e)
                    .field("position", "ProgramPoolImpl.spawn")
                    .warn("Unhandled error")
            }
        });
        Ok(())
    }
}
