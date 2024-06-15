#![allow(deprecated)]
use crate::rt_node_service::{CfgBound, LLMToolCallRequest};
use agent_rt::Context;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCallChunk,
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestToolMessage, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
    ChatCompletionTool, ChatCompletionToolType, CreateChatCompletionRequest,
    CreateChatCompletionRequestArgs, FunctionCall, Role,
};
use async_openai::Client;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use wd_tools::PFErr;

#[derive(Debug)]
pub struct OpenaiLLMService {
    openai_client: Client<OpenAIConfig>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct LLMNodeRequest {
    #[serde(default = "String::default")]
    pub prompt: String,
    #[serde(default = "LLMNodeRequest::default_model_35")]
    pub model: String,
    #[serde(default = "Vec::default")]
    pub tools: Vec<ChatCompletionTool>,
    #[serde(default = "Vec::default")]
    pub context: Vec<LLMContextMessage>,

    #[serde(default = "LLMNodeRequest::max_tokens_length")]
    pub max_tokens: u16,
    #[serde(default = "LLMNodeRequest::default_temperature")]
    pub temperature: f32,
    #[serde(default = "bool::default")]
    pub is_stream: bool,

    pub query: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct LLMContextMessage {
    pub role: String,
    pub content: String,

    pub call_id: String,
    pub call_name: String,
    pub call_args: String,
}

impl LLMContextMessage {
    pub fn to_chat_message(self) -> Option<ChatCompletionRequestMessage> {
        let Self {
            role,
            content,
            call_id,
            call_name,
            call_args,
        } = self;
        let msg = match role.to_lowercase().as_str() {
            "system" => ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content,
                role: Role::System,
                name: None,
            }),
            "assistant" => {
                let content = if content.is_empty() {
                    None
                } else {
                    Some(content)
                };
                ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                    content,
                    role: Role::Assistant,
                    name: None,
                    tool_calls: Option::from(vec![ChatCompletionMessageToolCall {
                        id: call_id,
                        r#type: ChatCompletionToolType::Function,
                        function: FunctionCall {
                            name: call_name,
                            arguments: call_args,
                        },
                    }]),

                    function_call: None,
                })
            }
            "user" => ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(content),
                role: Role::User,
                name: None,
            }),
            "tool" => ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                role: Role::Tool,
                content,
                tool_call_id: call_id,
            }),
            "function" => return None,
            _ => return None,
        };
        Some(msg)
    }
}

