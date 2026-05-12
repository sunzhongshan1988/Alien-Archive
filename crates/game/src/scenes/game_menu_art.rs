use runtime::{Color, Rect, Renderer, Vec2};

use crate::ui::{
    menu_style::inset_rect,
    menu_widgets::{contain_rect, draw_border, draw_screen_rect, screen_rect},
};

use super::GameMenuTab;

pub(super) fn nav_icon_texture_id(tab: GameMenuTab) -> &'static str {
    match tab {
        GameMenuTab::Profile => "menu.nav_profile",
        GameMenuTab::Inventory => "menu.nav_inventory",
        GameMenuTab::Codex => "menu.nav_codex",
        GameMenuTab::Map => "menu.nav_map",
        GameMenuTab::Quests => "menu.nav_quests",
        GameMenuTab::Settings => "menu.nav_settings",
    }
}

pub(super) fn bottom_action_texture_id(index: usize) -> &'static str {
    match index {
        0 => "menu.action_equip",
        1 => "menu.action_skills",
        2 => "menu.action_logs",
        3 => "menu.action_craft",
        4 => "menu.action_comms",
        _ => "menu.action_save",
    }
}

pub(super) fn draw_nav_icon(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    tab: GameMenuTab,
    icon: Rect,
    active: bool,
    scale: f32,
) {
    let scale = scale * 1.65;
    let color = if active {
        Color::rgba(0.72, 1.0, 0.90, 1.0)
    } else {
        Color::rgba(0.28, 0.68, 0.78, 0.92)
    };

    let texture_id = nav_icon_texture_id(tab);
    if let Some(image_size) = renderer.texture_size(texture_id) {
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(icon, image_size)),
            if active {
                Color::rgba(1.0, 1.0, 1.0, 1.0)
            } else {
                Color::rgba(0.70, 0.92, 0.96, 0.72)
            },
        );
        return;
    }

    match tab {
        GameMenuTab::Profile => {
            draw_screen_rect(renderer, viewport, inset_rect(icon, 6.0 * scale), color);
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 4.0 * scale, icon.origin.y + 15.0 * scale),
                    Vec2::new(14.0 * scale, 5.0 * scale),
                ),
                color,
            );
        }
        GameMenuTab::Inventory => {
            for col in 0..2 {
                for row in 0..2 {
                    draw_screen_rect(
                        renderer,
                        viewport,
                        Rect::new(
                            Vec2::new(
                                icon.origin.x + col as f32 * 12.0 * scale,
                                icon.origin.y + row as f32 * 12.0 * scale,
                            ),
                            Vec2::new(8.0 * scale, 8.0 * scale),
                        ),
                        color,
                    );
                }
            }
        }
        GameMenuTab::Codex => {
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 7.0 * scale, icon.origin.y),
                    Vec2::new(8.0 * scale, 22.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 2.0 * scale, icon.origin.y + 4.0 * scale),
                    Vec2::new(18.0 * scale, 4.0 * scale),
                ),
                color,
            );
        }
        GameMenuTab::Map => {
            draw_border(renderer, viewport, icon, 2.0 * scale, color);
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 5.0 * scale, icon.origin.y + 10.0 * scale),
                    Vec2::new(12.0 * scale, 3.0 * scale),
                ),
                color,
            );
        }
        GameMenuTab::Quests => {
            for row in 0..3 {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(icon.origin.x, icon.origin.y + row as f32 * 8.0 * scale),
                        Vec2::new(22.0 * scale, 3.0 * scale),
                    ),
                    color,
                );
            }
        }
        GameMenuTab::Settings => {
            draw_border(
                renderer,
                viewport,
                inset_rect(icon, 3.0 * scale),
                3.0 * scale,
                color,
            );
            draw_screen_rect(renderer, viewport, inset_rect(icon, 9.0 * scale), color);
        }
    }
}

pub(super) fn draw_bottom_icon(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    index: usize,
    scale: f32,
) {
    let color = Color::rgba(0.40, 0.92, 1.0, 0.94);
    let icon = Rect::new(
        Vec2::new(rect.origin.x + 22.0 * scale, rect.origin.y + 13.0 * scale),
        Vec2::new(42.0 * scale, 42.0 * scale),
    );
    let texture_id = bottom_action_texture_id(index);
    if let Some(image_size) = renderer.texture_size(texture_id) {
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(icon, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    match index {
        0 => {
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(icon.origin, Vec2::new(28.0 * scale, 7.0 * scale)),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 20.0 * scale, icon.origin.y + 4.0 * scale),
                    Vec2::new(7.0 * scale, 24.0 * scale),
                ),
                color,
            );
        }
        1 => {
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 5.0 * scale, icon.origin.y),
                    Vec2::new(6.0 * scale, 30.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 20.0 * scale, icon.origin.y + 4.0 * scale),
                    Vec2::new(6.0 * scale, 28.0 * scale),
                ),
                color,
            );
        }
        2 => {
            draw_border(renderer, viewport, icon, 2.0 * scale, color);
            for row in 0..3 {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            icon.origin.x + 7.0 * scale,
                            icon.origin.y + (8.0 + row as f32 * 8.0) * scale,
                        ),
                        Vec2::new(20.0 * scale, 2.0 * scale),
                    ),
                    color,
                );
            }
        }
        3 => {
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 7.0 * scale, icon.origin.y + 5.0 * scale),
                    Vec2::new(20.0 * scale, 6.0 * scale),
                ),
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 14.0 * scale, icon.origin.y + 11.0 * scale),
                    Vec2::new(6.0 * scale, 20.0 * scale),
                ),
                color,
            );
        }
        4 => {
            draw_border(
                renderer,
                viewport,
                inset_rect(icon, 4.0 * scale),
                2.0 * scale,
                color,
            );
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 10.0 * scale, icon.origin.y + 14.0 * scale),
                    Vec2::new(14.0 * scale, 6.0 * scale),
                ),
                color,
            );
        }
        _ => {
            draw_border(renderer, viewport, icon, 2.0 * scale, color);
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(icon.origin.x + 9.0 * scale, icon.origin.y + 6.0 * scale),
                    Vec2::new(16.0 * scale, 22.0 * scale),
                ),
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_icon_texture_ids_cover_every_game_menu_tab() {
        for tab in GameMenuTab::ALL {
            assert!(nav_icon_texture_id(tab).starts_with("menu.nav_"));
        }
    }

    #[test]
    fn bottom_action_texture_ids_fall_back_to_save_icon() {
        assert_eq!(bottom_action_texture_id(0), "menu.action_equip");
        assert_eq!(bottom_action_texture_id(99), "menu.action_save");
    }
}
