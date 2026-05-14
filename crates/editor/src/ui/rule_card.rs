use eframe::egui::{self, RichText, vec2};

use crate::ui::theme::{
    THEME_ACCENT_STRONG, THEME_BORDER, THEME_MUTED_TEXT, THEME_PANEL_BG_SOFT, THEME_TEXT,
};

pub(crate) fn add_rule_menu<T: Copy>(
    ui: &mut egui::Ui,
    label: &str,
    options: &[T],
    option_label: impl Fn(T) -> &'static str,
    mut on_add: impl FnMut(T),
) {
    ui.menu_button(label, |ui| {
        for option in options {
            if ui.button(option_label(*option)).clicked() {
                on_add(*option);
                ui.close();
            }
        }
    });
}

pub(crate) fn rule_card<R>(
    ui: &mut egui::Ui,
    number: usize,
    title: &str,
    subtitle: &str,
    draw_header_actions: impl FnOnce(&mut egui::Ui),
    draw_body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(THEME_PANEL_BG_SOFT)
        .stroke(egui::Stroke::new(1.0, THEME_BORDER))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{number:02}"))
                        .monospace()
                        .color(THEME_ACCENT_STRONG),
                );
                ui.label(RichText::new(title).strong().color(THEME_TEXT));
                if !subtitle.is_empty() {
                    ui.label(RichText::new(subtitle).color(THEME_MUTED_TEXT));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    draw_header_actions(ui);
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            draw_body(ui)
        })
}

pub(crate) fn card_section_header(ui: &mut egui::Ui, title: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.label(RichText::new(format!("{count}")).color(THEME_MUTED_TEXT));
    });
}

pub(crate) fn card_gap(ui: &mut egui::Ui) {
    ui.add_space(8.0);
}

pub(crate) fn compact_card_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .corner_radius(3.0)
            .min_size(vec2(42.0, 24.0)),
    )
}
