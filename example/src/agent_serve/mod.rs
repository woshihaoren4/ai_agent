pub mod common;
mod serve_entity;

use crate::proto;
use wd_agent::rt_node_service::{
    InjectorService, PythonCodeService, SelectorService, WorkflowService,
};
use crate::tools::default_tool_service;

pub async fn start(addr: &str) {
    //create service
    let openai_llm = wd_agent::rt_node_service::OpenaiLLMService::default();
    let var = wd_agent::rt_node_service::VarFlowChartService::default();
    let python = PythonCodeService::new("http://127.0.0.1:50001")
        .await
        .unwrap();
    let tool = default_tool_service();

    //build agent runtime
    let rt = agent_rt::Runtime::default()
        .register_service_layer("openai_llm", openai_llm)
        .register_service_layer("python", python)
        .register_service_layer("flow_chart_selector", SelectorService::default())
        .register_service_layer("flow_chart_injector", InjectorService::default())
        .register_service_layer("workflow", WorkflowService::default())
        .register_service_layer("tool", tool)
        .register_service_layer("flow_chart_var", var);

    //启动rpc服务
    let app = serve_entity::AgentServeEntity::new(rt);

    let addr = addr.parse().unwrap();

    wd_log::log_debug_ln!("grpc.Server lister addr[{}]", addr);

    tonic::transport::Server::builder()
        .add_service(proto::agent_service_server::AgentServiceServer::new(app))
        .serve(addr)
        .await
        .unwrap();
}
