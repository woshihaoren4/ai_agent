use crate::consts::{callback_self, go_next_or_over, AGENT_EXEC_STATUS, AGENT_TOOL_WRAPPER, MULTI_AGENT_RECALL_TOOLS, user_id_from_ctx};
use crate::llm::LLMNodeRequest;
use crate::tool::AgentTool;
use crate::{Language, Memory, prompt_from_ctx, prompt_to_ctx, PromptBuilder, PromptCommonTemplate};
use async_openai::types::{
    ChatChoice, ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs, ChatCompletionTool,
    CreateChatCompletionResponse, FinishReason,
};
use rt::{Context, Node, TaskInput, TaskOutput};
use std::sync::Arc;
use wd_tools::{PFArc, PFErr, PFOk};
use crate::short_long_memory::{ ShortLongMemoryMap};

#[derive(Clone)]
pub struct SingleAgentNode {
    prompt: Arc<dyn PromptBuilder>,
    tools: Vec<ChatCompletionTool>,
    // memory: Arc<dyn EasyMemory>,
    memory:Arc<dyn Memory>,

    id: String,
    max_context_window: usize,
    llm_model: String,
}
impl Default for SingleAgentNode {
    fn default() -> Self {
        let prompt = PromptCommonTemplate::default().arc();
        let tools = vec![];
        // let memory = Arc::new(SimpleMemory::default());
        let memory = Arc::new(ShortLongMemoryMap::default());
        let id = "single_agent".into();
        let max_context_window = 3;
        let llm_model = "gpt-3.5-turbo".into();
        Self {
            prompt,
            tools,
            memory,
            id,
            max_context_window,
            llm_model,
        }
    }
}

impl SingleAgentNode {
    pub fn set_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }
    // pub fn set_prompt<S: Into<String>>(mut self, pp: S) -> Self {
    //     self.prompt = pp.into();
    //     self
    // }
    pub fn set_prompt<S: PromptBuilder+'static>(mut self, pb: S) -> Self {
        self.prompt = Arc::new(pb);
        self
    }
    pub fn add_tool(mut self, tool: ChatCompletionTool) -> Self {
        self.tools.push(tool);
        self
    }
    pub fn set_llm<S:Into<String>>(mut self, llm: S) -> Self {
        self.llm_model = llm.into();
        self
    }
    // pub fn set_memory(mut self, memory: Arc<dyn EasyMemory>) -> Self {
    //     self.memory = memory;
    //     self
    // }
    pub fn set_memory(mut self, memory: Arc<dyn Memory>) -> Self {
        self.memory = memory;
        self
    }
    fn add_msg_to_context(ctx: Arc<Context>, msg: ChatCompletionRequestMessage) {
        let _: () = ctx.get(
            "session_context",
            move |x: Option<&mut Vec<ChatCompletionRequestMessage>>| {
                if let Some(list) = x {
                    list.push(msg);
                }
            },
        );
    }
    async fn exec_llm(&self, ctx: Arc<Context>) -> anyhow::Result<TaskOutput> {
        let mut context = ctx.get(
            "session_context",
            move |x: Option<&mut Vec<ChatCompletionRequestMessage>>| {
                if let Some(list) = x {
                    list.clone()
                } else {
                    vec![]
                }
            },
        );

        let mut req = LLMNodeRequest::default();
        req.prompt = prompt_from_ctx(&*ctx);
        // req.context = self.memory.load_context(self.max_context_window)?;
        req.context = self.memory.load_context(user_id_from_ctx(&*ctx).as_str(),self.max_context_window).await?;
        req.context.append(&mut context);
        req.tools.append(&mut self.tools.clone());
        let self_id = self.id();
        let req = ctx.get(
            MULTI_AGENT_RECALL_TOOLS,
            |tools: Option<&mut Vec<AgentTool>>| {
                if let Some(ts) = tools {
                    for i in ts.iter() {
                        if i.get_agent_id() != self_id {
                            req.tools.push(i.as_openai_tool())
                        }
                    }
                }
                return req;
            },
        );

        ctx.set(AGENT_EXEC_STATUS, 3usize);
        callback_self(ctx, self.id.clone(), self.llm_model.clone(), req)
    }
    async fn over(&self, ctx: Arc<Context>, msg: String) -> anyhow::Result<TaskOutput> {
        let user_question = ctx.get(
            "session_context",
            move |x: Option<&mut Vec<ChatCompletionRequestMessage>>| x.unwrap().remove(0),
        );
        let ai_response: ChatCompletionRequestMessage =
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(msg.clone())
                .build()
                .unwrap()
                .into();
        // self.memory
        //     .add_session_log(vec![user_question, ai_response]);

        self.memory.add_session_log(user_id_from_ctx(&*ctx).as_str(),vec![user_question, ai_response]).await;

        go_next_or_over(ctx, msg)
        // TaskOutput::from_value(msg).over().ok()
    }
    fn function_call(
        &self,
        ctx: Arc<Context>,
        tool: ChatCompletionMessageToolCall,
    ) -> anyhow::Result<TaskOutput> {
        // agent tool 不需要回来了
        if tool.function.name.starts_with(AGENT_TOOL_WRAPPER) {
            println!("to agent:>{}", tool.function.name);
            return TaskOutput::new(tool.function.name, 1usize).ok();
        }

        let tool_info = serde_json::to_string(&tool.function)?;
        println!("call  :-->{}", tool_info);
        let tool_calls = vec![tool.clone()];
        let _: () = ctx.get(
            "session_context",
            move |x: Option<&mut Vec<ChatCompletionRequestMessage>>| {
                let msg: ChatCompletionRequestMessage =
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .tool_calls(tool_calls)
                        // .content(tool_info)
                        .build()
                        .unwrap()
                        .into();
                if let Some(s) = x {
                    s.push(msg)
                };
            },
        );
        let name = tool.function.name.clone();
        ctx.set(AGENT_EXEC_STATUS, 2usize);
        callback_self(ctx, self.id.clone(), name, tool)
    }
}

