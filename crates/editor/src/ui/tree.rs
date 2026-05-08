use eframe::egui::{self, Color32, Response, RichText, Sense, UiBuilder, vec2};

use crate::ui::{
    badge::status_badge,
    theme::{THEME_ACCENT_DIM, THEME_PANEL_BG_SOFT},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeBadge<'a> {
    pub(crate) label: &'a str,
    pub(crate) color: Color32,
}

pub(crate) fn tree_row<'a>(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    detail: &str,
    badges: impl IntoIterator<Item = TreeBadge<'a>>,
) -> Response {
    let row_height = if detail.is_empty() { 24.0 } else { 30.0 };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::click());

    if selected {
        ui.painter()
            .rect_filled(rect.expand(1.0), 2.0, THEME_ACCENT_DIM);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect.expand(1.0), 2.0, THEME_PANEL_BG_SOFT);
    }

    ui.scope_builder(
        UiBuilder::new().max_rect(rect.shrink2(vec2(4.0, 2.0))),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(label).strong());
                for badge in badges {
                    status_badge(ui, badge.label, badge.color);
                }
                if !detail.is_empty() {
                    ui.small(detail);
                }
            });
        },
    );

    response
}
