use crate::app::plugin_view::{ArrayView, BoolView, CustomInputField, ObjectView, Output, GOTO};
use crate::app::state::{PluginService, PluginServiceWin, State};
use egui::{Context, RichText};

pub struct TopControlTools;

impl TopControlTools {
    pub fn render_input_vars(ui: &mut egui::Ui, field: &mut PluginService) {
        for (name, var) in field.input_vars.iter_mut() {
            match var.r#type.to_lowercase().as_str() {
                "string" | "str" => {
                    super::InputStrViewTextEdit::ui(ui, name, var);
                }
                "number" | "int" | "f32" | "float" | "double" => {
                    super::NumberSlider::ui(ui, name, var);
                }
                "bool" => {
                    BoolView::ui(ui, name, var);
                }
                "list" | "array" => {
                    ArrayView::ui(ui, name, var);
                }
                "object" | "obj" => {
                    ObjectView::ui(ui, name, var);
                }
                "null" => {
                    ui.label(format!("{name}: null"));
                }
                _ => {
                    ui.label(egui::WidgetText::RichText(
                        RichText::new(format!("unknown field type [{}] !!!", var.r#type))
                            .color(egui::Color32::RED),
                    ));
                }
            }
        }
    }

    pub fn ui(ctx: &Context, cfg: &mut State) {
        let mut update_code = vec![];
        let mut node_list = cfg
            .plugin
            .nodes
            .iter()
            .map(|x| x.0.clone())
            .collect::<Vec<_>>();

        for (name, node) in cfg.plugin.nodes.iter_mut() {
            let PluginServiceWin {
                open,
                pos,
                service,
                goto,
                goto_select,
                debug_output,
                no_ready_all,
                ..
            } = node;
            let mut update = false;
            let resp = egui::Window::new(name)
                .default_pos(pos.clone())
                .default_width(320.0)
                .default_height(600.0)
                .open(open)
                // .title_bar(false)
                // .resizable([true, false])
                .show(ctx, |ui| {
                    //渲染公共头部
                    //节点编号
                    ui.horizontal_top(|ui| {
                        ui.label("code: ");
                        if ui.text_edit_singleline(&mut service.code).lost_focus() {
                            update = true;
                        }
                    });
                    //节点描述
                    if !service.desc.is_empty() {
                        ui.label("description: ");
                        ui.label(service.desc.as_str());
                    }
                    //节点对应的服务类型
                    ui.horizontal_top(|ui| {
                        ui.label("service type: ");
                        ui.label(
                            egui::WidgetText::RichText(egui::RichText::new(
                                service.service_type.as_str(),
                            ))
                            .color(egui::Color32::BLUE),
                        );
                    });
                    //渲染入参
                    ui.separator();
                    ui.collapsing("INPUT", |ui| {
                        TopControlTools::render_input_vars(ui, service);
                        CustomInputField::custom_input_ui(ui, service);
                    });
                    //渲染跳出
                    ui.collapsing("GOTO", |ui| {
                        GOTO::ui(ui, name, goto_select, goto, &mut node_list, no_ready_all);
                    });
                    //渲染出参
                    ui.collapsing("OUTPUT", |ui| {
                        Output::ui(ui, &mut service.output_vars);
                    });
                    //渲染调试信息，如果有
                    if let Some(s) = debug_output {
                        ui.collapsing("DEBUG", |ui| {
                            super::DebugView::ui(ui, s.as_str());
                        });
                    }
                });
            //判断是否更新窗口名称
            if update {
                if let Some(s) = resp {
                    *pos = s.response.rect.min.clone();
                }
                update_code.push(name.clone());
            }
        }
        //更新
        for i in update_code {
            if let Some(node) = cfg.plugin.nodes.remove(i.as_str()) {
                cfg.plugin.nodes.insert(node.service.code.clone(), node);
            }
        }
    }
}