#[async_trait::async_trait]
impl Node for SingleAgentNode {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn go(&self, ctx: Arc<Context>, mut args: TaskInput) -> anyhow::Result<TaskOutput> {
        let mut status = ctx.remove::<usize>(AGENT_EXEC_STATUS);
        if status.is_none() {
            status = Some(1);
        }
        match status.unwrap() {
            //用户发问
            1 => {
                if ctx.get(
                    "session_context",
                    |x: Option<&mut Vec<ChatCompletionRequestMessage>>| x.is_none(),
                ) {
                    ctx.set(
                        "session_context",
                        Vec::<ChatCompletionRequestMessage>::new(),
                    );
                }

                let query = args.get_value::<String>();
                if let Some(q) = query {
                    if prompt_from_ctx(&*ctx).is_empty() {
                        prompt_to_ctx(&*ctx,self.prompt.build(user_id_from_ctx(&*ctx).as_str(),q.as_ref(),Language::Chinese).await);
                    }
                    let req: ChatCompletionRequestMessage =
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(q)
                            .build()
                            .unwrap()
                            .into();
                    Self::add_msg_to_context(ctx.clone(), req);
                }

                return self.exec_llm(ctx).await;
            }
            //工具执行结果
            2 => {
                let resp = args.get_value::<ChatCompletionRequestMessage>().unwrap();
                println!("tool  :-->{:?}", resp);
                Self::add_msg_to_context(ctx.clone(), resp);
                return self.exec_llm(ctx).await;
            }
            //模型回复
            3 => {
                let mut resp: CreateChatCompletionResponse = args.get_value().unwrap();
                let ChatChoice {
                    message,
                    finish_reason,
                    ..
                } = resp.choices.remove(0);
                match finish_reason.unwrap() {
                    FinishReason::Stop => {
                        return self.over(ctx, message.content.unwrap_or("无语".to_string())).await
                    }
                    FinishReason::ToolCalls => {
                        return self.function_call(ctx, message.tool_calls.unwrap().remove(0))
                    }
                    _ => return anyhow::anyhow!("unknown finish_reason").err(),
                }
            }
            //错误
            _ => return anyhow::anyhow!("single agent unknown status").err(),
        }
        // Err(anyhow::anyhow!(""))
    }
}

#[cfg(test)]
mod test {
    use crate::llm::LLMNode;
    use crate::single_agent::SingleAgentNode;
    use crate::tool::ToolNode;
    use rt::{Node, Runtime};
    use std::io::{BufRead, Write};
    use crate::PromptCommonTemplate;

    // cargo test single_agent::test::test_single_agent -- --nocapture
    #[tokio::test]
    async fn test_single_agent() {
        let weather_tool = ToolNode::mock_get_current_weather();
        let taobao_tool = ToolNode::mock_taobao();
        let llm_35: LLMNode = LLMNode::default();
        let pp = "你是一个智能助手，回答要幽默风趣，尽量简洁。";
        let agent = SingleAgentNode::default()
            .set_prompt::<PromptCommonTemplate>(PromptCommonTemplate::default().role(pp).into())
            .add_tool(weather_tool.as_openai_tool())
            .add_tool(taobao_tool.as_openai_tool());

        let mut rt = Runtime::new();
        rt.upsert_node(weather_tool.id(), weather_tool);
        rt.upsert_node(taobao_tool.id(), taobao_tool);
        rt.upsert_node(llm_35.id(), llm_35);
        rt.upsert_node(agent.id(), agent);
        rt.launch();

        let stdin = std::io::stdin().lock();
        let mut stdin = stdin.lines();
        println!("Prompt:-->{}", pp);
        print!("User  :-->");
        std::io::stdout().flush().unwrap();
        while let Some(Ok(query)) = stdin.next() {
            let answer: anyhow::Result<String> =
                rt.run("test_single_agent", "single_agent", query).await;
            if let Err(ref e) = answer {
                println!("error--->{}", e);
                return;
            }
            println!("AI    :-->{}", answer.unwrap());
            print!("User  :-->");
            std::io::stdout().flush().unwrap();
        }
    }
}
