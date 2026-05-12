use eframe::egui::{
    self, Color32, Pos2, Rect, Shape, Stroke,
    epaint::{Mesh, Vertex},
    vec2,
};

use crate::{ui::theme::*, util::geometry::normalize_rotation};

pub(crate) fn draw_grid(
    painter: &egui::Painter,
    clip_rect: Rect,
    map_rect: Rect,
    width: u32,
    height: u32,
    tile_size: f32,
) {
    if tile_size < 4.0 {
        return;
    }

    let stroke = Stroke::new(
        1.0,
        Color32::from_rgba_unmultiplied(
            THEME_MUTED_TEXT.r(),
            THEME_MUTED_TEXT.g(),
            THEME_MUTED_TEXT.b(),
            34,
        ),
    );
    let clipped = map_rect.intersect(clip_rect);
    if clipped.is_negative() {
        return;
    }

    for x in 0..=width {
        let screen_x = map_rect.min.x + x as f32 * tile_size;
        if screen_x < clip_rect.left() || screen_x > clip_rect.right() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(screen_x, clipped.top()),
                Pos2::new(screen_x, clipped.bottom()),
            ],
            stroke,
        );
    }

    for y in 0..=height {
        let screen_y = map_rect.min.y + y as f32 * tile_size;
        if screen_y < clip_rect.top() || screen_y > clip_rect.bottom() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(clipped.left(), screen_y),
                Pos2::new(clipped.right(), screen_y),
            ],
            stroke,
        );
    }
}

pub(crate) fn paint_transformed_image(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    rect: Rect,
    flip_x: bool,
    rotation: i32,
    tint: Color32,
) {
    let center = rect.center();
    let half_size = rect.size() * 0.5;
    let rotation = (normalize_rotation(rotation) as f32).to_radians();
    let cos = rotation.cos();
    let sin = rotation.sin();
    let corners = [
        vec2(-half_size.x, -half_size.y),
        vec2(half_size.x, -half_size.y),
        vec2(half_size.x, half_size.y),
        vec2(-half_size.x, half_size.y),
    ];
    let [uv_left, uv_right] = if flip_x { [1.0, 0.0] } else { [0.0, 1.0] };
    let uvs = [
        Pos2::new(uv_left, 0.0),
        Pos2::new(uv_right, 0.0),
        Pos2::new(uv_right, 1.0),
        Pos2::new(uv_left, 1.0),
    ];

    let mut mesh = Mesh::with_texture(texture_id);
    for (corner, uv) in corners.into_iter().zip(uvs) {
        let rotated = vec2(
            corner.x * cos - corner.y * sin,
            corner.x * sin + corner.y * cos,
        );
        mesh.vertices.push(Vertex {
            pos: center + rotated,
            uv,
            color: tint,
        });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(Shape::mesh(mesh));
}

pub(crate) fn zone_colors(zone_type: &str) -> (Color32, Color32) {
    match zone_type {
        content::semantics::ZONE_SCAN_AREA => (
            Color32::from_rgb(156, 166, 126),
            Color32::from_rgba_unmultiplied(156, 166, 126, 34),
        ),
        content::semantics::ZONE_MAP_TRANSITION => (
            THEME_WARNING,
            Color32::from_rgba_unmultiplied(
                THEME_WARNING.r(),
                THEME_WARNING.g(),
                THEME_WARNING.b(),
                34,
            ),
        ),
        content::semantics::ZONE_NO_SPAWN => (
            THEME_ERROR,
            Color32::from_rgba_unmultiplied(THEME_ERROR.r(), THEME_ERROR.g(), THEME_ERROR.b(), 34),
        ),
        content::semantics::ZONE_CAMERA_BOUNDS => (
            Color32::from_rgb(152, 156, 126),
            Color32::from_rgba_unmultiplied(152, 156, 126, 34),
        ),
        content::semantics::ZONE_WALK_SURFACE => (
            Color32::from_rgb(100, 184, 170),
            Color32::from_rgba_unmultiplied(100, 184, 170, 34),
        ),
        content::semantics::ZONE_SURFACE_GATE => (
            Color32::from_rgb(248, 198, 86),
            Color32::from_rgba_unmultiplied(248, 198, 86, 28),
        ),
        content::semantics::ZONE_COLLISION_AREA => (
            THEME_ERROR,
            Color32::from_rgba_unmultiplied(THEME_ERROR.r(), THEME_ERROR.g(), THEME_ERROR.b(), 28),
        ),
        content::semantics::ZONE_COLLISION_LINE => (
            Color32::from_rgb(236, 126, 92),
            Color32::from_rgba_unmultiplied(236, 126, 92, 20),
        ),
        content::semantics::ZONE_HAZARD => (
            Color32::from_rgb(236, 92, 74),
            Color32::from_rgba_unmultiplied(236, 92, 74, 34),
        ),
        content::semantics::ZONE_PROMPT => (
            Color32::from_rgb(112, 172, 255),
            Color32::from_rgba_unmultiplied(112, 172, 255, 34),
        ),
        content::semantics::ZONE_OBJECTIVE | content::semantics::ZONE_CHECKPOINT => (
            Color32::from_rgb(92, 230, 202),
            Color32::from_rgba_unmultiplied(92, 230, 202, 34),
        ),
        _ => (
            THEME_ACCENT,
            Color32::from_rgba_unmultiplied(
                THEME_ACCENT.r(),
                THEME_ACCENT.g(),
                THEME_ACCENT.b(),
                34,
            ),
        ),
    }
}
