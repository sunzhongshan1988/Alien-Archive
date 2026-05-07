use runtime::{Rect, Vec2};

pub mod metric {
    pub const PANEL_WIDTH: f32 = 1600.0;
    pub const PANEL_HEIGHT: f32 = 880.0;
    pub const TOP_HEIGHT: f32 = 88.0;
    pub const BOTTOM_HEIGHT: f32 = 96.0;
    pub const NAV_WIDTH: f32 = 218.0;
    pub const NAV_ITEM_HEIGHT: f32 = 86.0;
    pub const RETURN_BUTTON_WIDTH: f32 = 260.0;
    pub const RETURN_BUTTON_HEIGHT: f32 = 64.0;
    pub const LANGUAGE_CHOICE_WIDTH: f32 = 124.0;
    pub const LANGUAGE_CHOICE_HEIGHT: f32 = 50.0;
}

pub mod space {
    pub const VIEWPORT_MARGIN: f32 = 20.0;
    pub const OUTER: f32 = 54.0;
    pub const PANEL_GAP: f32 = 12.0;
    pub const CONTENT_PADDING: f32 = 22.0;
    pub const NAV_ITEM_GAP: f32 = 7.0;
    pub const NAV_ITEM_INSET_X: f32 = 9.0;
    pub const NAV_ITEM_TOP: f32 = 12.0;
    pub const CONTENT_HEADER: f32 = 110.0;
    pub const CONTENT_BOTTOM_TRIM: f32 = 132.0;
    pub const RETURN_BUTTON_RIGHT: f32 = 32.0;
    pub const RETURN_BUTTON_TOP: f32 = 16.0;
    pub const BOTTOM_ACTION_LEFT: f32 = 14.0;
    pub const BOTTOM_ACTION_RIGHT_CLEARANCE: f32 = 150.0;
    pub const BOTTOM_ACTION_TOP: f32 = 16.0;
    pub const BOTTOM_ACTION_GAP: f32 = 8.0;
    pub const LANGUAGE_CHOICE_RIGHT: f32 = 300.0;
    pub const LANGUAGE_CHOICE_GAP: f32 = 140.0;
    pub const LANGUAGE_CHOICE_TOP: f32 = 21.0;
    pub const INVENTORY_SLOT_SIZE: f32 = 66.0;
    pub const INVENTORY_SLOT_GAP: f32 = 10.0;
    pub const INVENTORY_SLOT_LEFT: f32 = 24.0;
    pub const INVENTORY_SLOT_TOP: f32 = 28.0;
}

pub mod grid {
    pub const INVENTORY_COLUMNS: usize = 6;
    pub const INVENTORY_ROWS: usize = 4;
    pub const INVENTORY_SLOTS: usize = INVENTORY_COLUMNS * INVENTORY_ROWS;
}

pub mod skin {
    pub const ROOT: &str = "menu.skin_root";
    pub const HEADER: &str = "menu.skin_header";
    pub const PANEL: &str = "menu.skin_panel";
    pub const CARD: &str = "menu.skin_card";
    pub const NAV_ACTIVE: &str = "menu.skin_nav_active";
    pub const NAV_IDLE: &str = "menu.skin_nav_idle";
    pub const BOTTOM_BUTTON: &str = "menu.skin_bottom_button";
    pub const RETURN_BUTTON: &str = "menu.skin_return_button";
    pub const SLOT_SELECTED: &str = "menu.skin_slot_selected";
    pub const SLOT_EMPTY: &str = "menu.skin_slot_empty";
    pub const LANGUAGE_TOGGLE: &str = "menu.skin_language_toggle";
}

pub mod icon {
    pub const BRAND_CRYSTAL: &str = "menu.brand_crystal";
    pub const RESOURCE_CRYSTAL: &str = "menu.resource_crystal";
    pub const RESOURCE_COIN: &str = "menu.resource_coin";
    pub const RETURN: &str = "menu.action_return";
}

#[derive(Clone, Copy)]
pub struct TextureAsset {
    pub texture_id: &'static str,
    pub path: &'static str,
}

