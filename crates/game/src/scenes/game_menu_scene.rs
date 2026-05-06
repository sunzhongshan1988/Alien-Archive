use std::path::Path;

use anyhow::Result;
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::Font;

use crate::ui::text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text};

use super::{GameContext, GameMenuTab, Language, RenderContext, Scene, SceneId};
use super::{inventory_scene, profile_scene};

const EXPLORER_PORTRAIT_TEXTURE_ID: &str = "game_menu.explorer_portrait";
const EXPLORER_PORTRAIT_PATH: &str = "assets/images/ui/profile/explorer_portrait.png";
const PANEL_WIDTH: f32 = 1140.0;
const PANEL_HEIGHT: f32 = 640.0;
const TOP_HEIGHT: f32 = 74.0;
const NAV_WIDTH: f32 = 214.0;
const PANEL_GAP: f32 = 16.0;
const OUTER_PADDING: f32 = 18.0;
const NAV_ITEM_HEIGHT: f32 = 64.0;
const NAV_ITEM_GAP: f32 = 8.0;
const CONTENT_PADDING: f32 = 22.0;
const INVENTORY_MENU_COLUMNS: usize = 6;
const INVENTORY_MENU_ROWS: usize = 4;
const INVENTORY_MENU_SLOTS: usize = INVENTORY_MENU_COLUMNS * INVENTORY_MENU_ROWS;

#[derive(Clone, Copy)]
struct LocalizedText {
    english: &'static str,
    chinese: &'static str,
}

impl LocalizedText {
    const fn new(english: &'static str, chinese: &'static str) -> Self {
        Self { english, chinese }
    }

    fn get(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.chinese,
            Language::English => self.english,
        }
    }
}

#[derive(Clone, Copy)]
struct CodexPreview {
    label: LocalizedText,
    progress: u32,
}

#[derive(Clone, Copy)]
struct QuestPreview {
    label: LocalizedText,
    status: LocalizedText,
    progress: u32,
}

const CODEX_PREVIEWS: &[CodexPreview] = &[
    CodexPreview {
        label: LocalizedText::new("Biology", "异星生物"),
        progress: 42,
    },
    CodexPreview {
        label: LocalizedText::new("Minerals", "矿物图谱"),
        progress: 55,
    },
    CodexPreview {
        label: LocalizedText::new("Ruins", "遗迹科技"),
        progress: 31,
    },
    CodexPreview {
        label: LocalizedText::new("Field Notes", "外勤笔记"),
        progress: 68,
    },
];

const QUEST_PREVIEWS: &[QuestPreview] = &[
    QuestPreview {
        label: LocalizedText::new("Secure Landing Site", "稳固着陆点"),
        status: LocalizedText::new("Active", "进行中"),
        progress: 75,
    },
    QuestPreview {
        label: LocalizedText::new("Survey Crystal Field", "调查晶体田"),
        status: LocalizedText::new("Tracked", "已追踪"),
        progress: 40,
    },
    QuestPreview {
        label: LocalizedText::new("Decode Ruin Signal", "解析遗迹信号"),
        status: LocalizedText::new("Pending", "待处理"),
        progress: 18,
    },
];

#[derive(Default)]
struct GameMenuText {
    language: Option<Language>,
    title: Option<TextSprite>,
    status: Option<TextSprite>,
    close_hint: Option<TextSprite>,
    nav_labels: Vec<TextSprite>,
    page_titles: Vec<TextSprite>,
    page_subtitles: Vec<TextSprite>,
    profile_name: Option<TextSprite>,
    profile_role: Option<TextSprite>,
    profile_id: Option<TextSprite>,
    profile_section_stats: Option<TextSprite>,
    profile_section_core: Option<TextSprite>,
    profile_section_research: Option<TextSprite>,
    profile_stats: Vec<(TextSprite, TextSprite)>,
    profile_core_stats: Vec<(TextSprite, TextSprite)>,
    profile_research_stats: Vec<(TextSprite, TextSprite)>,
    inventory_slot_details: Vec<Option<InventoryDetailText>>,
    inventory_hint: Option<TextSprite>,
    inventory_empty_title: Option<TextSprite>,
    inventory_empty_body: Option<TextSprite>,
    codex_cards: Vec<(TextSprite, TextSprite)>,
    map_labels: Vec<TextSprite>,
    quest_rows: Vec<(TextSprite, TextSprite)>,
    settings_language: Option<TextSprite>,
    settings_hint: Option<TextSprite>,
    language_values: Vec<TextSprite>,
    placeholder: Option<TextSprite>,
}

struct InventoryDetailText {
    name: TextSprite,
    category: TextSprite,
    quantity: TextSprite,
    rarity: TextSprite,
    stack_limit: TextSprite,
    research: TextSprite,
    lock_state: Option<TextSprite>,
}

pub struct GameMenuScene {
    language: Language,
    active_tab: GameMenuTab,
    selected_inventory_slot: usize,
    text: GameMenuText,
}

impl GameMenuScene {
    pub fn new(language: Language, active_tab: GameMenuTab) -> Self {
        Self {
            language,
            active_tab,
            selected_inventory_slot: 0,
            text: GameMenuText::default(),
        }
    }

    fn set_tab(&mut self, ctx: &mut GameContext, tab: GameMenuTab) {
        self.active_tab = tab;
        ctx.game_menu_tab = tab;
    }

    fn set_language(&mut self, ctx: &mut GameContext, language: Language) {
        if ctx.language == language {
            return;
        }

        ctx.language = language;
        self.language = language;
        self.text = GameMenuText::default();
    }

    fn draw_menu(&self, ctx: &mut RenderContext<'_>) {
        let viewport = ctx.renderer.screen_size();
        let layout = MenuLayout::new(viewport);

        draw_screen_rect(
            ctx.renderer,
            viewport,
            Rect::new(Vec2::ZERO, viewport),
            Color::rgba(0.0, 0.0, 0.0, 0.70),
        );
        self.draw_shell(ctx.renderer, viewport, &layout);
        self.draw_nav(ctx.renderer, viewport, &layout);
        self.draw_content(ctx.renderer, viewport, &layout);
    }

