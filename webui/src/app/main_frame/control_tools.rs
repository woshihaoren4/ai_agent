use std::collections::{BTreeMap, HashMap};
use std::isize;
use std::str::FromStr;
use crate::app::main_frame::MainView;
use crate::app::plugin_view::{GOTO, TopControlTools};
use crate::app::state::{PluginService, PluginServiceWin, State};
use eframe::Frame;
use egui::{Context, Widget};
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use crate::infra;

#[derive(Debug, Default)]
pub struct ControlTools {}

impl ControlTools {}

impl MainView for ControlTools {
    fn name(&self) -> &str {
        ""
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
        // self.ui_file_drag_and_drop(ctx)
        //渲染控件
        egui::TopBottomPanel::top("service node")
            .resizable(false)
            .show(ctx, |ui| {
                //llm
                ui.horizontal_wrapped(|ui| {
                    let mut services = vec![];
                    for (name, i) in cfg.plugin.services.iter() {
                        ui.menu_button(name, |ui| {
                            for i in i.iter() {
                                if ui.button(i.code.as_str()).clicked() {
                                    services.push(i.clone());
                                    cfg.debug_win
                                        .debug(format!("create a node: {}", i.code).as_str());
                                    // cfg.plugin_view.add_node(i.clone());
                                }
                            }
                        });
                    }
                    for i in services {
                        cfg.plugin.add_node(i);
                    }
                    ui.separator();
                    if egui::Button::new("clean")
                        .fill(egui::Color32::RED)
                        .ui(ui)
                        .clicked()
                    {
                        cfg.plugin.nodes.clear();
                    }
                    if egui::Button::new("reset")
                        .fill(egui::Color32::RED)
                        .ui(ui)
                        .clicked()
                    {
                        *cfg = Default::default();
                    }
                    if egui::Button::new("save")
                        .fill(egui::Color32::GREEN)
                        .ui(ui)
                        .clicked()
                    {
                        eframe::set_value(frame.storage_mut().unwrap(), eframe::APP_KEY, cfg);
                        cfg.debug_win.info("save success")
                    }
                    if egui::Button::new("debug")
                        .fill(egui::Color32::GREEN)
                        .ui(ui)
                        .clicked()
                    {
                        let body = WorkFlowDebugRequest::from(&cfg.plugin.nodes);
                        let result = infra::post_json_stream(format!("{}/api/v1/agent/call", cfg.project_cfg.server_addr).as_str(), &body, |x| x);
                        match result {
                            Ok(o) => {
                                cfg.plugin.debug_loader = o;
                            }
                            Err(e) => {
                                cfg.debug_win.error(format!("debug error:{:?}",e) .as_str());
                            }
                        }
                    }
                    match cfg
                        .plugin
                        .debug_loader
                        .try_get_obj::<WorkFlowDebugResponse>()
                    {
                        None => {}
                        Some(Ok(o)) => {
                            if let Some(ref node) = o.result {
                                if let Some(n) = cfg.plugin.nodes.get_mut(node.node_code.as_str()) {
                                    n.debug_output = Some(node.to_string());
                                }else{
                                    cfg.debug_win.warn(format!("not found win[{}]",node.node_code).as_str());
                                }
                            }else{
                                cfg.debug_win.warn(format!("workflow debug error:{:?}",o).as_str());
                            }
                        }
                        Some(Err(e)) => {
                            cfg.debug_win
                                .info(format!("load plugin_view error:{e}").as_str());
                        }
                    }
                    if !cfg.plugin.debug_loader.is_over() {
                        ctx.request_repaint();
                    }
                });
            });
        //渲染工具栏
        egui::SidePanel::right("")
            // .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let mut del_list = vec![];
                    for (name, i) in cfg.plugin.nodes.iter_mut() {
                        ui.horizontal(|ui| {
                            if ui.button(name).clicked() {
                                i.open = !i.open;
                            }
                            ui.separator();
                            if ui
                                .button(
                                    egui::WidgetText::RichText(egui::RichText::new("> delete"))
                                        .background_color(egui::Color32::RED),
                                )
                                .clicked()
                            {
                                del_list.push(name.clone());
                            }
                        });
                    }
                    //删除
                    for i in del_list {
                        cfg.plugin.nodes.remove(i.as_str());
                    }
                    ui.separator();
                    //渲染任务流
                    if ui.button("work-flow-view").clicked() {
                        cfg.work_plan.open = !cfg.work_plan.open;
                    }
                });
            });
        //渲染已存在的节点
        TopControlTools::ui(ctx, cfg);
    }
}

