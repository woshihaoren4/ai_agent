use std::collections::HashMap;
use serde::Deserialize;
use serde_json::Value;
use wd_tools::PFErr;
use crate::{Node, Plan, PlanNode, PlanResult};




#[derive(Clone,Default,Debug)]
pub struct GraphPlan {
    map:HashMap<String,PlanNode>,
    start_node_name:String,
    end_node_name:String,
}

impl GraphPlan{
    pub fn add_plan_node<N:Into<PlanNode>>(&mut self,node:N){
        let node = node.into();
        self.insert(node)
    }
    pub fn add_exec_node<N:Into<Node>>(&mut self,node:N){
        let node = node.into();
        self.add_plan_node((vec![],node,vec![]))
    }
    pub fn must_set_node_from<S:Into<String>,I:Iterator<Item=S>>(&mut self,name:&str, from:I){
        let from = from.map(|x|x.into()).collect::<Vec<_>>();
        if let Some(s) = self.map.get_mut(name){
            s.from = from;
        }
    }
    pub fn must_set_node_to<S:Into<String>,I:Iterator<Item=S>>(&mut self,name:&str, to:I){
        let to = to.map(|x|x.into()).collect::<Vec<_>>();
        if let Some(s) = self.map.get_mut(name){
            s.to = to;
        }
    }
    pub fn get_node_mut(&mut self,name:&str)->Option<&mut PlanNode>{
        self.map.get_mut(name)
    }
}

impl TryFrom<&str> for GraphPlan {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut graph = GraphPlan::default();
        GraphPlanBuilder::from_text(value,&mut graph)?;
        Ok(graph)
    }
}

impl Plan for GraphPlan {
    fn start_node_name(&self) -> &str {
        self.start_node_name.as_str()
    }
    fn end_node_name(&self) -> &str {
        self.end_node_name.as_str()
    }
    fn get(&mut self, name: &str) -> Option<&PlanNode> {
        self.map.get(name)
    }
    fn next(&mut self, name: &str) -> anyhow::Result<PlanResult> {
        if name == self.end_node_name.as_str() {
            return Ok(PlanResult::End)
        }
        let node = match self.map.get(name) {
            None => {
                return anyhow::anyhow!("GraphPlan.Node[{name}] not found").err()
            }
            Some(s) => s,
        };
        if node.to.is_empty() {
            return anyhow::anyhow!("GraphPlan.Node.next_nodes is empty").err()
        }
        let mut plan_nodes = vec![];
        for i in node.to.iter() {
            if let Some(n) = self.map.get(i) {
                for j in 0..n.from.len() {
                    if n.from[j].as_str() == i {
                        n.from.remove(j);
                        break
                    }
                }
            }else{
                return anyhow::anyhow!("GraphPlan.Node[{i}] not found").err()
            }
        }
        Ok(PlanResult::Wait)
    }
    fn remove(&mut self, name: &str) -> Option<PlanNode> {
        self.remove(name)
    }
    fn insert(&mut self, node: PlanNode) {
        self.map.insert(node.node.name.clone(),node);
    }
}

/// ### Example of declaring an execution plan in text
/// ```text
/// // This is a annotation
/// [setting]:toml:
///
///
///
/// // node setting,decoder name(json,yaml,toml,default is json), service, node name
/// [node]::service_1:node_a,node_b,node_c
/// {
///     "key1":"val1"
///     "key2":true,
/// }
/// [node]:toml:service_2:node_d:
/// key1="value1"
/// key2=110
///
/// // flow setting
/// // You can have multiple start nodes, but only one end node.
/// [flow]:flow_name_1
/// node_a -> node_b,node_c
/// node_b,node_c -> node_d
///
///
/// [flow]:flow_name_2
/// node_a,node_b -> node_c
/// node_c -> node_d
/// ```

macro_rules! text_builder_error {
    ($line:expr,$($arg:tt)*) => {
        anyhow::anyhow!("GraphPlanBuilder error,line:{},error:{}",$line,format!($($arg)*))
    };
}

pub struct GraphPlanBuilder;

#[derive(Deserialize)]
#[serde(default)]
struct GraphPlanBuilderSetting{
    start_node:String,
    end_node:String,
}
impl Default for GraphPlanBuilderSetting {
    fn default() -> Self {
        Self{
            start_node:"start".into(),
            end_node:"end".into()
        }
    }
}

