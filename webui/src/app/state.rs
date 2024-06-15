use crate::infra::{HttpResponsePromise, StreamResponse};
use egui::emath::TSTransform;
use egui::Pos2;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Default)]
pub struct State {
    pub layout_config: AppLayoutConfig,
    pub setting: Setting,
    pub project_cfg: ProjectConfig,
    pub plugin: Plugin,
    pub options_view: OptionsView,
    pub debug_win: DebugCfg,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug)]
pub struct ProjectConfig {
    //从哪个位置加载工具栏
    #[serde(default = "ProjectConfig::default_server_addr")]
    pub server_addr: String,
    #[serde(default = "ProjectConfig::default_auto_save_interval")]
    pub auto_save_interval: usize,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let server_addr = Self::default_server_addr();
        let auto_save_interval = Self::default_auto_save_interval();
        Self {
            server_addr,
            auto_save_interval,
        }
    }
}

impl ProjectConfig {
    pub fn default_server_addr() -> String {
        "http://127.0.0.1:50000".into()
    }
    pub fn default_auto_save_interval() -> usize {
        30
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct DebugCfg {
    pub level: String, // debug,info,warn,error,fatal
    pub log: String,
    pub open: bool,
}

impl DebugCfg {
    #[allow(dead_code)]
    pub fn debug<S: Into<String>>(&mut self, log: S) {
        self.level = "debug".into();
        self.log = log.into();
    }
    #[allow(dead_code)]
    pub fn info<S: Into<String>>(&mut self, log: S) {
        self.level = "info".into();
        self.log = log.into();
    }
    #[allow(dead_code)]
    pub fn warn<S: Into<String>>(&mut self, log: S) {
        self.level = "warn".into();
        self.log = log.into();
    }
    #[allow(dead_code)]
    pub fn error<S: Into<String>>(&mut self, log: S) {
        self.level = "error".into();
        self.log = log.into();
    }
    #[allow(dead_code)]
    pub fn fatal<S: Into<String>>(&mut self, log: S) {
        self.level = "fatal".into();
        self.log = log.into();
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct AppLayoutConfig {
    // pub top_open_menu:HashMap<String,bool>,
    pub selected_anchor: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct Setting {
    pub show: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Default)]
pub struct Plugin {
    #[serde(skip)]
    pub tools_loader: HttpResponsePromise,
    #[serde(skip)]
    pub debug_loader: StreamResponse,
    pub services: BTreeMap<String, Vec<PluginService>>,
    pub nodes: BTreeMap<String, PluginServiceWin>,
}

impl Plugin {
    pub fn add_node<T: Into<PluginService>>(&mut self, service: T) {
        let mut service = service.into();
        let len = self.nodes.len();

        service.code = format!("{}-{}", service.code, len);
        let code = service.code.clone();

        let node = PluginServiceWin {
            service,
            open: true,
            ..Default::default()
        };

        self.nodes.insert(code, node);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default, Clone)]
pub struct PluginServiceWin {
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub pos: Pos2,

    #[serde(default)]
    pub input_pos: Pos2,
    #[serde(default)]
    pub output_pos: Pos2,

    #[serde(default)]
    pub service: PluginService,

    #[serde(default)]
    pub goto: Vec<String>,
    #[serde(default)]
    pub goto_select: String,
    //是否需要等待所有进入分支完成？
    pub no_ready_all: bool,
    #[serde(default)]
    pub debug_output: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default, Clone)]
pub struct PluginService {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub ui_type: String,
    #[serde(default)]
    pub service_type: String,
    #[serde(default)]
    pub input_vars: BTreeMap<String, PluginToolInput>,
    #[serde(default)]
    pub custom_input_var: Vec<CustomInputVar>,
    #[serde(default)]
    pub output_vars: Value,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default, Clone)]
pub struct CustomInputVar {
    pub name: String,
    pub value: String,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default, Clone)]
pub struct PluginToolInput {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub value: Value,
    #[serde(default)]
    pub value_from: String,
    #[serde(default)]
    pub default: Value,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub immobilization: bool,
    #[serde(default)]
    pub ui_type: String,

    // type=number,
    #[serde(default)]
    pub ui_extend_slider: Option<UISlider>,
    // type=number,ui_type=enum
    #[serde(default)]
    pub ui_extend_enum: Option<Vec<String>>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Clone, Default)]
pub struct UISlider {
    #[serde(default)]
    pub slider_value: f64,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub speed: f32,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
#[derive(Debug, Default)]
pub struct OptionsView {
    pub workflow_open: bool,
    pub text_view_open: bool,
    pub text_view_content: String,
    pub transform: TSTransform,
}
