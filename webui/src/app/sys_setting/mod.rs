mod font;

pub fn sys_set(ctx: &egui::Context) {
    font::setup_custom_fonts(ctx);
}