pub const TEXTURES: &[TextureAsset] = &[
    TextureAsset {
        texture_id: skin::ROOT,
        path: "assets/images/ui/menu/skin_root_clean_large_ai.png",
    },
    TextureAsset {
        texture_id: skin::HEADER,
        path: "assets/images/ui/menu/skin_header_ai.png",
    },
    TextureAsset {
        texture_id: skin::PANEL,
        path: "assets/images/ui/menu/skin_panel_ai.png",
    },
    TextureAsset {
        texture_id: skin::CARD,
        path: "assets/images/ui/menu/skin_card_ai.png",
    },
    TextureAsset {
        texture_id: skin::NAV_ACTIVE,
        path: "assets/images/ui/menu/skin_nav_active_ai.png",
    },
    TextureAsset {
        texture_id: skin::NAV_IDLE,
        path: "assets/images/ui/menu/skin_nav_idle_ai.png",
    },
    TextureAsset {
        texture_id: skin::BOTTOM_BUTTON,
        path: "assets/images/ui/menu/skin_bottom_button_ai.png",
    },
    TextureAsset {
        texture_id: skin::RETURN_BUTTON,
        path: "assets/images/ui/menu/skin_return_button_ai.png",
    },
    TextureAsset {
        texture_id: skin::SLOT_SELECTED,
        path: "assets/images/ui/menu/skin_slot_selected_ai.png",
    },
    TextureAsset {
        texture_id: skin::SLOT_EMPTY,
        path: "assets/images/ui/menu/skin_slot_empty_ai.png",
    },
    TextureAsset {
        texture_id: skin::LANGUAGE_TOGGLE,
        path: "assets/images/ui/menu/skin_language_toggle_ai.png",
    },
    TextureAsset {
        texture_id: icon::BRAND_CRYSTAL,
        path: "assets/images/ui/menu/brand_crystal_ai.png",
    },
    TextureAsset {
        texture_id: icon::RESOURCE_CRYSTAL,
        path: "assets/images/ui/menu/resource_crystal_ai.png",
    },
    TextureAsset {
        texture_id: icon::RESOURCE_COIN,
        path: "assets/images/ui/menu/resource_coin_ai.png",
    },
    TextureAsset {
        texture_id: "menu.nav_profile",
        path: "assets/images/ui/menu/nav_profile_ai.png",
    },
    TextureAsset {
        texture_id: "menu.nav_inventory",
        path: "assets/images/ui/menu/nav_inventory_ai.png",
    },
    TextureAsset {
        texture_id: "menu.nav_codex",
        path: "assets/images/ui/menu/nav_codex_ai.png",
    },
    TextureAsset {
        texture_id: "menu.nav_map",
        path: "assets/images/ui/menu/nav_map_ai.png",
    },
    TextureAsset {
        texture_id: "menu.nav_quests",
        path: "assets/images/ui/menu/nav_quests_ai.png",
    },
    TextureAsset {
        texture_id: "menu.nav_settings",
        path: "assets/images/ui/menu/nav_settings_ai.png",
    },
    TextureAsset {
        texture_id: "menu.action_equip",
        path: "assets/images/ui/menu/action_equip_ai.png",
    },
    TextureAsset {
        texture_id: "menu.action_skills",
        path: "assets/images/ui/menu/action_skills_ai.png",
    },
    TextureAsset {
        texture_id: "menu.action_logs",
        path: "assets/images/ui/menu/action_logs_ai.png",
    },
    TextureAsset {
        texture_id: "menu.action_craft",
        path: "assets/images/ui/menu/action_craft_ai.png",
    },
    TextureAsset {
        texture_id: "menu.action_comms",
        path: "assets/images/ui/menu/action_comms_ai.png",
    },
    TextureAsset {
        texture_id: "menu.action_save",
        path: "assets/images/ui/menu/action_save_ai.png",
    },
    TextureAsset {
        texture_id: icon::RETURN,
        path: "assets/images/ui/menu/action_return_ai.png",
    },
    TextureAsset {
        texture_id: "menu.attr_survival",
        path: "assets/images/ui/menu/attr_survival_ai.png",
    },
    TextureAsset {
        texture_id: "menu.attr_mobility",
        path: "assets/images/ui/menu/attr_mobility_ai.png",
    },
    TextureAsset {
        texture_id: "menu.attr_scanning",
        path: "assets/images/ui/menu/attr_scanning_ai.png",
    },
    TextureAsset {
        texture_id: "menu.attr_gathering",
        path: "assets/images/ui/menu/attr_gathering_ai.png",
    },
    TextureAsset {
        texture_id: "menu.attr_analysis",
        path: "assets/images/ui/menu/attr_analysis_ai.png",
    },
    TextureAsset {
        texture_id: "menu.stat_health",
        path: "assets/images/ui/menu/stat_health_ai.png",
    },
    TextureAsset {
        texture_id: "menu.stat_stamina",
        path: "assets/images/ui/menu/stat_stamina_ai.png",
    },
    TextureAsset {
        texture_id: "menu.stat_armor",
        path: "assets/images/ui/menu/stat_armor_ai.png",
    },
    TextureAsset {
        texture_id: "menu.stat_carry",
        path: "assets/images/ui/menu/stat_carry_ai.png",
    },
    TextureAsset {
        texture_id: "menu.codex_alien_life",
        path: "assets/images/ui/menu/codex_alien_life_ai.png",
    },
    TextureAsset {
        texture_id: "menu.codex_relic_tech",
        path: "assets/images/ui/menu/codex_relic_tech_ai.png",
    },
    TextureAsset {
        texture_id: "menu.codex_star_geography",
        path: "assets/images/ui/menu/codex_star_geography_ai.png",
    },
    TextureAsset {
        texture_id: "menu.codex_civilization",
        path: "assets/images/ui/menu/codex_civilization_ai.png",
    },
];

