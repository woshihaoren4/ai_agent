use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use serde::Deserialize;
use serde_json::Value;
use wd_tools::{PFBox, PFErr};
use crate::{Node, Plan, PlanResult, GRAPH_PLAN_BUILDER_ANNOTATION,GRAPH_PLAN_BUILDER_FLOW, GRAPH_PLAN_BUILDER_NODE, GRAPH_PLAN_BUILDER_SETTING};

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

    fn show_next(&self,node:&str, s:&mut String,index:Vec<bool>,have:&mut HashSet<String>){
        if let Some(n) = self.graph.get(node) {
            let len = n.goto.len();
            s.push_str(format!("{}:{}({})\n",len,node,n.node_name).as_str());
            if have.contains(node) {
                return;
            }
            have.insert(node.to_string());
            for (i,next) in n.goto.iter().enumerate() {
                for i in index.iter() {
                    if *i {
                        s.push_str("│   ");
                    }else{
                        s.push_str("    ");
                    }
                }
                let index = if i == len - 1 {
                    s.push_str("└── ");
                    let mut vec = index.clone();
                    vec.push(false);vec
                }else{
                    s.push_str("├── ");
                    let mut vec = index.clone();
                    vec.push(true);vec
                };
                self.show_next(next,s,index,have);
            }
        }else{
            s.push_str("None\n");
        }
    }
}

