use std::time::Duration;
use crate::app::{sys_setting};
use crate::app::main_frame::AppEntity;
use crate::app::state::State;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AiAgentApp {
    state:State,
}

impl Default for AiAgentApp {
    fn default() -> Self {
        let state = State::default();
        Self {state}
    }
}

impl AiAgentApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        sys_setting::sys_set(&cc.egui_ctx);
        if let Some(storage) = cc.storage {
            let state = match eframe::get_value::<State>(storage, eframe::APP_KEY) {
                None => {
                    let mut state = State::default();
                    state.debug_win.info("not found app from localstorage");
                    state
                }
                Some(s) => {s}
            };
            return AiAgentApp{state}
        }
        let mut app = AiAgentApp::default();
        app.state.debug_win.info("create a default app");
        app
    }
}

impl eframe::App for AiAgentApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        AppEntity::default()
            .update(ctx,frame,&mut self.state)
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn auto_save_interval(&self) -> Duration {
        if self.state.project_cfg.auto_save_interval <= 0 {
            Duration::from_secs(5)
        }else{
            Duration::from_secs(self.state.project_cfg.auto_save_interval as u64)
        }

    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // Give the area behind the floating windows a different color, because it looks better:
        let color = egui::lerp(
            egui::Rgba::from(visuals.panel_fill)..=egui::Rgba::from(visuals.extreme_bg_color),
            0.5,
        );
        let color = egui::Color32::from(color);
        color.to_normalized_gamma_f32()
    }
}


