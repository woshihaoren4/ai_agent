use crate::rt_node_service::CfgBound;
use agent_rt::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use wd_tools::PFErr;

#[derive(Debug, Default)]
pub struct InjectorService {}

impl InjectorService {
    pub fn operate(src: &mut Value, mut value: Value, operate: String) -> anyhow::Result<()> {
        match operate.to_lowercase().as_str() {
            "=" => *src = value,
            "append" => {
                if let Value::Array(list) = src {
                    if let Value::Array(ref mut vec) = value {
                        list.append(vec);
                        return Ok(());
                    }
                    if value.is_null() {
                        return Ok(());
                    }
                    list.push(value);
                    return Ok(());
                }
                return anyhow::anyhow!("InjectorService.operate append from must is array").err();
            }
            _ => {
                return anyhow::anyhow!("InjectorService.operate not support operate[{}]", operate)
                    .err()
            }
        }
        Ok(())
    }
    pub fn update(
        pos: String,
        ctx: &Context,
        function: impl FnOnce(&mut Value) -> anyhow::Result<()> + 'static,
    ) -> anyhow::Result<()> {
        let mut ks = pos.as_str().split(".").collect::<VecDeque<&str>>();
        let code = ks.pop_front().unwrap_or("").to_string();
        return ctx.plan.update(
            code.as_str(),
            Box::new(move |x| {
                let mut ks = pos.split(".").collect::<VecDeque<&str>>();
                let code = ks.pop_front().unwrap_or("");
                if x.is_none() {
                    return anyhow::anyhow!("InjectorService.update not found node[{}]", code)
                        .err();
                }
                let plan = x.unwrap();
                let node = if let Some(ref mut s) = plan.cfg {
                    s
                } else {
                    return anyhow::anyhow!("InjectorService.update node[{}] cfg is null", code)
                        .err();
                };
                let mut cfg =
                    serde_json::from_str::<Value>(node.node_config.as_str()).unwrap_or(Value::Null);
                let mut mut_from = &mut cfg;
                loop {
                    if let Some(key) = ks.pop_front() {
                        match mut_from {
                            Value::Object(ref mut obj) => {
                                if let Some(val) = obj.get_mut(key) {
                                    mut_from = val;
                                    continue;
                                }
                            }
                            _ => {
                                return anyhow::anyhow!(
                                    "InjectorService.find field failed [{}]",
                                    pos
                                )
                                .err()
                            }
                        }
                        return anyhow::anyhow!("CfgBound.not find field[{}]", pos).err();
                    } else {
                        break;
                    }
                }
                let result = function(mut_from);
                node.node_config = serde_json::to_string(&cfg).unwrap_or("".into());
                return result;
            }),
        );
    }
}

#[async_trait::async_trait]
impl agent_rt::ServiceLayer for InjectorService {
    type Config = CfgBound<InjectorServiceConfig>;
    type Output = Value;

    async fn call(
        &self,
        _code: String,
        ctx: Arc<Context>,
        cfg: Self::Config,
    ) -> anyhow::Result<Self::Output> {
        let InjectorServiceConfig {
            from, to, operate, ..
        } = cfg.bound(&ctx)?;
        if to.is_empty() {
            return anyhow::anyhow!("InjectorService: from and to must have a value").err();
        }

        Self::update(to, &ctx, |x| Self::operate(x, from, operate))?;

        Ok(Value::Null)
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct InjectorServiceConfig {
    pub from: Value,
    pub to: String,
    pub default: Value,
    // =,append,
    pub operate: String,
}

#[cfg(test)]
mod test {
    use crate::rt_node_service::{InjectorService, VarFlowChartService};
    use agent_rt::{PlanBuilder, Runtime, END_NODE_CODE};
    use serde_json::Value;
    use wd_tools::PFArc;

    #[tokio::test]
    async fn test_injector() {
        let rt = Runtime::default()
            .register_service_layer("flow_chart_injector", InjectorService::default())
            .register_service_layer("flow_chart_var", VarFlowChartService::default())
            .launch();

        let cfg = serde_json::json!({
            "from":"{{start.input}}",
            "to":"end.output",
            "operate":"append"
        });

        let result = rt
            .ctx(
                "injector-001",
                PlanBuilder::start(
                    (
                        "injector",
                        "flow_chart_injector",
                        serde_json::to_string(&cfg).unwrap(),
                    ),
                    vec![END_NODE_CODE],
                )
                .end::<String, _>(vec![], ("end", "flow_chart_var", r#"{"output":["first"]}"#))
                .build(),
            )
            .arc()
            .block_on::<Value, _>(serde_json::json!({
                "input":["this is a injector service test"]
            }))
            .await
            .unwrap();

        println!("{}", result)
    }
}
