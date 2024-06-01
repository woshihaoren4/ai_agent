use eframe::Frame;
use egui::*;
use egui::emath::TSTransform;
use egui::epaint::PathShape;
use crate::app::main_frame::MainView;
use crate::app::state::State;

#[derive(Debug)]
pub struct WorkFlowView {
    open :bool
}
impl Default for WorkFlowView {
    fn default() -> Self {
        Self{open:true}
    }
}

impl WorkFlowView {

    fn draw_line_between_buttons(painter: &Painter, p1: Pos2, p4: Pos2) {
        let x = (p4.x-p1.x)/2.0+p1.x;
        // let y = (p4.y-p1.y)/2.0 +p1.y;
        let p2 = Pos2::new(x,p1.y);
        let p3 = Pos2::new(x,p4.y);

        let cubic_bezier = egui::epaint::CubicBezierShape::from_points_stroke([p1, p2, p3, p4], false,egui::Color32::TRANSPARENT,Stroke::new(2.0, egui::Color32::BLACK));

        painter.add(cubic_bezier);
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

            cfg.plugin.node_pos.clear();

            for (i,(_,n)) in cfg.plugin.node_tree.iter_mut().enumerate() {
                let id = egui::Area::new(id.with(("node-", i)))
                    .default_pos(Pos2::new(100.0,100.0))
                    // Need to cover up the pan_zoom demo window,
                    // but may also cover over other windows.
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        ui.set_clip_rect(transform.inverse() * rect);
                        egui::Frame::default()
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::same(8.0))
                            .stroke(ui.ctx().style().visuals.window_stroke)
                            .fill(ui.style().visuals.panel_fill)
                            .show(ui, |ui| {
                                ui.horizontal(|ui|{
                                    n.input_post = ui.label("> ").rect.center();
                                    let _= ui.button(format!("{}-{}",n.code,i));
                                    n.output_post = ui.label(" >").rect.center();
                                });
                            });
                    })
                    .response
                    .layer_id;
                ui.ctx().set_transform_layer(id, transform);
            }

        //连线
            let lid = egui::Area::new(id.with("line"))
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
                            for (_,i) in cfg.plugin.node_tree.iter(){
                                //没有下一个
                                if i.next_nodes.is_empty() {
                                    continue
                                }
                                for j in i.next_nodes.iter() {
                                    if let Some(k) = cfg.plugin.node_tree.get(j) {
                                        Self::draw_line_between_buttons(ui.painter(),i.output_post.clone(),k.input_post.clone());
                                    }
                                }
                            }

                            // let shape = egui::Shape::line(cfg.plugin.node_pos.clone(),Stroke::new(1.0,egui::Color32::BLACK));
                            // let idx = ui.painter().add(shape);
                        });
                })
                .response
                .layer_id;
            ui.ctx().set_transform_layer(lid, transform);

            //划线
            // for i in 0..cfg.plugin.node_pos.len() - 2{
            //     let p1 = cfg.plugin.node_pos[i].clone();
            //     let p2 = cfg.plugin.node_pos[i+1].clone();
            //     Self::draw_line_between_buttons(ui.painter(),p1,p2);
            // }
        // });
    }
}

impl MainView for WorkFlowView {
    fn name(&self) -> &str {
        ""
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame, cfg: &mut State) {
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