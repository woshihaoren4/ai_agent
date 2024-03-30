use rt::{Context, TaskOutput};
use std::any::Any;
use std::sync::Arc;
use wd_tools::PFOk;
use crate::Memory;

// pub const NEXT_NODE_ID:&'static str = "context_next_node_id";
pub const NEXT_NODE_IDS: &'static str = "context_next_node_ids";

pub const AGENT_EXEC_STATUS: &'static str = "agent_exec_status";

pub const AGENT_TOOL_WRAPPER: &'static str = "agent_tool_wrapper";

pub const MULTI_AGENT_RECALL_TOOLS: &'static str = "multi_agent_recall_tools";

pub const USER_ID:&'static str = "user_id";

pub const MEMORY:&'static str = "memory";

pub const PROMPT:&'static str = "prompt";

pub fn callback_self<T: Any + Send + Sync + 'static>(
    ctx: Arc<Context>,
    id: String,
    next_id: String,
    val: T,
) -> anyhow::Result<TaskOutput> {
    let result = ctx.get(NEXT_NODE_IDS, |opt: Option<&mut Vec<String>>| {
        if let Some(ids) = opt {
            ids.push(id);
            return None;
        }
        return Some(id);
    });
    if let Some(id) = result {
        ctx.set(NEXT_NODE_IDS, vec![id]);
    }
    TaskOutput::new(next_id, val).ok()
}
pub fn go_next_or_over<T: Any + Send + Sync + 'static>(
    ctx: Arc<Context>,
    resp: T,
) -> anyhow::Result<TaskOutput> {
    let result = ctx.get(NEXT_NODE_IDS, |ids: Option<&mut Vec<String>>| {
        if let Some(ids) = ids {
            return ids.pop();
        }
        return None;
    });

    let output = if let Some(id) = result {
        TaskOutput::new(id, resp)
    } else {
        TaskOutput::from_value(resp).over()
    };

    Ok(output)
}

pub fn user_id_from_ctx(ctx:&Context)->String{
    ctx.get(USER_ID,|x :Option<&mut String>|{
        x.map(|s|s.clone()).unwrap_or(String::new())
    })
}
pub fn user_id_to_ctx<S:Into<String>>(ctx:&Context,uid:S){
    ctx.set(USER_ID,uid.into());
}
pub fn memory_from_ctx_unwrap(ctx:&Context)->Arc<dyn Memory>{
    ctx.get(MEMORY,|x :Option<&mut Arc<dyn Memory>>|{
        x.map(|s|s.clone()).unwrap()
    })
}
pub fn memory_to_ctx(ctx:&Context,memory:Arc<dyn Memory>){
    ctx.set(MEMORY,memory);
}

pub fn prompt_from_ctx(ctx:&Context)->String{
    ctx.get(PROMPT,|x :Option<&mut String>|{
        x.map(|s|s.clone()).unwrap_or(String::new())
    })
}
pub fn prompt_to_ctx<S:Into<String>>(ctx:&Context,prompt:S){
    ctx.set(PROMPT,prompt.into());
}