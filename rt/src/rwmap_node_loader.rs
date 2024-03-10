use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::{Node, NodeLoader};

#[derive(Default)]
pub struct RWMapNodeLoader{
    map:RwLock<HashMap<String,Arc<dyn Node>>>,
}

impl NodeLoader for RWMapNodeLoader{
     fn get(&self, ids:&str ) ->anyhow::Result<Arc<dyn Node>> {
         let read = match  self.map.read() {
             Ok(o)=>o,
             Err(e) => {
                 return Err(anyhow::anyhow!("RWMapNodeLoader.get.lock failed {}",e))
             }
         };
        return if let Some(s) = read.get(ids) {
            Ok(s.clone())
        } else {
            Err(anyhow::anyhow!("node[{}] not found",ids))
        }
    }

     fn set(&self, nodes:Vec<(String,Arc<dyn Node>)>) {
        let mut write = self.map.write().unwrap();
        for (k,v) in nodes{
            write.insert(k,v);
        }
    }
}