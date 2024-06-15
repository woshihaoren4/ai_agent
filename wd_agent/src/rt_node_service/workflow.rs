use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wd_tools::{PFArc, PFErr};
use agent_rt::{Context, Plan};
use crate::rt_node_service::CfgBound;

#[async_trait::async_trait]
pub trait WorkflowLoader:Sync+Send{
    async fn load(&self,name:&str)->anyhow::Result<Box<dyn Plan>>;
}
pub struct WorkflowService{
    pub loader : Box<dyn WorkflowLoader+ 'static>
}

impl WorkflowService{
    pub fn new<I:WorkflowLoader+ 'static>(loader:I) ->Self{
        let loader = Box::new(loader);
        Self{loader}
    }
}

#[async_trait::async_trait]
impl agent_rt::ServiceLayer for WorkflowService{
    type Config = CfgBound<Value>;
    type Output = Value;

    async fn call(&self, _code: String, ctx: Arc<Context>, cfg: Self::Config) -> anyhow::Result<Self::Output> {
        let input = cfg.bound(&ctx)?;
        let name = if let Value::Object(ref map) = input{
            map.get("workflow_name").map(|x|{
                x.as_str().map(|x|x.to_string()).unwrap_or("".to_string())
            }).unwrap_or("".to_string())
        }else{
            return anyhow::anyhow!("WorkflowService.config must is object").err()
        };
        if name.is_empty() {
            return anyhow::anyhow!("WorkflowService: workflow_name is nil").err()
        }
        let plan = self.loader.load(name.as_str()).await?;
        let sub_task_code = format!("{}-{}",ctx.code,name);
        let output = ctx.sub_ctx(sub_task_code,plan)
            .arc()
            .block_on::<Value,_>(input)
            .await?;
        Ok(output)
    }
}

pub struct WorkflowLoaderFile{
    pub path:String
}
impl Default for WorkflowLoaderFile{
    fn default() -> Self {
        let path = "./workflow".into();
        Self{path}
    }
}

impl Default for WorkflowService {
    fn default() -> Self {
        WorkflowService::new(WorkflowLoaderFile::default())
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct WorkflowNodePlan{
    pub plan:Vec<WorkflowNode>,
}
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct WorkflowNode {
    pub code:String,
    pub service_type:String,
    pub cfg: String,
    pub ready_nodes: Vec<String>,
    pub goto_nodes: Vec<String>
}

#[async_trait::async_trait]
impl WorkflowLoader for WorkflowLoaderFile{
    async fn load(&self, name: &str) -> anyhow::Result<Box<dyn Plan>> {
        let path = format!("{}/{}",self.path,name);
        let data = tokio::fs::read(path).await?;

        let plan = serde_json::from_slice::<WorkflowNodePlan>(data.as_slice())?;

        let nodes = plan.plan.into_iter().map(|x|{
            let WorkflowNode{ code, service_type, cfg, ready_nodes, goto_nodes } = x;
            (ready_nodes,agent_rt::Node::new(code,service_type,cfg),goto_nodes)
        }).collect::<Vec<_>>();

        let plan = agent_rt::PlanBuilder::from(nodes).build();

        Ok(Box::new(plan) as Box<dyn Plan>)
    }
}

#[cfg(test)]
mod test{
    use serde_json::Value;
    use wd_tools::PFArc;
    use agent_rt::PlanBuilder;
    use crate::rt_node_service::{InjectorService, PythonCodeService, SelectorService};
    use crate::rt_node_service::workflow::WorkflowService;

    //cargo test rt_node_service::workflow::test::test_workflow -- --nocapture
    #[tokio::test]
    async fn test_workflow(){
        let openai_llm = crate::rt_node_service::OpenaiLLMService::default();
        let var = crate::rt_node_service::VarFlowChartService::default();
        let python = PythonCodeService::new("http://127.0.0.1:50001").await.unwrap();

        //build agent runtime
        let rt = agent_rt::Runtime::default()
            .register_service_layer("openai_llm", openai_llm)
            .register_service_layer("python",python)
            .register_service_layer("workflow",WorkflowService::default())
            .register_service_layer("flow_chart_selector",SelectorService::default())
            .register_service_layer("flow_chart_injector",InjectorService::default())
            .register_service_layer("flow_chart_var", var)
            .launch();

        let cfg = serde_json::json!({
            "workflow_name":"single_agent",
            "query":"{{start.query}}"
        });

        let output:Value = rt.ctx("test-wf-001",PlanBuilder::single_node("workflow",serde_json::to_string(&cfg).unwrap()).build())
            .arc()
            .block_on(serde_json::json!({
                "query":"我喜欢你"
            }))
            .await.unwrap();

        println!("{}",output);
    }
}