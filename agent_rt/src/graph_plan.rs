use std::collections::{HashMap, HashSet};
use serde::Deserialize;
use serde_json::Value;
use wd_tools::PFErr;
use crate::{Node, Plan, PlanResult};

#[derive(Clone,Default,Debug)]
pub struct GraphNode{
    pub node_name:String,
    pub over_nodes:HashSet<String>,
    pub goto_condition: Vec<String>,
    pub goto:Vec<String>,
}
impl GraphNode {
    pub fn new<N:Into<String>>(node_name:N)->Self{
        let mut gn = GraphNode::default();
        gn.node_name = node_name.into();
        gn
    }
    pub fn add_goto<N:Into<String>>(&mut self,name:N){
        let name = name.into();
        if !self.goto.contains(&name) {
            self.goto.push(name)
        }
    }
    pub fn add_goto_cond<N:Into<String>>(&mut self,name:N){
        let name = name.into();
        if !self.goto_condition.contains(&name) {
            self.goto_condition.push(name)
        }
    }
}

#[derive(Clone,Default,Debug)]
pub struct GraphPlan {
    nodes:HashMap<String,Node>,
    graph:HashMap<String,GraphNode>,
    start_node_name:String,
    end_node_name:String,
}

impl GraphPlan{
    pub fn add_plan_node<N:Into<Node>>(&mut self,node:N){
        let node = node.into();
        self.insert(node)
    }
    pub fn add_graph_node<S:Into<String>,N:Into<GraphNode>>(&mut self,name:S, node:N)->Option<GraphNode>{
        let node = node.into();
        self.graph.insert(name.into(),node)
    }
}

impl TryFrom<&str> for GraphPlan {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut graph = GraphPlan::default();
        GraphPlanBuilder::default().from_text(value,&mut graph)?;
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
    fn get(&mut self, name: &str) -> Option<&Node> {
        self.nodes.get(name)
    }
    fn next(&mut self, name: &str) -> anyhow::Result<PlanResult> {
        if name == self.end_node_name {
            return Ok(PlanResult::End)
        }
        let node = if let Some(node) = self.graph.get(name){
            node
        }else{
            return anyhow::anyhow!("GraphPlan.next GraphNode[{name}] not found").err()
        };
        let mut goto = vec![];
        'loop_nodes: for i in node.goto.iter(){
            let goto_node = if let Some(s) = self.graph.get_mut(i) {s}else{
                return anyhow::anyhow!("GraphPlan.next GraphNode[{i}] not found").err()
            };
            goto_node.over_nodes.insert(i.to_string());
            //判断条件是否达成
            for j in goto_node.goto_condition.iter(){
                if !goto_node.over_nodes.contains(j) {
                    continue 'loop_nodes
                }
            }
            goto_node.over_nodes.clear();
            if let Some(s) = self.nodes.get(i) {
                goto.push(s.clone())
            }else{
                return anyhow::anyhow!("GraphPlan.next Node[{i}] not found").err()
            }
        }
        if goto.is_empty() {
            return Ok(PlanResult::Wait)
        }
        Ok(PlanResult::Nodes(goto))
    }
    fn remove(&mut self, name: &str) -> Option<Node> {
        todo!()
    }
    fn insert(&mut self, node: Node) {
        todo!()
    }
}

/// ### Example of declaring an execution plan in text
/// ```text
/// // This is a annotation
/// [setting]::
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
/// [flow]:flow_name_2::
/// node_a,node_b -> node_c
/// node_c -> node_d
/// ```

macro_rules! text_builder_error {
    ($line:expr,$($arg:tt)*) => {
        anyhow::anyhow!("GraphPlanBuilder error,line:{},error:{}",$line,format!($($arg)*))
    };
}

