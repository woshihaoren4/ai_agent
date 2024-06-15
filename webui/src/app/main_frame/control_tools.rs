use crate::app::main_frame::debug::Debug;
use crate::app::main_frame::text_control_view::TextControlView;
use crate::app::main_frame::MainView;
use crate::app::plugin_view::TopControlTools;
use crate::app::state::State;
use eframe::Frame;
use egui::{Context, Widget};

#[derive(Debug, Default)]
pub struct ControlTools {}

impl MainView for ControlTools {
    fn name(&self) -> &str {
        ""
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
        // self.ui_file_drag_and_drop(ctx)
        //渲染控件
        egui::TopBottomPanel::top("service node")
            .resizable(false)
            .show(ctx, |ui| {
                //llm
                ui.horizontal_wrapped(|ui| {
                    let mut services = vec![];
                    for (name, i) in cfg.plugin.services.iter() {
                        ui.menu_button(name, |ui| {
                            for i in i.iter() {
                                if ui.button(i.code.as_str()).clicked() {
                                    services.push(i.clone());
                                    cfg.debug_win.debug(format!("create a node: {}", i.code));
                                    // cfg.plugin_view.add_node(i.clone());
                                }
                            }
                        });
                    }
                    for i in services {
                        cfg.plugin.add_node(i);
                    }
                    ui.separator();
                    if egui::Button::new("clean")
                        .fill(egui::Color32::RED)
                        .ui(ui)
                        .clicked()
                    {
                        cfg.plugin.nodes.clear();
                    }
                    if egui::Button::new("reset")
                        .fill(egui::Color32::RED)
                        .ui(ui)
                        .clicked()
                    {
                        *cfg = Default::default();
                    }
                    if egui::Button::new("save")
                        .fill(egui::Color32::GREEN)
                        .ui(ui)
                        .clicked()
                    {
                        eframe::set_value(frame.storage_mut().unwrap(), eframe::APP_KEY, cfg);
                        cfg.debug_win.info("save success")
                    }
                    Debug::debug_workflow(ctx, ui, cfg);
                });
            });
        //渲染工具栏
        egui::SidePanel::right("")
            // .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let mut del_list = vec![];
                    for (name, i) in cfg.plugin.nodes.iter_mut() {
                        ui.horizontal(|ui| {
                            if ui.button(name).clicked() {
                                i.open = !i.open;
                            }
                            ui.separator();
                            if ui
                                .button(
                                    egui::WidgetText::RichText(egui::RichText::new("> delete"))
                                        .background_color(egui::Color32::RED),
                                )
                                .clicked()
                            {
                                del_list.push(name.clone());
                            }
                        });
                    }
                    //删除
                    for i in del_list {
                        cfg.plugin.nodes.remove(i.as_str());
                    }
                    ui.separator();
                    //渲染任务流
                    if ui.button("work-flow-view").clicked() {
                        cfg.options_view.workflow_open = !cfg.options_view.workflow_open;
                    }
                    //渲染文本视角
                    if ui.button("plan-text-view").clicked() {
                        cfg.options_view.text_view_open = !cfg.options_view.text_view_open;
                    }
                });
            });
        //渲染已存在的节点
        TopControlTools::ui(ctx, cfg);
        //渲染text view
        TextControlView::ui(ctx, cfg);
    }
}
