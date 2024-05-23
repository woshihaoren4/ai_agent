mod context;
mod default_node_loader;
mod default_waker_pool;
mod define;
mod error;
mod in_out_put;
mod plan;
mod runtime;
mod runtime_middle;
mod service_layer;

pub use context::*;
pub use define::*;
pub use error::*;
pub use in_out_put::*;
pub use plan::*;
pub use runtime::*;
#[allow(unused_imports)]
pub use runtime_middle::*;
pub use service_layer::*;

#[cfg(test)]
mod tests {
    use crate::{Output, PlanBuilder, Runtime, END_NODE_CODE};
    use wd_tools::PFArc;

    #[tokio::test]
    pub async fn test_runtime_stack() {
        let rt = Runtime::default()
            .register_service_fn("node_id_1", |f| async move {
                wd_log::log_info_ln!("node1=>{:?}", f);
                Ok(Output::new("success".to_string()))
            })
            .launch();

        let plan = PlanBuilder::start(("A", "node_id_1"), vec!["B"])
            .sequence(
                vec![("B", "node_id_1", ""), ("C", "node_id_1", "")],
                END_NODE_CODE,
            )
            .end::<&str, _>(vec![], (END_NODE_CODE, "node_id_1", ""))
            .check_and_build()
            .unwrap();

        let ctx = rt.ctx("test001", plan).push_callback(|_x| {
            println!("over callback");
        });
        ctx.set_max_stack(2);
        let ctx = ctx.arc();

        let res = rt.block_on::<String>(ctx).await;
        println!("{:?}", res);
        assert_eq!(true, res.is_err());
    }

    #[tokio::test]
    pub async fn test_runtime_one() {
        let rt = Runtime::default()
            .register_service_fn("node_id_1", |f| async move {
                wd_log::log_info_ln!("node1=>{:?}", f);
                Ok(Output::new("success".to_string()).raw_to_ctx())
            })
            .launch();

        let plan = PlanBuilder::start((END_NODE_CODE, "node_id_1"), vec![""])
            .check_and_build()
            .unwrap();

        let ctx = rt.ctx("test001", plan).arc();
        let res = rt.block_on::<String>(ctx).await.unwrap();
        println!("{:?}", res);
        assert_eq!("success", res.as_str());
    }

    #[tokio::test]
    pub async fn test_runtime_sequence() {
        let rt = Runtime::default()
            .register_service_fn("node_id_1", |f| async move {
                wd_log::log_info_ln!("node1=>{:?}", f);
                Ok(Output::new("node1 success".to_string()))
            })
            .register_service_fn("node_id_2", |f| async move {
                wd_log::log_info_ln!("node2=>{:?}", f);
                Ok(Output::new("node2 success".to_string()))
            })
            .register_service_fn("node_id_3", |f| async move {
                wd_log::log_info_ln!("node3=>{:?}", f);
                Ok(Output::new("success".to_string()).raw_to_ctx())
            })
            .register_middle_fn(|f| async move {
                wd_log::log_debug_ln!("middle start --->1");
                let out = f.call().await.unwrap();
                wd_log::log_debug_ln!("middle over <---1");
                Ok(out)
            })
            .register_middle_fn(|f| async move {
                wd_log::log_debug_ln!("middle start --->2");
                let out = f.call().await.unwrap();
                wd_log::log_debug_ln!("middle over <----2");
                Ok(out)
            })
            .launch();

        let plan = PlanBuilder::start(("A", "node_id_1"), vec!["B"])
            .sequence(
                vec![("B", "node_id_2", ""), (END_NODE_CODE, "node_id_3", "")],
                "",
            )
            .check_and_build()
            .unwrap();

        let ctx = rt
            .ctx("test001", plan)
            .push_callback(|_x| {
                println!("over callback");
            })
            .arc();
        let res = rt.block_on::<String>(ctx).await.unwrap();
        println!("{}", res);
        assert_eq!("success", res.as_str());
    }

    //cargo test tests::test_runtime_sub_task -- --nocapture
    #[tokio::test]
    pub async fn test_runtime_sub_task() {
        let rt = Runtime::default()
            .register_service_fn("node1", |f| async move {
                wd_log::log_info_ln!("node1=>{:?}", f);
                if f.node_config.as_str() == "v2" {
                    Ok(Output::new("node1 v2 success".to_string()).raw_to_ctx())
                } else {
                    Ok(Output::new("node1 v1 success".to_string()).raw_to_ctx())
                }
            })
            .register_service_fn("node2", |f| async move {
                wd_log::log_info_ln!("node2=>{:?}", f);
                if f.node_config.as_str() != "" {
                    let s = f.ctx.remove::<String>(f.node_config.as_str()).unwrap();
                    Ok(Output::new(format!("n2-sign->{}", s)).raw_to_ctx())
                } else {
                    Ok(Output::new("node2 v1 success".to_string()).raw_to_ctx())
                }
            })
            .register_service_fn("n1_and_n2", |f| async move {
                wd_log::log_info_ln!("执行一个子任务");
                let code = format!("{}.{}", f.ctx.code, f.code);
                let plan = PlanBuilder::start(("n1", "node1"), vec![END_NODE_CODE])
                    .end(Vec::<String>::new(), (END_NODE_CODE, "node2", "n1"))
                    .check_and_build()?;

                let res = f.ctx.sub_ctx(code, plan).arc().block_on::<String>().await?;
                wd_log::log_info_ln!("子任务执行完成");
                Ok(Output::new(res).raw_to_ctx())
            })
            .register_middle_fn(|f| async move {
                wd_log::log_debug_ln!("middle start --->1");
                let out = f.call().await.unwrap();
                wd_log::log_debug_ln!("middle over <---1");
                Ok(out)
            })
            .launch();

        let plan = PlanBuilder::start(("A", "node1"), vec!["B"])
            .sequence(vec![("B", "n1_and_n2"), (END_NODE_CODE, "node2")], "")
            .check_and_build()
            .unwrap();

        let out = rt
            .ctx("test_runtime_sub_task", plan)
            .arc()
            .block_on::<String>()
            .await
            .unwrap();
        println!("--->{}", out);
    }

    // cargo test tests::test_runtime_panic -- --nocapture
    #[tokio::test]
    pub async fn test_runtime_panic() {
        let rt = Runtime::default()
            .register_service_fn("node_id_1", |f| async move {
                wd_log::log_info_ln!("node1=>{:?}", f);
                panic!("node1 test panic");
                // Ok(Output::new("success".to_string()).raw_to_ctx())
            })
            .launch();

        let plan = PlanBuilder::single_node("node_id_2", "{}")
            .check_and_build()
            .unwrap();

        let ctx = rt.ctx("test001", plan).arc();
        let res = rt.block_on::<String>(ctx).await;
        println!("{:?}", res);
        assert_eq!(true, res.is_err());
    }
}