pub mod color {
    use runtime::Color;

    pub const TRANSPARENT_IMAGE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);
    pub const SCREEN_OVERLAY: Color = Color::rgba(0.0, 0.0, 0.0, 0.70);

    pub const ROOT_FILL: Color = Color::rgba(0.013, 0.021, 0.030, 0.96);
    pub const ROOT_BORDER: Color = Color::rgba(0.24, 0.42, 0.54, 0.92);
    pub const ROOT_BRACKET: Color = Color::rgba(0.30, 0.88, 1.0, 0.95);

    pub const HEADER_FILL: Color = Color::rgba(0.012, 0.030, 0.040, 0.78);
    pub const HEADER_BORDER: Color = Color::rgba(0.12, 0.31, 0.39, 0.62);
    pub const HEADER_CELL_FILL: Color = Color::rgba(0.010, 0.026, 0.036, 0.78);
    pub const HEADER_CELL_BORDER: Color = Color::rgba(0.06, 0.16, 0.21, 0.86);
    pub const BRAND_FILL: Color = Color::rgba(0.016, 0.050, 0.062, 0.82);
    pub const BRAND_BORDER: Color = Color::rgba(0.12, 0.36, 0.45, 0.90);

    pub const PANEL_FILL: Color = Color::rgba(0.014, 0.030, 0.039, 0.86);
    pub const PANEL_BORDER: Color = Color::rgba(0.12, 0.25, 0.32, 0.82);
    pub const NAV_ACTIVE_FILL: Color = Color::rgba(0.045, 0.31, 0.40, 0.92);
    pub const NAV_IDLE_FILL: Color = Color::rgba(0.012, 0.038, 0.050, 0.78);
    pub const NAV_ACTIVE_BORDER: Color = Color::rgba(0.43, 0.91, 1.0, 0.92);
    pub const NAV_IDLE_BORDER: Color = Color::rgba(0.08, 0.20, 0.27, 0.82);
    pub const NAV_ACTIVE_BRACKET: Color = Color::rgba(0.45, 0.98, 1.0, 0.95);
    pub const NAV_IDLE_BRACKET: Color = Color::rgba(0.11, 0.32, 0.40, 0.72);
    pub const NAV_ACTIVE_ACCENT: Color = Color::rgba(0.70, 1.0, 0.90, 1.0);

    pub const BOTTOM_FILL: Color = Color::rgba(0.011, 0.028, 0.038, 0.82);
    pub const BOTTOM_BORDER: Color = Color::rgba(0.12, 0.31, 0.39, 0.70);
    pub const BOTTOM_BRACKET: Color = Color::rgba(0.25, 0.78, 0.92, 0.72);
    pub const BOTTOM_BUTTON_FILL: Color = Color::rgba(0.014, 0.052, 0.066, 0.70);
    pub const BOTTOM_BUTTON_BORDER: Color = Color::rgba(0.08, 0.24, 0.31, 0.78);
    pub const RETURN_FILL: Color = Color::rgba(0.090, 0.054, 0.018, 0.82);
    pub const RETURN_BORDER: Color = Color::rgba(0.92, 0.58, 0.20, 0.96);
    pub const RETURN_ACCENT: Color = Color::rgba(1.0, 0.72, 0.30, 0.96);

    pub const TEXT_PRIMARY: Color = Color::rgba(0.90, 1.0, 0.98, 1.0);
    pub const TEXT_SECONDARY: Color = Color::rgba(0.62, 0.82, 0.88, 0.96);
    pub const TEXT_MUTED: Color = Color::rgba(0.50, 0.68, 0.74, 0.94);
    pub const TEXT_DIM: Color = Color::rgba(0.48, 0.66, 0.72, 0.90);
    pub const TEXT_CYAN: Color = Color::rgba(0.42, 0.94, 1.0, 1.0);
    pub const TEXT_GREEN: Color = Color::rgba(0.68, 0.98, 0.36, 0.98);
    pub const TEXT_RETURN: Color = Color::rgba(1.0, 0.86, 0.54, 1.0);
    pub const TEXT_RETURN_SUB: Color = Color::rgba(0.78, 0.64, 0.42, 0.94);
}

