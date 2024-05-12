mod context;
mod in_out_put;
mod task;
mod define;
mod runtime;
mod error;
mod default_node_loader;
mod default_waker_pool;
mod plan;
mod runtime_middle;

pub use context::*;
pub use in_out_put::*;
pub use task::*;
pub use define::*;
pub use runtime::*;
pub use error::*;
pub use plan::*;


#[cfg(test)]
mod tests {
    use wd_tools::PFArc;
    use crate::{END_NODE_CODE, Output, PlanBuilder, Runtime, START_NODE_CODE};

    #[tokio::test]
    pub async fn test_runtime_simple(){
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
                Ok(Output::new("success".to_string())
                    .raw_to_ctx())
            })
            .register_middle_fn(|f|async move{
                wd_log::log_debug_ln!("middle start --->1");
                let out = f.call().await.unwrap();
                wd_log::log_debug_ln!("middle over <---1");
                Ok(out)
            })
            .register_middle_fn(|f|async move{
                wd_log::log_debug_ln!("middle start --->2");
                let out = f.call().await.unwrap();
                wd_log::log_debug_ln!("middle over <----2");
                Ok(out)
            })
            .launch();

        let plan = PlanBuilder::start((START_NODE_CODE,"node_id_1"),vec!["A"])
            .sequence(vec![("A", "node_id_2",""),(END_NODE_CODE,"node_id_3","")],"")
            .check_and_build().unwrap();

        let ctx = rt.ctx("test001", plan)
            .push_callback(|_x|{
                println!("over callback");
            }).arc();
        let res = rt.block_on::<String>(ctx).await.unwrap();
        println!("{}", res);
        assert_eq!("success", res.as_str());
    }
}
