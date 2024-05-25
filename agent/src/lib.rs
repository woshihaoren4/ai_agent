use async_openai::types::ChatCompletionRequestMessage;
use std::collections::HashMap;

mod consts;
mod infra;
mod llm;
mod memory;
mod multi_agent;
pub mod pkg;
mod prompt;
pub mod short_long_memory;
mod single_agent;
mod text_to_image;
mod tool;

pub use consts::*;
pub use infra::*;
pub use llm::*;
pub use memory::*;
pub use multi_agent::*;
pub use pkg::*;
pub use prompt::*;
pub use short_long_memory::*;
pub use single_agent::*;
pub use text_to_image::*;
pub use tool::*;

pub trait EasyMemory: Send + Sync {
    fn load_context(&self, max: usize) -> anyhow::Result<Vec<ChatCompletionRequestMessage>>;
    fn recall_user_tag(&self) -> anyhow::Result<HashMap<String, String>>;
    fn add_session_log(&self, record: Vec<ChatCompletionRequestMessage>);
}

#[async_trait::async_trait]
pub trait Memory: Send + Sync {
    //加载上下文
    async fn load_context(
        &self,
        user: &str,
        max: usize,
    ) -> anyhow::Result<Vec<ChatCompletionRequestMessage>>;
    //追加会话日志，可以在上下文中获取到
    async fn add_session_log(&self, user: &str, record: Vec<ChatCompletionRequestMessage>);

    //拉取用户标签
    async fn get_user_tag(&self, user: &str, tag: &str) -> anyhow::Result<String>;
    //给用户贴标签
    async fn set_user_tage(&self, user: &str, kvs: HashMap<String, String>);

    //召回长期记忆
    async fn recall_summary(
        &self,
        user: &str,
        query: &str,
        n: usize,
    ) -> anyhow::Result<Vec<String>>;
    //将记忆进行总结
    async fn summary_history(&self, user: &str);
}

#[async_trait::async_trait]
pub trait PromptBuilder: Send + Sync {
    async fn build(&self, uid: &str, query: &str, lg: Language) -> String;
}

#[async_trait::async_trait]
impl PromptBuilder for String {
    async fn build(&self, _uid: &str, _query: &str, _lg: Language) -> String {
        self.clone()
    }
}

#[cfg(test)]
mod test {
    use async_openai::types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    };
    use async_openai::Client;

    #[tokio::test]
    async fn test_openai() {
        let chat_req = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo")
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("你是一个讲笑话助手")
                    .build()
                    .unwrap()
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content("讲个笑话")
                    .build()
                    .unwrap()
                    .into(),
            ])
            .build()
            .unwrap();

        let client = Client::new();
        let resp = client.chat().create(chat_req).await.unwrap();
        for i in resp.choices {
            println!(
                "[{}] --->{}:{:?}",
                i.index, i.message.role, i.message.content
            );
        }
    }
}
