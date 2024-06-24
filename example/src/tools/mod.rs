use wd_agent::plugin_tools::PluginControlSchedule;
use wd_agent::rt_node_service::ToolService;

pub mod google_search;

pub fn default_tool_service()->ToolService{
    ToolService::from(PluginControlSchedule::default()
        .register_plugin("google",google_search::GooglePlugin::default())
        .to_tool_event())
}

#[cfg(test)]
mod test{
    use serde_json::Value;
    use wd_tools::PFArc;
    use agent_rt::PlanBuilder;

    #[tokio::test]
    async fn test_google_search(){
        let tool_service = super::default_tool_service();
        let rt = agent_rt::Runtime::default()
            .register_service_layer("tool",tool_service)
            .launch();

        let cfg = serde_json::json!({
            "name":"google.search",
            "q":"{{start.query}}",
        });

        let val:Value = rt.ctx("test-001", PlanBuilder::single_node("tool", serde_json::to_string(&cfg).unwrap()).build())
            .arc()
            .block_on(serde_json::json!({
                "query":"热辣滚烫"
            })).await.unwrap();

        println!("{}",val);
    }
}