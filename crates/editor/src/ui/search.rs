use eframe::egui::{self, TextEdit};

pub(crate) fn search_field(ui: &mut egui::Ui, value: &mut String, hint: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(
                TextEdit::singleline(value)
                    .hint_text(hint)
                    .desired_width(ui.available_width()),
            )
            .changed();
        if !value.is_empty() && ui.small_button("x").on_hover_text("清空搜索").clicked() {
            value.clear();
            changed = true;
        }
    });
    changed
}
