pub struct DebugView;

impl DebugView {
    pub fn ui(ui: &mut egui::Ui, out: &str) {
        super::Common::json_view_ui(ui, out);
    }
}