    fn draw_shell(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        draw_screen_rect(
            renderer,
            viewport,
            layout.root,
            Color::rgba(0.013, 0.021, 0.030, 0.96),
        );
        draw_border(
            renderer,
            viewport,
            layout.root,
            1.0 * layout.scale,
            Color::rgba(0.24, 0.42, 0.54, 0.92),
        );
        draw_corner_brackets(
            renderer,
            viewport,
            layout.root,
            22.0 * layout.scale,
            2.0 * layout.scale,
            Color::rgba(0.30, 0.88, 1.0, 0.95),
        );
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(
                    layout.root.origin.x + 20.0 * layout.scale,
                    layout.top.bottom(),
                ),
                Vec2::new(layout.root.size.x - 40.0 * layout.scale, 1.0 * layout.scale),
            ),
            Color::rgba(0.22, 0.74, 0.86, 0.74),
        );

        if let Some(title) = &self.text.title {
            draw_text_strong(
                renderer,
                title,
                viewport,
                layout.top.origin.x + 26.0 * layout.scale,
                layout.top.origin.y + 16.0 * layout.scale,
                Color::rgba(0.90, 1.0, 0.98, 1.0),
                layout.scale,
            );
        }
        if let Some(status) = &self.text.status {
            draw_text(
                renderer,
                status,
                viewport,
                layout.top.origin.x + 270.0 * layout.scale,
                layout.top.origin.y + 26.0 * layout.scale,
                Color::rgba(0.57, 0.80, 0.87, 0.94),
            );
        }
        if let Some(close_hint) = &self.text.close_hint {
            draw_text(
                renderer,
                close_hint,
                viewport,
                layout.top.right() - close_hint.size.x - 26.0 * layout.scale,
                layout.top.origin.y + 26.0 * layout.scale,
                Color::rgba(0.66, 0.82, 0.86, 0.94),
            );
        }
    }

    fn draw_nav(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        draw_screen_rect(
            renderer,
            viewport,
            layout.nav,
            Color::rgba(0.018, 0.034, 0.046, 0.92),
        );
        draw_border(
            renderer,
            viewport,
            layout.nav,
            1.0 * layout.scale,
            Color::rgba(0.13, 0.25, 0.32, 0.86),
        );

        for (index, tab) in GameMenuTab::ALL.iter().copied().enumerate() {
            let rect = layout.nav_item(index);
            let active = tab == self.active_tab;
            draw_screen_rect(
                renderer,
                viewport,
                rect,
                if active {
                    Color::rgba(0.06, 0.39, 0.52, 0.88)
                } else {
                    Color::rgba(0.018, 0.042, 0.055, 0.70)
                },
            );
            draw_border(
                renderer,
                viewport,
                rect,
                1.0 * layout.scale,
                if active {
                    Color::rgba(0.43, 0.91, 1.0, 0.92)
                } else {
                    Color::rgba(0.09, 0.18, 0.24, 0.82)
                },
            );
            if active {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(rect.origin.x, rect.origin.y),
                        Vec2::new(4.0 * layout.scale, rect.size.y),
                    ),
                    Color::rgba(0.70, 1.0, 0.90, 1.0),
                );
            }
            draw_nav_icon(renderer, viewport, tab, rect, active, layout.scale);
            if let Some(label) = self.text.nav_labels.get(index) {
                draw_text_strong(
                    renderer,
                    label,
                    viewport,
                    rect.origin.x + 58.0 * layout.scale,
                    rect.origin.y + 17.0 * layout.scale,
                    if active {
                        Color::rgba(0.94, 1.0, 0.98, 1.0)
                    } else {
                        Color::rgba(0.60, 0.78, 0.84, 0.96)
                    },
                    layout.scale,
                );
            }
        }
    }

    fn draw_content(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        draw_screen_rect(
            renderer,
            viewport,
            layout.content,
            Color::rgba(0.017, 0.029, 0.038, 0.92),
        );
        draw_border(
            renderer,
            viewport,
            layout.content,
            1.0 * layout.scale,
            Color::rgba(0.14, 0.26, 0.34, 0.86),
        );

        self.draw_page_header(renderer, viewport, layout);
        match self.active_tab {
            GameMenuTab::Profile => self.draw_profile_page(renderer, viewport, layout),
            GameMenuTab::Inventory => self.draw_inventory_page(renderer, viewport, layout),
            GameMenuTab::Codex => self.draw_codex_page(renderer, viewport, layout),
            GameMenuTab::Map => self.draw_map_page(renderer, viewport, layout),
            GameMenuTab::Quests => self.draw_quests_page(renderer, viewport, layout),
            GameMenuTab::Settings => self.draw_settings_page(renderer, viewport, layout),
        }
    }

    fn draw_page_header(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let index = tab_index(self.active_tab);
        if let Some(title) = self.text.page_titles.get(index) {
            draw_text_strong(
                renderer,
                title,
                viewport,
                layout.content.origin.x + CONTENT_PADDING * layout.scale,
                layout.content.origin.y + 18.0 * layout.scale,
                Color::rgba(0.88, 1.0, 0.98, 1.0),
                layout.scale,
            );
        }
        if let Some(subtitle) = self.text.page_subtitles.get(index) {
            draw_text(
                renderer,
                subtitle,
                viewport,
                layout.content.origin.x + CONTENT_PADDING * layout.scale,
                layout.content.origin.y + 54.0 * layout.scale,
                Color::rgba(0.55, 0.77, 0.84, 0.96),
            );
        }
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(
                    layout.content.origin.x + CONTENT_PADDING * layout.scale,
                    layout.content.origin.y + 88.0 * layout.scale,
                ),
                Vec2::new(
                    layout.content.size.x - CONTENT_PADDING * 2.0 * layout.scale,
                    2.0,
                ),
            ),
            Color::rgba(0.29, 0.86, 1.0, 0.72),
        );
    }

    fn draw_profile_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let profile = profile_scene::profile_overview(self.language);
        let content = layout.content_body();
        let portrait_panel = Rect::new(
            content.origin,
            Vec2::new(300.0 * layout.scale, content.size.y),
        );
        let stats_panel = Rect::new(
            Vec2::new(
                portrait_panel.right() + 16.0 * layout.scale,
                content.origin.y,
            ),
            Vec2::new(292.0 * layout.scale, content.size.y),
        );
        let modules_panel = Rect::new(
            Vec2::new(stats_panel.right() + 16.0 * layout.scale, content.origin.y),
            Vec2::new(
                content.right() - stats_panel.right() - 16.0 * layout.scale,
                content.size.y,
            ),
        );

        draw_inner_panel(renderer, viewport, portrait_panel, layout.scale);
        draw_inner_panel(renderer, viewport, stats_panel, layout.scale);
        draw_inner_panel(renderer, viewport, modules_panel, layout.scale);

        let portrait_frame = Rect::new(
            Vec2::new(
                portrait_panel.origin.x + 50.0 * layout.scale,
                portrait_panel.origin.y + 18.0 * layout.scale,
            ),
            Vec2::new(200.0 * layout.scale, 292.0 * layout.scale),
        );
        draw_screen_rect(
            renderer,
            viewport,
            portrait_frame,
            Color::rgba(0.025, 0.065, 0.075, 0.78),
        );
        draw_border(
            renderer,
            viewport,
            portrait_frame,
            1.0 * layout.scale,
            Color::rgba(0.42, 0.88, 1.0, 0.76),
        );
        if let Some(image_size) = renderer.texture_size(EXPLORER_PORTRAIT_TEXTURE_ID) {
            renderer.draw_image(
                EXPLORER_PORTRAIT_TEXTURE_ID,
                screen_rect(
                    viewport,
                    contain_rect(inset_rect(portrait_frame, 8.0 * layout.scale), image_size),
                ),
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );
        }
        if let Some(name) = &self.text.profile_name {
            draw_text_strong(
                renderer,
                name,
                viewport,
                portrait_panel.origin.x + 24.0 * layout.scale,
                portrait_panel.origin.y + 334.0 * layout.scale,
                Color::rgba(0.92, 1.0, 0.98, 1.0),
                layout.scale,
            );
        }
        if let Some(role) = &self.text.profile_role {
            draw_text(
                renderer,
                role,
                viewport,
                portrait_panel.origin.x + 24.0 * layout.scale,
                portrait_panel.origin.y + 372.0 * layout.scale,
                Color::rgba(0.60, 0.80, 0.86, 0.96),
            );
        }
        if let Some(id) = &self.text.profile_id {
            draw_text(
                renderer,
                id,
                viewport,
                portrait_panel.origin.x + 24.0 * layout.scale,
                portrait_panel.origin.y + 402.0 * layout.scale,
                Color::rgba(0.78, 0.68, 0.48, 0.96),
            );
        }

        if let Some(header) = &self.text.profile_section_stats {
            draw_text_strong(
                renderer,
                header,
                viewport,
                stats_panel.origin.x + 22.0 * layout.scale,
                stats_panel.origin.y + 24.0 * layout.scale,
                Color::rgba(0.78, 0.96, 1.0, 1.0),
                layout.scale,
            );
        }
        for (index, (label, value)) in self.text.profile_stats.iter().enumerate() {
            let stat = profile.vital_stats[index];
            let row_y = stats_panel.origin.y + (76.0 + index as f32 * 72.0) * layout.scale;
            draw_text(
                renderer,
                label,
                viewport,
                stats_panel.origin.x + 24.0 * layout.scale,
                row_y,
                Color::rgba(0.68, 0.88, 0.92, 0.96),
            );
            draw_text(
                renderer,
                value,
                viewport,
                stats_panel.right() - value.size.x - 24.0 * layout.scale,
                row_y,
                Color::rgba(0.90, 1.0, 0.96, 1.0),
            );
            draw_bar(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(
                        stats_panel.origin.x + 24.0 * layout.scale,
                        row_y + 32.0 * layout.scale,
                    ),
                    Vec2::new(
                        stats_panel.size.x - 48.0 * layout.scale,
                        12.0 * layout.scale,
                    ),
                ),
                stat.value as f32 / stat.max as f32,
                layout.scale,
            );
        }

        if let Some(header) = &self.text.profile_section_core {
            draw_text_strong(
                renderer,
                header,
                viewport,
                modules_panel.origin.x + 22.0 * layout.scale,
                modules_panel.origin.y + 24.0 * layout.scale,
                Color::rgba(0.78, 0.96, 1.0, 1.0),
                layout.scale,
            );
        }
        for (index, (label, value)) in self.text.profile_core_stats.iter().enumerate() {
            let stat = profile.core_stats[index];
            let row_y = modules_panel.origin.y + (68.0 + index as f32 * 38.0) * layout.scale;
            draw_text(
                renderer,
                label,
                viewport,
                modules_panel.origin.x + 24.0 * layout.scale,
                row_y,
                Color::rgba(0.68, 0.88, 0.92, 0.96),
            );
            draw_text(
                renderer,
                value,
                viewport,
                modules_panel.right() - value.size.x - 24.0 * layout.scale,
                row_y,
                Color::rgba(0.90, 1.0, 0.96, 1.0),
            );
            draw_score_pips(
                renderer,
                viewport,
                Vec2::new(
                    modules_panel.origin.x + 24.0 * layout.scale,
                    row_y + 25.0 * layout.scale,
                ),
                stat.value,
                layout.scale,
            );
        }

        if let Some(header) = &self.text.profile_section_research {
            draw_text_strong(
                renderer,
                header,
                viewport,
                modules_panel.origin.x + 22.0 * layout.scale,
                modules_panel.origin.y + 252.0 * layout.scale,
                Color::rgba(0.78, 0.96, 1.0, 1.0),
                layout.scale,
            );
        }
        for (index, (label, value)) in self.text.profile_research_stats.iter().enumerate() {
            let stat = profile.research_stats[index];
            let row_y = modules_panel.origin.y + (286.0 + index as f32 * 30.0) * layout.scale;
            draw_compact_stat_bar(
                renderer,
                viewport,
                modules_panel,
                label,
                value,
                stat.value as f32 / stat.max as f32,
                row_y,
                layout.scale,
            );
        }
    }

    fn draw_inventory_page(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &MenuLayout,
    ) {
        let content = layout.content_body();
        let grid_panel = Rect::new(
            content.origin,
            Vec2::new(540.0 * layout.scale, content.size.y),
        );
        let detail_panel = Rect::new(
            Vec2::new(grid_panel.right() + 16.0 * layout.scale, content.origin.y),
            Vec2::new(
                content.right() - grid_panel.right() - 16.0 * layout.scale,
                content.size.y,
            ),
        );
        draw_inner_panel(renderer, viewport, grid_panel, layout.scale);
        draw_inner_panel(renderer, viewport, detail_panel, layout.scale);

        let slots = inventory_scene::mvp_inventory_slots();
        for index in 0..INVENTORY_MENU_SLOTS {
            let slot = inventory_slot_rect(grid_panel, index, layout.scale);
            let item = slots.get(index).and_then(|slot| *slot);
            let selected = index == self.selected_inventory_slot;
            draw_screen_rect(
                renderer,
                viewport,
                slot,
                if selected {
                    Color::rgba(0.055, 0.19, 0.24, 0.96)
                } else if item.is_some() {
                    Color::rgba(0.025, 0.070, 0.086, 0.88)
                } else {
                    Color::rgba(0.012, 0.025, 0.032, 0.82)
                },
            );
            draw_border(
                renderer,
                viewport,
                slot,
                1.0 * layout.scale,
                if selected {
                    Color::rgba(0.58, 0.96, 1.0, 0.96)
                } else {
                    Color::rgba(0.12, 0.27, 0.35, 0.86)
                },
            );
            if let Some(item) = item {
                draw_screen_rect(
                    renderer,
                    viewport,
                    inset_rect(slot, 8.0 * layout.scale),
                    Color::rgba(0.0, 0.0, 0.0, 0.18),
                );
                draw_border(
                    renderer,
                    viewport,
                    inset_rect(slot, 7.0 * layout.scale),
                    1.0 * layout.scale,
                    Color::rgba(
                        item.rarity_color.r,
                        item.rarity_color.g,
                        item.rarity_color.b,
                        0.82,
                    ),
                );
                renderer.draw_image(
                    item.texture_id,
                    screen_rect(viewport, inset_rect(slot, 11.0 * layout.scale)),
                    Color::rgba(1.0, 1.0, 1.0, 1.0),
                );
                if item.locked {
                    draw_screen_rect(
                        renderer,
                        viewport,
                        Rect::new(
                            Vec2::new(
                                slot.right() - 18.0 * layout.scale,
                                slot.origin.y + 6.0 * layout.scale,
                            ),
                            Vec2::new(10.0 * layout.scale, 14.0 * layout.scale),
                        ),
                        Color::rgba(0.90, 0.68, 0.34, 0.92),
                    );
                }
            }
        }

        self.draw_inventory_detail(renderer, viewport, layout, detail_panel, &slots);
    }

    fn draw_inventory_detail(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &MenuLayout,
        detail_panel: Rect,
        slots: &[Option<inventory_scene::InventoryItemView>],
    ) {
        let selected_item = slots
            .get(self.selected_inventory_slot)
            .and_then(|slot| *slot);
        let preview = Rect::new(
            Vec2::new(
                detail_panel.origin.x + 24.0 * layout.scale,
                detail_panel.origin.y + 24.0 * layout.scale,
            ),
            Vec2::new(112.0 * layout.scale, 112.0 * layout.scale),
        );
        draw_screen_rect(
            renderer,
            viewport,
            preview,
            Color::rgba(0.020, 0.055, 0.068, 0.90),
        );
        draw_border(
            renderer,
            viewport,
            preview,
            1.0 * layout.scale,
            selected_item.map_or(Color::rgba(0.12, 0.27, 0.35, 0.86), |item| {
                Color::rgba(
                    item.rarity_color.r,
                    item.rarity_color.g,
                    item.rarity_color.b,
                    0.94,
                )
            }),
        );

        if let Some(item) = selected_item {
            renderer.draw_image(
                item.texture_id,
                screen_rect(viewport, inset_rect(preview, 16.0 * layout.scale)),
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );

            if let Some(Some(detail)) = self
                .text
                .inventory_slot_details
                .get(self.selected_inventory_slot)
            {
                draw_text_strong(
                    renderer,
                    &detail.name,
                    viewport,
                    preview.right() + 18.0 * layout.scale,
                    detail_panel.origin.y + 26.0 * layout.scale,
                    Color::rgba(0.92, 1.0, 0.98, 1.0),
                    layout.scale,
                );
                draw_text(
                    renderer,
                    &detail.rarity,
                    viewport,
                    preview.right() + 18.0 * layout.scale,
                    detail_panel.origin.y + 62.0 * layout.scale,
                    Color::rgba(
                        item.rarity_color.r,
                        item.rarity_color.g,
                        item.rarity_color.b,
                        1.0,
                    ),
                );
                if let Some(lock_state) = &detail.lock_state {
                    draw_text(
                        renderer,
                        lock_state,
                        viewport,
                        preview.right() + 18.0 * layout.scale,
                        detail_panel.origin.y + 94.0 * layout.scale,
                        Color::rgba(0.92, 0.72, 0.38, 0.96),
                    );
                }

                let rows = [
                    &detail.category,
                    &detail.quantity,
                    &detail.stack_limit,
                    &detail.research,
                ];
                for (index, row_text) in rows.iter().enumerate() {
                    let y = detail_panel.origin.y + (166.0 + index as f32 * 42.0) * layout.scale;
                    draw_screen_rect(
                        renderer,
                        viewport,
                        Rect::new(
                            Vec2::new(
                                detail_panel.origin.x + 24.0 * layout.scale,
                                y - 8.0 * layout.scale,
                            ),
                            Vec2::new(
                                detail_panel.size.x - 48.0 * layout.scale,
                                32.0 * layout.scale,
                            ),
                        ),
                        Color::rgba(0.022, 0.052, 0.066, 0.72),
                    );
                    draw_text(
                        renderer,
                        row_text,
                        viewport,
                        detail_panel.origin.x + 38.0 * layout.scale,
                        y - 4.0 * layout.scale,
                        Color::rgba(0.70, 0.90, 0.94, 0.96),
                    );
                }
            }
        } else {
            if let Some(title) = &self.text.inventory_empty_title {
                draw_text_strong(
                    renderer,
                    title,
                    viewport,
                    preview.right() + 18.0 * layout.scale,
                    detail_panel.origin.y + 36.0 * layout.scale,
                    Color::rgba(0.70, 0.88, 0.92, 0.96),
                    layout.scale,
                );
            }
            if let Some(body) = &self.text.inventory_empty_body {
                draw_text(
                    renderer,
                    body,
                    viewport,
                    detail_panel.origin.x + 24.0 * layout.scale,
                    detail_panel.origin.y + 166.0 * layout.scale,
                    Color::rgba(0.52, 0.72, 0.78, 0.94),
                );
            }
        }

        if let Some(hint) = &self.text.inventory_hint {
            draw_text(
                renderer,
                hint,
                viewport,
                detail_panel.origin.x + 24.0 * layout.scale,
                detail_panel.bottom() - 42.0 * layout.scale,
                Color::rgba(0.56, 0.76, 0.82, 0.94),
            );
        }
    }

    fn inventory_grid_panel(&self, layout: &MenuLayout) -> Rect {
        let content = layout.content_body();
        Rect::new(
            content.origin,
            Vec2::new(540.0 * layout.scale, content.size.y),
        )
    }

    fn handle_inventory_click(&mut self, point: Vec2, layout: &MenuLayout) {
        let grid_panel = self.inventory_grid_panel(layout);
        for index in 0..INVENTORY_MENU_SLOTS {
            if screen_point_in_rect(point, inventory_slot_rect(grid_panel, index, layout.scale)) {
                self.selected_inventory_slot = index;
                break;
            }
        }
    }

    fn move_inventory_selection(&mut self, dx: isize, dy: isize) {
        self.selected_inventory_slot = move_inventory_slot(self.selected_inventory_slot, dx, dy);
    }

    fn draw_codex_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let content = layout.content_body();
        for (index, (label, value)) in self.text.codex_cards.iter().enumerate() {
            let col = index % 2;
            let row = index / 2;
            let card = Rect::new(
                Vec2::new(
                    content.origin.x + col as f32 * (content.size.x * 0.5 + 8.0 * layout.scale),
                    content.origin.y + row as f32 * (166.0 * layout.scale),
                ),
                Vec2::new(
                    content.size.x * 0.5 - 8.0 * layout.scale,
                    144.0 * layout.scale,
                ),
            );
            draw_inner_panel(renderer, viewport, card, layout.scale);
            draw_codex_glyph(renderer, viewport, card, index, layout.scale);
            draw_text_strong(
                renderer,
                label,
                viewport,
                card.origin.x + 96.0 * layout.scale,
                card.origin.y + 28.0 * layout.scale,
                Color::rgba(0.90, 1.0, 0.98, 1.0),
                layout.scale,
            );
            draw_text(
                renderer,
                value,
                viewport,
                card.origin.x + 96.0 * layout.scale,
                card.origin.y + 62.0 * layout.scale,
                Color::rgba(0.62, 0.82, 0.88, 0.96),
            );
            draw_bar(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(
                        card.origin.x + 96.0 * layout.scale,
                        card.origin.y + 100.0 * layout.scale,
                    ),
                    Vec2::new(card.size.x - 128.0 * layout.scale, 10.0 * layout.scale),
                ),
                CODEX_PREVIEWS[index].progress as f32 / 100.0,
                layout.scale,
            );
        }
    }

    fn draw_map_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
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

        let points = [
            Vec2::new(0.18, 0.64),
            Vec2::new(0.32, 0.52),
            Vec2::new(0.48, 0.44),
            Vec2::new(0.68, 0.34),
            Vec2::new(0.78, 0.56),
        ];
        for window in points.windows(2) {
            let start = map_point(map_rect, window[0]);
            let end = map_point(map_rect, window[1]);
            draw_segment(renderer, viewport, start, end, 4.0 * layout.scale);
        }
        for (index, point) in points.iter().copied().enumerate() {
            let center = map_point(map_rect, point);
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(center.x - 9.0 * layout.scale, center.y - 9.0 * layout.scale),
                    Vec2::new(18.0 * layout.scale, 18.0 * layout.scale),
                ),
                if index == 0 {
                    Color::rgba(0.82, 0.68, 0.32, 1.0)
                } else {
                    Color::rgba(0.30, 0.88, 1.0, 0.92)
                },
            );
        }

        for (index, label) in self.text.map_labels.iter().enumerate() {
            draw_text(
                renderer,
                label,
                viewport,
                map_rect.origin.x + 24.0 * layout.scale,
                map_rect.origin.y + (24.0 + index as f32 * 34.0) * layout.scale,
                Color::rgba(0.70, 0.90, 0.94, 0.96),
            );
        }
    }

    fn draw_quests_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let content = layout.content_body();
        for (index, (label, status)) in self.text.quest_rows.iter().enumerate() {
            let row = Rect::new(
                Vec2::new(
                    content.origin.x,
                    content.origin.y + index as f32 * 118.0 * layout.scale,
                ),
                Vec2::new(content.size.x, 96.0 * layout.scale),
            );
            draw_inner_panel(renderer, viewport, row, layout.scale);
            draw_text_strong(
                renderer,
                label,
                viewport,
                row.origin.x + 24.0 * layout.scale,
                row.origin.y + 18.0 * layout.scale,
                Color::rgba(0.90, 1.0, 0.98, 1.0),
                layout.scale,
            );
            let status_rect = Rect::new(
                Vec2::new(
                    row.right() - 150.0 * layout.scale,
                    row.origin.y + 16.0 * layout.scale,
                ),
                Vec2::new(120.0 * layout.scale, 34.0 * layout.scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                status_rect,
                Color::rgba(0.08, 0.32, 0.42, 0.86),
            );
            draw_text_centered(
                renderer,
                status,
                viewport,
                status_rect.origin.x + status_rect.size.x * 0.5,
                status_rect.origin.y + 4.0 * layout.scale,
                Color::rgba(0.82, 1.0, 0.94, 1.0),
            );
            draw_bar(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(
                        row.origin.x + 24.0 * layout.scale,
                        row.origin.y + 64.0 * layout.scale,
                    ),
                    Vec2::new(row.size.x - 48.0 * layout.scale, 10.0 * layout.scale),
                ),
                QUEST_PREVIEWS[index].progress as f32 / 100.0,
                layout.scale,
            );
        }
    }

    fn draw_settings_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let content = layout.content_body();
        let language_row = Rect::new(
            content.origin,
            Vec2::new(content.size.x, 92.0 * layout.scale),
        );
        draw_inner_panel(renderer, viewport, language_row, layout.scale);
        if let Some(label) = &self.text.settings_language {
            draw_text_strong(
                renderer,
                label,
                viewport,
                language_row.origin.x + 24.0 * layout.scale,
                language_row.origin.y + 24.0 * layout.scale,
                Color::rgba(0.90, 1.0, 0.98, 1.0),
                layout.scale,
            );
        }
        for (index, language) in Language::SUPPORTED.iter().copied().enumerate() {
            let choice = layout.language_choice(index);
            let active = language == self.language;
            draw_screen_rect(
                renderer,
                viewport,
                choice,
                if active {
                    Color::rgba(0.08, 0.44, 0.60, 0.88)
                } else {
                    Color::rgba(0.025, 0.070, 0.086, 0.82)
                },
            );
            draw_border(
                renderer,
                viewport,
                choice,
                1.0 * layout.scale,
                if active {
                    Color::rgba(0.52, 0.96, 1.0, 0.92)
                } else {
                    Color::rgba(0.12, 0.27, 0.35, 0.86)
                },
            );
            if let Some(value) = self.text.language_values.get(index) {
                draw_text_centered(
                    renderer,
                    value,
                    viewport,
                    choice.origin.x + choice.size.x * 0.5,
                    choice.origin.y + 11.0 * layout.scale,
                    if active {
                        Color::rgba(0.94, 1.0, 0.98, 1.0)
                    } else {
                        Color::rgba(0.62, 0.82, 0.88, 0.96)
                    },
                );
            }
        }

        if let Some(hint) = &self.text.settings_hint {
            draw_text(
                renderer,
                hint,
                viewport,
                content.origin.x + 24.0 * layout.scale,
                content.origin.y + 126.0 * layout.scale,
                Color::rgba(0.58, 0.78, 0.84, 0.96),
            );
        }
        if let Some(placeholder) = &self.text.placeholder {
            draw_text(
                renderer,
                placeholder,
                viewport,
                content.origin.x + 24.0 * layout.scale,
                content.origin.y + 176.0 * layout.scale,
                Color::rgba(0.48, 0.66, 0.72, 0.90),
            );
        }
    }

    fn upload_textures(&mut self, renderer: &mut dyn Renderer, font: &Font<'static>) -> Result<()> {
        let language = self.language;
        self.text = GameMenuText {
            language: Some(language),
            ..GameMenuText::default()
        };

        self.text.title = Some(upload_text(
            renderer,
            font,
            "game_menu_title",
            "Alien Archive",
            32.0,
        )?);
        self.text.status = Some(upload_text(
            renderer,
            font,
            "game_menu_status",
            menu_status(language),
            19.0,
        )?);
        self.text.close_hint = Some(upload_text(
            renderer,
            font,
            "game_menu_close_hint",
            close_hint(language),
            18.0,
        )?);
        self.text.nav_labels = GameMenuTab::ALL
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_nav_{index}"),
                    tab_label(*tab, language),
                    match language {
                        Language::Chinese => 25.0,
                        Language::English => 22.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.page_titles = GameMenuTab::ALL
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_page_title_{index}"),
                    tab_title(*tab, language),
                    match language {
                        Language::Chinese => 31.0,
                        Language::English => 28.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.page_subtitles = GameMenuTab::ALL
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_page_subtitle_{index}"),
                    tab_subtitle(*tab, language),
                    18.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let profile = profile_scene::profile_overview(language);
        self.text.profile_name = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_name",
            profile.callsign,
            match language {
                Language::Chinese => 27.0,
                Language::English => 24.0,
            },
        )?);
        self.text.profile_role = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_role",
            profile.role,
            18.0,
        )?);
        self.text.profile_id = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_id",
            profile.id_line,
            17.0,
        )?);
        self.text.profile_section_stats = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_stats_header",
            stat_header(language),
            22.0,
        )?);
        self.text.profile_section_core = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_core_header",
            profile_core_header(language),
            22.0,
        )?);
        self.text.profile_section_research = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_research_header",
            profile_research_header(language),
            22.0,
        )?);
        self.text.profile_stats = profile
            .vital_stats
            .iter()
            .enumerate()
            .map(|(index, stat)| {
                Ok((
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_profile_stat_label_{index}"),
                        stat.label,
                        18.0,
                    )?,
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_profile_stat_value_{index}"),
                        &format!("{}/{}", stat.value, stat.max),
                        16.0,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.profile_core_stats = profile
            .core_stats
            .iter()
            .enumerate()
            .map(|(index, stat)| {
                Ok((
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_profile_core_label_{index}"),
                        stat.label,
                        17.0,
                    )?,
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_profile_core_value_{index}"),
                        &format!("{}/10", stat.value),
                        15.0,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.profile_research_stats = profile
            .research_stats
            .iter()
            .enumerate()
            .map(|(index, stat)| {
                Ok((
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_profile_research_label_{index}"),
                        stat.label,
                        16.0,
                    )?,
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_profile_research_value_{index}"),
                        &format!("{}/{}", stat.value, stat.max),
                        14.0,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        self.text.inventory_slot_details = upload_inventory_slot_details(renderer, font, language)?;
        self.text.inventory_empty_title = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_empty_title",
            empty_slot_title(language),
            22.0,
        )?);
        self.text.inventory_empty_body = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_empty_body",
            empty_slot_body(language),
            17.0,
        )?);
        self.text.inventory_hint = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_hint",
            inventory_hint(language),
            17.0,
        )?);

        self.text.codex_cards = CODEX_PREVIEWS
            .iter()
            .enumerate()
            .map(|(index, card)| {
                Ok((
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_codex_label_{index}"),
                        card.label.get(language),
                        22.0,
                    )?,
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_codex_value_{index}"),
                        &format!("{}%", card.progress),
                        17.0,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.map_labels = map_labels(language)
            .iter()
            .enumerate()
            .map(|(index, label)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_map_label_{index}"),
                    label,
                    17.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.quest_rows = QUEST_PREVIEWS
            .iter()
            .enumerate()
            .map(|(index, quest)| {
                Ok((
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_quest_label_{index}"),
                        quest.label.get(language),
                        22.0,
                    )?,
                    upload_text(
                        renderer,
                        font,
                        &format!("game_menu_quest_status_{index}"),
                        quest.status.get(language),
                        16.0,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.settings_language = Some(upload_text(
            renderer,
            font,
            "game_menu_settings_language",
            language_setting_label(language),
            22.0,
        )?);
        self.text.settings_hint = Some(upload_text(
            renderer,
            font,
            "game_menu_settings_hint",
            settings_hint(language),
            18.0,
        )?);
        self.text.language_values = Language::SUPPORTED
            .iter()
            .enumerate()
            .map(|(index, language)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_language_value_{index}"),
                    language_option_label(*language),
                    22.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.placeholder = Some(upload_text(
            renderer,
            font,
            "game_menu_placeholder",
            placeholder_text(language),
            17.0,
        )?);

        Ok(())
    }
}

impl Scene for GameMenuScene {
    fn id(&self) -> SceneId {
        SceneId::GameMenu
    }

    fn name(&self) -> &str {
        "GameMenuScene"
    }

    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        if renderer
            .texture_size(EXPLORER_PORTRAIT_TEXTURE_ID)
            .is_none()
        {
            renderer.load_texture(
                EXPLORER_PORTRAIT_TEXTURE_ID,
                Path::new(EXPLORER_PORTRAIT_PATH),
            )?;
        }
        inventory_scene::load_inventory_item_icons(renderer)?;

        let font = load_ui_font()?;
        self.upload_textures(renderer, &font)
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        _dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if self.language != ctx.language {
            self.language = ctx.language;
            self.text = GameMenuText::default();
        }

        let viewport = input.screen_size();
        let layout = MenuLayout::new(viewport);
        if let Some(cursor_position) = input.cursor_position() {
            if input.mouse_left_just_pressed() {
                for (index, tab) in GameMenuTab::ALL.iter().copied().enumerate() {
                    if screen_point_in_rect(cursor_position, layout.nav_item(index)) {
                        self.set_tab(ctx, tab);
                    }
                }

                if self.active_tab == GameMenuTab::Settings {
                    for (index, language) in Language::SUPPORTED.iter().copied().enumerate() {
                        if screen_point_in_rect(cursor_position, layout.language_choice(index)) {
                            self.set_language(ctx, language);
                        }
                    }
                }

                if self.active_tab == GameMenuTab::Inventory {
                    self.handle_inventory_click(cursor_position, &layout);
                }
            }
        }

        if input.just_pressed(Button::Pause) {
            return Ok(SceneCommand::Pop);
        }
        if input.just_pressed(Button::Inventory) {
            self.set_tab(ctx, GameMenuTab::Inventory);
        }
        if input.just_pressed(Button::Profile) {
            self.set_tab(ctx, GameMenuTab::Profile);
        }
        if self.active_tab == GameMenuTab::Inventory {
            if input.just_pressed(Button::Left) {
                self.move_inventory_selection(-1, 0);
            }
            if input.just_pressed(Button::Right) {
                self.move_inventory_selection(1, 0);
            }
            if input.just_pressed(Button::Up) {
                self.move_inventory_selection(0, -1);
            }
            if input.just_pressed(Button::Down) {
                self.move_inventory_selection(0, 1);
            }
        } else {
            if input.just_pressed(Button::Left) {
                self.set_tab(ctx, self.active_tab.previous());
            }
            if input.just_pressed(Button::Right) {
                self.set_tab(ctx, self.active_tab.next());
            }
            if input.just_pressed(Button::Up) {
                self.set_tab(ctx, self.active_tab.previous());
            }
            if input.just_pressed(Button::Down) {
                self.set_tab(ctx, self.active_tab.next());
            }
        }
        if self.active_tab == GameMenuTab::Settings
            && (input.just_pressed(Button::Confirm) || input.just_pressed(Button::Interact))
        {
            self.set_language(ctx, ctx.language.next());
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        if self.text.language != Some(self.language) {
            let font = load_ui_font()?;
            self.upload_textures(ctx.renderer, &font)?;
        }

        self.draw_menu(ctx);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct MenuLayout {
    scale: f32,
    root: Rect,
    top: Rect,
    nav: Rect,
    content: Rect,
}

impl MenuLayout {
    fn new(viewport: Vec2) -> Self {
        let scale = ((viewport.x - 36.0) / PANEL_WIDTH)
            .min((viewport.y - 32.0) / PANEL_HEIGHT)
            .min(1.0)
            .max(0.62);
        let root = Rect::new(
            Vec2::new(
                (viewport.x - PANEL_WIDTH * scale) * 0.5,
                (viewport.y - PANEL_HEIGHT * scale) * 0.5,
            ),
            Vec2::new(PANEL_WIDTH * scale, PANEL_HEIGHT * scale),
        );
        let top = Rect::new(root.origin, Vec2::new(root.size.x, TOP_HEIGHT * scale));
        let content_y = root.origin.y + (TOP_HEIGHT + PANEL_GAP) * scale;
        let nav = Rect::new(
            Vec2::new(root.origin.x + OUTER_PADDING * scale, content_y),
            Vec2::new(
                NAV_WIDTH * scale,
                root.bottom() - content_y - OUTER_PADDING * scale,
            ),
        );
        let content = Rect::new(
            Vec2::new(nav.right() + PANEL_GAP * scale, content_y),
            Vec2::new(
                root.right() - nav.right() - (PANEL_GAP + OUTER_PADDING) * scale,
                nav.size.y,
            ),
        );

        Self {
            scale,
            root,
            top,
            nav,
            content,
        }
    }

    fn nav_item(&self, index: usize) -> Rect {
        Rect::new(
            Vec2::new(
                self.nav.origin.x + 10.0 * self.scale,
                self.nav.origin.y
                    + 14.0 * self.scale
                    + index as f32 * (NAV_ITEM_HEIGHT + NAV_ITEM_GAP) * self.scale,
            ),
            Vec2::new(
                self.nav.size.x - 20.0 * self.scale,
                NAV_ITEM_HEIGHT * self.scale,
            ),
        )
    }

    fn content_body(&self) -> Rect {
        Rect::new(
            Vec2::new(
                self.content.origin.x + CONTENT_PADDING * self.scale,
                self.content.origin.y + 110.0 * self.scale,
            ),
            Vec2::new(
                self.content.size.x - CONTENT_PADDING * 2.0 * self.scale,
                self.content.size.y - 132.0 * self.scale,
            ),
        )
    }

    fn language_choice(&self, index: usize) -> Rect {
        let body = self.content_body();
        Rect::new(
            Vec2::new(
                body.right() - (300.0 - index as f32 * 140.0) * self.scale,
                body.origin.y + 21.0 * self.scale,
            ),
            Vec2::new(124.0 * self.scale, 50.0 * self.scale),
        )
    }
}

fn inventory_slot_rect(panel: Rect, index: usize, scale: f32) -> Rect {
    let slot_size = 66.0 * scale;
    let gap = 10.0 * scale;
    let col = index % INVENTORY_MENU_COLUMNS;
    let row = index / INVENTORY_MENU_COLUMNS;

    Rect::new(
        Vec2::new(
            panel.origin.x + 24.0 * scale + col as f32 * (slot_size + gap),
            panel.origin.y + 28.0 * scale + row as f32 * (slot_size + gap),
        ),
        Vec2::new(slot_size, slot_size),
    )
}

fn move_inventory_slot(selected: usize, dx: isize, dy: isize) -> usize {
    let col = selected % INVENTORY_MENU_COLUMNS;
    let row = selected / INVENTORY_MENU_COLUMNS;
    let next_col = (col as isize + dx).clamp(0, INVENTORY_MENU_COLUMNS as isize - 1) as usize;
    let next_row = (row as isize + dy).clamp(0, INVENTORY_MENU_ROWS as isize - 1) as usize;

    next_row * INVENTORY_MENU_COLUMNS + next_col
}

fn upload_inventory_slot_details(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    language: Language,
) -> Result<Vec<Option<InventoryDetailText>>> {
    let mut details = Vec::with_capacity(INVENTORY_MENU_SLOTS);
    for (index, item) in inventory_scene::mvp_inventory_slots()
        .into_iter()
        .enumerate()
    {
        let Some(item) = item else {
            details.push(None);
            continue;
        };

        details.push(Some(InventoryDetailText {
            name: upload_text(
                renderer,
                font,
                &format!("game_menu_inventory_detail_name_{index}"),
                item.name(language),
                24.0,
            )?,
            category: upload_text(
                renderer,
                font,
                &format!("game_menu_inventory_detail_category_{index}"),
                &format!("{}: {}", category_label(language), item.category(language)),
                17.0,
            )?,
            quantity: upload_text(
                renderer,
                font,
                &format!("game_menu_inventory_detail_quantity_{index}"),
                &format!("{}: {}", quantity_label(language), item.quantity),
                17.0,
            )?,
            rarity: upload_text(
                renderer,
                font,
                &format!("game_menu_inventory_detail_rarity_{index}"),
                &format!("{}: {}", rarity_label(language), item.rarity(language)),
                18.0,
            )?,
            stack_limit: upload_text(
                renderer,
                font,
                &format!("game_menu_inventory_detail_stack_{index}"),
                &format!("{}: {}", stack_limit_label(language), item.max_stack),
                17.0,
            )?,
            research: upload_text(
                renderer,
                font,
                &format!("game_menu_inventory_detail_research_{index}"),
                &format!("{}: {}%", research_label(language), item.research),
                17.0,
            )?,
            lock_state: if item.locked {
                Some(upload_text(
                    renderer,
                    font,
                    &format!("game_menu_inventory_detail_lock_{index}"),
                    locked_label(language),
                    17.0,
                )?)
            } else {
                None
            },
        }));
    }

    Ok(details)
}

fn tab_index(tab: GameMenuTab) -> usize {
    GameMenuTab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or_default()
}

fn tab_label(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "人物属性",
            GameMenuTab::Inventory => "背包",
            GameMenuTab::Codex => "图鉴",
            GameMenuTab::Map => "地图",
            GameMenuTab::Quests => "任务",
            GameMenuTab::Settings => "设置",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Profile",
            GameMenuTab::Inventory => "Inventory",
            GameMenuTab::Codex => "Codex",
            GameMenuTab::Map => "Map",
            GameMenuTab::Quests => "Quests",
            GameMenuTab::Settings => "Settings",
        },
    }
}

fn tab_title(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "外勤档案",
            GameMenuTab::Inventory => "背包",
            GameMenuTab::Codex => "异星图鉴",
            GameMenuTab::Map => "区域地图",
            GameMenuTab::Quests => "任务日志",
            GameMenuTab::Settings => "设置",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Field Dossier",
            GameMenuTab::Inventory => "Inventory",
            GameMenuTab::Codex => "Alien Codex",
            GameMenuTab::Map => "Region Map",
            GameMenuTab::Quests => "Quest Log",
            GameMenuTab::Settings => "Settings",
        },
    }
}

fn tab_subtitle(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "查看探索者状态、能力与装备模块",
            GameMenuTab::Inventory => "管理样本、消耗品、工具与关键物品",
            GameMenuTab::Codex => "追踪已发现的生物、矿物和遗迹资料",
            GameMenuTab::Map => "确认探索路线、入口和未调查区域",
            GameMenuTab::Quests => "查看当前目标和外勤进度",
            GameMenuTab::Settings => "调整语言与游戏内菜单偏好",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Review explorer status, aptitudes, and suit modules",
            GameMenuTab::Inventory => "Manage samples, consumables, tools, and key items",
            GameMenuTab::Codex => "Track discovered organisms, minerals, and ruin records",
            GameMenuTab::Map => "Check routes, entrances, and unsurveyed sectors",
            GameMenuTab::Quests => "Review active objectives and field progress",
            GameMenuTab::Settings => "Adjust language and in-game menu preferences",
        },
    }
}

fn menu_status(language: Language) -> &'static str {
    match language {
        Language::Chinese => "外勤菜单 · 可点击左侧切换",
        Language::English => "Field Menu · Click the left rail to switch",
    }
}

fn close_hint(language: Language) -> &'static str {
    match language {
        Language::Chinese => "Esc 关闭",
        Language::English => "Esc Close",
    }
}

fn stat_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "生命状态",
        Language::English => "Vital Status",
    }
}

fn profile_core_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "探索能力",
        Language::English => "Explorer Aptitudes",
    }
}

