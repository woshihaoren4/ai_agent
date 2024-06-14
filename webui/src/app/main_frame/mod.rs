use crate::app::main_frame::about::FrameAbout;
use crate::app::main_frame::control_tools::ControlTools;
use crate::app::main_frame::project::Project;
use crate::app::main_frame::setting::FrameSetting;
use crate::app::main_frame::work_flow_view::WorkFlowView;
use crate::app::state::State;
use crate::app::main_frame::debug::Debug;

mod about;
mod control_tools;
mod project;
mod setting;
mod work_flow_view;
mod debug;

pub use debug::*;

pub trait MainView {
    fn name(&self) -> &str;
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, cfg: &mut State);
}

pub struct AppEntity {
    items: Vec<Box<dyn MainView>>,
    tool_control: ControlTools,
    work_space: WorkFlowView,
    debug : Debug,
}
impl Default for AppEntity {
    fn default() -> Self {
        let tool_control = ControlTools::default();
        let work_space = WorkFlowView::default();
        let debug = Debug::default();
        let mut items: Vec<Box<dyn MainView>> = vec![];
        items.push(Box::new(Project::default()));
        items.push(Box::new(FrameSetting::default()));
        items.push(Box::new(FrameAbout::default()));
        // items.push(Box::new(ControlTools::default()));
        // items.push(Box::new(WorkSpace::default()));
        Self {
            items,
            tool_control,
            work_space,
            debug,
        }
    }
}

impl AppEntity {
    pub fn show_version(ui:&mut egui::Ui){
        ui.label(egui::WidgetText::from("This is only an alpha version and there may be major changes in the future！！！").color(egui::Color32::GRAY));
    }
    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, cfg: &mut State) {
        //先绘制最顶部的内容
        egui::TopBottomPanel::top("wrap_app_top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                // ui.visuals_mut().button_frame = false;
                // egui::widgets::global_dark_light_mode_switch(ui);

                if ui.label("◀").clicked() {
                    cfg.layout_config.selected_anchor = "".to_string();
                }
                ui.separator();

                let mut selected_anchor = cfg.layout_config.selected_anchor.as_str();
                for i in self.items.iter_mut() {
                    if i.name().is_empty() {
                        continue;
                    }
                    if ui
                        .selectable_label(selected_anchor == i.name(), i.name())
                        .clicked()
                    {
                        selected_anchor = i.name();
                    }
                }
                cfg.layout_config.selected_anchor = selected_anchor.to_string();
                //展示测试版本说明
                ui.separator();
                Self::show_version(ui);
            });
        });
        //底部
        self.debug.update(ctx,frame,cfg);
        //绘制中部
        for i in self.items.iter_mut() {
            i.update(ctx, frame, cfg);
        }
        //创建工具栏
        self.tool_control.update(ctx, frame, cfg);
        //创建工作区tr
        self.work_space.update(ctx, frame, cfg);
    }
}
