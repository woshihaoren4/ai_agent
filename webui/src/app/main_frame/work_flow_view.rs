use std::collections::HashMap;
use eframe::Frame;
use egui::*;
use egui::emath::TSTransform;
use crate::app::main_frame::MainView;
use crate::app::state::{State};

#[derive(Debug)]
pub struct WorkFlowView {
}
impl Default for WorkFlowView {
    fn default() -> Self {
        Self{}
    }
}

impl WorkFlowView {

    fn draw_line_between_buttons(painter: &Painter, p1: Pos2, p4: Pos2) {
        let x = (p4.x-p1.x)/2.0+p1.x;
        // let y = (p4.y-p1.y)/2.0 +p1.y;
        let p2 = Pos2::new(x,p1.y);
        let p3 = Pos2::new(x,p4.y);

        //贝塞尔
        let cubic_bezier = egui::epaint::CubicBezierShape::from_points_stroke([p1, p2, p3, p4], false,egui::Color32::TRANSPARENT,Stroke::new(2.0, egui::Color32::BLACK));
        painter.add(cubic_bezier);

        //箭头
        let origin = Pos2::new(p4.x - 10.0,p4.y - 10.0);
        painter.line_segment([origin,p4],Stroke::new(2.0, egui::Color32::BLACK));
        let origin = Pos2::new(p4.x - 10.0,p4.y + 10.0);
        painter.line_segment([origin,p4],Stroke::new(2.0, egui::Color32::BLACK));
    }

    fn make_node_area(id:egui::Id,ui:&mut egui::Ui,transform:&TSTransform,rect: Rect,name:&str,abnormal:bool)->(LayerId,Pos2,Pos2){
        let mut input_pos = Pos2::default();
        let mut output_pos = Pos2::default();

       let fill = if abnormal {
            egui::Color32::RED
        }else{
           ui.style().visuals.panel_fill
       };

        let id = egui::Area::new(id.with(("flow_node-", name)))
            .default_pos(Pos2::new(100.0,100.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(transform.inverse() * rect);
                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(ui.ctx().style().visuals.window_stroke)
                    .fill(fill)
                    .show(ui, |ui| {
                        ui.horizontal(|ui|{
                            //取左边中心点
                            let rect = ui.label(">").rect;
                            input_pos = egui::Pos2::new(rect.min.x-10.0,rect.center().y);
                            // if abnormal {
                            //     ui.add(egui::Button::new(name).fill(fill));
                            //     let _ = ui.button(name).;
                            // }else{
                            //     let _= ui.button(name);
                            // }
                            ui.add(egui::Button::new(name).fill(fill));
                            //取右边的最小点
                            let rect = ui.label(">").rect;
                            output_pos = egui::Pos2::new(rect.max.x+10.0,rect.center().y);
                        });
                    });
            })
            .response
            .layer_id;
        (id,input_pos,output_pos)
    }

    fn ui(&mut self, ui: &mut egui::Ui, cfg: &mut State) {
        // CentralPanel::default().show(ctx, |ui| {
        let (id, rect) = ui.allocate_space(ui.available_size());
        // let transform = Self::sense_control(ui,cfg);

        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        // Allow dragging the background as well.
        if response.dragged() {
            cfg.work_plan.transform.translation += response.drag_delta();
        }

        // Plot-like reset
        if response.double_clicked() {
            cfg.work_plan.transform = TSTransform::default();
        }

        let transform =
            TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) * cfg.work_plan.transform;

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Note: doesn't catch zooming / panning if a button in this PanZoom container is hovered.
            if response.hovered() {
                let pointer_in_layer = transform.inverse() * pointer;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // Zoom in on pointer:
                cfg.work_plan.transform = cfg.work_plan.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                cfg.work_plan.transform = TSTransform::from_translation(pan_delta) * cfg.work_plan.transform;
            }
        }

        //先画点
        for (name,node) in cfg.plugin.nodes.iter_mut() {
            let (id,p1,p2) = Self::make_node_area(id,ui,&transform,rect,name,false);
            node.input_pos = p1;
            node.output_pos = p2;
            ui.ctx().set_transform_layer(id, transform);
        }

        //连线
        let lid = egui::Area::new(id.with("workflow-line"))
            // .default_pos(pos)
            // Need to cover up the pan_zoom demo window,
            // but may also cover over other windows.
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(transform.inverse() * rect);
                egui::Frame::default()
                    // .rounding(egui::Rounding::same(4.0))
                    // .inner_margin(egui::Margin::same(8.0))
                    .stroke(ui.ctx().style().visuals.window_stroke)
                    .fill(ui.style().visuals.panel_fill)
                    .show(ui, |ui| {
                        let mut not_found_list = HashMap::new();
                        for (_name,node) in cfg.plugin.nodes.iter(){
                            for i in node.goto.iter(){
                                let next:Pos2 = match cfg.plugin.nodes.get(i) {
                                    None => {
                                        if let Some(p) = not_found_list.get(i) {
                                            *p
                                        }else{
                                            let (id,p1,_) = Self::make_node_area(id,ui,&transform,rect,i,true);
                                            ui.ctx().set_transform_layer(id, transform);
                                            not_found_list.insert(i.to_string(),p1);
                                            p1
                                        }
                                    }
                                    Some(n) => {
                                        n.input_pos
                                    }
                                };
                                Self::draw_line_between_buttons(ui.painter(),node.output_pos.clone(),next);
                            }
                        }

                        // let shape = egui::Shape::line(cfg.plugin_view.node_pos.clone(),Stroke::new(1.0,egui::Color32::BLACK));
                        // let idx = ui.painter().add(shape);
                    });
            })
            .response
            .layer_id;
        ui.ctx().set_transform_layer(lid, transform);

    }
}

impl MainView for WorkFlowView {
    fn name(&self) -> &str {
        ""
    }

    fn update(&mut self, ctx: &Context, _frame: &mut Frame, cfg: &mut State) {
        let mut open = cfg.work_plan.open;
        egui::Window::new("WorkFlow-view")
            .default_width(500.0)
            .default_height(500.0)
            .vscroll(false)
            .open(&mut open)
            .show(ctx, |ui| self.ui(ui,cfg));
        cfg.work_plan.open = open;
    }
}