impl From<&BTreeMap<String, PluginServiceWin>> for WorkFlowDebugRequest {
    fn from(value: &BTreeMap<String, PluginServiceWin>) -> Self {
        let mut map:HashMap<String,WorkFlowDebugPlanNode> = HashMap::new();
        for (code,win) in value.iter(){
            for i in win.goto.iter() {
                if let Some(node) = map.get_mut(i){
                    node.ready_nodes.push(code.to_string());
                }else{
                    let mut plan_node= WorkFlowDebugPlanNode::default();
                    plan_node.ready_nodes.push(code.to_string());
                    map.insert(i.to_string(),plan_node);
                }
            }
            if let Some(node) = map.get_mut(code){
                node.copy_from_service(win);
            }else{
                let mut node = WorkFlowDebugPlanNode::default();
                node.copy_from_service(win);
                map.insert(code.to_string(),node);
            }
        };

        let plan = map.into_iter().map(|x|x.1).collect::<Vec<_>>();
        WorkFlowDebugRequest{plan}
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct WorkFlowDebugRequest{
    pub plan:Vec<WorkFlowDebugPlanNode>
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct WorkFlowDebugPlanNode{
    pub code: String,
    pub service_type: String,
    pub cfg:String,
    pub ready_nodes:Vec<String>,
    pub goto_nodes:Vec<String>
}
impl WorkFlowDebugPlanNode{
    pub fn copy_from_service(&mut self,win:&PluginServiceWin){
        self.code = win.service.code.clone();
        self.service_type = win.service.service_type.clone();
        self.cfg = serde_json::to_string(&Self::cfg_from_service(&win.service)).unwrap_or("".into());
        self.goto_nodes = win.goto.clone();
    }
    pub fn cfg_from_service(ps:&PluginService)->Value{
        let mut map = serde_json::map::Map::new();
        for (name,input) in ps.input_vars.iter() {
            let value = match input.r#type.as_str() {
                "string" | "str" => {
                    input.value.clone()
                }
                "number" | "int" => {
                    if let Some(ref s ) = input.ui_extend_slider {
                        Value::Number(Number::from(s.slider_value as i32))
                    }else {
                        input.value.clone()
                    }
                }
                "f32" | "float" | "double" => {
                    if let Some(ref s ) = input.ui_extend_slider {
                        Value::Number(Number::from_f64(s.slider_value).unwrap())
                    }else {
                        input.value.clone()
                    }
                }
                "bool" => {
                    input.value.clone()
                }
                "list" => {
                    input.value.clone()
                }
                "object" => {
                    input.value.clone()
                }
                "null" => {
                    input.value.clone()
                }
                _=>{
                    Value::Null
                }
            };
            map.insert(name.to_string(),value);
        }
        for i in ps.custom_input_var.iter() {
            if i.value.as_str() == "true" {
                map.insert(i.name.clone(),Value::Bool(true));
            }else if i.value.as_str() == "false" {
                map.insert(i.name.clone(),Value::Bool(true));
            }else if i.value.starts_with("float:") {
                let f = f64::from_str(&i.value[6..]).unwrap_or(0.0f64);
                let value = Number::from_f64(f).unwrap_or(Number::from(0u8));
                map.insert(i.name.clone(),Value::Number(value));
            }else if i.value.starts_with("int:") {
                let f = isize::from_str(&i.value[5..]).unwrap_or(0);
                let value = Number::from(f);
                map.insert(i.name.clone(),Value::Number(value));
            }else if i.value.starts_with("obj:"){
                let result = serde_json::from_str::<Value>(&i.value[5..]);
                map.insert(i.name.clone(),result.unwrap_or(Value::Null));
            }else if i.value.starts_with("list:"){
                let result = serde_json::from_str::<Value>(&i.value[6..]);
                map.insert(i.name.clone(),result.unwrap_or(Value::Null));
            }else{
                map.insert(i.name.clone(),Value::String(i.value.clone()));
            }
        }
        return Value::Object(map);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct WorkFlowDebugResponse{
    #[serde(default)]
    pub code: i32,
    pub message: String,
    pub result: Option<WorkFlowDebugResult>,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct WorkFlowDebugResult{
    pub node_code: String,
    pub round: i32,
    pub output: Option<Value>,
}

impl WorkFlowDebugResult {
    pub fn to_string(&self)->String{
        if let Some(ref s) = self.output {
            serde_json::to_string_pretty(s).unwrap_or("".into())
        }else{
            "".into()
        }
    }
}