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

    fn show_code(ui: &mut egui::Ui, code: &str) {
        let code = Common::remove_leading_indentation(code.trim_start_matches('\n'));
        Common::rust_view_ui(ui, &code);
    }

    fn remove_leading_indentation(code: &str) -> String {
        fn is_indent(c: &u8) -> bool {
            matches!(*c, b' ' | b'\t')
        }

        let first_line_indent = code.bytes().take_while(is_indent).count();

        let mut out = String::new();

        let mut code = code;
        while !code.is_empty() {
            let indent = code.bytes().take_while(is_indent).count();
            let start = first_line_indent.min(indent);
            let end = code
                .find('\n')
                .map_or_else(|| code.len(), |endline| endline + 1);
            out += &code[start..end];
            code = &code[end..];
        }
        out
    }
}