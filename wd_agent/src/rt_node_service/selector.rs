use std::collections::VecDeque;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wd_tools::PFErr;
use agent_rt::Context;
use crate::rt_node_service::{CfgBound};

#[derive(Debug,Default)]
pub struct SelectorService{}

impl SelectorService {
    pub fn judge(vars:&mut VecDeque<Value>) ->anyhow::Result<bool>{
        let var1 = if let Some(s) = vars.pop_front(){
            s
        }else{
            return Ok(false)
        };
        let comparator = if let Some(s) = vars.pop_front() {
            if let Value::String(s) = s {s}else{
                return return anyhow::anyhow!("comparator must is string[==,!=,>,>=,<,<=,is_null,no_null,contain,no_contain]").err()
            }
        }else{
            return anyhow::anyhow!("comparator is nil,format: var1 [comparator] var2").err()
        };
        if comparator == "is_null" {
            return match var1 {
                Value::Null => Ok(true),
                Value::String(s) => Ok(s.is_empty()),
                Value::Array(a) => Ok(a.is_empty()),
                Value::Object(o) => Ok(o.is_empty()),
                _=> Ok(false)
            };
        }else if comparator == "no_null" {
            return match var1 {
                Value::Null => Ok(false),
                Value::String(s) => Ok(!s.is_empty()),
                Value::Array(a) => Ok(!a.is_empty()),
                Value::Object(o) => Ok(!o.is_empty()),
                _=> Ok(true)
            };
        }
        let var2 = if let Some(s) = vars.pop_front(){
            s
        }else{
            return anyhow::anyhow!("var2 is nil,format: var1 [comparator] var2").err()
        };
        return match comparator.as_str() {
            "=="=> Ok(var1 == var2),
            "!="=> Ok(var1 != var2),
            ">"=>{
                if let Some(f1) = var1.as_f64(){
                    if let Some(f2) = var2.as_f64(){
                        return Ok(f1>f2)
                    }
                }
                Ok(false)
            }
            ">="=>{
                if let Some(f1) = var1.as_f64(){
                    if let Some(f2) = var2.as_f64(){
                        return Ok(f1>=f2)
                    }
                }
                Ok(false)
            }
            "<"=>{
                if let Some(f1) = var1.as_f64(){
                    if let Some(f2) = var2.as_f64(){
                        return Ok(f1<f2)
                    }
                }
                Ok(false)
            }
            "<="=>{
                if let Some(f1) = var1.as_f64(){
                    if let Some(f2) = var2.as_f64(){
                        return Ok(f1<f2)
                    }
                }
                Ok(false)
            }
            // todo "contain"=>{}
            // todo "no_contain"=>{}
            _=> {
                anyhow::anyhow!("unknown comparator[{}]",comparator).err()
            }
        }
    }
}

#[async_trait::async_trait]
impl agent_rt::ServiceLayer for SelectorService {
    type Config = CfgBound<SelectorServiceConfig>;
    type Output = Value;

    async fn call(&self, code: String, ctx: Arc<Context>, cfg: Self::Config) -> anyhow::Result<Self::Output> {
        let cfg = cfg.bound(&ctx)?;
        let mut result = false;
        let all_true = cfg.condition == "且";

        let mut vars = cfg.vars.into_iter().collect::<VecDeque<Value>>();
        loop {
            if vars.is_empty() {
                break
            }
            let ok = Self::judge(&mut vars)?;
            if !all_true && ok {
                result = true;
                break
            }else if all_true && !ok {
                result = false;
                break
            }else{
                if vars.is_empty() {
                    result = true;
                }
            }
        }
        let go_next_node = if result {
            cfg.true_goto
        }else{
            cfg.false_goto
        };
        ctx.plan.update(code.as_str(),Box::new(|x|{
            if let Some(p) = x{
                p.go = vec![go_next_node];
            }
        }));
        Ok(Value::Null)
    }
}
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SelectorServiceConfig{
    pub condition : String, //或，且
    //三段式 var1 [comparator] var2
    //comparator ==,!=,>,>=,<,<=,is_null,no_null,contain,no_contain
    // if comparator is [is_null,no_null], do not need var2
    pub vars : Vec<Value>,
    pub true_goto : String,
    pub false_goto: String,
}

#[cfg(test)]
mod test{
    use serde_json::Value;
    use wd_tools::PFArc;
    use agent_rt::{END_NODE_CODE, PlanBuilder, Runtime};
    use crate::rt_node_service::python::{PythonCodeService, PythonCodeServiceRequest};
    use crate::rt_node_service::selector::SelectorService;
    use crate::rt_node_service::VarFlowChartService;

    //cargo test rt_node_service::selector::test::test_selector_service -- --nocapture
    #[tokio::test]
    pub async fn test_selector_service(){

        let rt = Runtime::default()
            .register_service_layer("selector", SelectorService::default())
            .register_service_layer("flow_chart_var", VarFlowChartService::default())
            .launch();

        let cfg = serde_json::json!({
            "condition": "且",
            "vars" : [
                "{{start.code}}",
                "==",
                "branch_001",
                "{{start.len}}",
                ">=",
                1,
            ],
            "true_goto":"branch_001",
            "false_goto":"branch_002"
        });

        let plan = PlanBuilder::start::<_,String>(("select-01","selector",serde_json::to_string(&cfg).unwrap()),vec![])
            .fission_from_code("select-01",("branch_001","flow_chart_var",r#"{"result":"select node branch_001"}"#),vec![END_NODE_CODE])
            .fission_from_code("select-01",("branch_002","flow_chart_var",r#"{"result":"select node branch_002"}"#),vec![END_NODE_CODE])
            .end::<String,_>(vec![],("end","flow_chart_var",r#"{"result":"{{branch_001.result}}{{branch_002.result}}"}"#))
            .build();

        let output = rt.ctx("selector-test-001",plan)
            .arc()
            .block_on::<Value,_>(serde_json::json!({
                "code":"branch_001",
                "len":1
            })).await.unwrap();

        println!("--> {}",output);
    }
}