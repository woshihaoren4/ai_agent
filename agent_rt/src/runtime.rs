use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use crate::{Context, Input, Output, Plan, PlanResult, ServiceLoader, ServiceMiddle};
use crate::consts::START_NODE_NAME;

pub struct Runtime{
    pub(crate) entity:Arc<RuntimeEntity>
}
pub struct RuntimeEntity{
    pub(crate) services : Arc<dyn ServiceLoader>,
    pub(crate) middles : Arc<Vec<Arc<dyn ServiceMiddle>>>,
}

impl Clone for Runtime{
    fn clone(&self) -> Self {
        Runtime{entity:self.entity.clone()}
    }
}

impl Runtime{
    pub fn context<P:Plan+ Sync>(&self,p:P)->Context{
        Context::new(self.clone(),p)
    }
}

impl Runtime{
    pub async fn execute_node(ctx: Arc<Context>,node_name: &str)->anyhow::Result<()>{
        let mut lock = ctx.plan.lock().await;
        let plan_result = lock.next(node_name)?;
        let nodes = match plan_result {
            PlanResult::Nodes(s) => s,
            PlanResult::End => {
                return Ok(())
            }
            PlanResult::Wait => {
                return Ok(())
            }
        };
        for i in nodes{

        }
        return Ok(())
    }
}