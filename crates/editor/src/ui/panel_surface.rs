use eframe::egui::{self, RichText};

use crate::ui::theme::{
    THEME_BORDER, THEME_MUTED_TEXT, THEME_PANEL_BG, THEME_PANEL_BG_SOFT, THEME_TEXT,
};

pub(crate) fn panel_surface<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(THEME_PANEL_BG)
        .inner_margin(egui::Margin::same(10))
        .show(ui, add_contents)
}

pub(crate) fn panel_header(ui: &mut egui::Ui, title: &str, subtitle: Option<&str>) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new(title).color(THEME_TEXT));
        if let Some(subtitle) = subtitle {
            ui.label(RichText::new(subtitle).color(THEME_MUTED_TEXT));
        }
    });
    ui.separator();
}

pub(crate) fn detail_surface<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(THEME_PANEL_BG_SOFT)
        .stroke(egui::Stroke::new(1.0, THEME_BORDER))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, add_contents)
}

pub(crate) fn empty_state(ui: &mut egui::Ui, title: &str, body: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(72.0);
        ui.heading(RichText::new(title).color(THEME_TEXT));
        ui.add_space(4.0);
        ui.label(RichText::new(body).color(THEME_MUTED_TEXT));
    });
}
