use eframe::egui::{self, RichText, vec2};

use crate::ui::theme::{THEME_ACCENT_STRONG, THEME_WARNING};

pub(crate) fn command_bar<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        add_contents(ui)
    })
}

pub(crate) fn command_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .corner_radius(3.0)
            .min_size(vec2(54.0, 26.0)),
    )
}

pub(crate) fn enabled_command_button(
    ui: &mut egui::Ui,
    enabled: bool,
    label: &str,
) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(label)
            .corner_radius(3.0)
            .min_size(vec2(54.0, 26.0)),
    )
}

pub(crate) fn command_status_badge(ui: &mut egui::Ui, label: &str, status: CommandBadgeStatus) {
    let color = match status {
        CommandBadgeStatus::Dirty => THEME_WARNING,
        CommandBadgeStatus::Ok => THEME_ACCENT_STRONG,
    };
    ui.label(RichText::new(label).color(color).monospace());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandBadgeStatus {
    Ok,
    Dirty,
}
