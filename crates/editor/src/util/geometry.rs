use content::AnchorKind;
use eframe::egui::{Modifiers, Pos2, Rect, Vec2, vec2};

pub(crate) fn fit_centered_rect(bounds: Rect, source_size: Vec2) -> Rect {
    let width = source_size.x.max(1.0);
    let height = source_size.y.max(1.0);
    let scale = (bounds.width() / width).min(bounds.height() / height);

    Rect::from_center_size(bounds.center(), vec2(width * scale, height * scale))
}

pub(crate) fn anchor_grid_to_world(tile_size: f32, x: f32, y: f32, anchor: AnchorKind) -> Vec2 {
    match anchor {
        AnchorKind::TopLeft => vec2(x * tile_size, y * tile_size),
        AnchorKind::Center => vec2((x + 0.5) * tile_size, (y + 0.5) * tile_size),
        AnchorKind::BottomCenter => vec2((x + 0.5) * tile_size, (y + 1.0) * tile_size),
    }
}

pub(crate) fn screen_rect_from_anchor(anchor: Pos2, size: Vec2, anchor_kind: AnchorKind) -> Rect {
    let min = match anchor_kind {
        AnchorKind::TopLeft => anchor,
        AnchorKind::Center => Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y * 0.5),
        AnchorKind::BottomCenter => Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y),
    };
    Rect::from_min_size(min, size)
}

pub(crate) fn resize_handle_rects(rect: Rect) -> [Rect; 4] {
    const SIZE: f32 = 9.0;
    [
        Rect::from_center_size(rect.left_top(), vec2(SIZE, SIZE)),
        Rect::from_center_size(rect.right_top(), vec2(SIZE, SIZE)),
        Rect::from_center_size(rect.left_bottom(), vec2(SIZE, SIZE)),
        Rect::from_center_size(rect.right_bottom(), vec2(SIZE, SIZE)),
    ]
}

pub(crate) fn normalize_rotation(rotation: i32) -> i32 {
    rotation.rem_euclid(360)
}

pub(crate) fn polygon_screen_center(points: &[Pos2]) -> Pos2 {
    if points.is_empty() {
        return Pos2::ZERO;
    }
    let sum = points
        .iter()
        .fold(vec2(0.0, 0.0), |sum, point| sum + point.to_vec2());
    Pos2::new(sum.x / points.len() as f32, sum.y / points.len() as f32)
}

pub(crate) fn distance_sq(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

pub(crate) fn snapped_delta(delta: [f32; 2], modifiers: Modifiers) -> [f32; 2] {
    if modifiers.alt {
        delta
    } else if modifiers.shift {
        [
            (delta[0] * 2.0).round() * 0.5,
            (delta[1] * 2.0).round() * 0.5,
        ]
    } else {
        [delta[0].round(), delta[1].round()]
    }
}
