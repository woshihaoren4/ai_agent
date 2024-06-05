use std::collections::BTreeMap;
use eframe::Frame;
use egui::{Context};
use crate::app::main_frame::MainView;
use crate::app::state::{PluginService, State};
use crate::infra;

#[derive(Debug,Default)]
pub struct Project{

}

impl MainView for Project{
    fn name(&self) -> &str {
        "project"
    }

    fn update(&mut self, ctx: &Context, _frame: &mut Frame, cfg: &mut State) {
        let open = cfg.layout_config.selected_anchor == self.name();
        egui::SidePanel::left("project")
            // .resizable(false)
            .show_animated(ctx,open,|ui|{
                ui.vertical_centered(|ui|{
                    ui.heading("🖥project")
                });
                ui.separator();
                egui::Grid::new("project")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .striped(true)
                    .show(ui,|ui| {
                        ui.label("serve addr:").on_hover_text("default: http://127.0.0.1:50000");
                        ui.text_edit_singleline(&mut cfg.project_cfg.server_addr);
                        ui.end_row();

                        ui.label("update plugin_view:");
                        if ui.button("LOAD").clicked() {
                            cfg.plugin.tools_loader = infra::get_json(format!("{}/api/v1/plugin",cfg.project_cfg.server_addr).as_str());
                        }
                        match cfg.plugin.tools_loader.try_get_value::<BTreeMap<String, Vec<PluginService>>>() {
                            None=>{}
                            Some(Ok(o))=>{
                                cfg.plugin.services = o;
                                cfg.debug_win.info("load plugin_view success");
                            }
                            Some(Err(e))=>{
                                cfg.debug_win.info(format!("load plugin_view error:{e}").as_str());
                            }
                        }
                        ui.end_row();

                        ui.label("auto save interval:");
                        let drag = egui::DragValue::new(&mut cfg.project_cfg.auto_save_interval)
                            .speed(0.1)
                            .suffix("s")
                            .clamp_range(1..=100);
                        ui.add(drag);
                        ui.end_row()

                    });
            });
        return;
    }
}