use eframe::egui::{self, RichText};

use crate::ui::{buttons::editor_icon_button, theme::THEME_TEXT};

pub(crate) fn panel_header(
    ui: &mut egui::Ui,
    title: &str,
    action_label: &str,
    action_tooltip: &str,
) -> bool {
    let mut action_clicked = false;
    ui.horizontal(|ui| {
        ui.heading(RichText::new(title).color(THEME_TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            action_clicked = editor_icon_button(ui, action_label, action_tooltip).clicked();
        });
    });
    ui.separator();
    action_clicked
}
