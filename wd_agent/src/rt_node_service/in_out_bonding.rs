use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use agent_rt::Context;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Var{
    pub var_name:String,
    pub from_position:String,
    pub default:Option<Value>,
}

pub trait VarFill{
    fn file_item(&mut self,var_name:String,value:Value)->anyhow::Result<()>;
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CfgBound<T:VarFill> {
    #[serde(default="Vec::new")]
    pub input: Vec<Var>,
    #[serde(flatten)]
    pub data: T,
}

impl<T:VarFill> CfgBound<T>{
    pub fn init(mut self, ctx:&Context) ->anyhow::Result<T>{
        self.init_var_from_ctx(ctx);
        let Self { input, mut data } = self;
        for i in input {
            if let Some(s) = i.default{
                data.file_item(i.var_name,s)?;
            }
        }
        Ok(data)
    }

    pub fn init_var_from_ctx(&mut self,ctx:&Context){
        let pos = self.input.iter().map(|x|x.from_position.as_str()).collect::<Vec<&str>>();
        let mut vals = vec![];
        for i in pos{
            if i.is_empty() {
                vals.push(Value::Null);
                continue
            }
            let mut ks = i.split(".").collect::<VecDeque<&str>>();
            let node_code = ks.pop_front().unwrap();
            let res = ctx.get_opt(node_code, |x:Option<&mut Value>|{
                if x.is_none() {
                    return None;
                }
                let x = x.unwrap();
                if let Some(s) = Self::find_value(ks,x) {
                    Some(s.clone())
                }else{
                    None
                }
            });
            if let Some(s) = res{
                vals.push(s);
            }else{
                vals.push(Value::Null);
            }
        }
        for (i,v) in self.input.iter_mut().zip(vals) {
            if v.is_null() && i.default.is_some() {
                continue
            }
            i.default = Some(v);
        }
    }

    fn find_value<'a>(mut key:VecDeque<&str>,val:&'a Value)->Option<&'a Value>{
        if key.is_empty() {
            return Some(val)
        }
        if let Value::Object(ref obj) = val{
            if let Some(val) = obj.get(key.pop_front().unwrap()) {
                return Self::find_value(key,val)
            }
        }
        None
    }
}

#[macro_export]
macro_rules! var_auto_inject {
    ($cfg:tt.$field:tt) => {
impl VarFill for $cfg{
    fn file_item(&mut self, var_name: String, value: Value) -> anyhow::Result<()> {
        if self.$field.is_empty() {
            return Ok(())
        }
        let var = format!("{{{{{var_name}}}");
        let val = match value {
            Value::Null => "".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if let Some(s) = n.as_u64() {
                    s.to_string()
                }else if let Some(s) = n.as_i64(){
                    s.to_string()
                }else if let Some(s) = n.as_f64() {
                    format!("{:.2}",s)
                }else{
                    unimplemented!()
                }
            }
            Value::String(s) => s,
            n @ _ => {
                return Err(anyhow::anyhow!("VarFill no support type:{:?}",n))
            }
        };
        self.$field = self.$field.replace(var.as_str(), val.as_str());
        Ok(())
    }
}
    };
}