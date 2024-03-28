use crate::consts::{callback_self, MULTI_AGENT_RECALL_TOOLS};
use crate::infra::{embedding_small_1536, top_n};
use crate::single_agent::SingleAgentNode;
use crate::tool::AgentTool;
use rt::{Context, Node, Runtime, TaskInput, TaskOutput};
use std::sync::Arc;
use wd_tools::PFOk;

#[derive(Debug)]
pub enum RecallMod {
    First,
    #[allow(dead_code)]
    Specific(String),
    Embedding(usize),
}

impl Default for RecallMod {
    fn default() -> Self {
        RecallMod::First
    }
}

// 先找到能够解决问题的agent
// 投入问题 得到答案
#[derive(Debug, Default)]
pub struct MultiAgent {
    agents: Vec<(String, String)>, //id description
    agent_tools: Vec<AgentTool>,
    agent_vec: Vec<Vec<f32>>,

    id: String,
    // 召回前几个
    debug: bool,
    recall_mod: RecallMod,
}

impl MultiAgent {
    pub fn new<S: Into<String>>(id: S) -> Self {
        let id = id.into();
        Self {
            id,
            ..Default::default()
        }
    }
    pub fn debug_mod(mut self) -> Self {
        self.debug = true;
        self
    }
    // fixme 实际召回比较准的方式应该是根据 query+pe+portrait+context 进行召回，或者走个小模型
    pub async fn agent_recall(&self, query: &str) -> anyhow::Result<(String, Vec<AgentTool>)> {
        match self.recall_mod {
            RecallMod::First => {
                let tools = self
                    .agent_tools
                    .iter()
                    .map(|t| t.clone())
                    .collect::<Vec<AgentTool>>();
                return (self.agents[0].0.clone(), tools).ok();
            }
            RecallMod::Specific(ref id) => {
                let tools = self
                    .agent_tools
                    .iter()
                    .map(|t| t.clone())
                    .collect::<Vec<AgentTool>>();
                return (id.clone(), tools).ok();
            }
            RecallMod::Embedding(ref n) => {
                let query_vec = embedding_small_1536(vec![query]).await?;
                let list = top_n(&query_vec[0], &self.agent_vec, *n);
                let mut tools = vec![];
                for i in list {
                    if let Some(i) = self.agent_tools.get(i) {
                        tools.push(i.clone());
                    }
                }
                return (tools[0].get_agent_id().to_string(), tools).ok();
            }
        }
    }
    pub fn add_tools_to_context(ctx: Arc<Context>, tools: Vec<AgentTool>) {
        ctx.set(MULTI_AGENT_RECALL_TOOLS, tools);
    }
    pub fn register_agent<S: Into<String>>(&mut self, agent: &SingleAgentNode, desc: S) {
        let desc = desc.into();
        self.agents.push((agent.id(), desc.clone()));
        self.agent_tools.push(AgentTool::new(agent.id(), desc));
    }
    pub fn add_self_to_rt(&self, rt: &mut Runtime) {
        for tool in self.agent_tools.iter() {
            rt.upsert_node(tool.id(), tool.clone());
        }
    }
    #[allow(dead_code)]
    pub fn recall_specific_mod(&mut self, agent_id: &str) {
        self.recall_mod = RecallMod::Specific(agent_id.to_string());
    }
    pub async fn enable_embedding_recall_mod(&mut self, top_n: usize) -> anyhow::Result<()> {
        let mut query = Vec::with_capacity(self.agents.len());
        for (_, i) in self.agents.iter() {
            query.push(i.as_str());
        }
        let vecs = embedding_small_1536(query).await?;
        self.agent_vec = vecs;
        self.recall_mod = RecallMod::Embedding(top_n);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Node for MultiAgent {
    fn id(&self) -> String {
        self.id.clone()
    }
    async fn go(&self, ctx: Arc<Context>, mut args: TaskInput) -> anyhow::Result<TaskOutput> {
        if self.debug && ctx.get_round() > 1 {
            let answer = args.get_value::<String>().unwrap();
            let node_info = ctx.get_flow_stack().pop().unwrap();
            let (_, node, _) = Context::flow_key_analyze(node_info.as_str());
            println!("{}:->{}", node, answer);
            return TaskOutput::from_value(answer).over().ok();
        }

        let query = args.get_value::<String>().unwrap();
        let (agent, tools) = self.agent_recall(query.as_str()).await?;
        Self::add_tools_to_context(ctx.clone(), tools);

        if self.debug {
            callback_self(ctx, self.id(), agent, query)
        } else {
            TaskOutput::new(agent, query).ok()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::llm::LLMNode;
    use crate::memory::SimpleMemory;
    use crate::multi_agent::MultiAgent;
    use crate::single_agent::SingleAgentNode;
    use crate::tool::ToolNode;
    use rt::{Node, Runtime};
    use std::io::{BufRead, Write};
    use wd_tools::PFArc;
    use crate::short_long_memory::{ShortLongMemory, ShortLongMemoryMap};

    // cargo test multi_agent::test::test_multi_agent -- --nocapture
    #[tokio::test]
    async fn test_multi_agent() {
        let weather_tool = ToolNode::mock_get_current_weather();
        let taobao_tool = ToolNode::mock_taobao();
        // let memory = SimpleMemory::default().arc();
        let memory = ShortLongMemoryMap::default().arc();
        let llm_35: LLMNode = LLMNode::default();

        let info_agent = SingleAgentNode::default()
            .set_id("info_AI")
            .set_prompt("你是一个信息查询助手。回答要严谨，简洁。")
            .add_tool(weather_tool.as_openai_tool())
            .set_memory(memory.clone());

        let life_agent = SingleAgentNode::default()
            .set_id("life_AI")
            .set_prompt("你是一个生活服务管家。回答要踏实，简洁。")
            .add_tool(taobao_tool.as_openai_tool())
            .set_memory(memory);

        let mut multi_agent = MultiAgent::new("unite_brain").debug_mod();
        multi_agent.register_agent(&info_agent, "查询新闻，天气，搜索");
        multi_agent.register_agent(&life_agent, "购物，外卖，旅游");
        // multi_agent.recall_specific_mod("life_AI");
        multi_agent.enable_embedding_recall_mod(3).await.unwrap();

        let mut rt = Runtime::new();
        rt.upsert_node(weather_tool.id(), weather_tool);
        rt.upsert_node(taobao_tool.id(), taobao_tool);
        rt.upsert_node(llm_35.id(), llm_35);
        rt.upsert_node(info_agent.id(), info_agent);
        rt.upsert_node(life_agent.id(), life_agent);
        multi_agent.add_self_to_rt(&mut rt);
        rt.upsert_node(multi_agent.id(), multi_agent);
        rt.launch();

        let stdin = std::io::stdin().lock();
        let mut stdin = stdin.lines();
        print!("User  :-->");
        std::io::stdout().flush().unwrap();
        while let Some(Ok(query)) = stdin.next() {
            let answer: anyhow::Result<String> =
                rt.run("test_multi_agent", "unite_brain", query).await;
            if let Err(ref e) = answer {
                println!("error--->{}", e);
                return;
            }
            // println!("AI    :-->{}", answer.unwrap());
            print!("User  :-->");
            std::io::stdout().flush().unwrap();
        }
    }
}