impl LLMNodeRequest {
    fn max_tokens_length() -> u16 {
        1024
    }
    fn default_temperature() -> f32 {
        0.7f32
    }
    fn default_model_35() -> String {
        "gpt-3.5-turbo".into()
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct LLMNodeResponse {
    pub answer: Option<String>,
    pub tools: Option<Vec<LLMToolCallRequest>>,
}

impl LLMNodeResponse {
    pub fn append_answer(&mut self, msg: &str) {
        if self.answer.is_none() {
            self.answer = Some("".into())
        }
        if let Some(ref mut s) = self.answer {
            s.push_str(msg)
        }
    }
    pub fn append_tools(&mut self, tools: Vec<ChatCompletionMessageToolCallChunk>) {
        if tools.is_empty() {
            return;
        }
        if self.tools.is_none() {
            self.tools = Some(vec![])
        }
        for i in tools {
            if let Some(ref mut vec) = self.tools {
                let id = i.id;
                if let Some(fcs) = i.function {
                    if fcs.name.is_some() {
                        vec.push(LLMToolCallRequest {
                            call_id: id,
                            name: fcs.name.unwrap(),
                            args: fcs.arguments.unwrap_or("".into()),
                        })
                    } else {
                        if let Some(t) = vec.last_mut() {
                            t.args.push_str(fcs.arguments.unwrap_or("".into()).as_str());
                        }
                    }
                }
            }
        }
    }
}

impl LLMNodeRequest {
    pub fn to_openai_chat_request(self) -> anyhow::Result<CreateChatCompletionRequest> {
        let LLMNodeRequest {
            prompt,
            model,
            tools,
            context,
            query,
            max_tokens,
            temperature,
            ..
        } = self;

        let mut msg_list = vec![];
        for i in context {
            if let Some(s) = i.to_chat_message() {
                msg_list.push(s);
            }
        }
        let mut context = msg_list;

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
impl Default for OpenaiLLMService {
    fn default() -> Self {
        let openai_client = Client::new();
        Self { openai_client }
    }
}
impl OpenaiLLMService {
    pub fn check(req: &LLMNodeRequest) -> anyhow::Result<()> {
        if req.model.is_empty() {
            return anyhow::anyhow!("module can not is nil").err();
        }
        Ok(())
    }
    pub fn set_channel_to_ctx(ctx: &mut Context) {
        ctx.set("openai_llm_stream_channel", VecDeque::<String>::new())
    }
    pub fn try_send_to_channel(ctx: &Context, msg: String) -> Option<String> {
        ctx.get_opt(
            "openai_llm_stream_channel",
            move |x: Option<&mut VecDeque<String>>| match x {
                Some(x) => {
                    x.push_back(msg);
                    None
                }
                None => Some(msg),
            },
        )
    }
    pub fn try_recv_from_channel(ctx: &Context) -> Option<String> {
        ctx.get_opt(
            "openai_llm_stream_channel",
            move |x: Option<&mut VecDeque<String>>| match x {
                Some(x) => {
                    let mut result = String::new();
                    while let Some(s) = x.pop_front() {
                        result.push_str(s.as_str());
                    }
                    Some(result)
                }
                None => None,
            },
        )
    }
}

#[async_trait::async_trait]
impl agent_rt::ServiceLayer for OpenaiLLMService {
    type Config = CfgBound<LLMNodeRequest>;
    type Output = LLMNodeResponse;

    async fn call(
        &self,
        _code: String,
        ctx: Arc<Context>,
        cfg: Self::Config,
    ) -> anyhow::Result<Self::Output> {
        // wd_log::log_debug_ln!("start call code[{}.{}.openai_llm]",ctx.code,code);
        let cfg = cfg.bound(&ctx)?;
        let req = cfg.to_openai_chat_request()?;
        let mut stream = self.openai_client.chat().create_stream(req).await?;

        let mut resp = LLMNodeResponse::default();
        while let Some(msg) = stream.next().await {
            let msg = match msg {
                Ok(o) => o,
                Err(e) => return Err(anyhow::Error::from(e)),
            };
            for i in msg.choices {
                //文本消息
                if let Some(s) = i.delta.content {
                    if let Some(s) = Self::try_send_to_channel(&ctx, s) {
                        resp.append_answer(s.as_str());
                    }
                }
                //工具调用
                if let Some(tools) = i.delta.tool_calls {
                    resp.append_tools(tools);
                }
            }
        }
        // wd_log::log_debug_ln!("over call code[{}.{}.openai_llm]",ctx.code,code);
        Ok(resp)
    }
}

#[cfg(test)]
mod test {
    use crate::rt_node_service::{LLMNodeResponse, OpenaiLLMService};
    use agent_rt::{CtxStatus, PlanBuilder, Runtime};
    use serde_json::Value;
    use std::io::{BufRead, Write};
    use std::time::Duration;
    use wd_tools::PFArc;

    //cargo test openai_llm::test::test_llm_node_chat -- --nocapture
    #[tokio::test]
    async fn test_llm_node_chat() {
        let rt = Runtime::default()
            .register_service_layer("openai_llm", OpenaiLLMService::default())
            .launch();
        let stdin = std::io::stdin().lock();
        let mut stdin = stdin.lines();

        print!("user --->");
        std::io::stdout().flush().unwrap();
        while let Some(Ok(query)) = stdin.next() {
            print!("ai   --->");
            std::io::stdout().flush().unwrap();

            let msg = format!("{{\"prompt\":\"你是一个智能助手\" ,\"query\":\"{query}\"}} ");
            let resp = rt
                .ctx(
                    "test001",
                    PlanBuilder::single_node("openai_llm", msg)
                        .check_and_build()
                        .unwrap(),
                )
                // .updates(OpenaiLLM::set_channel_to_ctx)
                .arc()
                .block_on::<Value, _>(())
                .await
                .unwrap();
            let resp = serde_json::from_value::<LLMNodeResponse>(resp).unwrap();

            print!("{}\nuser --->", resp.answer.unwrap_or("".into()));
            std::io::stdout().flush().unwrap();
        }
    }

    //cargo test openai_llm::test::test_llm_node_stream -- --nocapture
    #[tokio::test]
    async fn test_llm_node_stream() {
        let rt = Runtime::default()
            .register_service_layer("openai_llm", OpenaiLLMService::default())
            .launch();
        let stdin = std::io::stdin().lock();
        let mut stdin = stdin.lines();

        print!("user --->");
        std::io::stdout().flush().unwrap();
        while let Some(Ok(query)) = stdin.next() {
            print!("ai   --->");
            std::io::stdout().flush().unwrap();

            let msg = format!("{{\"prompt\":\"你是一个智能助手\" ,\"query\":\"{query}\"}} ");
            let ctx = rt
                .ctx(
                    "test001",
                    PlanBuilder::single_node("openai_llm", msg)
                        .check_and_build()
                        .unwrap(),
                )
                .updates(OpenaiLLMService::set_channel_to_ctx)
                .arc();
            ctx.clone().spawn(()).unwrap();

            loop {
                let status = ctx.status();
                let over = status == CtxStatus::SUCCESS || status == CtxStatus::ERROR;

                if let Some(s) = OpenaiLLMService::try_recv_from_channel(&ctx) {
                    print!("{}", s);
                    std::io::stdout().flush().unwrap();
                }

                if over {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            print!("\nuser --->");
            std::io::stdout().flush().unwrap();
        }
    }

    #[tokio::test]
    async fn test_llm_tools() {
        let rt = Runtime::default()
            .register_service_layer("openai_llm", OpenaiLLMService::default())
            .launch();

        let cfg = "{\"extend\":{},\"max_tokens\":512,\"model\":\"gpt-3.5-turbo\",\"prompt\":\"# 角色 #\\n你是一渣男，说话总是花言巧语。\\n\\n# 要求 #\\n对于喜欢你的人，你必须要送给她一个`LV包包`。\",\"query\":\"{{start.query}}\",\"temperature\":0.7,\"tools\":[{\"function\":{\"description\":\"在线送礼物\",\"name\":\"taobao_shop\",\"parameters\":{\"properties\":{\"gift\":{\"description\":\"礼物名称\",\"type\":\"string\"}},\"required\":[\"gift\"],\"type\":\"object\"}},\"type\":\"function\"}]}";

        let res = rt
            .ctx(
                "openai-tool-test-0001",
                PlanBuilder::single_node("openai_llm", cfg).build(),
            )
            .arc()
            .block_on::<Value, _>(serde_json::json!({
                "query":"我喜欢你"
            }))
            .await
            .unwrap();

        println!("{}", res);
    }
}
