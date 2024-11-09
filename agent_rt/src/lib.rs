mod context;
mod define;
mod error;
mod runtime;
mod plan;
mod consts;
mod middles;

pub use define::*;
pub use context::Context;
pub use error::Error;
pub use runtime::*;
pub use plan::*;
pub use consts::*;

#[cfg(test)]
mod tests {
    use crate::{GraphPlan, Output, RuntimeBuilder, ServiceLoaderImpl};

    //cargo test tests::test_simple_runtime -- --nocapture
    #[tokio::test]
    async fn test_simple_runtime(){
        let rt = RuntimeBuilder::default()
            .set_service_loader(ServiceLoaderImpl::default()
                .register_fn("start",|_ctx,_node|async {
                    Ok(Output::new("start_service_success".to_string()))
                })
                .register_fn("end",|_ctx,_node|async {
                    Ok(Output::new("end_service_success".to_string()))
                }))
            .register_service_middle_fn(|c,n|{
                println!("log middle -> node[{}]",n.name);
                c.next(n)
            })
            .register_task_start_hook_fn(|c|async move{
                println!("---> flow start:");
                Ok(())
            })
            .register_task_end_hook_fn(|c|async move{
                println!("<--- flow end;");
                Ok(())
            })
            .build();
        let result = rt.context(GraphPlan::test()).go::<_,String>("hello world").await;
        println!("{result:?}");
    }

}
