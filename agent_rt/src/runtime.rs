use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use crate::{ProgramPool, Context, Input, Output, Plan, PlanResult, ServiceLoader, ServiceMiddle};
use crate::consts::START_NODE_NAME;
use crate::context::ContextEntity;

pub struct Runtime{
    pub(crate) entity:Arc<RuntimeEntity>
}
pub struct RuntimeEntity{
    pub services : Box<dyn ServiceLoader>,
    pub middles : Box<Vec<Arc<dyn ServiceMiddle>>>,
    pub thread_pool : Box<dyn ProgramPool>
}

impl Clone for Runtime{
    fn clone(&self) -> Self {
        Runtime{entity:self.entity.clone()}
    }
}

impl Runtime{
    pub fn context<P:Plan+ Sync>(&self,p:P)->Context{
        ContextEntity::new(self.clone(),p).build()
    }
}

impl Runtime{
    pub async fn execute_node(ctx: Context,node_name: &str)->anyhow::Result<()>{
        let result = ctx.node_goto(node_name).await?;
        let nodes = match result {
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