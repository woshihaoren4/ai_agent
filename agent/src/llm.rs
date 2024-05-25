use crate::consts::go_next_or_over;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, CreateChatCompletionRequestArgs,
    CreateChatCompletionResponse,
};
use async_openai::Client;
use rt::{Context, TaskInput, TaskOutput};
use serde::Deserialize;
use std::sync::Arc;
use wd_tools::PFErr;

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

impl LLMNodeRequest {
    fn max_tokens_length() -> u16 {
        512u16
    }
    fn default_temperature() -> f32 {
        0.7f32
    }
    #[allow(dead_code)]
    pub fn from_query<S: Into<String>>(query: S) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct LLMNode {
    id: String,
    default_req: LLMNodeRequest,
    client: Client<OpenAIConfig>,
}
impl Default for LLMNode {
    fn default() -> Self {
        let id: String = "gpt-3.5-turbo".into();
        let mut default_req = LLMNodeRequest::default();
        default_req.max_tokens = LLMNodeRequest::max_tokens_length();
        default_req.temperature = LLMNodeRequest::default_temperature();
        default_req.model = id.clone();
        let client = Client::new();
        Self {
            default_req,
            id,
            client,
        }
    }
}

impl LLMNode {
    #[allow(dead_code)]
    pub fn new(id: String) -> Self {
        LLMNode {
            id,
            ..Default::default()
        }
    }
    #[allow(dead_code)]
    pub fn set_model<S: Into<String>>(mut self, model: S) -> Self {
        self.default_req.model = model.into();
        self
    }
    pub fn set_meta(mut self, req: LLMNodeRequest) -> Self {
        self.default_req = req;
        self
    }
    pub async fn chat(&self, req: LLMNodeRequest) -> anyhow::Result<CreateChatCompletionResponse> {
        let LLMNodeRequest {
            prompt,
            model,
            tools,
            mut context,
            query,
            max_tokens,
            temperature,
            ..
        } = req;

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

        let chat = self.client.chat();

        let resp = chat.create(req).await?;
        Ok(resp)
    }
}

#[async_trait::async_trait]
impl rt::Node for LLMNode {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn ready(&self, _ctx: Arc<Context>, args: &TaskInput) -> bool {
        args.args_len() > 0
    }

    async fn go(&self, ctx: Arc<Context>, mut args: TaskInput) -> anyhow::Result<TaskOutput> {
        let req = if let Some(query) = args.get_value::<String>() {
            let mut req = self.default_req.clone();
            req.query = query;
            req
        } else if let Some(mut req) = args.get_value::<LLMNodeRequest>() {
            // if req.query.is_empty() {
            //     return anyhow::anyhow!("llm:query is nil").err();
            // }
            if req.prompt.is_empty() {
                req.prompt = self.default_req.prompt.clone();
            }
            if req.model.is_empty() {
                req.model = self.default_req.model.clone();
            }
            if req.temperature < 0.1 {
                req.temperature = self.default_req.temperature;
            }
            if req.max_tokens == 0 {
                req.max_tokens = self.default_req.max_tokens
            }
            req.tools.append(&mut self.default_req.tools.clone());
            req
        } else {
            return anyhow::anyhow!("llm: task_input is unknown").err();
        };

        let resp = self.chat(req).await?;

        go_next_or_over(ctx, resp)
    }
}

#[cfg(test)]
mod test {
    use super::{LLMNode, LLMNodeRequest};
    use async_openai::types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionResponse,
    };
    use rt::Node;
    use rt::{Context, TaskInput, TaskOutput};
    use std::io::{BufRead, Write};
    use wd_tools::PFArc;

    // cargo test llm::test::test_llm_node -- --nocapture
    #[tokio::test]
    async fn test_llm_node() {
        let mut meta = LLMNodeRequest::default();
        meta.prompt = "你是一个智能助手，回答要幽默风趣，尽量简洁。".into();
        meta.model = "gpt-3.5-turbo".into();
        meta.max_tokens = 512;
        meta.temperature = 0.7f32;

        let llm: LLMNode = LLMNode::default().set_meta(meta);

        let mut ctx = vec![];

        let stdin = std::io::stdin().lock();
        let mut stdin = stdin.lines();

        println!("Prompt:-->{}", llm.default_req.prompt);
        print!("User  :-->");
        std::io::stdout().flush().unwrap();
        while let Some(Ok(query)) = stdin.next() {
            let message: ChatCompletionRequestMessage =
                ChatCompletionRequestUserMessageArgs::default()
                    .content(query)
                    .build()
                    .unwrap()
                    .into();
            ctx.push(message);

            let mut req = LLMNodeRequest::default();
            req.context = ctx.clone();

            let mut resp: TaskOutput = llm
                .go(Context::new("test").arc(), TaskInput::from_value(req))
                .await
                .unwrap();
            let resp = resp.get_value::<CreateChatCompletionResponse>().unwrap();

            let mut answer = String::new();
            for (i, e) in resp.choices.into_iter().enumerate() {
                if i != 0 {
                    answer.push_str(format!(" {}:", i).as_str());
                }
                answer.push_str(e.message.content.unwrap().as_str())
            }

            println!("AI    :-->{}", answer);

            ctx.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(answer)
                    .build()
                    .unwrap()
                    .into(),
            );

            print!("User  :-->");
            std::io::stdout().flush().unwrap();
        }
    }
}
