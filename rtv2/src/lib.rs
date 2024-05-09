mod context;
mod in_out_put;
mod task;
mod define;
mod runtime;
mod error;
mod default_node_loader;
mod default_waker_pool;
mod plan;

pub use context::*;
pub use in_out_put::*;
pub use task::*;
pub use define::*;
pub use runtime::*;
pub use error::*;

#[cfg(test)]
mod tests {
    use crate::{Output, Runtime};

    #[test]
    pub fn test_runtime_simple(){
        let rt = Runtime::default()
            .register_service_fn("node_id_1",|f|async move{
                wd_log::log_info_ln!("node1=>{:?}",f);
                Ok(Output::new("node1 success".to_string()))
            })
            .register_service_fn("node_id_2",|f|async move{
                wd_log::log_info_ln!("node2=>{:?}",f);
                Ok(Output::new("node2 success".to_string()))
            })
            .register_service_fn("node_id_3",|f|async move{
                wd_log::log_info_ln!("node3=>{:?}",f);
                Ok(Output::new("node3 success".to_string()))
            })
            .register_middle_fn(|f|async move{
                wd_log::log_debug_ln!("middle start --->1");
                let id = f.code.clone();
                let ctx = f.ctx.clone();
                let out = f.call().await.unwrap();
                ctx.set(id,out);
                wd_log::log_debug_ln!("middle over <---1");
                Ok(Output::null())
            })
            .register_middle_fn(|f|async move{
                wd_log::log_debug_ln!("middle start --->2");
                let out = f.call().await.unwrap();
                wd_log::log_debug_ln!("middle over <----2");
                Ok(out)
            })
            .launch();
        // rt.ctx("task1")
        // rt.spawn().unwrap()
    }
}