fn profile_research_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "研究专精",
        Language::English => "Research Focus",
    }
}

fn quantity_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "数量",
        Language::English => "Qty",
    }
}

fn category_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "类别",
        Language::English => "Category",
    }
}

fn rarity_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "稀有度",
        Language::English => "Rarity",
    }
}

fn stack_limit_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "最大堆叠",
        Language::English => "Max Stack",
    }
}

fn research_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "研究进度",
        Language::English => "Research",
    }
}

fn locked_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已锁定",
        Language::English => "Locked",
    }
}

fn empty_slot_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "空槽位",
        Language::English => "Empty Slot",
    }
}

fn empty_slot_body(language: Language) -> &'static str {
    match language {
        Language::Chinese => "此槽位当前没有物品。",
        Language::English => "There is no item in this slot.",
    }
}

fn inventory_hint(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已接入当前背包物品、图标、数量与稀有度。",
        Language::English => "Connected to current item icons, quantities, and rarities.",
    }
}

fn map_labels(language: Language) -> [&'static str; 3] {
    match language {
        Language::Chinese => ["当前位置: 着陆点", "目标: 晶体田", "未调查区域: 3"],
        Language::English => [
            "Current: Landing Site",
            "Target: Crystal Field",
            "Unsurveyed Sectors: 3",
        ],
    }
}

