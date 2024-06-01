use eframe::emath::Pos2;
use eframe::Frame;
use egui::{Context, Widget};
use crate::app::main_frame::MainView;
use crate::app::state::{State, Node};

#[derive(Debug)]
pub struct ControlTools{

}


impl Default for ControlTools{
    fn default() -> Self {
        Self{}
    }
}

impl ControlTools {
    fn ui(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
        // self.ui_file_drag_and_drop(ctx)
        //渲染工作节点
        egui::TopBottomPanel::top("service node")
            .resizable(false)
            .show(ctx,|ui|{
                //llm
                ui.horizontal_wrapped(|ui|{
                    ui.menu_button("LLM",|ui|{
                        if ui.button("openai-llm").clicked(){
                            cfg.plugin.add_node(Node::new("openai-LLM", "this is a LLM Node"));
                        }
                        if ui.button("llama 3").clicked(){

                        }
                        if ui.button("zhipu-glm").clicked(){

                        }
                    });
                    if ui.button("clean").clicked() {
                        cfg.plugin.node_tree.clear();
                    }
                });
            });

        egui::SidePanel::right("")
            // .resizable(false)
            .show(ctx,|ui|{
                ui.vertical_centered(|ui| {
                    for (_,i) in cfg.plugin.node_tree.iter_mut(){
                        if ui.button(i.code.as_str()).clicked() {
                            i.open = !i.open;
                        }
                    }
                });

            });

        for (_,i) in cfg.plugin.node_tree.iter_mut(){
            egui::Window::new(self.name())
                .default_width(320.0)
                .default_height(480.0)
                .open(&mut i.open)
                // .title_bar(false)
                .resizable([true, false])
                .show(ctx, |ui| {
                    ui.heading(i.code.as_str());
                    ui.separator();
                    ui.label(i.desc.as_str());
                });
        }
    }

}

impl MainView for ControlTools{
    fn name(&self) -> & str {
        ""
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
        // self.ui_file_drag_and_drop(ctx)
        //渲染控件
        egui::TopBottomPanel::top("service node")
            .resizable(false)
            .show(ctx,|ui|{
                //llm
                ui.horizontal_wrapped(|ui|{
                    ui.menu_button("LLM",|ui|{
                        if ui.button("openai-llm").clicked(){
                            cfg.plugin.add_node(Node::new("openai-LLM", "this is a LLM Node"));
                        }
                        if ui.button("llama 3").clicked(){

                        }
                        if ui.button("zhipu-glm").clicked(){

                        }
                    });
                    if egui::Button::new("clean").fill(egui::Color32::RED).ui(ui).clicked() {
                        cfg.plugin.node_tree.clear();
                    }
                    if egui::Button::new("reset").fill(egui::Color32::RED).ui(ui).clicked(){
                        *cfg = Default::default();
                    }
                });

            });
        //渲染工具栏
        egui::SidePanel::right("")
            // .resizable(false)
            .show(ctx,|ui|{
                ui.vertical_centered(|ui| {
                    for (_,i) in cfg.plugin.node_tree.iter_mut(){
                        if ui.button(i.code.as_str()).clicked() {
                            i.open = !i.open;
                        }
                    }
                    ui.separator();
                    //渲染任务流
                    if ui.button("work-flow-view").clicked() {
                        cfg.work_plan.open = !cfg.work_plan.open;
                    }
                });

            });
        //渲染已存在的节点
        for (_,i) in cfg.plugin.node_tree.iter_mut(){
            egui::Window::new(i.code.as_str())
                .default_pos(Pos2::new(100.0,100.0))
                .default_width(320.0)
                .default_height(480.0)
                .open(&mut i.open)
                // .title_bar(false)
                // .resizable([true, false])
                .show(ctx, |ui| {
                    ui.label(i.desc.as_str());
                    ui.separator();
                    ui.label("next nodes:");
                    let mut s = i.next_nodes.join(",");
                    egui::TextEdit::singleline(&mut s)
                        .hint_text("node-1,node-2")
                        .ui(ui);
                    i.next_nodes = s.split(",").map(|s|s.to_string()).collect();
                });
        }
    }

}