use std::collections::BTreeMap;
use egui::ahash::HashSet;
use egui::emath::TSTransform;
use egui::Pos2;

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Debug,Default)]
#[serde(default)]
pub struct State{
    pub open_window:HashSet<String>,
    pub layout_config:AppLayoutConfig,
    pub setting:Setting,
    pub plugin: Plugin,
    pub work_plan: WorkPlan,
}

impl State {
    pub fn contain_window(&self,key:&str)->bool{
        self.open_window.contains(key)
    }
    pub fn set_open_window(&mut self,key:&str,res:bool){
        if res{
            if !self.open_window.contains(key) {
                self.open_window.insert(key.to_string());
            }
        }else{
            self.open_window.remove(key);
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug,Default)]
pub struct AppLayoutConfig{
    // pub top_open_menu:HashMap<String,bool>,
    pub selected_anchor:String,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug,Default)]
pub struct Setting{
    pub show : bool
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug,Default)]
pub struct Plugin {
    pub node_tree:BTreeMap<String,Node>,
    pub node_pos:Vec<Pos2>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug,Default)]
pub struct WorkPlan {
    pub open:bool,
    pub transform: TSTransform,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug,Default)]
pub struct Node {
    pub code:String,
    pub desc:String,
    pub open:bool,
    pub input_post:Pos2,
    pub output_post:Pos2,
    pub config: NodeConfig,
    //通往哪个节点
    pub next_nodes:Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug,Default)]
pub struct NodeConfig{

}

impl Plugin {
    pub fn add_node<N:Into<Node>>(&mut self,node:N){
        let len = self.node_tree.len();
        let mut node = node.into();
        node.code = format!("{}-{}",node.code,len);
        let code = node.code.clone();
        self.node_tree.insert(code,node);
    }
}

impl Node {
    pub fn new<C:Into<String>,D:Into<String>>(code:C,desc:D)->Self{
        Self{
            code: code.into(),
            desc: desc.into(),
            open: true,
            ..Default::default()
        }
    }
}