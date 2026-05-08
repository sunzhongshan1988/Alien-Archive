use eframe::egui::{self, RichText};

use crate::ui::theme::THEME_MUTED_TEXT;

pub(crate) fn inspector_section(ui: &mut egui::Ui, title: &str) {
    ui.separator();
    ui.label(RichText::new(title).strong().color(THEME_MUTED_TEXT));
}