fn language_setting_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "语言",
        Language::English => "Language",
    }
}

fn language_option_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "English",
    }
}

fn settings_hint(language: Language) -> &'static str {
    match language {
        Language::Chinese => "菜单不会同时堆中英文；这里切换后全局界面会跟随语言刷新。",
        Language::English => {
            "The UI does not stack both languages; switch here to refresh the global language."
        }
    }
}

fn placeholder_text(language: Language) -> &'static str {
    match language {
        Language::Chinese => "音量、窗口、控制等设置后续接入。",
        Language::English => "Audio, display, and control settings will be connected later.",
    }
}

fn draw_screen_rect(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, color: Color) {
    renderer.draw_rect(screen_rect(viewport, rect), color);
}

fn screen_rect(viewport: Vec2, rect: Rect) -> Rect {
    Rect::new(
        Vec2::new(
            -viewport.x * 0.5 + rect.origin.x,
            -viewport.y * 0.5 + rect.origin.y,
        ),
        rect.size,
    )
}

fn screen_point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}

fn draw_text_strong(
    renderer: &mut dyn Renderer,
    text: &TextSprite,
    viewport: Vec2,
    x: f32,
    y: f32,
    color: Color,
    scale: f32,
) {
    draw_text(renderer, text, viewport, x, y, color);
    draw_text(
        renderer,
        text,
        viewport,
        x + 1.0_f32.max(scale),
        y,
        Color::rgba(color.r, color.g, color.b, color.a * 0.72),
    );
}

