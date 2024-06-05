use serde_json::{Number, Value};
use crate::app::state::{PluginToolInput, UISlider};

pub struct NumberSlider;

impl NumberSlider{
    pub fn ui(ui: &mut egui::Ui,name:&str, field:&mut PluginToolInput){
        if field.ui_slider.is_none() {
            let mut slider = UISlider::default();
            if field.default.is_number() {
                slider.slider_value = field.default.as_f64().unwrap_or(0.0);
            }
            field.ui_slider = Some(slider)
        }

        if !field.value.is_number() {
            if field.default.is_number() {
                field.value = field.default.clone();
                //init value speed
                field.ui_slider.as_mut().unwrap().slider_value = field.default.as_f64().unwrap_or(0.0);
                if field.ui_slider.as_mut().unwrap().speed < 0.0001 {
                    field.ui_slider.as_mut().unwrap().speed = 1.0
                }
            }else{
                field.value = Value::Number(Number::from_f64(0.0).unwrap())
            }
        }

        let UISlider{ slider_value: value, max, min, speed } = field.ui_slider.as_mut().unwrap();
        ui.horizontal(|ui|{
            ui.label(format!("{name}:"));
            ui.add(egui::DragValue::new(value).speed(*speed));
        });

        //大小限制
        if let Some( min) = min{
            if value < min {
                *value = *min;
            }
        }
        if let Some( max) = max{
            if value>max{
                *value = *max;
            }
        }

    }
}