use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wd_tools::PFOk;
use agent_rt::{Context, ServiceLayer};
use python_rt::client::Client;
use crate::rt_node_service::CfgBound;

#[derive(Debug)]
pub struct PythonCodeService {
    pub client: Client,
}
impl PythonCodeService{
    pub async fn new(url:&str)-> anyhow::Result<Self>{
        let client = Client::new(url).await?;
        Self{client}.ok()
    }
}

#[async_trait::async_trait]
impl ServiceLayer for PythonCodeService {
    type Config = CfgBound<PythonCodeServiceRequest>;
    type Output = Value;

    async fn call(&self, _code: String, ctx: Arc<Context>, cfg: Self::Config) -> anyhow::Result<Self::Output> {
        let PythonCodeServiceRequest { script_code, function_name, input } = cfg.bound(&ctx)?;
        self.client.eval_script_code::<_, _, _, Value>(script_code, function_name, input).await
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PythonCodeServiceRequest{
    pub script_code:String,
    pub function_name:String,
    pub input:Value,
}



#[cfg(test)]
mod test{
    use serde_json::Value;
    use wd_tools::PFArc;
    use agent_rt::{PlanBuilder, Runtime};
    use crate::rt_node_service::python::{PythonCodeService, PythonCodeServiceRequest};

    const PY_SCRIPT_CODE:&'static str = r#"
def handle(input):
    data=input.data
    return {"answer":"AI:"+data["answer"]}
"#;


    //cargo test rt_node_service::python::test::test_python_service -- --nocapture
    #[tokio::test]
    pub async fn test_python_service(){
        let py = PythonCodeService::new("http://127.0.0.1:50001").await.unwrap();
        let rt = Runtime::default()
            .register_service_layer("python", py)
            .launch();

        let cfg = serde_json::json!({
            "script_code":PY_SCRIPT_CODE,
            "function_name":"handle",
            "input": {
                "answer":"{{start.answer}}"
            }
        });

        let output = rt.ctx("py-test-001", PlanBuilder::single_node("python", serde_json::to_string(&cfg).unwrap()).build())
            .arc()
            .block_on::<Value,_>(serde_json::json!({
                "answer":"this is a input"
            })).await.unwrap();

        println!("--> {}",output);
    }
}