fn draw_inner_panel(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, scale: f32) {
    draw_screen_rect(
        renderer,
        viewport,
        rect,
        Color::rgba(0.014, 0.030, 0.039, 0.86),
    );
    draw_border(
        renderer,
        viewport,
        rect,
        1.0 * scale,
        Color::rgba(0.12, 0.25, 0.32, 0.82),
    );
}

fn draw_border(
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

fn draw_corner_brackets(
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

fn draw_nav_icon(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    tab: GameMenuTab,
    rect: Rect,
    active: bool,
    scale: f32,
) {
    let color = if active {
        Color::rgba(0.72, 1.0, 0.90, 1.0)
    } else {
        Color::rgba(0.28, 0.68, 0.78, 0.92)
    };
    let icon = Rect::new(
        Vec2::new(rect.origin.x + 22.0 * scale, rect.origin.y + 20.0 * scale),
        Vec2::new(22.0 * scale, 22.0 * scale),
    );

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

fn draw_bar(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, ratio: f32, scale: f32) {
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

fn draw_score_pips(
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

fn draw_compact_stat_bar(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    panel: Rect,
    label: &TextSprite,
    value: &TextSprite,
    ratio: f32,
    y: f32,
    scale: f32,
) {
    draw_text(
        renderer,
        label,
        viewport,
        panel.origin.x + 24.0 * scale,
        y,
        Color::rgba(0.68, 0.88, 0.92, 0.96),
    );
    draw_text(
        renderer,
        value,
        viewport,
        panel.right() - value.size.x - 24.0 * scale,
        y,
        Color::rgba(0.90, 1.0, 0.96, 1.0),
    );
    draw_bar(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(panel.origin.x + 24.0 * scale, y + 19.0 * scale),
            Vec2::new(panel.size.x - 48.0 * scale, 6.0 * scale),
        ),
        ratio,
        scale,
    );
}

fn draw_codex_glyph(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    index: usize,
    scale: f32,
) {
    let color = if index % 2 == 0 {
        Color::rgba(0.36, 0.88, 1.0, 0.92)
    } else {
        Color::rgba(0.80, 0.64, 0.96, 0.92)
    };
    let glyph = Rect::new(
        Vec2::new(card.origin.x + 28.0 * scale, card.origin.y + 34.0 * scale),
        Vec2::new(44.0 * scale, 44.0 * scale),
    );
    draw_screen_rect(
        renderer,
        viewport,
        glyph,
        Color::rgba(0.030, 0.070, 0.086, 0.90),
    );
    draw_border(renderer, viewport, glyph, 2.0 * scale, color);
    draw_screen_rect(renderer, viewport, inset_rect(glyph, 14.0 * scale), color);
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

fn inset_rect(rect: Rect, inset: f32) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x + inset, rect.origin.y + inset),
        Vec2::new(
            (rect.size.x - inset * 2.0).max(0.0),
            (rect.size.y - inset * 2.0).max(0.0),
        ),
    )
}

fn contain_rect(frame: Rect, image_size: Vec2) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_menu_tab_labels_exist_for_supported_languages() {
        for language in Language::SUPPORTED {
            assert!(!menu_status(language).is_empty());
            assert!(!close_hint(language).is_empty());
            assert!(!language_setting_label(language).is_empty());
            assert!(!settings_hint(language).is_empty());

            for tab in GameMenuTab::ALL {
                assert!(!tab_label(tab, language).is_empty());
                assert!(!tab_title(tab, language).is_empty());
                assert!(!tab_subtitle(tab, language).is_empty());
            }
        }
    }

    #[test]
    fn game_menu_tab_cycle_visits_every_section() {
        let mut tab = GameMenuTab::Profile;
        for expected in GameMenuTab::ALL.iter().copied().cycle().take(12) {
            assert_eq!(tab, expected);
            tab = tab.next();
        }
    }

    #[test]
    fn game_menu_layout_keeps_nav_and_content_inside_root() {
        let layout = MenuLayout::new(Vec2::new(1280.0, 720.0));

        assert!(layout.nav.origin.x >= layout.root.origin.x);
        assert!(layout.nav.bottom() <= layout.root.bottom());
        assert!(layout.content.origin.x > layout.nav.right());
        assert!(layout.content.right() <= layout.root.right());
        assert!(layout.content.bottom() <= layout.root.bottom());
    }

    #[test]
    fn game_menu_reads_shared_profile_and_inventory_views() {
        let profile = profile_scene::profile_overview(Language::Chinese);
        let inventory_slots = inventory_scene::mvp_inventory_slots();

        assert_eq!(profile.vital_stats.len(), 4);
        assert_eq!(profile.core_stats.len(), 5);
        assert_eq!(profile.research_stats.len(), 4);
        assert_eq!(inventory_slots.len(), 24);
        assert_eq!(inventory_slots.iter().flatten().count(), 12);
        assert!(
            inventory_slots
                .iter()
                .flatten()
                .all(|item| !item.texture_id.is_empty())
        );
    }

    #[test]
    fn inventory_menu_selection_moves_inside_grid_bounds() {
        assert_eq!(move_inventory_slot(0, -1, 0), 0);
        assert_eq!(move_inventory_slot(0, 1, 0), 1);
        assert_eq!(move_inventory_slot(0, 0, 1), INVENTORY_MENU_COLUMNS);
        assert_eq!(
            move_inventory_slot(INVENTORY_MENU_SLOTS - 1, 1, 0),
            INVENTORY_MENU_SLOTS - 1
        );
        assert_eq!(
            move_inventory_slot(INVENTORY_MENU_SLOTS - 1, 0, 1),
            INVENTORY_MENU_SLOTS - 1
        );
    }

    #[test]
    fn inventory_menu_slot_rects_do_not_overlap_neighbors() {
        let layout = MenuLayout::new(Vec2::new(1280.0, 720.0));
        let content = layout.content_body();
        let panel = Rect::new(
            content.origin,
            Vec2::new(540.0 * layout.scale, content.size.y),
        );
        let first = inventory_slot_rect(panel, 0, layout.scale);
        let second = inventory_slot_rect(panel, 1, layout.scale);

        assert!(first.right() < second.origin.x);
        assert!(first.bottom() <= panel.bottom());
    }
}
