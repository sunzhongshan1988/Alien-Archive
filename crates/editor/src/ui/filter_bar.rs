use eframe::egui::{self, Response, RichText, Stroke, vec2};

use crate::ui::theme::{
    THEME_ACCENT_DIM, THEME_ACCENT_STRONG, THEME_BORDER, THEME_MUTED_TEXT, THEME_PANEL_BG_SOFT,
    THEME_TEXT,
};

pub(crate) fn filter_bar<T>(
    ui: &mut egui::Ui,
    active: &mut T,
    options: impl IntoIterator<Item = (T, &'static str)>,
) where
    T: Copy + Eq,
{
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (value, label) in options {
            let response = filter_chip(ui, *active == value, label);
            if response.clicked() {
                *active = value;
            }
        }
    });
}

fn filter_chip(ui: &mut egui::Ui, selected: bool, label: &str) -> Response {
    let text = RichText::new(label).color(if selected {
        THEME_TEXT
    } else {
        THEME_MUTED_TEXT
    });
    let fill = if selected {
        THEME_ACCENT_DIM
    } else {
        ui.visuals().panel_fill
    };
    let stroke = Stroke::new(
        if selected { 1.5 } else { 1.0 },
        if selected {
            THEME_ACCENT_STRONG
        } else {
            THEME_BORDER
        },
    );
    let width = (label.chars().count() as f32 * 16.0 + 20.0).clamp(44.0, 96.0);

    let response = ui.add_sized(
        vec2(width, 26.0),
        egui::Button::new(text)
            .corner_radius(3.0)
            .fill(fill)
            .stroke(stroke),
    );
    if response.hovered() && !selected {
        ui.painter()
            .rect_filled(response.rect.shrink(1.0), 3.0, THEME_PANEL_BG_SOFT);
        ui.painter().text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::TextStyle::Button.resolve(ui.style()),
            THEME_MUTED_TEXT,
        );
    }
    response
}
