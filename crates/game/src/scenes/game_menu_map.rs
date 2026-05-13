use runtime::{Color, Rect, Renderer, Vec2};

use crate::ui::menu_style::{MenuLayout, inset_rect};
use crate::ui::menu_widgets::{draw_border, draw_inner_panel, draw_screen_rect};
use crate::ui::text::{TextSprite, draw_text};

const MAP_ROUTE_POINTS: [Vec2; 5] = [
    Vec2::new(0.18, 0.64),
    Vec2::new(0.32, 0.52),
    Vec2::new(0.48, 0.44),
    Vec2::new(0.68, 0.34),
    Vec2::new(0.78, 0.56),
];

pub(super) fn draw_map_page(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    layout: &MenuLayout,
    labels: &[TextSprite],
) {
    let content = layout.content_body();
    draw_inner_panel(renderer, viewport, content, layout.scale);
    let map_rect = inset_rect(content, 26.0 * layout.scale);
    draw_screen_rect(
        renderer,
        viewport,
        map_rect,
        Color::rgba(0.012, 0.038, 0.048, 0.94),
    );
    draw_border(
        renderer,
        viewport,
        map_rect,
        1.0 * layout.scale,
        Color::rgba(0.14, 0.34, 0.42, 0.88),
    );

    draw_map_grid(renderer, viewport, map_rect);
    draw_route(renderer, viewport, map_rect, layout.scale);
    draw_map_labels(renderer, viewport, map_rect, labels, layout.scale);
}

fn draw_map_grid(renderer: &mut dyn Renderer, viewport: Vec2, map_rect: Rect) {
    for col in 1..8 {
        let x = map_rect.origin.x + map_rect.size.x * col as f32 / 8.0;
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(x, map_rect.origin.y),
                Vec2::new(1.0, map_rect.size.y),
            ),
            Color::rgba(0.07, 0.17, 0.21, 0.68),
        );
    }

    for row in 1..5 {
        let y = map_rect.origin.y + map_rect.size.y * row as f32 / 5.0;
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(map_rect.origin.x, y),
                Vec2::new(map_rect.size.x, 1.0),
            ),
            Color::rgba(0.07, 0.17, 0.21, 0.68),
        );
    }
}

fn draw_route(renderer: &mut dyn Renderer, viewport: Vec2, map_rect: Rect, scale: f32) {
    for window in MAP_ROUTE_POINTS.windows(2) {
        let start = map_point(map_rect, window[0]);
        let end = map_point(map_rect, window[1]);
        draw_segment(renderer, viewport, start, end, 4.0 * scale);
    }

    for (index, point) in MAP_ROUTE_POINTS.iter().copied().enumerate() {
        let center = map_point(map_rect, point);
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(center.x - 9.0 * scale, center.y - 9.0 * scale),
                Vec2::new(18.0 * scale, 18.0 * scale),
            ),
            if index == 0 {
                Color::rgba(0.82, 0.68, 0.32, 1.0)
            } else {
                Color::rgba(0.30, 0.88, 1.0, 0.92)
            },
        );
    }
}

fn draw_map_labels(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    map_rect: Rect,
    labels: &[TextSprite],
    scale: f32,
) {
    for (index, label) in labels.iter().enumerate() {
        draw_text(
            renderer,
            label,
            viewport,
            map_rect.origin.x + 24.0 * scale,
            map_rect.origin.y + (24.0 + index as f32 * 34.0) * scale,
            Color::rgba(0.70, 0.90, 0.94, 0.96),
        );
    }
}

fn draw_segment(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    start: Vec2,
    end: Vec2,
    thickness: f32,
) {
    let steps = 18;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let point = Vec2::new(
            start.x + (end.x - start.x) * t,
            start.y + (end.y - start.y) * t,
        );
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(point.x - thickness * 0.5, point.y - thickness * 0.5),
                Vec2::new(thickness, thickness),
            ),
            Color::rgba(0.28, 0.86, 1.0, 0.72),
        );
    }
}

fn map_point(rect: Rect, point: Vec2) -> Vec2 {
    Vec2::new(
        rect.origin.x + rect.size.x * point.x,
        rect.origin.y + rect.size.y * point.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_point_projects_normalized_coordinates_into_rect() {
        let rect = Rect::new(Vec2::new(10.0, 20.0), Vec2::new(200.0, 100.0));

        assert_eq!(map_point(rect, Vec2::new(0.25, 0.5)), Vec2::new(60.0, 70.0));
    }

    #[test]
    fn map_route_has_enough_points_to_draw_segments() {
        assert!(MAP_ROUTE_POINTS.len() >= 2);
        assert_eq!(
            MAP_ROUTE_POINTS.windows(2).count(),
            MAP_ROUTE_POINTS.len() - 1
        );
    }
}
