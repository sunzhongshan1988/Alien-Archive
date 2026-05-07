use runtime::{Color, Rect, Renderer, Vec2};

use crate::ui::menu_style::{color, skin};
use crate::ui::text::{TextSprite, draw_text};

pub fn draw_texture_rect(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    texture_id: &str,
    rect: Rect,
    tint: Color,
) -> bool {
    if renderer.texture_size(texture_id).is_none() {
        return false;
    }

    renderer.draw_image(texture_id, screen_rect(viewport, rect), tint);
    true
}

pub fn draw_texture_nine_slice(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    texture_id: &str,
    rect: Rect,
    source_edge: f32,
    dest_edge: f32,
    tint: Color,
) -> bool {
    let Some(texture_size) = renderer.texture_size(texture_id) else {
        return false;
    };

    let source_edge = source_edge
        .min(texture_size.x * 0.45)
        .min(texture_size.y * 0.45)
        .max(1.0);
    let dest_edge = dest_edge
        .min(rect.size.x * 0.35)
        .min(rect.size.y * 0.35)
        .max(1.0);

    let src_x = [0.0, source_edge, texture_size.x - source_edge];
    let src_w = [source_edge, texture_size.x - source_edge * 2.0, source_edge];
    let src_y = [0.0, source_edge, texture_size.y - source_edge];
    let src_h = [source_edge, texture_size.y - source_edge * 2.0, source_edge];
    let dst_x = [
        rect.origin.x,
        rect.origin.x + dest_edge,
        rect.right() - dest_edge,
    ];
    let dst_w = [dest_edge, rect.size.x - dest_edge * 2.0, dest_edge];
    let dst_y = [
        rect.origin.y,
        rect.origin.y + dest_edge,
        rect.bottom() - dest_edge,
    ];
    let dst_h = [dest_edge, rect.size.y - dest_edge * 2.0, dest_edge];

    for row in 0..3 {
        for col in 0..3 {
            if src_w[col] <= 0.0 || src_h[row] <= 0.0 || dst_w[col] <= 0.0 || dst_h[row] <= 0.0 {
                continue;
            }

            renderer.draw_image_region(
                texture_id,
                screen_rect(
                    viewport,
                    Rect::new(
                        Vec2::new(dst_x[col], dst_y[row]),
                        Vec2::new(dst_w[col], dst_h[row]),
                    ),
                ),
                Rect::new(
                    Vec2::new(src_x[col], src_y[row]),
                    Vec2::new(src_w[col], src_h[row]),
                ),
                tint,
            );
        }
    }

    true
}

pub fn draw_screen_rect(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, color: Color) {
    renderer.draw_rect(screen_rect(viewport, rect), color);
}

pub fn screen_rect(viewport: Vec2, rect: Rect) -> Rect {
    Rect::new(
        Vec2::new(
            -viewport.x * 0.5 + rect.origin.x,
            -viewport.y * 0.5 + rect.origin.y,
        ),
        rect.size,
    )
}

pub fn screen_point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}

pub fn draw_text_strong(
    renderer: &mut dyn Renderer,
    text: &TextSprite,
    viewport: Vec2,
    x: f32,
    y: f32,
    color: Color,
    scale: f32,
) {
    draw_text(
        renderer,
        text,
        viewport,
        x + scale.max(1.0),
        y + scale.max(1.0),
        Color::rgba(0.0, 0.0, 0.0, color.a * 0.36),
    );
    draw_text(renderer, text, viewport, x, y, color);
}

pub fn draw_inner_panel(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, scale: f32) {
    let textured = draw_texture_nine_slice(
        renderer,
        viewport,
        skin::PANEL,
        rect,
        46.0,
        26.0 * scale,
        Color::rgba(1.0, 1.0, 1.0, 0.96),
    );
    if !textured {
        draw_screen_rect(renderer, viewport, rect, color::PANEL_FILL);
        draw_border(renderer, viewport, rect, 1.0 * scale, color::PANEL_BORDER);
    }
}

pub fn draw_header_cell(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, scale: f32) {
    let textured = draw_texture_nine_slice(
        renderer,
        viewport,
        skin::HEADER,
        rect,
        36.0,
        18.0 * scale,
        Color::rgba(1.0, 1.0, 1.0, 0.88),
    );
    if !textured {
        draw_screen_rect(renderer, viewport, rect, color::HEADER_CELL_FILL);
        draw_border(
            renderer,
            viewport,
            rect,
            1.0 * scale,
            color::HEADER_CELL_BORDER,
        );
    }
}

