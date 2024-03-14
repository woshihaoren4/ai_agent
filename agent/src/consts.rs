use std::any::Any;
use std::sync::Arc;
use wd_tools::PFOk;
use rt::{Context, TaskOutput};

// pub const NEXT_NODE_ID:&'static str = "context_next_node_id";
pub const NEXT_NODE_IDS:&'static str = "context_next_node_ids";

pub const AGENT_EXEC_STATUS:&'static str = "agent_exec_status";

pub const AGENT_TOOL_WRAPPER:&'static str = "agent_tool_wrapper";

pub const MULTI_AGENT_RECALL_TOOLS:&'static str = "multi_agent_recall_tools";

pub fn callback_self<T:Any+Send+Sync+'static>(ctx:Arc<Context>,id:String,next_id:String,val:T)->anyhow::Result<TaskOutput>{
    let result = ctx.get(NEXT_NODE_IDS,|opt:Option<&mut Vec<String>>|{
        if let Some(ids) = opt{
            ids.push(id);
            return None
        }
        return Some(id)
    });
    if let Some(id) = result {
        ctx.set(NEXT_NODE_IDS,vec![id]);
    }
    TaskOutput::new(next_id,val).ok()
}
pub fn go_next_or_over<T:Any+Send+Sync+'static>(ctx:Arc<Context>,resp:T)->anyhow::Result<TaskOutput>{
    let result = ctx.get(NEXT_NODE_IDS,|ids:Option<&mut Vec<String>>|{
        if let Some(ids)=ids{
            return ids.pop()
        }
        return None
    });

    let output = if let Some(id) = result{
        TaskOutput::new(id,resp)
    }else{
        TaskOutput::from_value(resp).over()
    };

    Ok(output)
}