#[derive(Clone, Copy)]
pub struct MenuLayout {
    pub scale: f32,
    pub root: Rect,
    pub top: Rect,
    pub nav: Rect,
    pub content: Rect,
    pub bottom: Rect,
}

impl MenuLayout {
    pub fn new(viewport: Vec2) -> Self {
        let scale = ((viewport.x - space::VIEWPORT_MARGIN) / metric::PANEL_WIDTH)
            .min((viewport.y - space::VIEWPORT_MARGIN) / metric::PANEL_HEIGHT)
            .min(1.0)
            .max(0.50);
        let root = Rect::new(
            Vec2::new(
                (viewport.x - metric::PANEL_WIDTH * scale) * 0.5,
                (viewport.y - metric::PANEL_HEIGHT * scale) * 0.5,
            ),
            Vec2::new(metric::PANEL_WIDTH * scale, metric::PANEL_HEIGHT * scale),
        );
        let top = Rect::new(
            Vec2::new(
                root.origin.x + space::OUTER * scale,
                root.origin.y + space::OUTER * scale,
            ),
            Vec2::new(
                root.size.x - space::OUTER * 2.0 * scale,
                metric::TOP_HEIGHT * scale,
            ),
        );
        let bottom = Rect::new(
            Vec2::new(
                root.origin.x + space::OUTER * scale,
                root.bottom() - (metric::BOTTOM_HEIGHT + space::OUTER) * scale,
            ),
            Vec2::new(
                root.size.x - space::OUTER * 2.0 * scale,
                metric::BOTTOM_HEIGHT * scale,
            ),
        );
        let content_y = top.bottom() + space::PANEL_GAP * scale;
        let content_height = bottom.origin.y - content_y - space::PANEL_GAP * scale;
        let nav = Rect::new(
            Vec2::new(top.origin.x, content_y),
            Vec2::new(metric::NAV_WIDTH * scale, content_height),
        );
        let content = Rect::new(
            Vec2::new(nav.right() + space::PANEL_GAP * scale, content_y),
            Vec2::new(
                top.right() - nav.right() - space::PANEL_GAP * scale,
                content_height,
            ),
        );

        Self {
            scale,
            root,
            top,
            nav,
            content,
            bottom,
        }
    }

    pub fn nav_item(&self, index: usize) -> Rect {
        Rect::new(
            Vec2::new(
                self.nav.origin.x + space::NAV_ITEM_INSET_X * self.scale,
                self.nav.origin.y
                    + space::NAV_ITEM_TOP * self.scale
                    + index as f32 * (metric::NAV_ITEM_HEIGHT + space::NAV_ITEM_GAP) * self.scale,
            ),
            Vec2::new(
                self.nav.size.x - space::NAV_ITEM_INSET_X * 2.0 * self.scale,
                metric::NAV_ITEM_HEIGHT * self.scale,
            ),
        )
    }

