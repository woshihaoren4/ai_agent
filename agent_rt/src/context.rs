use std::sync::Arc;
use wd_tools::sync::Am;
use crate::{Error, Node, Output, Runtime, Plan, PlanResult};

pub struct Context{
    pub ce : Arc<ContextEntity>
}

pub struct ContextEntity{
    rt : Runtime,
    pub plan: Am<dyn Plan + Sync>
}
impl ContextEntity {
    pub fn new<P:Plan+ Sync>(rt:Runtime,p:P)->Self{
        ContextEntity{ rt, plan: Am::new(p)}
    }
    pub fn build(self)->Context{
        Context{ce:Arc::new(self)}
    }
}

impl Context {
    pub async fn node_goto(&self,node_name:&str)->anyhow::Result<PlanResult>{
        let mut lock = self.ce.plan.lock().await;
        lock.next(node_name)
    }
    pub async fn next(self, mut node:Node) -> anyhow::Result<Output> {
        let middles = &self.ce.rt.entity.middles;
        if node.middle_index < middles.len() {
            let middle = &middles[node.middle_index];
            if !middle.filter(&node) {
                node.middle_index +=1;
                return self.next(node).await
            }
            middle.call(self,node).await
        }else if node.middle_index == middles.len() {
            let service = node.service.clone();
            service.call(self,node).await
        }else{
            //Will not execute
            wd_log::log_error_ln!("[agent_rt] Context.next to will not execute");
            Error::NextNodeNull.into()
        }
    }
}