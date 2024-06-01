use eframe::Frame;
use egui::Context;
use crate::app::main_frame::MainView;
use crate::app::state::{Setting, State};

#[derive(Debug)]
pub struct FrameSetting<'a>{
    name:&'a str,
}
impl Default for FrameSetting<'_>{
    fn default() -> Self {
        let name = "setting";
        Self{name}
    }
}

impl MainView for FrameSetting<'_>{
    fn name(&self) -> & str {
        self.name
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
        let open = cfg.layout_config.selected_anchor == self.name;
        egui::SidePanel::left("setting")
            // .resizable(false)
            .show_animated(ctx,open,|ui|{
                ui.vertical_centered(|ui|{
                    ui.heading("⚙ setting")
                });
                ui.separator();
                ctx.settings_ui(ui);
            });
    }
}