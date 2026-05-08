use eframe::egui::{
    self, Color32, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextureHandle, vec2,
};

use crate::ui::theme::{
    THEME_ACCENT_DIM, THEME_ACCENT_STRONG, THEME_BORDER, THEME_PANEL_BG_SOFT, THEME_TEXT,
};

pub(crate) const ASSET_TILE_SIZE: egui::Vec2 = egui::Vec2::new(56.0, 64.0);
const ASSET_THUMB_SIZE: egui::Vec2 = egui::Vec2::new(40.0, 40.0);

pub(crate) fn asset_grid_columns(ui: &egui::Ui) -> usize {
    (ui.available_width() / ASSET_TILE_SIZE.x).floor().max(1.0) as usize
}

pub(crate) fn asset_tile(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    texture: Option<&TextureHandle>,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(ASSET_TILE_SIZE, Sense::click());
    paint_selectable_rect(ui, rect, selected, response.hovered(), 3.0);

    let image_slot = Rect::from_min_size(rect.min + vec2(8.0, 4.0), ASSET_THUMB_SIZE);
    paint_thumbnail(ui, image_slot, texture);

    let label_rect = Rect::from_min_size(rect.min + vec2(4.0, 46.0), vec2(48.0, 14.0));
    ui.painter().text(
        label_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Small.resolve(ui.style()),
        THEME_TEXT,
    );

    response
}

fn paint_selectable_rect(ui: &egui::Ui, rect: Rect, selected: bool, hovered: bool, radius: f32) {
    let fill = if selected {
        THEME_ACCENT_DIM
    } else if hovered {
        Color32::from_rgb(42, 40, 35)
    } else {
        THEME_PANEL_BG_SOFT
    };
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(
            if selected { 2.0 } else { 1.0 },
            if selected {
                THEME_ACCENT_STRONG
            } else {
                THEME_BORDER
            },
        ),
        StrokeKind::Inside,
    );
}

fn paint_thumbnail(ui: &egui::Ui, rect: Rect, texture: Option<&TextureHandle>) {
    if let Some(texture) = texture {
        let image_rect = fit_centered_rect(rect, texture.size_vec2());
        ui.painter().image(
            texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter().rect_filled(rect, 2.0, THEME_PANEL_BG_SOFT);
    }
}

fn fit_centered_rect(slot: Rect, image_size: egui::Vec2) -> Rect {
    if image_size.x <= 0.0 || image_size.y <= 0.0 {
        return slot;
    }
    let scale = (slot.width() / image_size.x).min(slot.height() / image_size.y);
    let size = image_size * scale;
    Rect::from_center_size(slot.center(), size)
}
