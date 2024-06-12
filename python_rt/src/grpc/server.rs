use crate::grpc::common;
use crate::grpc::common::serde_value_to_prost_struct;
use crate::proto::proto;
use crate::proto::proto::{CallFunctionRequest, CallFunctionResponse};
use crate::py_runtime::PyScriptEntity;
use prost_types::Struct;
use serde_json::Value;
use std::collections::BTreeMap;
use tonic::{Request, Response, Status};
use wd_tools::{PFErr, PFOk};

/// ## Direct run service
/// grpc::server::Server::default()
/// .run("[::1]:50001")
/// .await
/// .unwrap();
#[derive(Debug, Default)]
pub struct Server {}
#[tonic::async_trait]
impl proto::python_runtime_service_server::PythonRuntimeService for Server {
    async fn call_function(
        &self,
        request: Request<CallFunctionRequest>,
    ) -> Result<Response<CallFunctionResponse>, Status> {
        let req = request.into_inner();

        let py_entity = match Self::req_to_py_entity(&req) {
            Ok(o) => o,
            Err(e) => {
                return Self::resp_error(400, e.to_string());
            }
        };

        let result = py_entity.eval_function(
            req.function_name.as_str(),
            Self::output_to_args(req.function_input),
        );
        let value = match result {
            Ok(o) => o,
            Err(e) => {
                return Self::resp_error(500, e.to_string());
            }
        };

        if let Some(s) = serde_value_to_prost_struct(&value) {
            Self::success("success".into(), s)
        } else {
            let msg =
                serde_json::to_string(&value).unwrap_or("parse output value failed".to_string());
            Self::success(
                msg,
                Struct {
                    fields: BTreeMap::new(),
                },
            )
        }
    }
}

impl Server {
    pub fn success(msg: String, output: Struct) -> Result<Response<CallFunctionResponse>, Status> {
        let resp = CallFunctionResponse {
            code: 0,
            msg,
            output: Some(output),
        };
        Ok(Response::new(resp))
    }
    pub fn resp_error(code: i32, e: String) -> Result<Response<CallFunctionResponse>, Status> {
        let resp = CallFunctionResponse {
            code,
            msg: e.to_string(),
            output: None,
        };
        Ok(Response::new(resp))
    }
    pub fn output_to_args(value: Option<prost_types::Struct>) -> Value {
        if let Some(s) = value {
            common::prost_struct_to_serde_value(s)
        } else {
            Value::Null
        }
    }
    pub fn req_to_py_entity(req: &CallFunctionRequest) -> anyhow::Result<PyScriptEntity> {
        let CallFunctionRequest {
            src,
            script_code,
            module_name,
            file_name,
            sys_path,
            ..
        } = req;
        let mut entity = if *src == 0 {
            if let Some(code) = script_code {
                PyScriptEntity::new(code, module_name)
            } else {
                return anyhow::anyhow!("if src is [SRC_TYPE_SCRIPT] script_code can not nil")
                    .err();
            }
        } else if *src == 1 {
            PyScriptEntity::from_module(module_name)
        } else {
            unreachable!()
        };
        if let Some(s) = file_name {
            entity = entity.set_file_name(s);
        }
        if let Some(s) = sys_path {
            entity = entity.set_sys_path(s);
        }
        entity.ok()
    }

    pub async fn run(self, addr: &str) -> anyhow::Result<()> {
        let addr = addr.parse()?;
        let greeter = Server::default();

        wd_log::log_debug_ln!("grpc.Server lister addr[{}]", addr);

        tonic::transport::Server::builder()
            .add_service(
                proto::python_runtime_service_server::PythonRuntimeServiceServer::new(greeter),
            )
            .serve(addr)
            .await?;

        Ok(())
    }
}
