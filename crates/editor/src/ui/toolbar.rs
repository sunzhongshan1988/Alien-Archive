use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, Context as EguiContext, FontId, Rect, Sense, StrokeKind, TextureOptions,
    Vec2, vec2,
};

use crate::ToolKind;
use crate::ui::buttons::{LUCIDE_EYE_OFF_URI, LUCIDE_EYE_URI, LUCIDE_TRASH_2_URI};
use crate::ui::theme::{THEME_ACCENT_STRONG, THEME_MUTED_TEXT};

pub(crate) const TOOLBAR_HEIGHT: f32 = 32.0;
const TOOLBAR_CONTROL_HEIGHT: f32 = 26.0;
const TOOL_BUTTON_SIZE: Vec2 = Vec2::new(34.0, 28.0);
const TOOL_ICON_SELECT_URI: &str = "bytes://editor/lucide/mouse-pointer-2.svg";
const TOOL_ICON_BRUSH_URI: &str = "bytes://editor/lucide/paintbrush.svg";
const TOOL_ICON_BUCKET_URI: &str = "bytes://editor/lucide/paint-bucket.svg";
const TOOL_ICON_RECTANGLE_URI: &str = "bytes://editor/lucide/square.svg";
const TOOL_ICON_ERASE_URI: &str = "bytes://editor/lucide/eraser.svg";
const TOOL_ICON_EYEDROPPER_URI: &str = "bytes://editor/lucide/pipette.svg";
const TOOL_ICON_STAMP_URI: &str = "bytes://editor/lucide/stamp.svg";
const TOOL_ICON_COLLISION_URI: &str = "bytes://editor/lucide/brick-wall.svg";
const TOOL_ICON_ZONE_URI: &str = "bytes://editor/lucide/scan.svg";
const TOOL_ICON_PAN_URI: &str = "bytes://editor/lucide/hand.svg";
const TOOL_ICON_ZOOM_URI: &str = "bytes://editor/lucide/zoom-in.svg";
const TOOL_ICON_FALLBACK_URI: &str = TOOL_ICON_SELECT_URI;

pub(crate) fn toolbar_label(ui: &mut egui::Ui, text: &str) {
    let width = toolbar_label_width(text);
    let (rect, _) = ui.allocate_exact_size(vec2(width, TOOLBAR_CONTROL_HEIGHT), Sense::hover());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        THEME_MUTED_TEXT,
    );
}

pub(crate) fn toolbar_centered<R>(
    ui: &mut egui::Ui,
    size: Vec2,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.with_layout(
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            add_contents,
        )
        .inner
    })
}

pub(crate) fn toolbar_tool_button(
    ui: &mut egui::Ui,
    selected: bool,
    tool: ToolKind,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(TOOL_BUTTON_SIZE, Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect_filled(rect, 3.0, visuals.weak_bg_fill);
    ui.painter()
        .rect_stroke(rect, 3.0, visuals.bg_stroke, StrokeKind::Inside);
    draw_tool_icon(
        ui,
        tool,
        rect.shrink2(vec2(5.0, 3.0)),
        if selected {
            THEME_ACCENT_STRONG
        } else {
            visuals.fg_stroke.color
        },
    );
    response.on_hover_text(tool.menu_label())
}

pub(crate) fn toolbar_command_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    ui.add(
        egui::Button::new(text)
            .corner_radius(3.0)
            .min_size(vec2(width, TOOLBAR_CONTROL_HEIGHT)),
    )
}

fn toolbar_label_width(text: &str) -> f32 {
    (text.chars().count() as f32 * 14.0 + 8.0).max(34.0)
}

