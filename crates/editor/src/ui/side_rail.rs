use eframe::egui;

use crate::ui::buttons::editor_icon_button;

pub(crate) fn collapsed_side_rail(ui: &mut egui::Ui, action_label: &str, tooltip: &str) -> bool {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(6.0);
        clicked = editor_icon_button(ui, action_label, tooltip).clicked();
    });
    clicked
}
