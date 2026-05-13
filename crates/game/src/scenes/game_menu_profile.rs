use runtime::{Color, Rect, Renderer, Vec2};

use crate::save::PlayerProfileSave;
use crate::ui::menu_style::{color, inset_rect};
use crate::ui::menu_widgets::{contain_rect, draw_bar, draw_border, draw_screen_rect, screen_rect};
use crate::ui::text::{TextSprite, draw_text, draw_text_centered};

pub(super) fn xp_ratio(profile: &PlayerProfileSave) -> f32 {
    if profile.xp_next == 0 {
        return 0.0;
    }

    profile.xp as f32 / profile.xp_next as f32
}

pub(super) fn draw_attribute_icon(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    index: usize,
    scale: f32,
) {
    let texture_id = attribute_icon_texture_id(index);
    if let Some(image_size) = renderer.texture_size(texture_id) {
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(rect, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    let color = Color::rgba(0.44, 0.94, 1.0, 0.94);
    let cx = rect.origin.x + rect.size.x * 0.5;
    let cy = rect.origin.y + rect.size.y * 0.5;
    match index {
        0 => {
            draw_border(renderer, viewport, rect, 2.0 * scale, color);
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(cx - 3.0 * scale, rect.origin.y + 5.0 * scale),
                    Vec2::new(6.0 * scale, 18.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(rect.origin.x + 5.0 * scale, cy - 3.0 * scale),
                    Vec2::new(18.0 * scale, 6.0 * scale),
                ),
                color,
            );
        }
        1 => {
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(rect.origin.x + 5.0 * scale, rect.origin.y + 3.0 * scale),
                    Vec2::new(9.0 * scale, 22.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(rect.origin.x + 13.0 * scale, rect.origin.y + 17.0 * scale),
                    Vec2::new(12.0 * scale, 8.0 * scale),
                ),
                color,
            );
        }
        2 => {
            draw_border(
                renderer,
                viewport,
                inset_rect(rect, 4.0 * scale),
                2.0 * scale,
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(cx - 12.0 * scale, cy - 1.0 * scale),
                    Vec2::new(24.0 * scale, 2.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(cx - 1.0 * scale, cy - 12.0 * scale),
                    Vec2::new(2.0 * scale, 24.0 * scale),
                ),
                color,
            );
        }
        3 => {
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(rect.origin.x + 5.0 * scale, rect.origin.y + 7.0 * scale),
                    Vec2::new(20.0 * scale, 5.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(rect.origin.x + 10.0 * scale, rect.origin.y + 12.0 * scale),
                    Vec2::new(5.0 * scale, 14.0 * scale),
                ),
                color,
            );
        }
        _ => {
            draw_border(
                renderer,
                viewport,
                inset_rect(rect, 5.0 * scale),
                2.0 * scale,
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(cx - 10.0 * scale, cy - 2.0 * scale),
                    Vec2::new(20.0 * scale, 4.0 * scale),
                ),
                color,
            );
        }
    }
}

pub(super) fn draw_status_card(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    label: &TextSprite,
    value: &TextSprite,
    ratio: f32,
    index: usize,
    scale: f32,
) {
    let content = card;
    let center_x = content.origin.x + content.size.x * 0.5;
    let accent = vital_status_accent(index);
    draw_soft_status_block(renderer, viewport, content, accent);

    let icon_size = 54.0 * scale;
    let group_top = content.origin.y + ((content.size.y - 138.0 * scale) * 0.5).max(0.0);
    draw_status_icon(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(center_x - icon_size * 0.5, group_top + 12.0 * scale),
            Vec2::new(icon_size, icon_size),
        ),
        index,
        scale,
    );
    draw_text_centered(
        renderer,
        label,
        viewport,
        center_x,
        group_top + 78.0 * scale,
        Color::rgba(0.88, 1.0, 0.98, 1.0),
    );
    draw_text_centered(
        renderer,
        value,
        viewport,
        center_x,
        group_top + 101.0 * scale,
        Color::rgba(0.62, 0.86, 0.92, 0.96),
    );
    draw_bar(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(content.origin.x + 10.0 * scale, group_top + 129.0 * scale),
            Vec2::new(content.size.x - 20.0 * scale, 7.0 * scale),
        ),
        ratio,
        scale,
    );
}

