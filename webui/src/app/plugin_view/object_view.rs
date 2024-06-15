use crate::app::state::PluginToolInput;
use serde_json::Value;

pub struct ObjectView;

impl ObjectView {
    pub fn ui(ui: &mut egui::Ui, name: &str, field: &mut PluginToolInput) {
        if !field.value.is_string() {
            let s = if field.default.is_object() {
                serde_json::to_string(&field.default).unwrap_or("{}".into())
            } else {
                "{}".into()
            };
            field.value = Value::String(s)
        }
        ui.collapsing(name, |ui| {
            let max_height = ui.available_height() - 300.0;
            if let Value::String(ref mut s) = field.value {
                egui::ScrollArea::vertical()
                    .id_source(name)
                    .max_height(max_height)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(s)
                                .font(egui::TextStyle::Monospace) // for cursor height
                                // .code_editor()
                                .desired_rows(8)
                                .lock_focus(true)
                                .desired_width(f32::INFINITY), // .layouter(&mut layouter),
                        );
                    });
            }
        });
    }
}
