use eframe::egui::{self, Response, RichText, Sense, Stroke, StrokeKind, vec2};

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
    let response = ui.add(egui::Label::new(text).sense(Sense::click()));
    let rect = response.rect.expand2(vec2(8.0, 3.0));
    let fill = if selected {
        THEME_ACCENT_DIM
    } else if response.hovered() {
        THEME_PANEL_BG_SOFT
    } else {
        ui.visuals().panel_fill
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(
            if selected { 1.5 } else { 1.0 },
            if selected {
                THEME_ACCENT_STRONG
            } else {
                THEME_BORDER
            },
        ),
        StrokeKind::Inside,
    );
    response
}