#[derive(Clone,Debug,Default,Deserialize)]
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
#[derive(Debug,Default,Clone)]
struct GraphPlanBuilderFlow{
    from_is_flow : bool,
    from:String,
    to_is_flow: bool,
    to:String,
    goto_need_verify:bool,
}
impl  GraphPlanBuilderFlow {
    pub fn from_name(&self,flow:&str)->String{
        GraphPlanBuilder::make_graph_name(flow,self.from.as_str())
    }
    pub fn to_name(&self,flow:&str)->String{
        GraphPlanBuilder::make_graph_name(flow,self.to.as_str())
    }
}
#[derive(Debug,Default)]
pub struct GraphPlanBuilder{
    setting : GraphPlanBuilderSetting,
    nodes:HashMap<String,Node>,
    graph:HashMap<String,GraphNode>,
    flows: HashMap<String,(Vec<String>,Vec<String>)>,
    // 0:none 1:setting 2:node 3:flow
    on_type: i8,
    on_node: (Vec<Node>,String),
    on_node_config: String,
    on_setting: String,
    // from node name, true:necessary false:unnecessary, to node name
    on_flow_name:String,
    on_flow: Vec<GraphPlanBuilderFlow>,
}

impl GraphPlanBuilder {
    fn add_goto_graph_node(map:&mut HashMap<String,GraphNode>,from_name:String,from_node:String,to_name:String,to_node:String,goto_need_verify:bool){
        //给to节点增加判断条件
        if goto_need_verify {
            if let Some(f) = map.get_mut(to_name.as_str()) {
                f.add_goto_cond(from_name.clone())
            }else{
                let mut graph = GraphNode::new(to_node);
                graph.add_goto_cond(from_name.as_str());
                map.insert(to_name.clone(),graph);
            }
        }else{
            if !map.contains_key(to_name.as_str()) {
                let graph = GraphNode::new(to_node);
                map.insert(to_name.clone(),graph);
            }
        }
        //给from节点增加goto
        if let Some(f) = map.get_mut(from_name.as_str()) {
            f.add_goto(to_name);
        }else{
            let mut graph = GraphNode::new(from_node);
            graph.add_goto(to_name);
            map.insert(from_name,graph);
        }
    }
    fn add_flow(&mut self,flow_name:&str,flow:&GraphPlanBuilderFlow)->anyhow::Result<()>{
        //校验
        if !flow.from_is_flow {
            if !self.nodes.contains_key(flow.from.as_str()) {
                return anyhow::anyhow!("Node[{from}] not found").err()
            }
        }else {
            if let Some((_start,end)) = self.flows.get(flow.from.as_str()) {
                for i in end{
                    if let Some(n) = self.graph.get(i) {
                        if !self.nodes.contains_key(n.node_name.as_str()) {
                            return anyhow::anyhow!("Node[{from}] not found").err()
                        }
                    }else{
                        return anyhow::anyhow!("Graph[{from}] not found").err()
                    }
                }
            }else{
                return anyhow::anyhow!("Flow[{from}] not found").err()
            }
        }
        if !flow.to_is_flow{
            if !self.nodes.contains_key(flow.to.as_str()) {
                return anyhow::anyhow!("Node[{from}] not found").err()
            }
        }else{
            if let Some((start,_end)) = self.flows.get(flow.from.as_str()) {
                for i in start{
                    if let Some(n) = self.graph.get(i) {
                        if !self.nodes.contains_key(n.node_name.as_str()) {
                            return anyhow::anyhow!("Node[{from}] not found").err()
                        }
                    }else{
                        return anyhow::anyhow!("Graph[{from}] not found").err()
                    }
                }
            }else{
                return anyhow::anyhow!("Flow[{from}] not found").err()
            }
        }
        //组装
        if !flow.from_is_flow && !flow.to_is_flow { //两个节点
            Self::add_goto_graph_node(&mut self.graph,flow.from_name(flow_name),flow.from.clone(),flow.to_name(flow_name),flow.to.clone(),flow.goto_need_verify);
        }else if flow.from_is_flow && !flow.to_is_flow { //flow->node
            let (_start,end) = self.flows.get(flow.from.as_str()).unwrap();
            for i in end{
                Self::add_goto_graph_node(&mut self.graph,i.to_string(),"".into(),flow.to_name(flow_name),flow.to.clone(),true);
            }
        }else if !flow.from_is_flow && flow.to_is_flow { //node->flow
            let (start,_end) = self.flows.get(flow.to.as_str()).unwrap();
            for i in start{
                Self::add_goto_graph_node(&mut self.graph,flow.from_name(flow_name),flow.from.clone(),i.to_string(),"".into(),flow.goto_need_verify);
            }
        }else{ //flow->flow
            let (_start,end) = self.flows.get(flow.from.as_str()).unwrap();
            let (start,_end) = self.flows.get(flow.to.as_str()).unwrap();
            for i in end{
                for j in start{
                    Self::add_goto_graph_node(&mut self.graph,i.to_string(),"".into(),j.to_string(),"".into(),true);
                }
            }
        }

        Ok(())
    }
    fn change_type(&mut self,ty:i8)->anyhow::Result<()>{
        match self.on_type {
            0=>{}
            1=>{
                self.setting = toml::from_str::<GraphPlanBuilderSetting>(self.on_setting.as_str())?;
                self.on_setting = String::new();
            }
            2=>{
                let (nodes,format) = std::mem::take(&mut self.on_node);
                let config = std::mem::take(&mut self.on_node_config);
                let config = match format.to_lowercase().as_str() {
                    ""|"json"=>{
                        serde_json::from_str::<Value>(self.on_node_config.as_str())?
                    }
                    "toml"=>{
                        toml::from_str::<Value>(self.on_node_config.as_str())?
                    }
                    "yaml"=>{
                        serde_yaml::from_str::<Value>(self.on_node_config.as_str())?
                    }
                    "custom"=>{
                        Value::String(config)
                    }
                    _=>{
                        return anyhow::anyhow!("unknown node format[{format}]").err()
                    }
                };
                for mut n in nodes{
                    self.nodes.insert(n.name.clone(),n.set_value(config.clone()));
                }
            }
            3=>{
                let flows = std::mem::take(&mut self.on_flow);
                let flow_name = std::mem::take(&mut self.on_flow_name);
                for (from,need,to) in flows{
                    if self.nodes.contains_key(from.as_str()) {
                        if let Some(f) = self.graph.get_mut(from.as_str()) {
                            if !f.goto.contains(&to) {
                                f.goto.push(to.clone());
                            }
                        }else{
                            self.graph.insert(Self::make_graph_name(flow_name.as_str(),from.as_str()),GraphNode::new(from.clone()).add_goto(to.clone()));
                        }
                    }else if let Some((_start,end)) = self.flows.get(from.as_str()){
                        for i in end.clone() {
                            if let Some(f) = self.graph.get_mut(from.as_str()) {
                                if !f.goto.contains(&to) {
                                    f.goto.push(to.clone());
                                }
                            }else{
                                self.graph.insert(Self::make_graph_name(flow_name.as_str(),from.as_str()),GraphNode::new(from.clone()).add_goto(to.clone()));
                            }
                        }
                    }else {
                        return anyhow::anyhow!("Node or Flow [{from}] not found").err()
                    }
                }
            }
            _=>{

            }
        };
        self.on_type = ty;
        Ok(())
    }
    fn start_setting(&self,line:&str)->anyhow::Result<()>{
        let list = Self::remove_annotation_title_and_split(line);

        Ok(())
    }
    fn make_graph_name(flow_name: &str,graph_node_name:&str)->String{
        if flow_name.is_empty() {
            graph_node_name.to_string()
        }else{
            format!("{}.{}",flow_name,graph_node_name)
        }
    }
    #[inline]
    fn ratasbc(input: &str,title:&str) -> Vec<&'_ str> {
        Self::rm_annotation_title_space_and_split_by_col(input,title)
    }
    fn rm_annotation_title_space_and_split_by_col(input: &str,title:&str) -> Vec<&'_ str> {
        let list = input.splitn(2,"//").collect::<Vec<_>>();
        let line = list[0].trim_start_matches(title).replace(" ","");
        line.split(":").collect::<Vec<_>>()
    }
}


impl GraphPlanBuilder{
    pub fn parse(&mut self,txt:&str)->anyhow::Result<()>{
        let lines = txt.split("\n").collect::<Vec<_>>();
        for i in lines {
            match i.to_lowercase().as_str()  {
                GRAPH_PLAN_BUILDER_ANNOTATION=> { //注释
                    continue
                }
                GRAPH_PLAN_BUILDER_SETTING=> {

                }
                GRAPH_PLAN_BUILDER_NODE=>{

                }
                GRAPH_PLAN_BUILDER_FLOW=>{

                }
                _=> {

                }
            }
        }
        Ok(())
    }
    pub fn build(self){

    }
}
