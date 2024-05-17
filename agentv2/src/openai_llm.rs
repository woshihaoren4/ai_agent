use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionResponseMessage, ChatCompletionTool, CreateChatCompletionRequest, CreateChatCompletionRequestArgs};
use serde::{Deserialize, Serialize};
use wd_tools::PFErr;
use rt::{Flow, Output};

#[derive(Debug)]
pub struct LLM{
    openai_client : Client<OpenAIConfig>,
}


#[derive(Debug, Default, Clone, Deserialize)]
pub struct LLMNodeRequest {
    #[serde(default = "String::default")]
    pub prompt: String,
    #[serde(default = "String::default")]
    pub model: String,
    #[serde(default = "Vec::default")]
    pub tools: Vec<ChatCompletionTool>,
    #[serde(default = "Vec::default")]
    pub context: Vec<ChatCompletionRequestMessage>,

    #[serde(default = "LLMNodeRequest::max_tokens_length")]
    pub max_tokens: u16,
    #[serde(default = "LLMNodeRequest::default_temperature")]
    pub temperature: f32,

    pub query: String,
}
impl LLMNodeRequest{
    pub fn to_openai_chat_request(self)->anyhow::Result<CreateChatCompletionRequest>{
        let LLMNodeRequest {
            prompt,
            model,
            tools,
            mut context,
            query,
            max_tokens,
            temperature,
            ..
        } = self;

        if !prompt.is_empty() {
            if let Some(p) = context.get_mut(0) {
                if let ChatCompletionRequestMessage::System(_pe) = p {
                    // pe.content = {
                    //     prompt.push_str("\r\n");
                    //     prompt.push_str(pe.content.as_str());
                    //     prompt
                    // }
                } else {
                    context.insert(
                        0,
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(prompt)
                            .build()
                            .unwrap()
                            .into(),
                    )
                }
            } else {
                context.insert(
                    0,
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(prompt)
                        .build()
                        .unwrap()
                        .into(),
                )
            }
        }

        if !query.is_empty() {
            context.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(query)
                    .build()
                    .unwrap()
                    .into(),
            );
        }

        let mut req = CreateChatCompletionRequestArgs::default();

        req.max_tokens(max_tokens);
        req.temperature(temperature);
        req.model(model);
        req.messages(context);
        if !tools.is_empty() {
            req.tools(tools);
        }
        let req = req.build()?;
        Ok(req)
    }
}
impl Default for LLM{
    fn default() -> Self {
        let openai_client = Client::new();
        Self{openai_client}
    }
}
impl LLM{
    pub fn check(req:&LLMNodeRequest)->anyhow::Result<()>{
        if req.model.is_empty() {
            return anyhow::anyhow!("module can not is nil").err()
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl rt::Service for LLM{
    async fn call(&self, flow: Flow) -> anyhow::Result<Output> {
        let cfg:LLMNodeRequest = serde_json::from_str(flow.node_config.as_str())?;
        let req = cfg.to_openai_chat_request()?;
        let resp = self.openai_client.chat().create(req).await?;
        let msg_list = resp.choices.into_iter().map(|x| x.message).collect::<Vec<ChatCompletionResponseMessage>>();
        return Ok(Output::new(msg_list))
    }
}



