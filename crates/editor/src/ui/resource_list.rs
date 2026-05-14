use eframe::egui::{self, RichText};

use crate::ui::{
    search::search_field,
    theme::{THEME_ACCENT_STRONG, THEME_MUTED_TEXT, THEME_WARNING},
    tree::{TreeBadge, tree_row},
};

pub(crate) fn resource_list_header(
    ui: &mut egui::Ui,
    title: &str,
    path: &str,
    visible_count: usize,
    total_count: usize,
    dirty: bool,
) {
    ui.heading(title);
    ui.label(RichText::new(path).color(THEME_MUTED_TEXT).monospace());
    ui.add_space(4.0);
    let count_text = if visible_count == total_count {
        format!("{total_count} 个条目")
    } else {
        format!("{visible_count} / {total_count} 个条目")
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(count_text).color(THEME_MUTED_TEXT));
        if dirty {
            ui.label(RichText::new("未保存").color(THEME_WARNING).monospace());
        } else {
            ui.label(
                RichText::new("已保存")
                    .color(THEME_ACCENT_STRONG)
                    .monospace(),
            );
        }
    });
}

pub(crate) fn resource_search(ui: &mut egui::Ui, value: &mut String, placeholder: &str) {
    search_field(ui, value, placeholder);
}

pub(crate) fn resource_row<'a>(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    detail: &str,
    badges: impl IntoIterator<Item = TreeBadge<'a>>,
) -> egui::Response {
    tree_row(ui, selected, label, detail, badges)
}
