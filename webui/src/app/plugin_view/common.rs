pub struct Common;

impl Common{
    #[allow(dead_code)]
    pub(crate) fn rust_view_ui(ui: &mut egui::Ui, code: &str) {
        let language = "rs";
        let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
        egui_extras::syntax_highlighting::code_view_ui(ui, &theme, code, language);
    }
    #[allow(dead_code)]
    pub(crate) fn py_view_ui(ui: &mut egui::Ui, code: &str) {
        let language = "py";
        let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
        egui_extras::syntax_highlighting::code_view_ui(ui, &theme, code, language);
    }
    pub(crate) fn json_view_ui(ui: &mut egui::Ui, code: &str) {
        let language = "toml";
        let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
        egui_extras::syntax_highlighting::code_view_ui(ui, &theme, code, language);
    }
}