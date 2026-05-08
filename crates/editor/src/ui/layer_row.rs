use eframe::egui::{self, Response, RichText, Sense, UiBuilder, vec2};

use crate::ui::theme::{THEME_ACCENT_DIM, THEME_PANEL_BG_SOFT, THEME_TEXT};

#[derive(Debug)]
pub(crate) struct LayerRowResponse {
    pub(crate) selected_clicked: bool,
}

pub(crate) fn layer_row(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    count: usize,
    visible: &mut bool,
    locked: &mut bool,
) -> LayerRowResponse {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
    paint_row_background(ui, selected, &response);

    let mut visible_changed = false;
    let mut locked_changed = false;
    ui.scope_builder(
        UiBuilder::new().max_rect(rect.shrink2(vec2(4.0, 2.0))),
        |ui| {
            ui.horizontal(|ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new(format!("{label} ({count})")).color(THEME_TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    locked_changed = ui.checkbox(locked, "锁").changed();
                    visible_changed = ui.checkbox(visible, "显").changed();
                });
            });
        },
    );

    LayerRowResponse {
        selected_clicked: response.clicked() && !visible_changed && !locked_changed,
    }
}

fn paint_row_background(ui: &egui::Ui, selected: bool, response: &Response) {
    if selected {
        ui.painter()
            .rect_filled(response.rect.expand(1.0), 2.0, THEME_ACCENT_DIM);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(response.rect.expand(1.0), 2.0, THEME_PANEL_BG_SOFT);
    }
}
