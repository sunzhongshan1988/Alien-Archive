use eframe::egui::{self, Button, Rect, Response, TextureOptions, Vec2, vec2};

pub(crate) const ICON_BUTTON_SIZE: Vec2 = Vec2::new(24.0, 24.0);
pub(crate) const LUCIDE_EYE_URI: &str = "bytes://editor/lucide/eye.svg";
pub(crate) const LUCIDE_EYE_OFF_URI: &str = "bytes://editor/lucide/eye-off.svg";
pub(crate) const LUCIDE_TRASH_2_URI: &str = "bytes://editor/lucide/trash-2.svg";

pub(crate) fn editor_icon_button(ui: &mut egui::Ui, label: &str, tooltip: &str) -> Response {
    ui.add_sized(ICON_BUTTON_SIZE, Button::new(label).corner_radius(3.0))
        .on_hover_text(tooltip)
}

pub(crate) fn editor_svg_icon_button_at(
    ui: &mut egui::Ui,
    rect: Rect,
    icon_uri: &'static str,
    tooltip: &str,
) -> Response {
    let response = ui
        .put(rect, Button::new("").frame(false).corner_radius(3.0))
        .on_hover_text(tooltip);
    let visuals = ui.style().interact(&response);
    let icon_rect = Rect::from_center_size(rect.center(), vec2(16.0, 16.0));
    let pixel_size = (ui.pixels_per_point() * icon_rect.size()).round();
    let size_hint = egui::load::SizeHint::Size {
        width: pixel_size.x.max(1.0) as u32,
        height: pixel_size.y.max(1.0) as u32,
        maintain_aspect_ratio: true,
    };
    if ui
        .ctx()
        .try_load_texture(icon_uri, TextureOptions::default(), size_hint)
        .is_ok()
    {
        egui::Image::from_uri(icon_uri)
            .tint(visuals.fg_stroke.color)
            .paint_at(ui, icon_rect);
    }
    response
}
