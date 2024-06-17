use serde_json::Value;
use crate::app::plugin_view::Common;
use crate::app::state::PluginToolInput;

pub struct BoolView;


impl BoolView{
    pub fn ui(ui: &mut egui::Ui, name: &str, field: &mut PluginToolInput) {
        if !field.value.is_boolean() {
            if field.default.is_boolean() {
                field.value = field.default.clone();
            }else{
                field.value = Value::Bool(false);
            }
        }
        
        
        ui.horizontal(|ui|{
            ui.label(name);
            if let Value::Bool(ref mut b) = field.value {
                ui.add(Common::toggle(b));
            }
        });
    }
}