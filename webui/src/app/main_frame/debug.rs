use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use eframe::emath::Align;
use eframe::Frame;
use egui::{Context, Widget};
use serde_json::{Number, Value};
use crate::app::main_frame::MainView;
use crate::app::state::{PluginService, PluginServiceWin, State};
use crate::infra;

#[derive(Debug, Default)]
pub struct Debug{

}

impl Debug{
    pub fn debug_workflow(ctx: &Context,ui:&mut egui::Ui,cfg: &mut State) {
        let debug_text = if !cfg.plugin.debug_loader.is_waiting() {
            "debug"
        }else{
            "waiting"
        };
        //渲染组件
        if egui::Button::new(debug_text)
            .fill(egui::Color32::GREEN)
            .ui(ui)
            .clicked()
        {
            if !cfg.plugin.debug_loader.is_waiting() {
                //清理debug信息
                for (_,i) in cfg.plugin.nodes.iter_mut(){
                    i.debug_output = None;
                }
                //发起请求
                match WorkFlowDebugRequest::new(&cfg.plugin.nodes) {
                    Ok(body) => {
                        cfg.debug_win.debug(serde_json::to_string(&body).unwrap_or("".into()));
                        let result = infra::post_json_stream(format!("{}/api/v1/agent/call", cfg.project_cfg.server_addr).as_str(), &body, |x| x);
                        match result {
                            Ok(o) => {
                                cfg.plugin.debug_loader = o;
                            }
                            Err(e) => {
                                cfg.debug_win.error(format!("debug error:{:?}",e));
                            }
                        }
                    }
                    Err(e) => {
                        cfg.debug_win.error(format!("debug request parse error:{:?}",e));
                    }
                }
            }
        }
        if cfg.plugin.debug_loader.is_waiting() {
            ctx.request_repaint();
        }else{
            return;
        }
        match cfg
            .plugin
            .debug_loader
            .try_get_string()
        {
            None => {}
            Some(Ok(o)) => {
                match serde_json::from_str::<WorkFlowDebugResponse>(o.as_str()) {
                    Ok(resp)=>{
                        if let Some(ref node) = resp.result {
                            if let Some(n) = cfg.plugin.nodes.get_mut(node.node_code.as_str()) {
                                n.debug_output = Some(node.to_string());
                            }else{
                                cfg.debug_win.warn(format!("not found node win[{}]",node.node_code));
                            }
                        }else{
                            let text = format!("debug response result is null! \n -> output <-\n{}",o);
                            cfg.debug_win.warn(text);
                            cfg.plugin.debug_loader.stop();
                        }
                    }
                    Err(e)=>{
                        let text = format!("debug failed:{} \n -> output <-\n{}",e,o);
                        cfg.debug_win.warn(text);
                        cfg.plugin.debug_loader.stop();
                    }
                } ;

            }
            Some(Err(e)) => {
                cfg.debug_win
                    .error(format!("debug error:{e}"));
                cfg.plugin.debug_loader.stop();
            }
        }

    }
}

impl MainView for Debug{
    fn name(&self) -> &str {
        "debug"
    }

    fn update(&mut self, ctx: &Context, _frame: &mut Frame, cfg: &mut State) {
        if cfg.debug_win.open {
            let mut info = format!("[{}] \n {}", cfg.debug_win.level, cfg.debug_win.log);
            egui::Window::new("DebugInfoDetail")
                .open(&mut cfg.debug_win.open)
                .show(ctx,|ui|{
                    egui::TextEdit::multiline(&mut info)
                        .show(ui);
                });
        }else{
            egui::TopBottomPanel::bottom("debug_panel")
                .min_height(20.0)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                        ui.horizontal(|ui|{
                            if ui.button("detail").clicked() {
                                cfg.debug_win.open = true;
                            }
                            match cfg.debug_win.level.as_str() {
                                "info" | "debug" => {
                                    ui.label(format!("[{}] {}", cfg.debug_win.level, cfg.debug_win.log));
                                }
                                "warn" | "error" | "fatal" => {
                                    ui.label( egui::WidgetText::from(format!("[{}] {}", cfg.debug_win.level, cfg.debug_win.log)).color(egui::Color32::RED));
                                }
                                _ => {
                                    ui.label(format!("[{}] {}", cfg.debug_win.level, cfg.debug_win.log));
                                }
                            }
                        })

                    });
                });
        }

    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct WorkFlowDebugRequest{
    pub plan:Vec<WorkFlowDebugPlanNode>
}

