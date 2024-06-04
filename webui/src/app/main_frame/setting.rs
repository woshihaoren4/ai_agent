use eframe::Frame;
use egui::Context;
use crate::app::main_frame::MainView;
use crate::app::state::{ State};

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

    fn update(&mut self, ctx: &Context, _frame: &mut Frame, cfg: &mut State) {
        let open = cfg.layout_config.selected_anchor == self.name;
        egui::SidePanel::left("setting")
            // .resizable(false)
            .show_animated(ctx,open,|ui|{
                ui.vertical_centered(|ui|{
                    ui.heading("⚙ setting")
                });
                ui.separator();
                ctx.settings_ui(ui);
                ui.separator();
                //设置风格
                let mut theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
                ui.collapsing("Theme", |ui| {
                    ui.group(|ui| {
                        theme.ui(ui);
                        theme.clone().store_in_memory(ui.ctx());
                    });
                });
            });
    }
}