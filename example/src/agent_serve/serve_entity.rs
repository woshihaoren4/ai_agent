use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tonic::{Request, Response, Status};
use tonic::codegen::tokio_stream::Stream;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use wd_tools::PFArc;
use agent_rt::{CtxStatus, Flow, Output};
use crate::proto;
use crate::proto::{AgentServiceCallRequest, AgentServiceCallResponse, AgentServiceNode, AgentServiceResult};

type CallResponseStream = Pin<Box<dyn Stream<Item = Result<AgentServiceCallResponse, Status>> + Send>>;

pub struct AgentServeEntity{
    pub rt : Arc<agent_rt::Runtime>
}
impl AgentServeEntity{
    pub fn new(rt:agent_rt::Runtime)->Self{
        let rt = rt.register_middle_fn(Self::debug_channel_send)
            .launch();
        Self{rt}
    }
    pub async fn debug_channel_send(flow: Flow) -> anyhow::Result<Output> {
        let ctx = flow.ctx.clone();
        let mut resp = AgentServiceCallResponse::default();
        let mut asr = AgentServiceResult::default();
        asr.node_code = flow.code.clone();
        asr.round = ctx.used_stack() as i32;
        resp.result = Some(asr);

        let channel = ctx.get("debug_channel",|x:&mut Sender<Result<AgentServiceCallResponse,Status>>|x.clone());
        let result = flow.call().await;
        if channel.is_none() {
            return result
        }
        let chan = channel.unwrap();


        match &result{
            Ok(out) => {
                resp.message = "success".into();
                if let Some(val) = out.any.downcast_ref::<serde_json::Value>() {
                    if let Some(ref mut s) = resp.result {
                        s.output = super::common::serde_value_to_prost_struct(val)
                    }
                }
            }
            Err(e) => {
                resp.code = 500;
                resp.message = e.to_string();
            }
        }
        let _ = chan.send(Ok(resp)).await;
        return result
    }
}

#[async_trait::async_trait]
impl proto::agent_service_server::AgentService for AgentServeEntity{
    type CallStream = CallResponseStream;

    async fn call(&self, request: Request<AgentServiceCallRequest>) -> Result<Response<Self::CallStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<AgentServiceCallResponse, Status>>(128);
        let AgentServiceCallRequest{ task_code, plan, input, mode } = request.into_inner();

        let nodes = plan.into_iter().map(|x|{
            let AgentServiceNode{ code, service_type, cfg, ready_nodes, goto_nodes } = x;
            (ready_nodes,agent_rt::Node::new(code,service_type,cfg),goto_nodes)
        }).collect::<Vec<_>>();
        //
        let plan = agent_rt::PlanBuilder::from(nodes).build();

        let ctx = self.rt.ctx(task_code,plan).push_callback(|c|{
            if c.status() != CtxStatus::SUCCESS {
                let channel = c.get("debug_channel",|x:&mut Sender<Result<AgentServiceCallResponse,Status>>|x.clone());
                if let Some(chan)=channel {
                    let mut resp = AgentServiceCallResponse::default();
                    resp.code = 500;
                    resp.message = format!("{:?}",c.end_output::<String>());
                    let _ = chan.try_send(Ok(resp));
                }
            }
        }).arc();
        ctx.set("debug_channel",tx.clone());
        if let Err(e) = ctx.spawn(input){
            let mut resp = AgentServiceCallResponse::default();
            resp.code = 500;
            resp.message = e.to_string();
            tx.send(Ok(resp)).await.unwrap();
        }

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::CallStream
        ))
    }
}