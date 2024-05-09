use std::collections::HashMap;
use crate::{Node, START_NODE_CODE};


pub struct PlanBuilder{
    map:HashMap<String,Vec<Node>>
}

impl PlanBuilder{
    pub fn start<N:Into<Node>>(node:N)->Self{
        let mut node = node.into();
        node.code = START_NODE_CODE.to_string();
        let mut map = HashMap::new();
        map.insert(START_NODE_CODE.to_string(),vec![node.into()]);
        Self{map}
    }
    pub fn add_node<S:Into<String>,N:Into<Node>>(&mut self,parent_code:S,node:N)->&mut Self{
        let code = parent_code.into();
        let node = node.into();
        if let Some(ns) = self.map.get_mut(code.as_str()){
             ns.push(node);
        }else{
            self.map.insert(code,vec![node]);
        }
        self
    }
}