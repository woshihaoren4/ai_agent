use eframe::Frame;
use egui::Context;
use crate::app::main_frame::MainView;
use crate::app::state::State;

#[derive(Debug)]
pub struct FrameAbout<'a>{
    name:&'a str,
}
impl Default for FrameAbout<'_>{
    fn default() -> Self {
        let name = "about";
        Self{name}
    }
}

impl MainView for FrameAbout<'_>{
    fn name(&self) -> &str {
        self.name
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
        let open = cfg.layout_config.selected_anchor == self.name;
        egui::SidePanel::left("~ about ~")
            // .resizable(false)
            .show_animated(ctx,open,|ui|{
                ui.vertical_centered(|ui|{
                    ui.heading("about")
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("webui running inside");
                    ui.hyperlink_to(
                        "ai_agent",
                        "https://github.com/woshihaoren4/ai_agent",
                    );
                    ui.label(".");
                });
                //折叠显示
                // #[cfg(target_arch = "wasm32")]
                // ui.collapsing("Web infra (location)", |ui| {
                //     ui.monospace(format!("{:#?}", frame.infra().web_info.location));
                // });
                //不折叠显示
                #[cfg(target_arch = "wasm32")]
                egui::CollapsingHeader::new("Web infra (location)")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.monospace(format!("{:#?}", frame.info().web_info.location));
                    });
            });
        return;
    }
}