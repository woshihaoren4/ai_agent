use std::collections::BTreeMap;
use egui::Context;
use crate::app::main_frame::WorkFlowDebugRequest;
use crate::app::state::{PluginServiceWin, State};

pub struct TextControlView;

impl TextControlView{
    pub fn ui(ctx: &Context,cfg: &mut State){
        if !cfg.options_view.text_view_open {
            return;
        }
        egui::Window::new("plan-text-view")
            .open(&mut cfg.options_view.text_view_open)
            .show(ctx,|ui|{
                ui.horizontal(|ui|{
                    if ui.button("plan").clicked() {
                        let context = WorkFlowDebugRequest::new(&cfg.plugin.nodes)
                            .map(|req|{
                                serde_json::to_string_pretty(&req).unwrap_or_else(|err|{
                                    format!("json plan error:{}",err)
                                })
                            })
                            .unwrap_or_else(|err|{
                                format!("generate plan error:{}",err)
                            });
                        cfg.options_view.text_view_content = context;
                    }
                    if ui.button("down plugin").clicked() {
                        let s = serde_json::to_string(&cfg.plugin.nodes).unwrap_or_else(|e| e.to_string());
                        cfg.options_view.text_view_content = s;
                    }
                    if ui.button("up plugin").clicked() {
                        match serde_json::from_str::<BTreeMap<String, PluginServiceWin>>(cfg.options_view.text_view_content.as_str()) {
                            Ok(o)=>{
                                cfg.plugin.nodes = o;
                                cfg.debug_win.info("upload plugin success!!!");
                            }
                            Err(e)=>{
                                cfg.debug_win.error(format!("upload plugin error:{}",e));
                            }
                        }
                    }
                });
                ui.separator();
                let width = ui.available_width();
                egui::TextEdit::multiline(&mut cfg.options_view.text_view_content)
                    .desired_width(width)
                    .show(ui);
            });
    }
}