fn draw_tool_icon(ui: &egui::Ui, tool: ToolKind, rect: Rect, color: Color32) {
    let uri = tool_icon_uri(tool);
    let pixel_size = (ui.pixels_per_point() * rect.size()).round();
    let size_hint = egui::load::SizeHint::Size {
        width: pixel_size.x.max(1.0) as u32,
        height: pixel_size.y.max(1.0) as u32,
        maintain_aspect_ratio: false,
    };
    let uri = if ui
        .ctx()
        .try_load_texture(uri, TextureOptions::default(), size_hint)
        .is_err()
    {
        TOOL_ICON_FALLBACK_URI
    } else {
        uri
    };

    egui::Image::from_uri(uri).tint(color).paint_at(ui, rect);
}

fn tool_icon_uri(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Select => TOOL_ICON_SELECT_URI,
        ToolKind::Brush => TOOL_ICON_BRUSH_URI,
        ToolKind::Bucket => TOOL_ICON_BUCKET_URI,
        ToolKind::Rectangle => TOOL_ICON_RECTANGLE_URI,
        ToolKind::Erase => TOOL_ICON_ERASE_URI,
        ToolKind::Eyedropper => TOOL_ICON_EYEDROPPER_URI,
        ToolKind::Stamp => TOOL_ICON_STAMP_URI,
        ToolKind::Collision => TOOL_ICON_COLLISION_URI,
        ToolKind::Zone => TOOL_ICON_ZONE_URI,
        ToolKind::Pan => TOOL_ICON_PAN_URI,
        ToolKind::Zoom => TOOL_ICON_ZOOM_URI,
    }
}

pub(crate) fn configure_tool_icons(ctx: &EguiContext) {
    egui_extras::install_image_loaders(ctx);

    for (uri, svg) in [
        (
            TOOL_ICON_SELECT_URI,
            include_str!("../../assets/icons/lucide/mouse-pointer-2.svg"),
        ),
        (
            TOOL_ICON_BRUSH_URI,
            include_str!("../../assets/icons/lucide/paintbrush.svg"),
        ),
        (
            TOOL_ICON_BUCKET_URI,
            include_str!("../../assets/icons/lucide/paint-bucket.svg"),
        ),
        (
            TOOL_ICON_RECTANGLE_URI,
            include_str!("../../assets/icons/lucide/square.svg"),
        ),
        (
            TOOL_ICON_ERASE_URI,
            include_str!("../../assets/icons/lucide/eraser.svg"),
        ),
        (
            TOOL_ICON_EYEDROPPER_URI,
            include_str!("../../assets/icons/lucide/pipette.svg"),
        ),
        (
            TOOL_ICON_STAMP_URI,
            include_str!("../../assets/icons/lucide/stamp.svg"),
        ),
        (
            TOOL_ICON_COLLISION_URI,
            include_str!("../../assets/icons/lucide/brick-wall.svg"),
        ),
        (
            TOOL_ICON_ZONE_URI,
            include_str!("../../assets/icons/lucide/scan.svg"),
        ),
        (
            TOOL_ICON_PAN_URI,
            include_str!("../../assets/icons/lucide/hand.svg"),
        ),
        (
            TOOL_ICON_ZOOM_URI,
            include_str!("../../assets/icons/lucide/zoom-in.svg"),
        ),
        (
            LUCIDE_EYE_URI,
            include_str!("../../assets/icons/lucide/eye.svg"),
        ),
        (
            LUCIDE_EYE_OFF_URI,
            include_str!("../../assets/icons/lucide/eye-off.svg"),
        ),
        (
            LUCIDE_TRASH_2_URI,
            include_str!("../../assets/icons/lucide/trash-2.svg"),
        ),
    ] {
        let bytes = normalize_svg_icon(svg).into_bytes().into_boxed_slice();
        ctx.include_bytes(uri, egui::load::Bytes::Shared(Arc::from(bytes)));
    }
}

fn normalize_svg_icon(svg: &str) -> String {
    svg.replace("currentColor", "#ffffff")
        .replace("black", "#ffffff")
        .replace("#000000", "#ffffff")
        .replace("#000", "#ffffff")
}
