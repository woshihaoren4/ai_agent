use crate::grpc::common::{prost_struct_to_serde_value, serde_value_to_prost_struct};
use crate::proto::proto::python_runtime_service_client::PythonRuntimeServiceClient;
use crate::proto::proto::{CallFunctionRequest, CallFunctionResponse, SrcType};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::sync::Arc;
use tonic::transport::{Channel, Uri};
use wd_tools::pool::{ObjFactor, ObjPool};
use wd_tools::{PFErr, PFOk};

#[derive(Clone, Debug)]
pub struct Client {
    pub pool: Arc<ObjPool<PythonRuntimeServiceClient<Channel>>>,
}
pub struct Factory {
    uri: Uri,
}
impl Client {
    #[allow(dead_code)]
    pub async fn new(uri: &str) -> anyhow::Result<Self> {
        let uri = uri.parse()?;
        let factory = Factory { uri };
        let pool = ObjPool::new(10, 1, factory);
        Self { pool }.ok()
    }
    #[allow(dead_code)]
    pub async fn call_function(
        &self,
        req: CallFunctionRequest,
    ) -> anyhow::Result<CallFunctionResponse> {
        self.pool
            .defer(move |mut conn| async move {
                let result = conn.deref_mut().call_function(req).await;

                let resp = match result {
                    Ok(o) => o.into_inner(),
                    Err(e) => {
                        return anyhow::anyhow!("Client.call_function error:{}", e.to_string())
                            .err()
                    }
                };

                Ok(resp)
            })
            .await
    }
    #[allow(dead_code)]
    pub async fn raw_eval<C, M, F, S, Args, Output>(
        &self,
        ty: SrcType,
        code: Option<C>,
        module_name: M,
        sys_path: Option<S>,
        function_name: F,
        args: Args,
    ) -> anyhow::Result<Output>
    where
        C: Into<String>,
        M: Into<String>,
        F: Into<String>,
        S: Into<String>,
        Args: Serialize,
        Output: for<'a> Deserialize<'a>,
    {
        let value = serde_json::to_value(args)?;
        if !value.is_object() {
            return anyhow::anyhow!("function args must is object").err();
        }
        let mut req = CallFunctionRequest::default();
        req.set_src(ty);
        if let Some(s) = code {
            req.script_code = Some(s.into());
        }
        req.module_name = module_name.into();
        if let Some(s) = sys_path {
            req.sys_path = Some(s.into());
        }
        req.function_name = function_name.into();
        req.function_input = serde_value_to_prost_struct(&value);

        let out = self.call_function(req).await?;
        if out.code != 0 {
            return anyhow::anyhow!(
                "raw_eval_script_code failed code[{}] msg[{}]",
                out.code,
                out.msg
            )
            .err();
        }
        if let Some(s) = out.output {
            let value = prost_struct_to_serde_value(s);
            let out = match serde_json::from_value(value) {
                Ok(o) => o,
                Err(e) => {
                    return anyhow::anyhow!("raw_eval_script_code failed parse output error:{}", e)
                        .err()
                }
            };
            return Ok(out);
        }
        anyhow::anyhow!("output is nil , resp={:?}", out).err()
    }
    #[allow(dead_code)]
    pub async fn eval_script_code<
        C: Into<String>,
        F: Into<String>,
        Args: Serialize,
        Output: for<'a> Deserialize<'a>,
    >(
        &self,
        code: C,
        function_name: F,
        args: Args,
    ) -> anyhow::Result<Output> {
        self.raw_eval(
            SrcType::Script,
            Some(code),
            "default_module",
            None::<String>,
            function_name,
            args,
        )
        .await
    }
    #[allow(dead_code)]
    pub async fn eval_module<
        C: Into<String>,
        S: Into<String>,
        F: Into<String>,
        Args: Serialize,
        Output: for<'a> Deserialize<'a>,
    >(
        &self,
        sys_path: S,
        module_name: C,
        function_name: F,
        args: Args,
    ) -> anyhow::Result<Output> {
        self.raw_eval(
            SrcType::Module,
            None::<String>,
            module_name,
            Some(sys_path),
            function_name,
            args,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ObjFactor<PythonRuntimeServiceClient<Channel>> for Factory {
    async fn make(&self) -> Option<PythonRuntimeServiceClient<Channel>> {
        match PythonRuntimeServiceClient::connect(self.uri.clone()).await {
            Ok(o) => Some(o),
            Err(e) => {
                wd_log::log_error_ln!("connect PythonRuntimeServiceClient error:{}", e);
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::grpc::client::Client;

    #[tokio::test]
    async fn test_client() {
        let _client = Client::new("http://[::1]:50001").await.unwrap();
    }
}