#[derive(Default)]
enum GraphPlanBuilderEnum{
    Setting(String),
    Node(Vec<String>,String,String,String), //node_name,decoder name,service,content
    Flow(String,Vec<(String,String)>), //flow_name, flow_config
    #[default]
    None,
}
impl GraphPlanBuilderEnum {
    pub fn assemble(&mut self,graph:&mut GraphPlan,mut new_builder:GraphPlanBuilderEnum)->anyhow::Result<()>{
        std::mem::swap(self,&mut new_builder);
        match new_builder {
            GraphPlanBuilderEnum::Setting(config) => {
                let setting = toml::from_str::<GraphPlanBuilderSetting>(config.as_str())?;
                graph.start_node_name = setting.start_node;
                graph.end_node_name = setting.end_node;
            }
            GraphPlanBuilderEnum::Node(nodes_name,decode,service, config) => {
                let value = match decode.to_lowercase().as_str() {
                    "" | "json"=>{
                        serde_json::from_str::<Value>(config.as_str())?
                    }
                    "toml"=>{
                        toml::from_str::<Value>(config.as_str())?
                    }
                    "yaml"=>{
                        serde_yaml::from_str::<Value>(config.as_str())?
                    }
                    "custom"=>{
                        Value::String(config)
                    }
                    _=>{
                        return anyhow::anyhow!("unknown decode format:{decode}").err()
                    }
                };
                for i in nodes_name{
                    graph.add_exec_node(Node::new(i).set_service_name(service.as_str()).set_value(value.clone()));
                }
            }
            GraphPlanBuilderEnum::Flow(_flow_name, nodes) => {
                for (f,t) in nodes.into_iter() {
                    if let Some(s) = graph.get_node_mut(f.as_str()) {
                        if !s.to.contains(&t) {
                            s.to.push(t.clone());
                        }
                    }else{
                        return anyhow::anyhow!("not found node[{f}]").err()
                    }
                    if let Some(s) = graph.get_node_mut(t.as_str()) {
                        if !s.from.contains(&f) {
                            s.from.push(f)
                        }
                    }else{
                        return anyhow::anyhow!("not found node[{t}]").err()
                    }
                }
            }
            GraphPlanBuilderEnum::None => {
                let setting = crate::graph_plan::GraphPlanBuilderSetting::default();
                graph.start_node_name = setting.start_node;
                graph.end_node_name = setting.end_node;
            }
        }
        Ok(())
    }
    pub fn in_setting(&mut self,graph:&mut GraphPlan)->anyhow::Result<()>{
        self.assemble(graph,GraphPlanBuilderEnum::Setting("".into()))
    }
    pub fn in_node<D:Into<String>,S:Into<String>>(&mut self,nodes:Vec<String>,decoder:D,service:S,graph:&mut GraphPlan)->anyhow::Result<()>{
        self.assemble(graph,GraphPlanBuilderEnum::Node(nodes,decoder.into(),service.into(),"".into()))
    }
    pub fn in_flow<S:Into<String>>(&mut self,name:S,graph:&mut GraphPlan)->anyhow::Result<()>{
        self.assemble(graph,GraphPlanBuilderEnum::Flow(name.into(),vec![]))
    }
    pub fn push_line(&mut self,line:&str)->anyhow::Result<()>{
        match self {
            GraphPlanBuilderEnum::Setting(cfg) => {
                if !cfg.is_empty() {
                    cfg.push_str("\n")
                }
                cfg.push_str(line)
            }
            GraphPlanBuilderEnum::Node(_, _, _, cfg) => {
                if !cfg.is_empty() {
                    cfg.push_str("\n")
                }
                cfg.push_str(line)
            }
            GraphPlanBuilderEnum::Flow(_, nodes) => {
                let list = line.split("->").collect::<Vec<_>>();
                let len = list.len();
                for i in 0..len-1{
                    nodes.push((list[i].to_string(),list[i+1].to_string()));
                }
            }
            GraphPlanBuilderEnum::None => {}
        }
        Ok(())
    }
}

impl GraphPlanBuilder{
    pub fn from_text(txt:&str,graph:&mut GraphPlan)->anyhow::Result<()>{
        let mut builder = GraphPlanBuilderEnum::default();
        let lines = txt.split("\n").collect::<Vec<_>>();
        for (i,line) in lines.into_iter().enumerate(){
            if line.starts_with("//") {
                //注释
                continue
            }else if line.starts_with("[setting]:") {
                builder.in_setting(graph)?
            }else if line.starts_with("[node]:") {
                let split = line.split(":").collect::<Vec<_>>();
                if split.len() < 4 {
                    return text_builder_error!(i,"node format error, ex:[node]:json:service_name:node_name_1,node_name_2:").err()
                }
                let nodes = split[3].split(",").map(|x|x.to_string()).collect::<Vec<_>>();
                builder.in_node(nodes,split[1],split[2],graph)?
            }else if line.starts_with("[flow]:"){
                builder.in_flow("",graph)?
            }else{
                builder.push_line(line)?;
                // return text_builder_error!(i,"Unknown identifier").err()
            }
        }
        builder.assemble(graph,Default::default())
    }
}
