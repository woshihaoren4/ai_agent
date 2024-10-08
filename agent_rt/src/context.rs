use std::sync::Arc;
use wd_tools::sync::Am;
use crate::{Error, Node, Output, Runtime, Plan};

pub struct Context{
    // middles :  VecDeque<Arc<dyn ServiceMiddle>>,
    rt : Runtime,
    pub plan: Arc<Am<dyn Plan + Sync>>
}
// public
impl Context {
    pub async fn next(self:Arc<Self>, mut node:Node) -> anyhow::Result<Output> {
        let middles = &self.rt.entity.middles;
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
// private
impl Context{
    pub(crate) fn new<P:Plan+ Sync>(rt:Runtime,p:P)->Self{
        Self{ rt, plan: Arc::new(Am::new(p)) }
    }
}
