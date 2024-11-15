use crate::{Context, Node, Output, PlanResult, Runtime};

impl Runtime {
    pub async fn input_output_middle(mut ctx: Context, node: Node) -> anyhow::Result<Output> {
        let result = Ok(Output::new(0));
        // 校验任务状态
        if !ctx.is_running() {
            return result;
        }
        //开始执行任务
        let node_name = node.name.clone();
        let node_result = ctx.clone().next(node).await;
        if let Err(e) = node_result {
            ctx.error(e);
            return result;
        }
        let plan = ctx.ctx(|c| c.plan.next(&node_name)).await;
        if let Ok(out) = node_result {
            ctx = ctx.set_box_any(node_name, out.into_box());
        }
        if let Err(e) = plan {
            ctx.error(e);
            return result;
        }
        //继续执行后续的节点
        let plan = plan.unwrap();
        match plan {
            PlanResult::Nodes(nodes) => {
                let rt = ctx.ctx(|c| c.rt.clone()).await;
                for mut i in nodes {
                    match rt.entity.services.load(i.service_name.as_str()).await {
                        None => {
                            ctx.error(anyhow::anyhow!(
                                "Runtime:node[{}].service[{}] not found",
                                i.name,
                                i.service_name
                            ));
                            break;
                        }
                        Some(s) => {
                            i = i.set_service_are(s);
                        }
                    };
                    let fut = ctx.clone().next(i);
                    if let Err(e) = rt.entity.thread_pool.push(Box::pin(fut)).await {
                        ctx.error(e);
                        break;
                    }
                }
            }
            PlanResult::End => {
                ctx.success();
            }
            PlanResult::Wait => {}
        }
        result
    }
}
