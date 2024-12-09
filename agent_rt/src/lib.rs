mod consts;
mod context;
mod define;
mod error;
mod middles;
mod graph_plan;
mod runtime;

pub use consts::*;
pub use context::Context;
pub use define::*;
pub use error::Error;
pub use graph_plan::*;
pub use runtime::*;

#[cfg(test)]
mod tests {
    use crate::{GraphPlan, Output, RuntimeBuilder, ServiceLoaderImpl};

    const FLOW_CONFIG:&'static str = r#"\
// [setting]::
// start_node = "start"

[node]::add_format:end
{
    "prefix":"->"
}

[flow]:::
start -> end
    "#;

    //cargo test tests::test_simple_runtime -- --nocapture
    #[tokio::test]
    async fn test_simple_runtime() {
        let rt = RuntimeBuilder::default()
            .set_service_loader(
                ServiceLoaderImpl::default()
                    .register_fn("add_format", |_ctx, _node| async {
                        println!("service --->1");
                        Ok(Output::new("start_service_success".to_string()))
                    })
                    .register_fn("end", |_ctx, _node| async {
                        println!("service --->2");
                        Ok(Output::new("end_service_success".to_string()))
                    }),
            )
            .register_service_middle_fn(|c, n| {
                println!("log middle -> node[{}]", n.name);
                c.next(n)
            })
            .register_task_start_hook_fn(|c| async move {
                println!("---> flow start:");
                Ok(())
            })
            .register_task_end_hook_fn(|c| async move {
                println!("<--- flow end;");
                Ok(())
            })
            .build();
        let plan = GraphPlan::try_from(FLOW_CONFIG).unwrap();
        println!("plan:{}",plan);
        let result = rt
            .context(plan)
            .go::<_, String>("hello world")
            .await;
        println!("{result:?}");
        assert_eq!("start_service_success", result.unwrap().as_str())
    }
}
