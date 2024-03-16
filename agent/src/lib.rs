use async_openai::types::ChatCompletionRequestMessage;
use std::collections::HashMap;

mod consts;
mod infra;
mod llm;
mod memory;
mod multi_agent;
mod single_agent;
mod tool;

pub trait Memory: Send + Sync {
    fn load_context(&self, max: usize) -> anyhow::Result<Vec<ChatCompletionRequestMessage>>;
    fn recall_user_tag(&self) -> anyhow::Result<HashMap<String, String>>;
    fn add_session_log(&self, record: Vec<ChatCompletionRequestMessage>);
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
