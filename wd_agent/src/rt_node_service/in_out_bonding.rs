use std::collections::{ VecDeque};
use std::marker::PhantomData;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use wd_tools::{PFErr, SimpleRegexMatch};
use agent_rt::Context;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CfgBound<T>{
    #[serde(flatten)]
    pub inner:Value,
    #[serde(skip)]
    pub _p :PhantomData<T>,
}

impl<T> CfgBound<T>
    where T:for<'a> serde::Deserialize<'a>
{
    pub fn raw_bound_value(self,ctx:&Context)->anyhow::Result<Value>{
        Self::bound_value(self.inner, ctx)
    }
    pub fn bound(self,ctx:&Context)->anyhow::Result<T>
    {
        let value = self.raw_bound_value(ctx)?;
        let t:T = serde_json::from_value(value)?;Ok(t)
    }
    fn bound_value(value:Value,ctx:&Context)->anyhow::Result<Value>{
        match value {
            // Value::Null => {}
            // Value::Bool(_) => {}
            // Value::Number(_) => {}
            Value::String(mut s) => {
                let a = s.as_str();
                let list = Self::string_type(a).into_iter().map(|x|x.to_string()).collect::<Vec<String>>();
                if list.is_empty() {
                    return Ok(Value::String(s))
                }
                if list[0].len() + 4 == s.len() {
                    if let Some(s) = Self::get_value_from_ctx(list[0].as_str(),ctx) {
                        return Ok(s)
                    }
                    return anyhow::anyhow!("not found var[{}]",list[0]).err()
                }
                for i in list {
                    let val = if let Some(val) = Self::get_value_from_ctx(i.as_str(),ctx) {
                      val
                    }else{
                        return anyhow::anyhow!("not found var[{}]",i).err()
                    };
                    s = s.replace(format!("{{{{{i}}}}}").as_str(),val.as_str().unwrap_or(""));
                }
                Ok(Value::String(s))
            }
            Value::Array(list) => {
                let mut vec = vec![];
                for i in list {
                    let val = Self::bound_value(i, ctx)?;
                    vec.push(val);
                }
                Ok(Value::Array(vec))
            }
            Value::Object(obj) => {
                let mut map = Map::new();
                for (k,v) in obj {
                    let val = Self::bound_value(v, ctx)?;
                    map.insert(k,val);
                }
                Ok(Value::Object(map))
            }
            _=>{
                Ok(value)
            }
        }
    }
    fn string_type(s: &str) ->Vec<&str> {
        let list = s.regex(r"\{\{(.*?)\}\}").unwrap_or(vec![]);
        list
    }
    fn get_value_from_ctx(pos:&str,ctx:&Context) ->Option<Value> {
        let mut ks = pos.split(".").collect::<VecDeque<&str>>();
        let code = ks.pop_front()?;
        let res = ctx.get_opt(code,|x:Option<&mut Value>|{
            let mut x = x?;
            loop {
                if let Some(key) = ks.pop_front() {
                    match x {
                        Value::Array(ref mut list) => {
                            if let Ok(index) = usize::from_str(key) {
                                if let Some(val) = list.get_mut(index) {
                                    x = val;
                                    continue
                                }
                            }
                        }
                        Value::Object(ref mut obj) => {
                            if let Some(val) = obj.get_mut(key) {
                                x = val;
                                continue
                            }
                        }
                        _=>{
                            return None
                        }
                    }
                    return None
                }else{
                    return Some(x.clone());
                }
            }
        });
        return res
    }
}

#[cfg(test)]
mod test{
    use serde::{Deserialize, Serialize};
    use agent_rt::PlanBuilder;
    use crate::rt_node_service::CfgBound;

    #[derive(Debug, Default, Clone, Deserialize, Serialize)]
    pub struct TestConfig{
        #[serde(default)]
        pub prompt:String,

        pub nb:f32,
        pub open:bool,
        pub query:String,
        pub list:Vec<isize>
    }

    #[test]
    fn test_cfg_bound(){
        let json1 = serde_json::json!({
           "hello":"world",
            "number":1,
            "list":[2,3,4],
            "map":{
                "a":true
            }
        });
        let json2 = serde_json::json!({
           "key":"this is a key",
            "float":2.1
        });

        let rt = agent_rt::Runtime::default().launch();
        let ctx = rt.ctx("test01",PlanBuilder::single_node("1","").build());
        ctx.set("j1",json1);
        ctx.set("j2",json2);

        let cb:CfgBound<TestConfig> = serde_json::from_str(r#"{
        "prompt":"j1.hello:{{j1.hello}}",
        "nb":"{{j2.float}}",
        "open":"{{j1.map.a}}",
        "query":"{{j2.key}}",
        "list":"{{j1.list}}"
        }"#).unwrap();

        let tc = cb.bound(&ctx).unwrap();
        println!("{tc:?}");

        assert_eq!("j1.hello:world",tc.prompt.as_str());
        assert_eq!(2.1f32,tc.nb);
        assert_eq!(true,tc.open);
        assert_eq!("this is a key",tc.query);
        assert_eq!(3,tc.list.len());

    }
}