    pub fn content_body(&self) -> Rect {
        Rect::new(
            Vec2::new(
                self.content.origin.x + space::CONTENT_PADDING * self.scale,
                self.content.origin.y + space::CONTENT_HEADER * self.scale,
            ),
            Vec2::new(
                self.content.size.x - space::CONTENT_PADDING * 2.0 * self.scale,
                self.content.size.y - space::CONTENT_BOTTOM_TRIM * self.scale,
            ),
        )
    }

    pub fn dashboard_body(&self) -> Rect {
        inset_rect(self.content, space::PANEL_GAP * self.scale)
    }

    pub fn return_button(&self) -> Rect {
        Rect::new(
            Vec2::new(
                self.bottom.right()
                    - (metric::RETURN_BUTTON_WIDTH + space::RETURN_BUTTON_RIGHT) * self.scale,
                self.bottom.origin.y + space::RETURN_BUTTON_TOP * self.scale,
            ),
            Vec2::new(
                metric::RETURN_BUTTON_WIDTH * self.scale,
                metric::RETURN_BUTTON_HEIGHT * self.scale,
            ),
        )
    }

    pub fn bottom_action(&self, index: usize, action_count: usize) -> Rect {
        let return_button = self.return_button();
        let strip_x = self.bottom.origin.x + space::BOTTOM_ACTION_LEFT * self.scale;
        let strip_w =
            return_button.origin.x - strip_x - space::BOTTOM_ACTION_RIGHT_CLEARANCE * self.scale;
        let gap = space::BOTTOM_ACTION_GAP * self.scale;
        let count = action_count.max(1);
        let width = (strip_w - gap * count.saturating_sub(1) as f32) / count as f32;

        Rect::new(
            Vec2::new(
                strip_x + index as f32 * (width + gap),
                self.bottom.origin.y + space::BOTTOM_ACTION_TOP * self.scale,
            ),
            Vec2::new(width.max(0.0), metric::RETURN_BUTTON_HEIGHT * self.scale),
        )
    }

    pub fn language_choice(&self, index: usize) -> Rect {
        let body = self.content_body();
        Rect::new(
            Vec2::new(
                body.right()
                    - (space::LANGUAGE_CHOICE_RIGHT - index as f32 * space::LANGUAGE_CHOICE_GAP)
                        * self.scale,
                body.origin.y + space::LANGUAGE_CHOICE_TOP * self.scale,
            ),
            Vec2::new(
                metric::LANGUAGE_CHOICE_WIDTH * self.scale,
                metric::LANGUAGE_CHOICE_HEIGHT * self.scale,
            ),
        )
    }
}

pub fn inventory_slot_rect(panel: Rect, index: usize, scale: f32) -> Rect {
    let slot_size = space::INVENTORY_SLOT_SIZE * scale;
    let gap = space::INVENTORY_SLOT_GAP * scale;
    let col = index % grid::INVENTORY_COLUMNS;
    let row = index / grid::INVENTORY_COLUMNS;

    Rect::new(
        Vec2::new(
            panel.origin.x + space::INVENTORY_SLOT_LEFT * scale + col as f32 * (slot_size + gap),
            panel.origin.y + space::INVENTORY_SLOT_TOP * scale + row as f32 * (slot_size + gap),
        ),
        Vec2::new(slot_size, slot_size),
    )
}

pub fn move_inventory_slot(selected: usize, dx: isize, dy: isize) -> usize {
    let col = selected % grid::INVENTORY_COLUMNS;
    let row = selected / grid::INVENTORY_COLUMNS;
    let next_col = (col as isize + dx).clamp(0, grid::INVENTORY_COLUMNS as isize - 1) as usize;
    let next_row = (row as isize + dy).clamp(0, grid::INVENTORY_ROWS as isize - 1) as usize;

    next_row * grid::INVENTORY_COLUMNS + next_col
}

pub fn inset_rect(rect: Rect, inset: f32) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x + inset, rect.origin.y + inset),
        Vec2::new(
            (rect.size.x - inset * 2.0).max(0.0),
            (rect.size.y - inset * 2.0).max(0.0),
        ),
    )
}
