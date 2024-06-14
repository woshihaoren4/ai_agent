use serde_json::Value;
use crate::app::state::PluginToolInput;

pub struct ArrayView;

impl ArrayView{
    pub fn ui(ui: &mut egui::Ui, name: &str, field: &mut PluginToolInput) {
        if !field.value.is_array() {
            if field.default.is_array() {
                field.value = field.default.clone();
            }else{
                field.value = Value::Array(vec![]);
            }
        }
        ui.collapsing(name,|ui|{
            ui.horizontal(|ui|{
                if ui.button("add item ⊕").clicked() {
                    if let Value::Array(ref mut list) = field.value {
                        list.push(Value::String("".into()));
                    }
                }
            });
            let mut del_index = None;
            for (i,val) in field.value.as_array_mut().unwrap().iter_mut().enumerate() {
                ui.horizontal(|ui|{
                    if ui.button("  ⛔").clicked() {
                        del_index = Some(i);
                    }
                    ui.label(format!(" {}:",i));
                    if let Value::String(s) = val {
                        ui.text_edit_singleline(s);
                    }else{
                        ui.label(format!("no support type:[{}]",val));
                    }
                });
            }
            if let Some(i) = del_index{
                let list = field.value.as_array_mut().unwrap();
                list.remove(i);
            }
        });
    }
}