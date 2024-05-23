use crate::plugin_tools::{ Tool};
use std::future::Future;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait ToolFunction: Send {
    async fn call(&self, args: String) -> anyhow::Result<String>;
}

#[async_trait::async_trait]
impl<T, Fut> ToolFunction for T
where
    T: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<String>> + Send,
{
    async fn call(&self, args: String) -> anyhow::Result<String> {
        self(args).await
    }
}

impl<T> From<T> for Tool
where
    T: ToolFunction + Sync + 'static,
{
    fn from(value: T) -> Self {
        Tool::Custom(Arc::new(value))
    }
}

// impl<T> From<T> for Plugin
//     where T:ToolFunction + Sync+'static
// {
//     fn from(value: T) -> Self {
//         Plugin::default()
//             .add_tool("",value)
//     }
// }
