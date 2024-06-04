use serde_json::Value;

pub struct Output;

impl Output{
    pub fn ui(ui: &mut egui::Ui,output:&mut Value){
        if !output.is_string() {
            let s = serde_json::to_string_pretty(output).unwrap_or("output vars parse failed!!!".to_string());
            *output = Value::String(s);
        }
        if let Value::String(s) = output {
            super::Common::json_view_ui(ui,s.as_str());
        }
    }
}