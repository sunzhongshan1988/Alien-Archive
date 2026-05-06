use std::sync::Arc;

use eframe::egui::{
    self, Color32, Context as EguiContext, Rect, Sense, StrokeKind, TextureOptions, Vec2, vec2,
};

use crate::ToolKind;
use crate::ui::theme::{THEME_ACCENT_STRONG, THEME_MUTED_TEXT};

pub(crate) const TOOLBAR_HEIGHT: f32 = 32.0;
const TOOL_BUTTON_SIZE: Vec2 = Vec2::new(34.0, 28.0);
const TOOL_ICON_FALLBACK_URI: &str = "bytes://editor/tools/fallback.svg";

pub(crate) fn toolbar_label(ui: &mut egui::Ui, text: &str) {
    ui.add_sized(
        [ui.spacing().interact_size.y.max(34.0), 26.0],
        egui::Label::new(egui::RichText::new(text).color(THEME_MUTED_TEXT)),
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
    ui.add_sized([width, 26.0], egui::Button::new(text).corner_radius(3.0))
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
        ToolKind::Select => "bytes://editor/tools/select.svg",
        ToolKind::Brush => "bytes://editor/tools/brush.svg",
        ToolKind::Bucket => "bytes://editor/tools/bucket.svg",
        ToolKind::Rectangle => "bytes://editor/tools/rectangle.svg",
        ToolKind::Erase => "bytes://editor/tools/erase.svg",
        ToolKind::Eyedropper => "bytes://editor/tools/eyedropper.svg",
        ToolKind::Collision => "bytes://editor/tools/collision.svg",
        ToolKind::Zone => "bytes://editor/tools/zone.svg",
        ToolKind::Pan => "bytes://editor/tools/pan.svg",
        ToolKind::Zoom => "bytes://editor/tools/zoom.svg",
    }
}

pub(crate) fn configure_tool_icons(ctx: &EguiContext) {
    egui_extras::install_image_loaders(ctx);

    for (uri, svg) in [
        (
            TOOL_ICON_FALLBACK_URI,
            include_str!("../../assets/icons/tools/fallback.svg"),
        ),
        (
            "bytes://editor/tools/select.svg",
            include_str!("../../assets/icons/tools/select.svg"),
        ),
        (
            "bytes://editor/tools/brush.svg",
            include_str!("../../assets/icons/tools/brush.svg"),
        ),
        (
            "bytes://editor/tools/bucket.svg",
            include_str!("../../assets/icons/tools/bucket.svg"),
        ),
        (
            "bytes://editor/tools/rectangle.svg",
            include_str!("../../assets/icons/tools/rectangle.svg"),
        ),
        (
            "bytes://editor/tools/erase.svg",
            include_str!("../../assets/icons/tools/erase.svg"),
        ),
        (
            "bytes://editor/tools/eyedropper.svg",
            include_str!("../../assets/icons/tools/eyedropper.svg"),
        ),
        (
            "bytes://editor/tools/collision.svg",
            include_str!("../../assets/icons/tools/collision.svg"),
        ),
        (
            "bytes://editor/tools/zone.svg",
            include_str!("../../assets/icons/tools/zone.svg"),
        ),
        (
            "bytes://editor/tools/pan.svg",
            include_str!("../../assets/icons/tools/pan.svg"),
        ),
        (
            "bytes://editor/tools/zoom.svg",
            include_str!("../../assets/icons/tools/zoom.svg"),
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
