use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wd_tools::PFErr;
use crate::{Context, Flow, Input, NextNodeResult, Output, RTError, Service, ServiceLoader, START_NODE_CODE, WakerWaitPool};

#[derive(Debug,Clone)]
pub struct Runtime{
    status: Arc<AtomicUsize>, // 1:await 2:running 3:quiting 4:dead
    //先注册的先执行
    middle:VecDeque<Arc<dyn Service>>,
    nodes:Arc<dyn ServiceLoader>,
    wakers:Arc<dyn WakerWaitPool>,
}

impl Runtime {
    pub async fn is_running(&self)->bool{
        self.status.load(Ordering::Relaxed) == 2
    }
    pub async fn spawn(&self,ctx:Arc<Context>,input:Input)->anyhow::Result<()>{
        //检查状态
        if !self.is_running(){
            return RTError::RuntimeDisable.into().err()
        }
        //任务统计

        //准备执行
        Runtime::exec_next_node(ctx, START_NODE_CODE).await;
    }

    async fn exec_next_node(ctx:Arc<Context>,node_id:&str){
        let result = ctx.plan.next(ctx.clone(), node_id);
        let nodes = match result {
            NextNodeResult::Over => {
                //找到end并返回
                return
            }
            NextNodeResult::Wait => {
                return
            }
            NextNodeResult::Nodes(s) => s,
        };
        for i in nodes{
            let mut middle = ctx.middle.clone();
            match ctx.nodes.get(i.node_type_id.as_str()) {
                None => return RTError::UnknownNodeId(i.node_type_id).into().err(),
                Some(n) => middle.push_back(n),
            };
            let flow = Flow::new(i, ctx.clone(), middle);
            tokio::spawn(async move{
                let code = flow.code.clone();
                let ctx = flow.ctx.clone();
                if let Err(e) = flow.next().await {
                    wd_log::log_error_ln!("Runtime.exec_next_node:Unanticipated errors:{}",e);
                    ctx.error_over(e)
                }else{
                    Runtime::exec_next_node(ctx,code.as_str()).await
                }
            });
        };
    }
}