impl TryFrom<&str> for GraphPlan {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut builder =  GraphPlanBuilder::default();
        builder.parse(value)?;
        Ok(builder.build())
    }
}
impl Display for GraphPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s= String::new();
        let mut have = HashSet::new();
        let index = vec![];
        self.show_next(self.start_node_name.as_str(),&mut s,index,&mut have);
        write!(f, "{}", s)
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
        let node_goto_ns = if let Some(node) = self.graph.get(name){
            node.goto.clone()
        }else{
            return anyhow::anyhow!("GraphPlan.next GraphNode[{name}] not found").err()
        };
        let mut goto = vec![];
        'loop_nodes: for i in node_goto_ns{
            let goto_node = if let Some(s) = self.graph.get_mut(i.as_str()) {s}else{
                return anyhow::anyhow!("GraphPlan.next GraphNode[{i}] not found").err()
            };
            goto_node.over_nodes.insert(name.to_string());
            //判断条件是否达成
            for j in goto_node.goto_condition.iter(){
                if !goto_node.over_nodes.contains(j) {
                    continue 'loop_nodes
                }
            }
            goto_node.over_nodes.clear();
            if let Some(s) = self.nodes.get(goto_node.node_name.as_str()) {
                goto.push(s.clone())
            }else{
                return anyhow::anyhow!("GraphPlan.next Node[{}] not found",goto_node.node_name).err()
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

#[derive(Clone,Debug,Deserialize)]
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
#[derive(Debug)]
pub struct GraphPlanBuilder{
    setting : GraphPlanBuilderSetting,
    nodes:HashMap<String,Node>,
    graph:HashMap<String,GraphNode>,
    flows: HashMap<String,(Vec<String>,Vec<String>)>,
    // 0:none 1:setting 2:node 3:flow
    on_type: i8,
    on_node: Vec<Node>,
    on_node_format:String,
    on_node_config: String,
    on_setting: String,
    // from node name, true:necessary false:unnecessary, to node name
    on_flow_name:String,
    on_flow: Vec<GraphPlanBuilderFlow>,
}

impl Default for GraphPlanBuilder {
    fn default() -> Self {
        let nodes = HashMap::from([("start".to_string(),Node::new("start"))]);
        Self{
            nodes,
            setting: Default::default(),
            graph: Default::default(),
            flows: Default::default(),
            on_type: 0,
            on_node: vec![],
            on_node_format: "".to_string(),
            on_node_config: "".to_string(),
            on_setting: "".to_string(),
            on_flow_name: "".to_string(),
            on_flow: vec![],
        }
    }
}

impl GraphPlanBuilder {
    fn add_goto_graph_node(map:&mut HashMap<String,GraphNode>,from_name:String,from_node:&str,to_name:String,to_node:&str,goto_need_verify:bool){
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
    fn add_flow(&mut self,flow_name:&str,flow:GraphPlanBuilderFlow)->anyhow::Result<()>{
        //校验
        if !flow.from_is_flow {
            if !self.nodes.contains_key(flow.from.as_str()) {
                return anyhow::anyhow!("Node[{}] not found",flow.from).err()
            }
        }else {
            if let Some((_start,end)) = self.flows.get(flow.from.as_str()) {
                for i in end{
                    if let Some(n) = self.graph.get(i) {
                        if !self.nodes.contains_key(n.node_name.as_str()) {
                            return anyhow::anyhow!("Node[{}] not found",n.node_name).err()
                        }
                    }else{
                        return anyhow::anyhow!("Graph[{}] not found",i).err()
                    }
                }
            }else{
                return anyhow::anyhow!("Flow[{}] not found",flow.from).err()
            }
        }
        if !flow.to_is_flow{
            if !self.nodes.contains_key(flow.to.as_str()) {
                return anyhow::anyhow!("Node[{}] not found",flow.to).err()
            }
        }else{
            if let Some((start,_end)) = self.flows.get(flow.to.as_str()) {
                for i in start{
                    if let Some(n) = self.graph.get(i) {
                        if !self.nodes.contains_key(n.node_name.as_str()) {
                            return anyhow::anyhow!("Node[{}] not found",n.node_name).err()
                        }
                    }else{
                        return anyhow::anyhow!("Graph[{}] not found",i).err()
                    }
                }
            }else{
                return anyhow::anyhow!("Flow[{}] not found",flow.to).err()
            }
        }
        //组装
        if !flow.from_is_flow && !flow.to_is_flow { //两个节点
            Self::add_goto_graph_node(&mut self.graph,flow.from_name(flow_name),flow.from.as_str(),flow.to_name(flow_name),flow.to.as_str(),flow.goto_need_verify);
        }else if flow.from_is_flow && !flow.to_is_flow { //flow->node
            let (_start,end) = self.flows.get(flow.from.as_str()).unwrap();
            for i in end{
                Self::add_goto_graph_node(&mut self.graph,i.to_string(),"",flow.to_name(flow_name),flow.to.as_str(),true);
            }
        }else if !flow.from_is_flow && flow.to_is_flow { //node->flow
            let (start,_end) = self.flows.get(flow.to.as_str()).unwrap();
            for i in start{
                Self::add_goto_graph_node(&mut self.graph,flow.from_name(flow_name),flow.from.as_str(),i.to_string(),"",flow.goto_need_verify);
            }
        }else{ //flow->flow
            let (_start,end) = self.flows.get(flow.from.as_str()).unwrap();
            let (start,_end) = self.flows.get(flow.to.as_str()).unwrap();
            for i in end{
                for j in start{
                    Self::add_goto_graph_node(&mut self.graph,i.to_string(),"",j.to_string(),"",true);
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
                let nodes = std::mem::take(&mut self.on_node);
                let format = std::mem::take(&mut self.on_node_format);
                let config = std::mem::take(&mut self.on_node_config);
                let config = match format.to_lowercase().as_str() {
                    ""|"json"=>{
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
                for i in flows{
                    self.add_flow(flow_name.as_str(),i)?;
                }
            }
            _=>{
                return anyhow::anyhow!("unknown parse type[{}]",self.on_type).err()
            }
        };
        self.on_type = ty;
        Ok(())
    }
    fn start_parse_setting(&mut self,_line:&str)->anyhow::Result<()>{
        // let list = Self::ratasbc(line,GRAPH_PLAN_BUILDER_SETTING);
        self.change_type(1)
    }
    fn start_parse_node(&mut self,line:&str)->anyhow::Result<()>{
        self.change_type(2)?;
        let list = Self::ratasbc(line,GRAPH_PLAN_BUILDER_NODE);
        if list.len() != 3 {
            return anyhow::anyhow!("node header format incorrectness，ex：[node]:service_name:format_name(json,toml,yaml,custom):node_name1,node_name2").err()
        }
        let format_name = list[0].to_lowercase();
        let service_name = list[1].as_str();
        let mut nodes = list[2].split(",").map(|s|s.to_string()).collect::<Vec<_>>();

        if format_name != "" && format_name != "json" && format_name != "toml" && format_name != "yaml" && format_name != "custom" {
            return anyhow::anyhow!("unknown node config format[{format_name}]").err()
        }
        if nodes.is_empty() || (nodes.len() == 1 && nodes[0].is_empty()) {
            nodes = vec![service_name.to_string()];
        }

        self.on_node_format = format_name;
        for i in nodes {
            self.on_node.push(Node::new(i).set_service_name(service_name.clone()))
        }
        Ok(())
    }
    fn start_parse_flow(&mut self,line:&str)->anyhow::Result<()>{
        self.change_type(3)?;
        let list = Self::ratasbc(line,GRAPH_PLAN_BUILDER_FLOW);
        if list.len() != 3 {
            return anyhow::anyhow!("node header format incorrectness，ex：[flow]:flow_name:start_node1,start_node2:end_node1,end_node2").err()
        }
        self.on_flow_name = list[0].to_string();
        if self.flows.contains_key(&self.on_flow_name) {
            return anyhow::anyhow!("flow name[{}] repeat",list[0]).err();
        }
        let start_nodes = if list[1].is_empty() {
            vec!["start".to_string()]
        }else{
            list[1].split(",").map(|s|Self::make_graph_name(&self.on_flow_name,s)).collect::<Vec<_>>()
        };
        let end_nodes = if list[2].is_empty() {
            vec!["end".to_string()]
        }else{
            list[2].split(",").map(|s|Self::make_graph_name(&self.on_flow_name,s)).collect::<Vec<_>>()
        };
        self.flows.insert(list[0].to_string(),(start_nodes,end_nodes));
        Ok(())
    }
    fn parse_body(&mut self,line:&str)->anyhow::Result<()>{
        match self.on_type {
            0=>{}
            1=>{
                let s = line.replace("//","#");
                if !self.on_setting.is_empty() {
                    self.on_setting.push_str("\n");
                }
                self.on_setting.push_str(s.as_str());
            }
            2=>{
                if !self.on_node_config.is_empty() {
                    self.on_node_config.push_str("\n");
                }
                self.on_node_config.push_str(line);
            }
            3=>{
                let line = line.replace(" ","");
                if line.is_empty() {
                    return Ok(())
                }
                let list = line.split("->").collect::<Vec<_>>();
                if list.len() < 2 {
                    return anyhow::anyhow!("flow: there are at least two nodes").err()
                }
                for i in 0..list.len()-1{
                    let from_ns = list[i].split(",").collect::<Vec<_>>();
                    let to_ns = list[i+1].split(",").collect::<Vec<_>>();
                    for f in from_ns {
                        let (from_name,from_is_flow,_) = Self::flow_node_name(f);
                        for t in to_ns.iter(){
                            let (to_name,to_is_flow,goto_need_verify) = Self::flow_node_name(*t);
                            self.on_flow.push(GraphPlanBuilderFlow{
                                from_is_flow,
                                to_is_flow,
                                goto_need_verify,
                                from: from_name.to_string(),
                                to: to_name.to_string(),
                            })
                        }
                    }
                }
            }
            _=>{
                return anyhow::anyhow!("unknown GraphPlanBuilder.type [{}]",self.on_type).err()
            }
        }
        Ok(())
    }
    fn flow_node_name(from_name_all:&str)->(&str,bool,bool){ //name,is flow,to need
        if from_name_all.starts_with(".") {
            let name = &from_name_all[1..];
            return (name,true,true)
        }else if from_name_all.starts_with("_"){
            let name = &from_name_all[1..];
            return (name,false,true)
        }
        (from_name_all,false,true)
    }
    fn make_graph_name(flow_name: &str,graph_node_name:&str)->String{
        if flow_name.is_empty() {
            graph_node_name.to_string()
        }else{
            format!("{}.{}",flow_name,graph_node_name)
        }
    }
    #[inline]
    fn ratasbc<'a>(input: &'a str,title:&'a str) -> Vec<String> {
        Self::rm_annotation_title_space_and_split_by_col(input,title)
    }
    fn rm_annotation_title_space_and_split_by_col<'a>(input: &'a str,title:&'a str) -> Vec<String> {
        let list = input.splitn(2,"//").collect::<Vec<_>>();
        let line = list[0].trim_start_matches(title).replace(" ","");
        line.split(":").map(|s|s.into()).collect::<Vec<_>>()
    }
}


impl GraphPlanBuilder{
    pub fn error(index:usize,err:anyhow::Error)->anyhow::Result<()>{
        anyhow::anyhow!("GraphPlanBuilder:line[{index}] error:{err}").err()
    }
    pub fn parse(&mut self,txt:&str)->anyhow::Result<()>{
        let lines = txt.split("\n").collect::<Vec<_>>();
        for (i,s) in lines.into_iter().enumerate(){
            if s.starts_with(GRAPH_PLAN_BUILDER_ANNOTATION){
                    continue
            }else if s.starts_with(GRAPH_PLAN_BUILDER_SETTING){
                if let Err(e) = self.start_parse_setting(s){
                    return Self::error(i,e)
                }
            }else if s.starts_with(GRAPH_PLAN_BUILDER_NODE){
                if let Err(e) = self.start_parse_node(s) {
                    return Self::error(i,e)
                }
            }else if s.starts_with(GRAPH_PLAN_BUILDER_FLOW){
                if let Err(e) = self.start_parse_flow(s){
                    return Self::error(i,e)
                }
            }else{
                if let Err(e) = self.parse_body(s){
                    return Self::error(i,e)
                }
            }
        }
        if let Err(e) = self.change_type(0){
            return Self::error(u32::MAX as usize,e)
        }
        Ok(())
    }
    pub fn build(self)->GraphPlan{
        GraphPlan{
            nodes: self.nodes,
            graph: self.graph,
            start_node_name: self.setting.start_node,
            end_node_name: self.setting.end_node,
        }
    }
}

#[cfg(test)]
mod test{
    use crate::GraphPlan;

    const SINGLE_CASE:&'static str = r#"
// 设置
[setting]::
start_node = "main_flow.a" // default is "start"
// end_node = "f" default is "end"
// 节点
[node]::service_1:a,b,c
{
    "key1":"val1",
    "key2":true
}
[node]:toml:service_2:d,e,f,end
key1="value1"
key2=110

// 流程1
[flow]:flow_1:a:f
a -> b,c -> d
d->_e->f

// 流程2
[flow]:main_flow::
a->d->.flow_1->end
    "#;

    #[test]
    fn test_graph_plan_build(){
        let plan = GraphPlan::try_from(SINGLE_CASE).unwrap();
        println!("{}",plan);
    }
}