impl WorkFlowDebugRequest {
    pub fn new(value:&BTreeMap<String, PluginServiceWin>)->anyhow::Result<Self> {
        let mut map:HashMap<String,WorkFlowDebugPlanNode> = HashMap::new();
        for (code,win) in value.iter(){
            for i in win.goto.iter() {
                if let Some(node) = map.get_mut(i){
                    if !node.no_ready_all {
                        node.ready_nodes.push(code.to_string());
                    }
                }else{
                    let mut plan_node= WorkFlowDebugPlanNode::default();
                    if !plan_node.no_ready_all {
                        plan_node.ready_nodes.push(code.to_string());
                    }
                    map.insert(i.to_string(),plan_node);
                }
            }
            if let Some(node) = map.get_mut(code){
                node.copy_from_service(win)?;
            }else{
                let mut node = WorkFlowDebugPlanNode::default();
                node.copy_from_service(win)?;
                map.insert(code.to_string(),node);
            }
        };

        let plan = map.into_iter().map(|x|x.1).collect::<Vec<_>>();
        Ok(WorkFlowDebugRequest { plan })
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct WorkFlowDebugPlanNode{
    pub code: String,
    pub service_type: String,
    pub cfg:String,
    pub ready_nodes:Vec<String>,
    pub goto_nodes:Vec<String>,
    #[serde(skip)]
    pub no_ready_all: bool,
}
impl WorkFlowDebugPlanNode{
    pub fn copy_from_service(&mut self,win:&PluginServiceWin) -> anyhow::Result<()>{
        self.code = win.service.code.clone();
        self.service_type = win.service.service_type.clone();
        self.cfg = serde_json::to_string(&Self::cfg_from_service(&win.service)?).unwrap_or("".into());
        self.goto_nodes = win.goto.clone();
        if win.no_ready_all {
            self.no_ready_all = true;
            self.ready_nodes = vec![]
        }
        Ok(())
    }
    pub fn cfg_from_service(ps:&PluginService)->anyhow::Result<Value>{
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
                "list" | "array" => {
                    let mut array = vec![];
                    if let Value::Array(ref list) = input.value {
                        for i in list{
                            if let Some(s) = i.as_str() {
                                array.push(Self::string_to_value(s)?);
                            }else{
                                return Err(anyhow::anyhow!("parse array type[{}.{}] failed!",ps.code,name))
                            }
                        }
                    }
                    Value::Array(array)
                }
                "object" | "obj" => {
                    if let Some(s) = input.value.as_str(){
                        if s.starts_with("{{") {
                            Value::from(s.to_string())
                        }else{
                            match serde_json::from_str::<Value>(s){
                                Ok(o) => o,
                                Err(e) => {
                                    return Err(anyhow::anyhow!("parse object type[{}.{}] error:{}",ps.code,name,e))
                                }
                            }
                        }
                    }else{
                        input.value.clone()
                    }
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
            let value = Self::string_to_value(i.value.as_str())?;
            map.insert(i.name.clone(),value);
        }
        return Ok(Value::Object(map));
    }
    pub fn string_to_value(s:&str)->anyhow::Result<Value> {
        if s == "true" {
            Ok(Value::Bool(true))
        }else if s == "false" {
            Ok(Value::Bool(true))
        }else if s == "null" {
            Ok(Value::Null)
        }else if s.starts_with("float:") {
            let f = f64::from_str(&s[6..])?;
            let value = Number::from_f64(f).unwrap_or(Number::from(0));
            Ok(Value::Number(value))
        }else if s.starts_with("int:") {
            let f = isize::from_str(&s[5..])?;
            let value = Number::from(f);
            Ok(Value::Number(value))
        }else if s.starts_with("obj:"){
            let value = serde_json::from_str::<Value>(&s[5..])?;
            Ok(value)
        }else if s.starts_with("list:"){
            let value = serde_json::from_str::<Value>(&s[6..])?;
            Ok(value)
        }else{
            Ok(Value::String(s.to_string()))
        }
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