pub fn draw_panel_title(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    panel: Rect,
    title: Option<&TextSprite>,
    scale: f32,
    color: Color,
) {
    let title_x = panel.origin.x + 28.0 * scale;
    let title_y = panel.origin.y + 18.0 * scale;
    let rule_y = panel.origin.y + 50.0 * scale;
    if let Some(title) = title {
        draw_text_strong(renderer, title, viewport, title_x, title_y, color, scale);
    }
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(panel.origin.x + 22.0 * scale, rule_y),
            Vec2::new(panel.size.x - 44.0 * scale, 1.0_f32.max(scale * 0.75)),
        ),
        Color::rgba(color.r, color.g, color.b, 0.32),
    );
}

pub fn draw_border(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    thickness: f32,
    color: Color,
) {
    let thickness = thickness.max(1.0);
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(rect.origin, Vec2::new(rect.size.x, thickness)),
        color,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(rect.origin.x, rect.bottom() - thickness),
            Vec2::new(rect.size.x, thickness),
        ),
        color,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(rect.origin, Vec2::new(thickness, rect.size.y)),
        color,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(rect.right() - thickness, rect.origin.y),
            Vec2::new(thickness, rect.size.y),
        ),
        color,
    );
}

pub fn draw_corner_brackets(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    length: f32,
    thickness: f32,
    color: Color,
) {
    let thickness = thickness.max(1.0);
    for (x, y, horizontal_x, vertical_y) in [
        (rect.origin.x, rect.origin.y, rect.origin.x, rect.origin.y),
        (
            rect.right() - length,
            rect.origin.y,
            rect.right() - thickness,
            rect.origin.y,
        ),
        (
            rect.origin.x,
            rect.bottom() - thickness,
            rect.origin.x,
            rect.bottom() - length,
        ),
        (
            rect.right() - length,
            rect.bottom() - thickness,
            rect.right() - thickness,
            rect.bottom() - length,
        ),
    ] {
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(Vec2::new(x, y), Vec2::new(length, thickness)),
            color,
        );
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(horizontal_x, vertical_y),
                Vec2::new(thickness, length),
            ),
            color,
        );
    }
}

pub fn draw_bar(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, ratio: f32, scale: f32) {
    draw_screen_rect(
        renderer,
        viewport,
        rect,
        Color::rgba(0.025, 0.050, 0.060, 0.95),
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            rect.origin,
            Vec2::new(rect.size.x * ratio.clamp(0.0, 1.0), rect.size.y),
        ),
        Color::rgba(0.34, 0.88, 1.0, 0.94),
    );
    draw_border(
        renderer,
        viewport,
        rect,
        1.0 * scale,
        Color::rgba(0.16, 0.32, 0.38, 0.82),
    );
}

pub fn draw_score_pips(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    origin: Vec2,
    value: u32,
    scale: f32,
) {
    for pip in 0..10 {
        let filled = pip < value as usize;
        let rect = Rect::new(
            Vec2::new(origin.x + pip as f32 * 15.0 * scale, origin.y),
            Vec2::new(10.0 * scale, 10.0 * scale),
        );
        draw_screen_rect(
            renderer,
            viewport,
            rect,
            if filled {
                Color::rgba(0.32, 0.86, 1.0, 0.94)
            } else {
                Color::rgba(0.035, 0.070, 0.085, 0.86)
            },
        );
        draw_border(
            renderer,
            viewport,
            rect,
            1.0 * scale,
            Color::rgba(0.12, 0.25, 0.31, 0.84),
        );
    }
}

pub fn contain_rect(frame: Rect, image_size: Vec2) -> Rect {
    if image_size.x <= 0.0 || image_size.y <= 0.0 || frame.size.x <= 0.0 || frame.size.y <= 0.0 {
        return frame;
    }

    let scale = (frame.size.x / image_size.x).min(frame.size.y / image_size.y);
    let size = image_size * scale;
    Rect::new(
        Vec2::new(
            frame.origin.x + (frame.size.x - size.x) * 0.5,
            frame.origin.y + (frame.size.y - size.y) * 0.5,
        ),
        size,
    )
}