pub(super) fn draw_equipment_status_row(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    row: Rect,
    label: &TextSprite,
    value: &TextSprite,
    ratio: f32,
    index: usize,
    scale: f32,
) {
    let accent = vital_status_accent(index);
    draw_screen_rect(
        renderer,
        viewport,
        row,
        Color::rgba(accent.r, accent.g, accent.b, 0.060),
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(row.origin, Vec2::new(3.0_f32.max(2.0 * scale), row.size.y)),
        Color::rgba(accent.r, accent.g, accent.b, 0.58),
    );

    let label_y = row.origin.y + 6.0 * scale;
    draw_text(
        renderer,
        label,
        viewport,
        row.origin.x + 14.0 * scale,
        label_y,
        color::TEXT_PRIMARY,
    );
    draw_text(
        renderer,
        value,
        viewport,
        row.right() - value.size.x - 14.0 * scale,
        label_y,
        Color::rgba(0.62, 0.86, 0.92, 0.96),
    );

    let bar_y = row.bottom() - 10.0 * scale;
    draw_bar(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(row.origin.x + 14.0 * scale, bar_y),
            Vec2::new(row.size.x - 28.0 * scale, 5.0_f32.max(4.0 * scale)),
        ),
        ratio,
        scale,
    );
}

pub(super) fn draw_compact_stat_bar(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    row: Rect,
    label: &TextSprite,
    value: &TextSprite,
    ratio: f32,
    scale: f32,
) {
    let label_y = row.origin.y - 5.0 * scale;
    let bar_height = 6.0 * scale;
    let bar_y = row.bottom() - bar_height;
    draw_text(
        renderer,
        label,
        viewport,
        row.origin.x,
        label_y,
        Color::rgba(0.68, 0.88, 0.92, 0.96),
    );
    draw_text(
        renderer,
        value,
        viewport,
        row.right() - value.size.x,
        label_y,
        Color::rgba(0.90, 1.0, 0.96, 1.0),
    );
    draw_bar(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(row.origin.x, bar_y),
            Vec2::new(row.size.x, bar_height),
        ),
        ratio,
        scale,
    );
}

fn attribute_icon_texture_id(index: usize) -> &'static str {
    match index {
        0 => "menu.attr_survival",
        1 => "menu.attr_mobility",
        2 => "menu.attr_scanning",
        3 => "menu.attr_gathering",
        _ => "menu.attr_analysis",
    }
}

fn status_icon_texture_id(index: usize) -> &'static str {
    match index {
        0 => "menu.stat_health",
        1 => "menu.stat_stamina",
        2 => "menu.stat_armor",
        _ => "menu.stat_carry",
    }
}

fn vital_status_accent(index: usize) -> Color {
    match index {
        0 => Color::rgba(0.32, 0.90, 1.0, 1.0),
        1 => Color::rgba(0.26, 0.66, 1.0, 1.0),
        2 => Color::rgba(0.52, 0.96, 0.88, 1.0),
        _ => Color::rgba(1.0, 0.70, 0.36, 1.0),
    }
}

fn draw_soft_status_block(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, accent: Color) {
    draw_screen_rect(
        renderer,
        viewport,
        rect,
        Color::rgba(accent.r, accent.g, accent.b, 0.055),
    );
}

fn draw_status_icon(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    index: usize,
    scale: f32,
) {
    let texture_id = status_icon_texture_id(index);
    if let Some(image_size) = renderer.texture_size(texture_id) {
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(rect, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    draw_attribute_icon(renderer, viewport, rect, index, scale);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_ratio_handles_zero_next_level_requirement() {
        let mut profile = PlayerProfileSave::default();
        profile.xp = 75;
        profile.xp_next = 0;

        assert_eq!(xp_ratio(&profile), 0.0);
    }

    #[test]
    fn xp_ratio_uses_profile_progress() {
        let mut profile = PlayerProfileSave::default();
        profile.xp = 250;
        profile.xp_next = 1_000;

        assert_eq!(xp_ratio(&profile), 0.25);
    }

    #[test]
    fn texture_id_helpers_keep_stable_fallbacks() {
        assert_eq!(attribute_icon_texture_id(0), "menu.attr_survival");
        assert_eq!(attribute_icon_texture_id(99), "menu.attr_analysis");
        assert_eq!(status_icon_texture_id(0), "menu.stat_health");
        assert_eq!(status_icon_texture_id(99), "menu.stat_carry");
    }

    #[test]
    fn status_accent_uses_carry_color_for_unknown_indices() {
        assert_eq!(vital_status_accent(99), Color::rgba(1.0, 0.70, 0.36, 1.0));
    }
}
