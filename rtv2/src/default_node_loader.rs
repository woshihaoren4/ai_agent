use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, RwLock};
use crate::{Service, ServiceLoader};

#[derive(Default)]
pub struct DefaultNodeLoader{
    map:RwLock<HashMap<String,Arc<dyn Service>>>,
}

impl ServiceLoader for DefaultNodeLoader{
    fn get(&self, ids: &str) -> Option<Arc<dyn Service>> {
        let map = match self.map.read(){
            Ok(o)=>o,
            Err(e)=>{
                wd_log::log_error_ln!("DefaultNodeLoader,get error:{}",e);
                return None
            }
        };
        if let Some(a) = map.get(ids){
            Some(a.clone())
        }else{
            None
        }
    }

    fn set(&self, nodes: Vec<(String, Arc<dyn Service>)>) {
        let mut map = match self.map.write(){
            Ok(o)=>o,
            Err(e)=>{
                wd_log::log_error_ln!("DefaultNodeLoader,set error:{}",e);
                return
            }
        };
        for (k,v) in nodes{
            map.insert(k,v);
        }
    }
}