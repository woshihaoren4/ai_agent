use crate::app::state::{CustomInputVar, PluginService};
use egui::{Vec2, Widget};

pub struct CustomInputField {}

impl CustomInputField {
    pub fn custom_input_ui(ui: &mut egui::Ui, service: &mut PluginService) {
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("⊕").on_hover_text("add custom field").clicked() {
                let mut var = CustomInputVar::default();
                var.name = format!("var-{}", service.custom_input_var.len());
                service.custom_input_var.push(var);
            }
        });
        let mut del_index = None;
        egui::Grid::new(format!("CustomInputField-{}", service.code))
            .num_columns(3)
            // .striped(true)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                for (index, i) in service.custom_input_var.iter_mut().enumerate() {
                    egui::TextEdit::singleline(&mut i.name)
                        .min_size(Vec2::from([150.0, 0.0]))
                        .ui(ui);
                    egui::TextEdit::singleline(&mut i.value)
                        .min_size(Vec2::from([150.0, 0.0]))
                        .ui(ui);
                    if ui.button("⛔").clicked() {
                        del_index = Some(index);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = del_index {
            service.custom_input_var.remove(i);
        }
    }
}
