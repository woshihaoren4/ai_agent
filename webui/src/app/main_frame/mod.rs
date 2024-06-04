use eframe::emath::{Align};
use crate::app::main_frame::about::FrameAbout;
use crate::app::main_frame::work_flow_view::WorkFlowView;
use crate::app::main_frame::setting::FrameSetting;
use crate::app::main_frame::control_tools::ControlTools;
use crate::app::main_frame::project::Project;
use crate::app::state::{State};

mod setting;
mod about;
mod control_tools;
mod work_flow_view;
mod project;

pub trait MainView {
    fn name(&self)->&str;
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, cfg:&mut State);
}

pub struct AppEntity {
    items: Vec<Box<dyn MainView>>,
    tool_control : ControlTools,
    work_space: WorkFlowView,
}
impl Default for AppEntity{
    fn default() -> Self {
        let tool_control = ControlTools::default();
        let work_space = WorkFlowView::default();
        let mut items:Vec<Box<dyn MainView>> = vec![];
        items.push(Box::new(Project::default()));
        items.push(Box::new(FrameSetting::default()));
        items.push(Box::new(FrameAbout::default()));
        // items.push(Box::new(ControlTools::default()));
        // items.push(Box::new(WorkSpace::default()));
        Self{items,tool_control,work_space}
    }
}


impl AppEntity {
    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, cfg:&mut State){
        //先绘制最顶部的内容
        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                // ui.visuals_mut().button_frame = false;
                // egui::widgets::global_dark_light_mode_switch(ui);

                if ui.label("◀").clicked(){
                    cfg.layout_config.selected_anchor = "".to_string();
                }
                ui.separator();

                let mut selected_anchor = cfg.layout_config.selected_anchor.as_str();
                for i in self.items.iter_mut(){
                    if i.name().is_empty() {
                        continue
                    }
                    if ui.selectable_label(selected_anchor == i.name(),i.name())
                        .clicked() {
                        selected_anchor = i.name();
                    }
                }
                cfg.layout_config.selected_anchor = selected_anchor.to_string();
            });
        });
        //底部
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                ui.label(cfg.debug_win.log.as_str());
            });
        });
        //绘制中部
        for i in self.items.iter_mut(){
            i.update(ctx,frame,cfg);
        }
        //创建工具栏
        self.tool_control.update(ctx,frame,cfg);
        //创建工作区tr
        self.work_space.update(ctx,frame,cfg);
    }
}