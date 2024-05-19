use std::sync::Arc;
use rt::{Context, ServiceLayer};
use crate::LLMNodeTools;

#[async_trait::async_trait]
pub trait ToolLoader:Send +Sync{
    async fn load(&self,name:String)->Box<dyn Tool>;
}
#[async_trait::async_trait]
pub trait Tool{
    async fn call(&self,args:String)-> anyhow::Result<String>;
}

#[derive(Debug)]
pub struct ToolService{
    loader:Box<dyn ToolLoader>
}

#[async_trait::async_trait]
impl ServiceLayer for ToolService {
    type Config = LLMNodeTools;
    type Output = ();

    async fn call(&self, code: String, ctx: Arc<Context>, cfg: Self::Config) -> anyhow::Result<Self::Output> {
        todo!()
    }
}