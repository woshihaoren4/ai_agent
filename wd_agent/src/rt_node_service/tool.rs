use crate::plugin_tools::PluginControlSchedule;
use agent_rt::{Context, ServiceLayer};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use crate::rt_node_service::in_out_bonding::CfgBound;

#[async_trait::async_trait]
pub trait ToolEvent: Send {
    async fn call(&self, name: &str, args: String) -> anyhow::Result<String>;
}

pub struct ToolService {
    loader: Box<dyn ToolEvent + Sync + 'static>,
}

impl<T: ToolEvent + Sync + 'static> From<T> for ToolService {
    fn from(value: T) -> Self {
        let loader = Box::new(value);
        Self { loader }
    }
}
// impl<T,O> From<T> for ToolService
//     where T:Into<O>,O:ToolEvent
// {
//     fn from(value: T) -> Self {
//         PluginControl::from(value.into()).into()
//     }
// }
impl Default for ToolService {
    fn default() -> Self {
        Self::from(PluginControlSchedule::default().to_tool_event())
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct LLMToolCallRequest {
    pub call_id: Option<String>,
    pub name: String,
    #[serde(default="String::default")]
    pub args:String,
}

impl LLMToolCallRequest {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct LLMToolCallResponse {
    pub call_id: Option<String>,
    // pub code: isize,
    pub content: String,
}

#[async_trait::async_trait]
impl ServiceLayer for ToolService {
    type Config = CfgBound<LLMToolCallRequest>;
    type Output = LLMToolCallResponse;

    async fn call(
        &self,
        code: String,
        ctx: Arc<Context>,
        cfg: Self::Config,
    ) -> anyhow::Result<Self::Output> {
        let cfg = cfg.bound(&ctx)?;

        let LLMToolCallRequest {
            call_id,
            name,
            args,
        } = cfg;
        wd_log::log_debug_ln!("code[{}] exec tool[{}] args:{:?}", code, name, args);

        let content = self
            .loader
            .call(name.as_str(), args)
            .await?;

        let resp = LLMToolCallResponse { call_id, content };
        wd_log::log_debug_ln!("code[{}] exec tool[{}] result[{:?}]", code, name, resp);
        Ok(resp)
    }
}

#[cfg(test)]
mod test {

    use crate::plugin_tools::PluginControlSchedule;
    use crate::rt_node_service::{LLMNodeResponse, OpenaiLLMService, ToolService};
    use agent_rt::{PlanBuilder, Runtime};
    use serde_json::Value;
    use wd_tools::PFArc;

    //cargo test rt_node_service::tool::test::test_llm_tools -- --nocapture
    #[tokio::test]
    async fn test_llm_tools() {
        let rt = Runtime::default()
            .register_service_layer("openai_llm", OpenaiLLMService::default())
            .register_service_layer(
                "function_call",
                ToolService::from(
                    PluginControlSchedule::default()
                        .register_plugin("search_weather", |x| async move {
                            println!("tool args --->{}", x);
                            Ok("晴朗，风力3级".to_string())
                        })
                        .to_tool_event(),
                ),
            )
            .launch();
        let query = "shanghai天气怎么样".to_string();
        let tool = r#"{"type":"function","function":{"name":"search_weather","description":"查询天气","parameters":{"type":"object","properties":{"location":{"type":"string","description":"the city","enum":["beijing","shanghai"]}},"required":["location"]}}}"#;
        let msg = format!(
            "{{\"prompt\":\"你是一个智能助手\" ,\"query\":\"{query}\",\"tools\":[{tool}]}}"
        );
        // let msg = format!("{{\"prompt\":\"你是一个智能助手\" ,\"query\":\"{query}\"}}");
        let ctx = rt
            .ctx(
                "test001",
                PlanBuilder::single_node("openai_llm", msg)
                    .check_and_build()
                    .unwrap(),
            )
            // .updates(OpenaiLLMService::set_channel_to_ctx)
            .arc();
        let llm_resp = ctx.clone().block_on::<Value,_>(()).await.unwrap();
        let resp: LLMNodeResponse = serde_json::from_value(llm_resp).unwrap();
        for i in resp.tools.unwrap() {
            let tool_resp = rt
                .ctx(
                    "tool_call_test001",
                    PlanBuilder::single_node("function_call", i.as_json())
                        .check_and_build()
                        .unwrap(),
                )
                .arc()
                .block_on::<Value,_>(())
                .await
                .unwrap();
            println!("tool resp --->{}", tool_resp.to_string());
        }
    }
}
