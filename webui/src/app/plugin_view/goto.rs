use egui::RichText;

pub struct GOTO;

impl GOTO{
    pub fn ui(ui: &mut egui::Ui,name:&str,select:&mut String,goto:&mut Vec<String>,other:&mut Vec<String>){
        egui::Grid::new(format!("goto-grid-{name}"))
            .num_columns(2)
            .striped(true)
            .spacing([20.0, 4.0])
            .show(ui,|ui|{
                let mut del_list = vec![];
            for (i,n) in goto.iter_mut().enumerate(){
                if n == name {
                    continue
                }
                ui.label(n.as_str());
                if ui.button(egui::WidgetText::RichText(RichText::new("> delete")).background_color(egui::Color32::RED)).clicked() {
                    del_list.push(i);
                }
                ui.end_row();
            }
                for i in del_list.into_iter().rev(){
                    goto.remove(i);
                }
        });
        //追加
        ui.label("Add a downward branch:");
        ui.horizontal(|ui|{
            egui::ComboBox::from_label("select a node")
                .selected_text(select.as_str())
                .show_ui(ui,|ui|{
                    for i in other.iter() {
                        ui.selectable_value(select,i.to_string(),i.to_string());
                    }
                });
            if ui.button("< ADD").clicked() {
                let s = std::mem::take(select);
                if s.as_str() != name {
                    goto.push(s);
                }
            }
        });
        ui.horizontal(|ui|{
            ui.label("input node code:");
            ui.text_edit_singleline(select);
            if ui.button("< ADD").clicked() {
                let s = std::mem::take(select);
                if s.as_str() != name {
                    goto.push(s);
                }
            }
        });
    }
}