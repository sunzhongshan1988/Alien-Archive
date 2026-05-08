use eframe::egui::{self, Button, Response, Vec2};

pub(crate) const ICON_BUTTON_SIZE: Vec2 = Vec2::new(24.0, 24.0);

pub(crate) fn editor_icon_button(ui: &mut egui::Ui, label: &str, tooltip: &str) -> Response {
    ui.add_sized(ICON_BUTTON_SIZE, Button::new(label).corner_radius(3.0))
        .on_hover_text(tooltip)
}
