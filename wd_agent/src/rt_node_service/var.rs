use std::sync::Arc;
use serde_json::Value;
use agent_rt::Context;
use crate::rt_node_service::CfgBound;

#[derive(Debug)]
pub struct VarFlowChartService {}

#[async_trait::async_trait]
impl agent_rt::ServiceLayer for VarFlowChartService{
    type Config = CfgBound<Value>;
    type Output = Value;

    async fn call(&self, _code: String, ctx: Arc<Context>, cfg: Self::Config) -> anyhow::Result<Self::Output> {
        let var = cfg.raw_bound_value(&ctx)?;
        Ok(var)
    }
}

#[cfg(test)]
mod test {
    use serde_json::Value;
    use wd_tools::PFArc;
    use agent_rt::PlanBuilder;

    #[tokio::test]
    async fn test_var() {
        let rt = agent_rt::Runtime::default()
            .register_service_layer("flow_chart_var", super::VarFlowChartService {})
            .launch();
        let out_val:Value = rt.ctx("test001",PlanBuilder::single_node("flow_chart_var",r#"{
                "prompt":"you are {{start.specialty}} assistant.",
                "query":"{{start.query}}",
                "temperature":"{{start.temperature}}"
            }"#).build())
            .arc()
            .block_on(serde_json::json!({
                "query":"hello world",
                "temperature":0.7,
                "specialty":"coding"
            })).await.unwrap();

        println!("{out_val:?}");
    }

}