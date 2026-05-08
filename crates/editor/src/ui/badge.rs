use eframe::egui::{self, Color32, RichText};

pub(crate) fn status_badge(ui: &mut egui::Ui, label: &str, color: Color32) {
    ui.label(RichText::new(label).color(color).strong());
}
