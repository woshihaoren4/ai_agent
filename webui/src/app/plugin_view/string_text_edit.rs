use crate::app::state::PluginToolInput;
use egui::RichText;
use serde_json::Value;

pub struct InputStrViewTextEdit;

impl InputStrViewTextEdit {
    pub fn ui(ui: &mut egui::Ui, name: &str, field: &mut PluginToolInput) {
        if !field.value.is_string() {
            if field.default.is_string() {
                field.value = field.default.clone();
            } else {
                field.value = Value::String(
                    field
                        .default
                        .as_str()
                        .map(|x| x.to_string())
                        .unwrap_or_default(),
                );
            }
        }

        let mut max_height = ui.available_height() - 300.0;

        match field.ui_type.to_lowercase().as_str() {
            "" | "text_edit_single" => {
                ui.horizontal(|ui| {
                    ui.label(format!("{name}:"));
                    if let Value::String(ref mut s) = field.value {
                        ui.text_edit_singleline(s);
                    }
                });
            }
            "text_edit_multi" => {
                ui.collapsing(name, |ui| {
                    if max_height > 500.0 {
                        max_height = 500.0;
                    }
                    ui.label(format!("{name}:"));
                    if let Value::String(ref mut s) = field.value {
                        egui::ScrollArea::vertical()
                            .max_height(max_height)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(s)
                                        .font(egui::TextStyle::Monospace) // for cursor height
                                        // .code_editor()
                                        .desired_rows(10)
                                        .lock_focus(true)
                                        .desired_width(f32::INFINITY), // .layouter(&mut layouter),
                                );
                            });
                    }
                });
            }
            "script_code" => {
                ui.collapsing(name, |ui| {
                    if max_height > 800.0 {
                        max_height = 800.0;
                    }

                    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                        let language = "py";
                        let theme =
                            egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
                        let mut layout_job = egui_extras::syntax_highlighting::highlight(
                            ui.ctx(),
                            &theme,
                            string,
                            language,
                        );
                        layout_job.wrap.max_width = wrap_width;
                        ui.fonts(|f| f.layout_job(layout_job))
                    };

                    ui.label(format!("{name}:"));
                    if let Value::String(ref mut s) = field.value {
                        egui::ScrollArea::vertical()
                            .max_height(max_height)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(s)
                                        .font(egui::TextStyle::Monospace) // for cursor height
                                        .code_editor()
                                        .desired_rows(10)
                                        .lock_focus(true)
                                        .desired_width(f32::INFINITY)
                                        .layouter(&mut layouter),
                                );
                            });
                    }
                });
            }
            "enum" => {
                ui.horizontal(|ui| {
                    ui.label(format!("{name}:"));
                    if let Value::String(ref mut s) = field.value {
                        egui::ComboBox::from_label("select a value")
                            .selected_text(s.as_str())
                            .show_ui(ui, |ui| {
                                if let Some(ref es) = field.ui_extend_enum {
                                    for i in es.iter() {
                                        ui.selectable_value(s, i.to_string(), i);
                                    }
                                }
                            });
                    }
                });
            }
            _ => {
                ui.label(egui::WidgetText::RichText(RichText::new(format!(
                    "Field[{name}] ui_type[{}] not support",
                    field.r#type
                ))));
            }
        }
    }
}
