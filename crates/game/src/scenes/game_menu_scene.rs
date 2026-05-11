use std::{collections::BTreeMap, ops::Range, path::Path};

use anyhow::Result;
use content::CodexEntry;
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::Font;

use crate::objectives::ObjectiveMenuRow;
use crate::save::{ActivityLogEntrySave, InventorySave, PlayerProfileSave};
use crate::ui::game_menu_content::{
    BOTTOM_ACTIONS, activity_category_label, activity_log_empty, activity_log_header,
    category_label, close_hint, codex_discoveries_title, compact_vital_label, empty_slot_body,
    empty_slot_title, inventory_hint, language_option_label, language_setting_label, locked_label,
    map_labels, menu_status, placeholder_text, profile_core_header, profile_level_label,
    profile_research_header, quantity_label, rarity_label, research_label, return_label,
    return_sublabel, settings_hint, stack_limit_label, stat_header, tab_index, tab_label,
    tab_sublabel, tab_subtitle, tab_title, top_location_label, top_location_value,
    top_status_label, top_status_value,
};
use crate::ui::layout::{Align, Grid, Insets, Justify, Stack};
use crate::ui::menu_style::{
    self, MenuLayout, color, grid, icon, inset_rect, inventory_slot_rect, move_inventory_slot,
    skin, space,
};
use crate::ui::menu_widgets::{
    contain_rect, draw_bar, draw_border, draw_corner_brackets, draw_header_cell, draw_inner_panel,
    draw_panel_title, draw_score_pips, draw_screen_rect, draw_text_strong, draw_texture_nine_slice,
    draw_texture_rect, screen_point_in_rect, screen_rect,
};
use crate::ui::text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text};

use super::{GameContext, GameMenuTab, Language, RenderContext, Scene, SceneId};
use super::{inventory_scene, profile_scene};

const EXPLORER_PORTRAIT_TEXTURE_ID: &str = "game_menu.explorer_portrait";
const EXPLORER_PORTRAIT_PATH: &str = "assets/images/ui/profile/explorer_portrait.png";
const CODEX_VISIBLE_ROWS: usize = 4;
const CODEX_VISIBLE_COLUMNS: usize = 2;
const CODEX_VISIBLE_COUNT: usize = CODEX_VISIBLE_ROWS * CODEX_VISIBLE_COLUMNS;
const ACTIVITY_LOG_ROW_HEIGHT: f32 = 55.0;
const ACTIVITY_LOG_ROW_GAP: f32 = 8.0;
const ACTIVITY_LOG_HEADER_HEIGHT: f32 = 62.0;
const ACTIVITY_LOG_BOTTOM_PADDING: f32 = 16.0;
const MENU_TOAST_VISIBLE_TIME: f32 = 2.8;
const MENU_TOAST_WIDTH: f32 = 560.0;
const MENU_TOAST_HEIGHT: f32 = 42.0;

#[derive(Default)]
struct GameMenuText {
    language: Option<Language>,
    title: Option<TextSprite>,
    status: Option<TextSprite>,
    close_hint: Option<TextSprite>,
    nav_labels: Vec<TextSprite>,
    nav_sublabels: Vec<TextSprite>,
    top_location_label: Option<TextSprite>,
    top_location_value: Option<TextSprite>,
    top_status_label: Option<TextSprite>,
    top_status_value: Option<TextSprite>,
    top_crystals: Option<TextSprite>,
    top_credits: Option<TextSprite>,
    page_titles: Vec<TextSprite>,
    page_subtitles: Vec<TextSprite>,
    profile_name: Option<TextSprite>,
    profile_role: Option<TextSprite>,
    profile_id: Option<TextSprite>,
    profile_level_label: Option<TextSprite>,
    profile_level_value: Option<TextSprite>,
    profile_xp_value: Option<TextSprite>,
    profile_section_stats: Option<TextSprite>,
    profile_section_core: Option<TextSprite>,
    profile_section_research: Option<TextSprite>,
    profile_stats: Vec<(TextSprite, TextSprite)>,
    profile_core_stats: Vec<(TextSprite, TextSprite)>,
    profile_research_stats: Vec<(TextSprite, TextSprite)>,
    inventory_capacity: Option<TextSprite>,
    inventory_slot_counts: Vec<Option<TextSprite>>,
    inventory_slot_details: Vec<Option<InventoryDetailText>>,
    inventory_hint: Option<TextSprite>,
    inventory_empty_title: Option<TextSprite>,
    inventory_empty_body: Option<TextSprite>,
    codex_discoveries_title: Option<TextSprite>,
    codex_capacity: Option<TextSprite>,
    codex_cards: Vec<CodexSummaryText>,
    codex_entries: Vec<CodexEntryCardText>,
    map_labels: Vec<TextSprite>,
    quest_rows: Vec<ObjectiveRowText>,
    activity_log_header: Option<TextSprite>,
    activity_log_empty: Option<TextSprite>,
    activity_log_rows: Vec<ActivityLogRowText>,
    settings_language: Option<TextSprite>,
    settings_hint: Option<TextSprite>,
    language_values: Vec<TextSprite>,
    bottom_action_labels: Vec<TextSprite>,
    bottom_action_sublabels: Vec<TextSprite>,
    return_label: Option<TextSprite>,
    return_sublabel: Option<TextSprite>,
    placeholder: Option<TextSprite>,
    toast: Option<TextSprite>,
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

struct CodexSummaryText {
    label: TextSprite,
    value: TextSprite,
    ratio: f32,
}

struct CodexEntryCardText {
    title: TextSprite,
    category: TextSprite,
    status: TextSprite,
    description_lines: Vec<TextSprite>,
    unlocked: bool,
}

struct ObjectiveRowText {
    label: TextSprite,
    status: TextSprite,
    progress: u32,
}

struct ActivityLogRowText {
    category: TextSprite,
    title: TextSprite,
    detail: TextSprite,
    meta: TextSprite,
    category_key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GameMenuToastTone {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug)]
struct GameMenuToast {
    message: String,
    tone: GameMenuToastTone,
    remaining: f32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CodexMenuSnapshot {
    entries: Vec<CodexEntryView>,
}

impl CodexMenuSnapshot {
    fn from_context(ctx: &GameContext) -> Self {
        Self {
            entries: ctx
                .codex_database
                .entries()
                .iter()
                .map(|entry| {
                    CodexEntryView::from_entry(entry, ctx.scanned_codex_ids.contains(&entry.id))
                })
                .collect(),
        }
    }

    fn unlocked_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.unlocked).count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CodexEntryView {
    id: String,
    title: String,
    category: String,
    description: String,
    unlocked: bool,
}

impl CodexEntryView {
    fn from_entry(entry: &CodexEntry, unlocked: bool) -> Self {
        Self {
            id: entry.id.clone(),
            title: non_empty_or(&entry.title, &entry.id).to_owned(),
            category: non_empty_or(&entry.category, "Unknown").to_owned(),
            description: entry.description.clone(),
            unlocked,
        }
    }
}

pub struct GameMenuScene {
    language: Language,
    active_tab: GameMenuTab,
    selected_inventory_slot: usize,
    selected_codex_entry: usize,
    activity_log_scroll: usize,
    codex_snapshot: CodexMenuSnapshot,
    profile_snapshot: PlayerProfileSave,
    inventory_snapshot: InventorySave,
    collection_snapshot: usize,
    objective_snapshot: Vec<ObjectiveMenuRow>,
    activity_log_snapshot: Vec<ActivityLogEntrySave>,
    text: GameMenuText,
    toast: Option<GameMenuToast>,
}

impl GameMenuScene {
    pub fn new(ctx: &GameContext) -> Self {
        Self {
            language: ctx.language,
            active_tab: ctx.game_menu_tab,
            selected_inventory_slot: 0,
            selected_codex_entry: 0,
            activity_log_scroll: 0,
            codex_snapshot: CodexMenuSnapshot::from_context(ctx),
            profile_snapshot: ctx.save_data.profile.clone(),
            inventory_snapshot: ctx.save_data.inventory.clone(),
            collection_snapshot: ctx.save_data.world.collected_entities.len(),
            objective_snapshot: ctx.objective_menu_rows(),
            activity_log_snapshot: ctx.save_data.activity_log.entries.clone(),
            text: GameMenuText::default(),
            toast: None,
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
        self.toast = None;
        self.text = GameMenuText::default();
        ctx.request_save();
    }

    fn sync_codex_snapshot(&mut self, ctx: &GameContext) {
        let snapshot = CodexMenuSnapshot::from_context(ctx);
        if snapshot == self.codex_snapshot {
            return;
        }

        self.codex_snapshot = snapshot;
        self.selected_codex_entry = self
            .selected_codex_entry
            .min(self.codex_snapshot.entries.len().saturating_sub(1));
        self.text = GameMenuText::default();
    }

    fn sync_save_snapshot(&mut self, ctx: &mut GameContext) {
        ctx.sync_derived_profile_state();
        let activity_log_changed = self.activity_log_snapshot != ctx.save_data.activity_log.entries;
        let collection_changed =
            self.collection_snapshot != ctx.save_data.world.collected_entities.len();
        let objective_rows = ctx.objective_menu_rows();
        let objective_changed = self.objective_snapshot != objective_rows;
        if self.profile_snapshot == ctx.save_data.profile
            && self.inventory_snapshot == ctx.save_data.inventory
            && !collection_changed
            && !objective_changed
            && !activity_log_changed
        {
            return;
        }

        self.profile_snapshot = ctx.save_data.profile.clone();
        self.inventory_snapshot = ctx.save_data.inventory.clone();
        self.collection_snapshot = ctx.save_data.world.collected_entities.len();
        self.objective_snapshot = objective_rows;
        self.activity_log_snapshot = ctx.save_data.activity_log.entries.clone();
        self.selected_inventory_slot = self.selected_inventory_slot.min(
            inventory_scene::inventory_slots(&self.inventory_snapshot)
                .len()
                .saturating_sub(1),
        );
        if activity_log_changed {
            self.activity_log_scroll = 0;
        } else {
            self.clamp_activity_log_scroll();
        }
        self.text = GameMenuText::default();
    }

    fn max_activity_log_scroll(&self, visible_rows: usize) -> usize {
        self.activity_log_snapshot
            .len()
            .saturating_sub(visible_rows.max(1))
    }

    fn clamp_activity_log_scroll(&mut self) {
        self.activity_log_scroll = self
            .activity_log_scroll
            .min(self.activity_log_snapshot.len().saturating_sub(1));
    }

    fn scroll_activity_log(&mut self, rows: isize, visible_rows: usize) {
        if rows == 0 {
            return;
        }

        let current = self.activity_log_scroll as isize;
        let max_scroll = self.max_activity_log_scroll(visible_rows) as isize;
        self.activity_log_scroll = (current + rows).clamp(0, max_scroll) as usize;
    }

    fn set_activity_log_scroll_from_track(&mut self, cursor_y: f32, layout: &MenuLayout) {
        let log_panel = activity_log_panel_rect(layout);
        let visible_rows = activity_log_visible_capacity(log_panel, layout.scale);
        let max_scroll = self.max_activity_log_scroll(visible_rows);
        if max_scroll == 0 {
            self.activity_log_scroll = 0;
            return;
        }

        let track = activity_log_scrollbar_track_rect(log_panel, layout.scale);
        let ratio = ((cursor_y - track.origin.y) / track.size.y).clamp(0.0, 1.0);
        self.activity_log_scroll = (ratio * max_scroll as f32).round() as usize;
    }

    fn show_toast(&mut self, message: impl Into<String>, tone: GameMenuToastTone) {
        self.toast = Some(GameMenuToast {
            message: message.into(),
            tone,
            remaining: MENU_TOAST_VISIBLE_TIME,
        });
        self.text = GameMenuText::default();
    }

    fn activity_collection_count(&self) -> usize {
        let carried_find_count = self
            .inventory_snapshot
            .slots
            .iter()
            .flatten()
            .filter(|stack| !stack.locked && stack.quantity > 0)
            .count();
        self.activity_log_snapshot
            .iter()
            .filter(|entry| entry.category == "pickup")
            .count()
            .max(self.collection_snapshot)
            .max(carried_find_count)
    }

    fn update_toast(&mut self, dt: f32) {
        if let Some(toast) = &mut self.toast {
            toast.remaining -= dt;
            if toast.remaining <= 0.0 {
                self.toast = None;
                self.text = GameMenuText::default();
            }
        }
    }

    fn handle_bottom_action(&mut self, ctx: &mut GameContext, index: usize) {
        match index {
            0 => {
                self.set_tab(ctx, GameMenuTab::Inventory);
                self.show_toast(
                    game_menu_tab_jump_message(self.language),
                    GameMenuToastTone::Info,
                );
            }
            2 => {
                self.set_tab(ctx, GameMenuTab::Quests);
                self.show_toast(
                    game_menu_log_jump_message(self.language),
                    GameMenuToastTone::Info,
                );
            }
            index if index + 1 == BOTTOM_ACTIONS.len() => match ctx.save_now() {
                Ok(()) => {
                    self.sync_save_snapshot(ctx);
                    self.show_toast(
                        game_menu_save_success_message(self.language, &ctx.save_path),
                        GameMenuToastTone::Success,
                    );
                }
                Err(error) => {
                    self.show_toast(
                        game_menu_save_error_message(self.language, &error.to_string()),
                        GameMenuToastTone::Error,
                    );
                }
            },
            _ => {
                self.show_toast(
                    game_menu_action_pending_message(self.language),
                    GameMenuToastTone::Info,
                );
            }
        }
    }

    fn draw_menu(&self, ctx: &mut RenderContext<'_>) {
        let viewport = ctx.renderer.screen_size();
        let layout = MenuLayout::new(viewport);

        draw_screen_rect(
            ctx.renderer,
            viewport,
            Rect::new(Vec2::ZERO, viewport),
            color::SCREEN_OVERLAY,
        );
        self.draw_shell(ctx.renderer, viewport, &layout);
        self.draw_nav(ctx.renderer, viewport, &layout);
        self.draw_content(ctx.renderer, viewport, &layout);
        self.draw_bottom_bar(ctx.renderer, viewport, &layout);
        self.draw_toast(ctx.renderer, viewport, &layout);
    }

    fn draw_shell(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let root_textured = draw_texture_rect(
            renderer,
            viewport,
            skin::ROOT,
            layout.root,
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        if !root_textured {
            draw_screen_rect(renderer, viewport, layout.root, color::ROOT_FILL);
            draw_border(
                renderer,
                viewport,
                layout.root,
                1.0 * layout.scale,
                color::ROOT_BORDER,
            );
            draw_corner_brackets(
                renderer,
                viewport,
                layout.root,
                22.0 * layout.scale,
                2.0 * layout.scale,
                color::ROOT_BRACKET,
            );
        }

        if !root_textured {
            draw_screen_rect(renderer, viewport, layout.top, color::HEADER_FILL);
            draw_border(
                renderer,
                viewport,
                layout.top,
                1.0 * layout.scale,
                color::HEADER_BORDER,
            );
        }

        let header_panels = Stack::horizontal()
            .padding(Insets::new(18.0, 18.0, 0.0, 0.0).scaled(layout.scale))
            .justify(Justify::SpaceBetween)
            .align(Align::Stretch)
            .fixed_main(
                layout.top,
                &[
                    350.0 * layout.scale,
                    460.0 * layout.scale,
                    185.0 * layout.scale,
                    250.0 * layout.scale,
                ],
                None,
            );
        let brand = header_panels[0];
        let brand_textured = draw_texture_nine_slice(
            renderer,
            viewport,
            skin::HEADER,
            brand,
            36.0,
            18.0 * layout.scale,
            Color::rgba(1.0, 1.0, 1.0, 0.88),
        );
        if !brand_textured {
            draw_screen_rect(renderer, viewport, brand, color::BRAND_FILL);
            draw_border(
                renderer,
                viewport,
                brand,
                1.0 * layout.scale,
                color::BRAND_BORDER,
            );
        }

        let logo = Rect::new(
            Vec2::new(
                brand.origin.x + 18.0 * layout.scale,
                brand.origin.y + 13.0 * layout.scale,
            ),
            Vec2::new(58.0 * layout.scale, 58.0 * layout.scale),
        );
        if let Some(image_size) = renderer.texture_size(icon::BRAND_CRYSTAL) {
            renderer.draw_image(
                icon::BRAND_CRYSTAL,
                screen_rect(viewport, contain_rect(logo, image_size)),
                color::TRANSPARENT_IMAGE,
            );
        } else if renderer.texture_size("item_alien_crystal_sample").is_some() {
            draw_screen_rect(
                renderer,
                viewport,
                logo,
                Color::rgba(0.030, 0.090, 0.105, 0.80),
            );
            draw_border(
                renderer,
                viewport,
                logo,
                1.0 * layout.scale,
                Color::rgba(0.25, 0.78, 0.92, 0.84),
            );
            renderer.draw_image(
                "item_alien_crystal_sample",
                screen_rect(viewport, inset_rect(logo, 4.0 * layout.scale)),
                color::TRANSPARENT_IMAGE,
            );
        } else {
            draw_screen_rect(
                renderer,
                viewport,
                logo,
                Color::rgba(0.030, 0.090, 0.105, 0.80),
            );
            draw_border(
                renderer,
                viewport,
                logo,
                1.0 * layout.scale,
                Color::rgba(0.25, 0.78, 0.92, 0.84),
            );
            draw_crystal_glyph(renderer, viewport, logo, layout.scale);
        }

        if let Some(title) = &self.text.title {
            draw_text_strong(
                renderer,
                title,
                viewport,
                brand.origin.x + 88.0 * layout.scale,
                brand.origin.y + 25.0 * layout.scale,
                Color::rgba(0.55, 0.96, 1.0, 1.0),
                layout.scale,
            );
        }

        let location_panel = header_panels[1];
        draw_header_cell(renderer, viewport, location_panel, layout.scale);
        if let (Some(label), Some(value)) =
            (&self.text.top_location_label, &self.text.top_location_value)
        {
            draw_header_text_group(
                renderer,
                viewport,
                location_panel,
                label,
                value,
                color::TEXT_CYAN,
                layout.scale,
            );
        }

        let status_panel = header_panels[2];
        draw_header_cell(renderer, viewport, status_panel, layout.scale);
        if let (Some(label), Some(value)) =
            (&self.text.top_status_label, &self.text.top_status_value)
        {
            draw_header_text_group(
                renderer,
                viewport,
                status_panel,
                label,
                value,
                color::TEXT_GREEN,
                layout.scale,
            );
        }

        let resource_panel = header_panels[3];
        draw_header_cell(renderer, viewport, resource_panel, layout.scale);
        if let (Some(crystals), Some(credits)) = (&self.text.top_crystals, &self.text.top_credits) {
            draw_header_resources(
                renderer,
                viewport,
                resource_panel,
                crystals,
                credits,
                layout.scale,
            );
        }
    }

    fn draw_nav(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        for (index, tab) in GameMenuTab::ALL.iter().copied().enumerate() {
            let rect = layout.nav_item(index);
            let active = tab == self.active_tab;
            let button_skin = if active {
                skin::NAV_ACTIVE
            } else {
                skin::NAV_IDLE
            };
            let button_textured = draw_texture_nine_slice(
                renderer,
                viewport,
                button_skin,
                rect,
                50.0,
                22.0 * layout.scale,
                Color::rgba(1.0, 1.0, 1.0, if active { 0.98 } else { 0.82 }),
            );
            if !button_textured {
                draw_screen_rect(
                    renderer,
                    viewport,
                    rect,
                    if active {
                        color::NAV_ACTIVE_FILL
                    } else {
                        color::NAV_IDLE_FILL
                    },
                );
                draw_border(
                    renderer,
                    viewport,
                    rect,
                    1.0 * layout.scale,
                    if active {
                        color::NAV_ACTIVE_BORDER
                    } else {
                        color::NAV_IDLE_BORDER
                    },
                );
                draw_corner_brackets(
                    renderer,
                    viewport,
                    rect,
                    14.0 * layout.scale,
                    2.0 * layout.scale,
                    if active {
                        color::NAV_ACTIVE_BRACKET
                    } else {
                        color::NAV_IDLE_BRACKET
                    },
                );
            }
            if active && !button_textured {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(rect.origin.x, rect.origin.y),
                        Vec2::new(4.0 * layout.scale, rect.size.y),
                    ),
                    color::NAV_ACTIVE_ACCENT,
                );
            }

            let nav_padding = Insets::new(30.0, 10.0, 0.0, 0.0).scaled(layout.scale);
            let icon_width = 52.0 * layout.scale;
            let nav_gap = 6.0 * layout.scale;
            let text_width =
                (rect.size.x - nav_padding.left - nav_padding.right - icon_width - nav_gap)
                    .max(0.0);
            let nav_parts = Stack::horizontal()
                .padding(nav_padding)
                .gap(nav_gap)
                .align(Align::Center)
                .fixed_main(rect, &[icon_width, text_width], Some(58.0 * layout.scale));
            let icon_rect = nav_parts[0];
            let text_rect = nav_parts[1];

            draw_nav_icon(renderer, viewport, tab, icon_rect, active, layout.scale);
            if let (Some(label), Some(sublabel)) = (
                self.text.nav_labels.get(index),
                self.text.nav_sublabels.get(index),
            ) {
                draw_two_line_menu_text(
                    renderer,
                    viewport,
                    text_rect,
                    label,
                    sublabel,
                    if active {
                        Color::rgba(0.94, 1.0, 0.98, 1.0)
                    } else {
                        Color::rgba(0.60, 0.78, 0.84, 0.96)
                    },
                    if active {
                        Color::rgba(0.72, 0.96, 1.0, 0.95)
                    } else {
                        Color::rgba(0.50, 0.68, 0.74, 0.92)
                    },
                    layout.scale,
                    0.0,
                    false,
                );
            }
        }
    }

    fn draw_bottom_bar(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        if renderer.texture_size(skin::ROOT).is_none() {
            let bottom_textured = draw_texture_nine_slice(
                renderer,
                viewport,
                skin::HEADER,
                layout.bottom,
                36.0,
                20.0 * layout.scale,
                Color::rgba(1.0, 1.0, 1.0, 0.82),
            );
            if !bottom_textured {
                draw_screen_rect(renderer, viewport, layout.bottom, color::BOTTOM_FILL);
                draw_border(
                    renderer,
                    viewport,
                    layout.bottom,
                    1.0 * layout.scale,
                    color::BOTTOM_BORDER,
                );
                draw_corner_brackets(
                    renderer,
                    viewport,
                    layout.bottom,
                    18.0 * layout.scale,
                    2.0 * layout.scale,
                    color::BOTTOM_BRACKET,
                );
            }
        }

        for (index, _action) in BOTTOM_ACTIONS.iter().enumerate() {
            let rect = layout.bottom_action(index, BOTTOM_ACTIONS.len());
            let button_textured = draw_texture_nine_slice(
                renderer,
                viewport,
                skin::BOTTOM_BUTTON,
                rect,
                36.0,
                18.0 * layout.scale,
                Color::rgba(1.0, 1.0, 1.0, 0.90),
            );
            if !button_textured {
                draw_screen_rect(renderer, viewport, rect, color::BOTTOM_BUTTON_FILL);
                draw_border(
                    renderer,
                    viewport,
                    rect,
                    1.0 * layout.scale,
                    color::BOTTOM_BUTTON_BORDER,
                );
            }
            draw_bottom_icon(renderer, viewport, rect, index, layout.scale);
            if let (Some(label), Some(sublabel)) = (
                self.text.bottom_action_labels.get(index),
                self.text.bottom_action_sublabels.get(index),
            ) {
                let text_rect = Rect::new(
                    Vec2::new(rect.origin.x + 66.0 * layout.scale, rect.origin.y),
                    Vec2::new(rect.size.x - 74.0 * layout.scale, rect.size.y),
                );
                draw_two_line_menu_text(
                    renderer,
                    viewport,
                    text_rect,
                    label,
                    sublabel,
                    Color::rgba(0.88, 1.0, 0.98, 1.0),
                    Color::rgba(0.46, 0.80, 0.92, 0.94),
                    layout.scale,
                    -1.0 * layout.scale,
                    false,
                );
            }
        }

        let return_button = layout.return_button();
        let return_textured = draw_texture_nine_slice(
            renderer,
            viewport,
            skin::RETURN_BUTTON,
            return_button,
            38.0,
            18.0 * layout.scale,
            Color::rgba(1.0, 1.0, 1.0, 0.98),
        );
        if !return_textured {
            draw_screen_rect(renderer, viewport, return_button, color::RETURN_FILL);
            draw_border(
                renderer,
                viewport,
                return_button,
                1.0 * layout.scale,
                color::RETURN_BORDER,
            );
            draw_corner_brackets(
                renderer,
                viewport,
                return_button,
                14.0 * layout.scale,
                2.0 * layout.scale,
                color::RETURN_ACCENT,
            );
        }
        draw_return_icon(renderer, viewport, return_button, layout.scale);
        if let (Some(label), Some(sublabel)) = (&self.text.return_label, &self.text.return_sublabel)
        {
            let text_rect = Rect::new(
                Vec2::new(
                    return_button.origin.x + 92.0 * layout.scale,
                    return_button.origin.y,
                ),
                Vec2::new(
                    return_button.size.x - 104.0 * layout.scale,
                    return_button.size.y,
                ),
            );
            draw_two_line_menu_text(
                renderer,
                viewport,
                text_rect,
                label,
                sublabel,
                color::TEXT_RETURN,
                color::TEXT_RETURN_SUB,
                layout.scale,
                -1.0 * layout.scale,
                false,
            );
        }
    }

    fn draw_toast(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let (Some(toast), Some(text)) = (&self.toast, &self.text.toast) else {
            return;
        };
        let rect = game_menu_toast_rect(layout);
        draw_screen_rect(renderer, viewport, rect, game_menu_toast_fill(toast.tone));
        draw_border(
            renderer,
            viewport,
            rect,
            1.0 * layout.scale,
            game_menu_toast_border(toast.tone),
        );
        draw_text_centered(
            renderer,
            text,
            viewport,
            rect.origin.x + rect.size.x * 0.5,
            centered_text_y(rect, text, 0.0),
            game_menu_toast_text(toast.tone),
        );
    }

    fn draw_content(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        if self.active_tab == GameMenuTab::Profile {
            self.draw_profile_page(renderer, viewport, layout);
            return;
        }

        self.draw_page_header(renderer, viewport, layout);
        match self.active_tab {
            GameMenuTab::Profile => {}
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
                layout.content.origin.x + space::CONTENT_PADDING * layout.scale,
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
                layout.content.origin.x + space::CONTENT_PADDING * layout.scale,
                layout.content.origin.y + 54.0 * layout.scale,
                Color::rgba(0.55, 0.77, 0.84, 0.96),
            );
        }
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(
                    layout.content.origin.x + space::CONTENT_PADDING * layout.scale,
                    layout.content.origin.y + 88.0 * layout.scale,
                ),
                Vec2::new(
                    layout.content.size.x - space::CONTENT_PADDING * 2.0 * layout.scale,
                    1.0,
                ),
            ),
            Color::rgba(0.29, 0.86, 1.0, 0.34),
        );
    }

    fn draw_profile_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let profile = profile_scene::profile_overview(self.language, &self.profile_snapshot);
        let content = layout.dashboard_body();
        let gap = 12.0 * layout.scale;
        let portrait_panel = Rect::new(
            content.origin,
            Vec2::new(360.0 * layout.scale, content.size.y),
        );
        let center_panel = Rect::new(
            Vec2::new(
                portrait_panel.right() + 16.0 * layout.scale,
                content.origin.y,
            ),
            Vec2::new(420.0 * layout.scale, content.size.y),
        );
        let right_column = Rect::new(
            Vec2::new(center_panel.right() + gap, content.origin.y),
            Vec2::new(content.right() - center_panel.right() - gap, content.size.y),
        );

        draw_inner_panel(renderer, viewport, portrait_panel, layout.scale);
        draw_corner_brackets(
            renderer,
            viewport,
            portrait_panel,
            18.0 * layout.scale,
            2.0 * layout.scale,
            Color::rgba(0.82, 0.62, 0.28, 0.80),
        );
        draw_panel_title(
            renderer,
            viewport,
            portrait_panel,
            self.text.profile_name.as_ref(),
            layout.scale,
            Color::rgba(0.46, 0.95, 1.0, 1.0),
        );

        let portrait_frame = Rect::new(
            Vec2::new(
                portrait_panel.origin.x + 24.0 * layout.scale,
                portrait_panel.origin.y + 60.0 * layout.scale,
            ),
            Vec2::new(
                portrait_panel.size.x - 48.0 * layout.scale,
                (portrait_panel.size.y - 220.0 * layout.scale)
                    .clamp(300.0 * layout.scale, 430.0 * layout.scale),
            ),
        );
        if let Some(image_size) = renderer.texture_size(EXPLORER_PORTRAIT_TEXTURE_ID) {
            renderer.draw_image(
                EXPLORER_PORTRAIT_TEXTURE_ID,
                screen_rect(
                    viewport,
                    contain_rect(inset_rect(portrait_frame, 2.0 * layout.scale), image_size),
                ),
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );
        }
        let level_panel = Rect::new(
            Vec2::new(
                portrait_panel.origin.x + 24.0 * layout.scale,
                portrait_panel.bottom() - 72.0 * layout.scale,
            ),
            Vec2::new(
                portrait_panel.size.x - 48.0 * layout.scale,
                52.0 * layout.scale,
            ),
        );
        let info_y = (level_panel.origin.y - 64.0 * layout.scale)
            .max(portrait_frame.bottom() + 12.0 * layout.scale);
        if let Some(role) = &self.text.profile_role {
            draw_text(
                renderer,
                role,
                viewport,
                portrait_panel.origin.x + 24.0 * layout.scale,
                info_y,
                Color::rgba(0.60, 0.80, 0.86, 0.96),
            );
        }
        if let Some(id) = &self.text.profile_id {
            draw_text(
                renderer,
                id,
                viewport,
                portrait_panel.origin.x + 24.0 * layout.scale,
                info_y + 28.0 * layout.scale,
                Color::rgba(0.78, 0.68, 0.48, 0.96),
            );
        }

        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(level_panel.origin, Vec2::new(level_panel.size.x, 1.0)),
            Color::rgba(0.28, 0.86, 1.0, 0.32),
        );
        if let Some(level_label) = &self.text.profile_level_label {
            draw_text(
                renderer,
                level_label,
                viewport,
                level_panel.origin.x,
                level_panel.origin.y + 10.0 * layout.scale,
                Color::rgba(0.68, 0.88, 0.92, 0.96),
            );
        }
        if let Some(level_value) = &self.text.profile_level_value {
            draw_text_strong(
                renderer,
                level_value,
                viewport,
                level_panel.origin.x,
                level_panel.origin.y + 28.0 * layout.scale,
                Color::rgba(0.35, 0.94, 1.0, 1.0),
                layout.scale,
            );
        }
        if let Some(xp_value) = &self.text.profile_xp_value {
            draw_text(
                renderer,
                xp_value,
                viewport,
                level_panel.right() - xp_value.size.x,
                level_panel.origin.y + 10.0 * layout.scale,
                Color::rgba(0.58, 0.94, 1.0, 0.96),
            );
        }
        draw_bar(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(
                    level_panel.origin.x + 76.0 * layout.scale,
                    level_panel.origin.y + 34.0 * layout.scale,
                ),
                Vec2::new(level_panel.size.x - 76.0 * layout.scale, 8.0 * layout.scale),
            ),
            xp_ratio(&self.profile_snapshot),
            layout.scale,
        );

        let research_height =
            (right_column.size.y * 0.38).clamp(180.0 * layout.scale, 240.0 * layout.scale);
        let codex_height = right_column.size.y - research_height - gap;
        let attributes_height = codex_height;
        let vital_height = research_height;
        let attributes_panel = Rect::new(
            center_panel.origin,
            Vec2::new(center_panel.size.x, attributes_height),
        );
        let vital_panel = Rect::new(
            Vec2::new(center_panel.origin.x, attributes_panel.bottom() + gap),
            Vec2::new(center_panel.size.x, vital_height),
        );
        let research_panel = Rect::new(
            right_column.origin,
            Vec2::new(right_column.size.x, research_height),
        );
        let codex_panel = Rect::new(
            Vec2::new(right_column.origin.x, research_panel.bottom() + gap),
            Vec2::new(
                right_column.size.x,
                right_column.bottom() - research_panel.bottom() - gap,
            ),
        );

        draw_inner_panel(renderer, viewport, attributes_panel, layout.scale);
        draw_inner_panel(renderer, viewport, vital_panel, layout.scale);
        draw_inner_panel(renderer, viewport, research_panel, layout.scale);
        draw_inner_panel(renderer, viewport, codex_panel, layout.scale);
        if let Some(header) = &self.text.profile_section_core {
            draw_panel_title(
                renderer,
                viewport,
                attributes_panel,
                Some(header),
                layout.scale,
                Color::rgba(0.46, 0.95, 1.0, 1.0),
            );
        }
        let row_count = self.text.profile_core_stats.len();
        let row_gap = 6.0 * layout.scale;
        let row_height = 40.0 * layout.scale;
        let rows_area = Rect::new(
            Vec2::new(
                attributes_panel.origin.x + 20.0 * layout.scale,
                attributes_panel.origin.y + 64.0 * layout.scale,
            ),
            Vec2::new(
                attributes_panel.size.x - 40.0 * layout.scale,
                attributes_panel.size.y - 84.0 * layout.scale,
            ),
        );
        let row_sizes = vec![row_height; row_count];
        let attribute_rows = Stack::vertical()
            .gap(row_gap)
            .fixed_main(rows_area, &row_sizes, None);
        for (index, (label, value)) in self.text.profile_core_stats.iter().enumerate() {
            let stat = profile.core_stats[index];
            let Some(row) = attribute_rows.get(index).copied() else {
                continue;
            };
            let icon_size = 30.0 * layout.scale;
            let icon_rect = Rect::new(
                Vec2::new(row.origin.x, row.origin.y + (row.size.y - icon_size) * 0.5),
                Vec2::new(icon_size, icon_size),
            );
            let text_x = row.origin.x + 48.0 * layout.scale;
            let text_y = row.origin.y - 2.0 * layout.scale;
            let value_y = text_y + 1.0 * layout.scale;
            let pips_y = row.origin.y + row.size.y - 12.0 * layout.scale;

            draw_attribute_icon(renderer, viewport, icon_rect, index, layout.scale);
            draw_text(
                renderer,
                label,
                viewport,
                text_x,
                text_y,
                Color::rgba(0.68, 0.88, 0.92, 0.96),
            );
            draw_text(
                renderer,
                value,
                viewport,
                row.right() - value.size.x,
                value_y,
                Color::rgba(0.90, 1.0, 0.96, 1.0),
            );
            draw_score_pips(
                renderer,
                viewport,
                Vec2::new(text_x, pips_y),
                stat.value,
                layout.scale,
            );
        }

        if let Some(header) = &self.text.profile_section_research {
            draw_panel_title(
                renderer,
                viewport,
                research_panel,
                Some(header),
                layout.scale,
                Color::rgba(0.46, 0.95, 1.0, 1.0),
            );
        }
        let research_count = self.text.profile_research_stats.len();
        let research_rows_area = Rect::new(
            Vec2::new(
                research_panel.origin.x + 24.0 * layout.scale,
                research_panel.origin.y + 56.0 * layout.scale,
            ),
            Vec2::new(
                research_panel.size.x - 48.0 * layout.scale,
                research_panel.size.y - 76.0 * layout.scale,
            ),
        );
        let research_gap = 8.0 * layout.scale;
        let research_row_height = if research_count == 0 {
            0.0
        } else {
            ((research_rows_area.size.y - research_gap * research_count.saturating_sub(1) as f32)
                / research_count as f32)
                .max(24.0 * layout.scale)
        };
        let research_row_sizes = vec![research_row_height; research_count];
        let research_rows = Stack::vertical().gap(research_gap).fixed_main(
            research_rows_area,
            &research_row_sizes,
            None,
        );
        for (index, (label, value)) in self.text.profile_research_stats.iter().enumerate() {
            let stat = profile.research_stats[index];
            let Some(row) = research_rows.get(index).copied() else {
                continue;
            };
            draw_compact_stat_bar(
                renderer,
                viewport,
                row,
                label,
                value,
                stat.value as f32 / stat.max as f32,
                layout.scale,
            );
        }

        let stat_count = self.text.profile_stats.len();
        let stat_gap = 12.0 * layout.scale;
        let stat_area = inset_rect(vital_panel, 18.0 * layout.scale);
        let stat_width = if stat_count == 0 {
            0.0
        } else {
            ((stat_area.size.x - stat_gap * stat_count.saturating_sub(1) as f32)
                / stat_count as f32)
                .max(0.0)
        };
        let stat_columns = Stack::horizontal().gap(stat_gap).fixed_main(
            stat_area,
            &vec![stat_width; stat_count],
            None,
        );
        for (index, (label, value)) in self.text.profile_stats.iter().enumerate() {
            let stat = profile.vital_stats[index];
            let Some(card) = stat_columns.get(index).copied() else {
                continue;
            };
            draw_status_card(
                renderer,
                viewport,
                card,
                label,
                value,
                stat.value as f32 / stat.max as f32,
                index,
                layout.scale,
            );
        }

        draw_panel_title(
            renderer,
            viewport,
            codex_panel,
            self.text.codex_discoveries_title.as_ref(),
            layout.scale,
            Color::rgba(0.46, 0.95, 1.0, 1.0),
        );
        if let Some(capacity) = &self.text.codex_capacity {
            draw_text(
                renderer,
                capacity,
                viewport,
                codex_panel.right() - capacity.size.x - 22.0 * layout.scale,
                codex_panel.origin.y + 18.0 * layout.scale,
                Color::rgba(0.70, 0.90, 0.94, 0.96),
            );
        }
        let card_row = Rect::new(
            Vec2::new(
                codex_panel.origin.x + 18.0 * layout.scale,
                codex_panel.origin.y + 62.0 * layout.scale,
            ),
            Vec2::new(
                codex_panel.size.x - 36.0 * layout.scale,
                codex_panel.size.y - 84.0 * layout.scale,
            ),
        );
        let card_slots = Stack::horizontal()
            .gap(8.0 * layout.scale)
            .even(card_row, self.text.codex_cards.len());
        for (index, card_text) in self.text.codex_cards.iter().enumerate() {
            let card = card_slots[index];
            draw_codex_discovery_card(
                renderer,
                viewport,
                card,
                index,
                &card_text.label,
                &card_text.value,
                card_text.ratio,
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
        let panel_gap = 16.0 * layout.scale;
        let grid_width = 540.0 * layout.scale;
        let detail_width = content.size.x - grid_width - panel_gap;
        let panels = Stack::horizontal().gap(panel_gap).fixed_main(
            content,
            &[grid_width, detail_width],
            None,
        );
        let [grid_panel, detail_panel] = [panels[0], panels[1]];
        draw_inner_panel(renderer, viewport, grid_panel, layout.scale);
        draw_inner_panel(renderer, viewport, detail_panel, layout.scale);

        let slots = inventory_scene::inventory_slots(&self.inventory_snapshot);
        for index in 0..grid::INVENTORY_SLOTS {
            let slot = inventory_slot_rect(grid_panel, index, layout.scale);
            let item = slots.get(index).and_then(|slot| *slot);
            let selected = index == self.selected_inventory_slot;
            let count = self
                .text
                .inventory_slot_counts
                .get(index)
                .and_then(|slot| slot.as_ref());
            draw_inventory_slot(
                renderer,
                viewport,
                slot,
                item,
                count,
                selected,
                layout.scale,
            );
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
            Color::rgba(0.020, 0.055, 0.068, 0.52),
        );
        draw_border(
            renderer,
            viewport,
            preview,
            1.0 * layout.scale,
            selected_item.map_or(Color::rgba(0.12, 0.27, 0.35, 0.58), |item| {
                Color::rgba(
                    item.rarity_color.r,
                    item.rarity_color.g,
                    item.rarity_color.b,
                    0.66,
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
        let panels = Stack::horizontal().gap(16.0 * layout.scale).fixed_main(
            content,
            &[540.0 * layout.scale, content.size.x - 556.0 * layout.scale],
            None,
        );
        panels[0]
    }

    fn handle_inventory_click(&mut self, point: Vec2, layout: &MenuLayout) {
        let grid_panel = self.inventory_grid_panel(layout);
        for index in 0..grid::INVENTORY_SLOTS {
            if screen_point_in_rect(point, inventory_slot_rect(grid_panel, index, layout.scale)) {
                self.selected_inventory_slot = index;
                break;
            }
        }
    }

    fn move_inventory_selection(&mut self, dx: isize, dy: isize) {
        self.selected_inventory_slot = move_inventory_slot(self.selected_inventory_slot, dx, dy);
    }

    fn move_codex_selection(&mut self, dx: isize, dy: isize) {
        let len = self.codex_snapshot.entries.len();
        if len == 0 {
            self.selected_codex_entry = 0;
            return;
        }

        let col = self.selected_codex_entry % CODEX_VISIBLE_COLUMNS;
        let row = self.selected_codex_entry / CODEX_VISIBLE_COLUMNS;
        let next_col = (col as isize + dx).clamp(0, CODEX_VISIBLE_COLUMNS as isize - 1) as usize;
        let max_row = (len - 1) / CODEX_VISIBLE_COLUMNS;
        let next_row = (row as isize + dy).clamp(0, max_row as isize) as usize;
        self.selected_codex_entry = (next_row * CODEX_VISIBLE_COLUMNS + next_col).min(len - 1);
    }

    fn draw_codex_page(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &MenuLayout) {
        let content = layout.content_body();
        if self.text.codex_entries.is_empty() {
            draw_inner_panel(renderer, viewport, content, layout.scale);
            if let Some(placeholder) = &self.text.placeholder {
                draw_text(
                    renderer,
                    placeholder,
                    viewport,
                    content.origin.x + 24.0 * layout.scale,
                    content.origin.y + 24.0 * layout.scale,
                    color::TEXT_DIM,
                );
            }
            return;
        }

        let grid = Rect::new(content.origin, Vec2::new(content.size.x, content.size.y));
        let cards = Grid::new(CODEX_VISIBLE_COLUMNS, CODEX_VISIBLE_ROWS)
            .gap(14.0 * layout.scale, 12.0 * layout.scale)
            .cells(grid);
        let page_start = (self.selected_codex_entry / CODEX_VISIBLE_COUNT) * CODEX_VISIBLE_COUNT;
        for (slot, index) in (page_start
            ..self
                .text
                .codex_entries
                .len()
                .min(page_start + CODEX_VISIBLE_COUNT))
            .enumerate()
        {
            let card = cards[slot];
            let selected = index == self.selected_codex_entry;
            draw_codex_entry_card(
                renderer,
                viewport,
                card,
                &self.text.codex_entries[index],
                selected,
                index,
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
        let objective_panel = activity_objective_panel_rect(layout);
        draw_inner_panel(renderer, viewport, objective_panel, layout.scale);

        let card_gap = 14.0 * layout.scale;
        let visible_count = self.text.quest_rows.len().min(3);
        let card_width = (objective_panel.size.x
            - 48.0 * layout.scale
            - card_gap * (visible_count.saturating_sub(1)) as f32)
            / visible_count.max(1) as f32;
        for (index, row_text) in self.text.quest_rows.iter().take(3).enumerate() {
            let row = Rect::new(
                Vec2::new(
                    objective_panel.origin.x
                        + 24.0 * layout.scale
                        + index as f32 * (card_width + card_gap),
                    objective_panel.origin.y + 24.0 * layout.scale,
                ),
                Vec2::new(card_width, 118.0 * layout.scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                row,
                Color::rgba(0.020, 0.050, 0.060, 0.72),
            );
            draw_border(
                renderer,
                viewport,
                row,
                1.0 * layout.scale,
                Color::rgba(0.16, 0.36, 0.42, 0.76),
            );
            draw_text_strong(
                renderer,
                &row_text.label,
                viewport,
                row.origin.x + 18.0 * layout.scale,
                row.origin.y + 18.0 * layout.scale,
                color::TEXT_PRIMARY,
                layout.scale,
            );
            let status_rect = Rect::new(
                Vec2::new(
                    row.origin.x + 18.0 * layout.scale,
                    row.origin.y + 50.0 * layout.scale,
                ),
                Vec2::new(112.0 * layout.scale, 28.0 * layout.scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                status_rect,
                Color::rgba(0.08, 0.32, 0.42, 0.86),
            );
            draw_text_centered(
                renderer,
                &row_text.status,
                viewport,
                status_rect.origin.x + status_rect.size.x * 0.5,
                status_rect.origin.y + 1.0 * layout.scale,
                Color::rgba(0.82, 1.0, 0.94, 1.0),
            );
            draw_bar(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(
                        row.origin.x + 18.0 * layout.scale,
                        row.origin.y + 92.0 * layout.scale,
                    ),
                    Vec2::new(row.size.x - 36.0 * layout.scale, 9.0 * layout.scale),
                ),
                row_text.progress as f32 / 100.0,
                layout.scale,
            );
        }

        let log_panel = activity_log_panel_rect(layout);
        draw_inner_panel(renderer, viewport, log_panel, layout.scale);
        if let Some(header) = &self.text.activity_log_header {
            draw_text_strong(
                renderer,
                header,
                viewport,
                log_panel.origin.x + 24.0 * layout.scale,
                log_panel.origin.y + 18.0 * layout.scale,
                color::TEXT_PRIMARY,
                layout.scale,
            );
        }

        if self.text.activity_log_rows.is_empty() {
            if let Some(empty) = &self.text.activity_log_empty {
                draw_text(
                    renderer,
                    empty,
                    viewport,
                    log_panel.origin.x + 24.0 * layout.scale,
                    log_panel.origin.y + 72.0 * layout.scale,
                    color::TEXT_MUTED,
                );
            }
            return;
        }

        let visible_rows = activity_log_visible_capacity(log_panel, layout.scale);
        let has_scrollbar = self.text.activity_log_rows.len() > visible_rows;
        let row_height = ACTIVITY_LOG_ROW_HEIGHT * layout.scale;
        let row_gap = ACTIVITY_LOG_ROW_GAP * layout.scale;
        let row_area = activity_log_row_area_rect(log_panel, layout.scale);
        let row_width_trim = if has_scrollbar { 52.0 } else { 36.0 } * layout.scale;
        let visible_range = activity_log_visible_range(
            self.text.activity_log_rows.len(),
            self.activity_log_scroll,
            visible_rows,
        );
        for (slot, index) in visible_range.enumerate() {
            let row_text = &self.text.activity_log_rows[index];
            let row = Rect::new(
                Vec2::new(
                    row_area.origin.x,
                    row_area.origin.y + slot as f32 * (row_height + row_gap),
                ),
                Vec2::new(log_panel.size.x - row_width_trim, row_height),
            );
            draw_screen_rect(
                renderer,
                viewport,
                row,
                Color::rgba(0.018, 0.040, 0.050, 0.72),
            );
            draw_border(
                renderer,
                viewport,
                row,
                1.0 * layout.scale,
                Color::rgba(0.10, 0.24, 0.30, 0.82),
            );

            let badge = Rect::new(
                Vec2::new(
                    row.origin.x + 12.0 * layout.scale,
                    row.origin.y + 13.0 * layout.scale,
                ),
                Vec2::new(72.0 * layout.scale, 28.0 * layout.scale),
            );
            let badge_color = activity_category_color(&row_text.category_key);
            draw_screen_rect(renderer, viewport, badge, badge_color);
            draw_text_centered(
                renderer,
                &row_text.category,
                viewport,
                badge.origin.x + badge.size.x * 0.5,
                badge.origin.y + 1.0 * layout.scale,
                Color::rgba(0.88, 1.0, 0.96, 1.0),
            );
            draw_text_strong(
                renderer,
                &row_text.title,
                viewport,
                row.origin.x + 102.0 * layout.scale,
                row.origin.y + 7.0 * layout.scale,
                color::TEXT_PRIMARY,
                layout.scale,
            );
            draw_text(
                renderer,
                &row_text.detail,
                viewport,
                row.origin.x + 102.0 * layout.scale,
                row.origin.y + 29.0 * layout.scale,
                color::TEXT_MUTED,
            );
            draw_text(
                renderer,
                &row_text.meta,
                viewport,
                row.right() - row_text.meta.size.x - 14.0 * layout.scale,
                row.origin.y + 8.0 * layout.scale,
                Color::rgba(0.46, 0.70, 0.76, 0.88),
            );
        }
        if has_scrollbar {
            draw_activity_log_scrollbar(
                renderer,
                viewport,
                log_panel,
                self.text.activity_log_rows.len(),
                self.activity_log_scroll,
                visible_rows,
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
            let label_y = centered_text_y(language_row, label, 0.0);
            draw_text_strong(
                renderer,
                label,
                viewport,
                language_row.origin.x + 24.0 * layout.scale,
                label_y,
                color::TEXT_PRIMARY,
                layout.scale,
            );
        }
        for (index, language) in Language::SUPPORTED.iter().copied().enumerate() {
            let choice = layout.language_choice(index);
            let active = language == self.language;
            let choice_skin = if active {
                "menu.skin_nav_active"
            } else {
                "menu.skin_language_toggle"
            };
            let choice_textured = draw_texture_nine_slice(
                renderer,
                viewport,
                choice_skin,
                choice,
                46.0,
                18.0 * layout.scale,
                Color::rgba(1.0, 1.0, 1.0, if active { 0.96 } else { 0.82 }),
            );
            if !choice_textured {
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
            }
            if let Some(value) = self.text.language_values.get(index) {
                draw_text_centered(
                    renderer,
                    value,
                    viewport,
                    choice.origin.x + choice.size.x * 0.5,
                    centered_text_y(choice, value, 0.0),
                    if active {
                        Color::rgba(0.94, 1.0, 0.98, 1.0)
                    } else {
                        color::TEXT_SECONDARY
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
                color::TEXT_DIM,
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
            "ALIEN ARCHIVE",
            32.0,
        )?);
        self.text.status = Some(upload_text(
            renderer,
            font,
            "game_menu_status",
            menu_status(language).as_ref(),
            19.0,
        )?);
        self.text.close_hint = Some(upload_text(
            renderer,
            font,
            "game_menu_close_hint",
            close_hint(language).as_ref(),
            18.0,
        )?);
        self.text.top_location_label = Some(upload_text(
            renderer,
            font,
            "game_menu_top_location_label",
            top_location_label(language).as_ref(),
            14.0,
        )?);
        self.text.top_location_value = Some(upload_text(
            renderer,
            font,
            "game_menu_top_location_value",
            top_location_value(language).as_ref(),
            20.0,
        )?);
        self.text.top_status_label = Some(upload_text(
            renderer,
            font,
            "game_menu_top_status_label",
            top_status_label(language).as_ref(),
            14.0,
        )?);
        self.text.top_status_value = Some(upload_text(
            renderer,
            font,
            "game_menu_top_status_value",
            top_status_value(language).as_ref(),
            20.0,
        )?);
        self.text.top_crystals = Some(upload_text(
            renderer,
            font,
            "game_menu_top_crystals",
            &self.activity_collection_count().to_string(),
            20.0,
        )?);
        self.text.top_credits = Some(upload_text(
            renderer,
            font,
            "game_menu_top_credits",
            &format!(
                "{} / {}",
                self.codex_snapshot.unlocked_count(),
                self.codex_snapshot.entries.len()
            ),
            20.0,
        )?);
        self.text.nav_labels = GameMenuTab::ALL
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_nav_{index}"),
                    tab_label(*tab, language).as_ref(),
                    match language {
                        Language::Chinese => 25.0,
                        Language::English => 22.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.nav_sublabels = GameMenuTab::ALL
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_nav_sub_{index}"),
                    tab_sublabel(*tab, language).as_ref(),
                    16.0,
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
                    tab_title(*tab, language).as_ref(),
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
                    tab_subtitle(*tab, language).as_ref(),
                    18.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let profile = profile_scene::profile_overview(language, &self.profile_snapshot);
        self.text.profile_name = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_name",
            &profile.callsign,
            match language {
                Language::Chinese => 27.0,
                Language::English => 24.0,
            },
        )?);
        self.text.profile_role = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_role",
            &profile.role,
            18.0,
        )?);
        self.text.profile_id = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_id",
            &profile.id_line,
            17.0,
        )?);
        self.text.profile_level_label = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_level_label",
            profile_level_label(language).as_ref(),
            16.0,
        )?);
        self.text.profile_level_value = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_level_value",
            &self.profile_snapshot.level.to_string(),
            36.0,
        )?);
        self.text.profile_xp_value = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_xp_value",
            &format!(
                "{} / {} XP",
                self.profile_snapshot.xp, self.profile_snapshot.xp_next
            ),
            17.0,
        )?);
        self.text.profile_section_stats = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_stats_header",
            stat_header(language).as_ref(),
            22.0,
        )?);
        self.text.profile_section_core = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_core_header",
            profile_core_header(language).as_ref(),
            22.0,
        )?);
        self.text.profile_section_research = Some(upload_text(
            renderer,
            font,
            "game_menu_profile_research_header",
            profile_research_header(language).as_ref(),
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
                        compact_vital_label(index, language).as_ref(),
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
                        &format!("{}/{}", stat.value, stat.max),
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

        let inventory_slots = inventory_scene::inventory_slots(&self.inventory_snapshot);
        let inventory_used = inventory_slots.iter().filter(|slot| slot.is_some()).count();
        self.text.inventory_capacity = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_capacity",
            &format!("{inventory_used} / {}", inventory_slots.len()),
            17.0,
        )?);
        self.text.inventory_slot_counts = inventory_slots
            .iter()
            .copied()
            .enumerate()
            .map(|(index, item)| match item {
                Some(item) => Ok(Some(upload_text(
                    renderer,
                    font,
                    &format!("game_menu_inventory_count_{index}"),
                    &item.quantity.to_string(),
                    14.0,
                )?)),
                None => Ok(None),
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.inventory_slot_details =
            upload_inventory_slot_details(renderer, font, language, &self.inventory_snapshot)?;
        self.text.inventory_empty_title = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_empty_title",
            empty_slot_title(language).as_ref(),
            22.0,
        )?);
        self.text.inventory_empty_body = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_empty_body",
            empty_slot_body(language).as_ref(),
            17.0,
        )?);
        self.text.inventory_hint = Some(upload_text(
            renderer,
            font,
            "game_menu_inventory_hint",
            inventory_hint(language).as_ref(),
            17.0,
        )?);

        self.text.codex_discoveries_title = Some(upload_text(
            renderer,
            font,
            "game_menu_codex_discoveries_title",
            codex_discoveries_title(language).as_ref(),
            20.0,
        )?);
        self.text.codex_capacity = Some(upload_text(
            renderer,
            font,
            "game_menu_codex_capacity",
            &format!(
                "{} / {}",
                self.codex_snapshot.unlocked_count(),
                self.codex_snapshot.entries.len()
            ),
            17.0,
        )?);
        self.text.codex_cards = codex_summary_views(&self.codex_snapshot, language)
            .iter()
            .enumerate()
            .map(|(index, summary)| {
                Ok(CodexSummaryText {
                    label: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_codex_label_{index}"),
                        &summary.label,
                        22.0,
                    )?,
                    value: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_codex_value_{index}"),
                        &summary.value,
                        17.0,
                    )?,
                    ratio: summary.ratio,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.codex_entries =
            upload_codex_entry_cards(renderer, font, language, &self.codex_snapshot)?;
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
        self.text.quest_rows = self
            .objective_snapshot
            .iter()
            .enumerate()
            .take(3)
            .map(|(index, objective)| {
                Ok(ObjectiveRowText {
                    label: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_quest_label_{index}"),
                        &short_activity_text(&objective.title, 22),
                        22.0,
                    )?,
                    status: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_quest_status_{index}"),
                        &objective.status,
                        16.0,
                    )?,
                    progress: objective.progress,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.activity_log_header = Some(upload_text(
            renderer,
            font,
            "game_menu_activity_log_header",
            activity_log_header(language).as_ref(),
            22.0,
        )?);
        self.text.activity_log_empty = Some(upload_text(
            renderer,
            font,
            "game_menu_activity_log_empty",
            activity_log_empty(language).as_ref(),
            16.0,
        )?);
        self.text.activity_log_rows = self
            .activity_log_snapshot
            .iter()
            .rev()
            .enumerate()
            .map(|(index, entry)| {
                Ok(ActivityLogRowText {
                    category: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_activity_category_{index}"),
                        activity_category_label(&entry.category, language).as_ref(),
                        14.0,
                    )?,
                    title: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_activity_title_{index}"),
                        &short_activity_text(&entry.title, 28),
                        17.0,
                    )?,
                    detail: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_activity_detail_{index}"),
                        &short_activity_text(&entry.detail, 58),
                        14.0,
                    )?,
                    meta: upload_text(
                        renderer,
                        font,
                        &format!("game_menu_activity_meta_{index}"),
                        &activity_log_meta(entry, language),
                        13.0,
                    )?,
                    category_key: entry.category.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.settings_language = Some(upload_text(
            renderer,
            font,
            "game_menu_settings_language",
            language_setting_label(language).as_ref(),
            22.0,
        )?);
        self.text.settings_hint = Some(upload_text(
            renderer,
            font,
            "game_menu_settings_hint",
            settings_hint(language).as_ref(),
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
                    language_option_label(*language).as_ref(),
                    22.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.bottom_action_labels = BOTTOM_ACTIONS
            .iter()
            .enumerate()
            .map(|(index, action)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_bottom_action_label_{index}"),
                    action.label.get(language).as_ref(),
                    21.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.bottom_action_sublabels = BOTTOM_ACTIONS
            .iter()
            .enumerate()
            .map(|(index, action)| {
                upload_text(
                    renderer,
                    font,
                    &format!("game_menu_bottom_action_sublabel_{index}"),
                    action.sublabel.get(language).as_ref(),
                    13.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.return_label = Some(upload_text(
            renderer,
            font,
            "game_menu_return_label",
            return_label(language).as_ref(),
            23.0,
        )?);
        self.text.return_sublabel = Some(upload_text(
            renderer,
            font,
            "game_menu_return_sublabel",
            return_sublabel(language).as_ref(),
            13.0,
        )?);
        self.text.placeholder = Some(upload_text(
            renderer,
            font,
            "game_menu_placeholder",
            placeholder_text(language).as_ref(),
            17.0,
        )?);
        if let Some(toast) = &self.toast {
            self.text.toast = Some(upload_text(
                renderer,
                font,
                "game_menu_toast",
                &toast.message,
                match language {
                    Language::Chinese => 20.0,
                    Language::English => 18.0,
                },
            )?);
        }

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
        load_menu_textures(renderer)?;
        inventory_scene::load_inventory_item_icons(renderer)?;

        let font = load_ui_font()?;
        self.upload_textures(renderer, &font)
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        self.update_toast(dt);
        self.sync_codex_snapshot(ctx);
        self.sync_save_snapshot(ctx);

        if self.language != ctx.language {
            self.language = ctx.language;
            self.text = GameMenuText::default();
        }

        let viewport = input.screen_size();
        let layout = MenuLayout::new(viewport);
        if self.active_tab == GameMenuTab::Quests {
            let hovered_log = input
                .cursor_position()
                .is_some_and(|point| screen_point_in_rect(point, activity_log_panel_rect(&layout)));
            if hovered_log {
                let wheel = input.mouse_wheel_y();
                if wheel.abs() > 0.0 {
                    let rows = wheel.abs().ceil() as isize;
                    let visible_rows = activity_log_visible_capacity(
                        activity_log_panel_rect(&layout),
                        layout.scale,
                    );
                    self.scroll_activity_log(if wheel < 0.0 { rows } else { -rows }, visible_rows);
                }
            }
        }
        if let Some(cursor_position) = input.cursor_position() {
            if input.mouse_left_just_pressed() {
                if screen_point_in_rect(cursor_position, layout.return_button()) {
                    return Ok(SceneCommand::Pop);
                }
                for index in 0..BOTTOM_ACTIONS.len() {
                    if screen_point_in_rect(
                        cursor_position,
                        layout.bottom_action(index, BOTTOM_ACTIONS.len()),
                    ) {
                        self.handle_bottom_action(ctx, index);
                        return Ok(SceneCommand::None);
                    }
                }
                for (index, tab) in GameMenuTab::ALL.iter().copied().enumerate() {
                    if screen_point_in_rect(cursor_position, layout.nav_item(index)) {
                        self.set_tab(ctx, tab);
                    }
                }

                if self.active_tab == GameMenuTab::Quests
                    && screen_point_in_rect(
                        cursor_position,
                        activity_log_scrollbar_track_rect(
                            activity_log_panel_rect(&layout),
                            layout.scale,
                        ),
                    )
                {
                    self.set_activity_log_scroll_from_track(cursor_position.y, &layout);
                    return Ok(SceneCommand::None);
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
        } else if self.active_tab == GameMenuTab::Codex {
            if input.just_pressed(Button::Left) {
                self.move_codex_selection(-1, 0);
            }
            if input.just_pressed(Button::Right) {
                self.move_codex_selection(1, 0);
            }
            if input.just_pressed(Button::Up) {
                self.move_codex_selection(0, -1);
            }
            if input.just_pressed(Button::Down) {
                self.move_codex_selection(0, 1);
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

fn load_menu_textures(renderer: &mut dyn Renderer) -> Result<()> {
    for texture in menu_style::TEXTURES {
        if renderer.texture_size(texture.texture_id).is_none() {
            renderer.load_texture(texture.texture_id, Path::new(texture.path))?;
        }
    }

    Ok(())
}

fn nav_icon_texture_id(tab: GameMenuTab) -> &'static str {
    match tab {
        GameMenuTab::Profile => "menu.nav_profile",
        GameMenuTab::Inventory => "menu.nav_inventory",
        GameMenuTab::Codex => "menu.nav_codex",
        GameMenuTab::Map => "menu.nav_map",
        GameMenuTab::Quests => "menu.nav_quests",
        GameMenuTab::Settings => "menu.nav_settings",
    }
}

fn bottom_action_texture_id(index: usize) -> &'static str {
    match index {
        0 => "menu.action_equip",
        1 => "menu.action_skills",
        2 => "menu.action_logs",
        3 => "menu.action_craft",
        4 => "menu.action_comms",
        _ => "menu.action_save",
    }
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

fn codex_thumbnail_texture_id(index: usize) -> &'static str {
    match index {
        0 => "menu.codex_alien_life",
        1 => "menu.codex_relic_tech",
        2 => "menu.codex_star_geography",
        _ => "menu.codex_civilization",
    }
}

fn xp_ratio(profile: &PlayerProfileSave) -> f32 {
    if profile.xp_next == 0 {
        return 0.0;
    }

    profile.xp as f32 / profile.xp_next as f32
}

fn upload_inventory_slot_details(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    language: Language,
    inventory: &InventorySave,
) -> Result<Vec<Option<InventoryDetailText>>> {
    let mut details = Vec::with_capacity(grid::INVENTORY_SLOTS);
    for (index, item) in inventory_scene::inventory_slots(inventory)
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
                    locked_label(language).as_ref(),
                    17.0,
                )?)
            } else {
                None
            },
        }));
    }

    Ok(details)
}

#[derive(Clone, Debug)]
struct CodexSummaryView {
    label: String,
    value: String,
    ratio: f32,
}

fn codex_summary_views(snapshot: &CodexMenuSnapshot, language: Language) -> Vec<CodexSummaryView> {
    let mut by_category = BTreeMap::<String, (usize, usize)>::new();
    for entry in &snapshot.entries {
        let counts = by_category.entry(entry.category.clone()).or_default();
        counts.1 += 1;
        if entry.unlocked {
            counts.0 += 1;
        }
    }

    let mut summaries = by_category
        .into_iter()
        .map(|(category, (unlocked, total))| CodexSummaryView {
            label: category,
            value: format!("{unlocked} / {total}"),
            ratio: unlocked as f32 / total.max(1) as f32,
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| right.ratio.total_cmp(&left.ratio));
    summaries.truncate(4);

    if summaries.is_empty() {
        summaries.push(CodexSummaryView {
            label: match language {
                Language::Chinese => "图鉴数据库".to_owned(),
                Language::English => "Codex Database".to_owned(),
            },
            value: "0 / 0".to_owned(),
            ratio: 0.0,
        });
    }

    summaries
}

fn upload_codex_entry_cards(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    language: Language,
    snapshot: &CodexMenuSnapshot,
) -> Result<Vec<CodexEntryCardText>> {
    snapshot
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let title = if entry.unlocked {
                entry.title.as_str()
            } else {
                locked_codex_title(language)
            };
            let description = if entry.unlocked {
                non_empty_or(&entry.description, codex_empty_description(language))
            } else {
                locked_codex_description(language)
            };
            let description_lines = wrap_text(description, 54, 2);

            Ok(CodexEntryCardText {
                title: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_codex_entry_title_{index}"),
                    title,
                    19.0,
                )?,
                category: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_codex_entry_category_{index}"),
                    &entry.category,
                    14.0,
                )?,
                status: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_codex_entry_status_{index}"),
                    codex_status_label(language, entry.unlocked),
                    14.0,
                )?,
                description_lines: description_lines
                    .iter()
                    .enumerate()
                    .map(|(line_index, line)| {
                        upload_text(
                            renderer,
                            font,
                            &format!("game_menu_codex_entry_desc_{index}_{line_index}"),
                            line,
                            13.0,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?,
                unlocked: entry.unlocked,
            })
        })
        .collect()
}

fn locked_codex_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "未识别条目",
        Language::English => "Undiscovered Entry",
    }
}

fn codex_status_label(language: Language, unlocked: bool) -> &'static str {
    match (language, unlocked) {
        (Language::Chinese, true) => "已解锁",
        (Language::Chinese, false) => "未扫描",
        (Language::English, true) => "Unlocked",
        (Language::English, false) => "Locked",
    }
}

fn locked_codex_description(language: Language) -> &'static str {
    match language {
        Language::Chinese => "靠近目标并完成扫描后显示完整记录。",
        Language::English => "Scan the target to unlock the full field record.",
    }
}

fn codex_empty_description(language: Language) -> &'static str {
    match language {
        Language::Chinese => "该条目还没有正文记录。",
        Language::English => "No field note has been written for this entry.",
    }
}

fn wrap_text(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let next_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if next_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
            if lines.len() == max_lines {
                break;
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn non_empty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let value = value.trim();
    if value.is_empty() { fallback } else { value }
}

fn short_activity_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_owned();
    }

    let mut shortened = trimmed
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    shortened.push_str("...");
    shortened
}

fn activity_log_meta(entry: &ActivityLogEntrySave, language: Language) -> String {
    let scene = if entry.scene.trim().is_empty() {
        "-"
    } else {
        entry.scene.as_str()
    };
    let prefix = match language {
        Language::Chinese => "记录",
        Language::English => "Log",
    };
    format!("{prefix} #{:03} · {scene}", entry.sequence)
}

fn activity_category_color(category: &str) -> Color {
    match category {
        "pickup" => Color::rgba(0.06, 0.34, 0.22, 0.88),
        "scan" => Color::rgba(0.07, 0.28, 0.42, 0.88),
        "unlock" => Color::rgba(0.30, 0.20, 0.48, 0.88),
        "status" => Color::rgba(0.42, 0.24, 0.08, 0.88),
        "objective" => Color::rgba(0.04, 0.36, 0.34, 0.88),
        _ => Color::rgba(0.16, 0.24, 0.28, 0.88),
    }
}

fn draw_crystal_glyph(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, scale: f32) {
    let colors = [
        Color::rgba(0.26, 0.88, 1.0, 0.96),
        Color::rgba(0.68, 0.46, 1.0, 0.86),
        Color::rgba(0.80, 1.0, 1.0, 0.82),
    ];
    for (index, color) in colors.iter().copied().enumerate() {
        let w = (10.0 + index as f32 * 4.0) * scale;
        let h = (30.0 + index as f32 * 9.0) * scale;
        let x = rect.origin.x + rect.size.x * (0.32 + index as f32 * 0.16);
        let y = rect.bottom() - h - 8.0 * scale;
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(Vec2::new(x, y), Vec2::new(w, h)),
            color,
        );
    }
}

fn draw_resource_marker(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    center: Vec2,
    texture_id: &str,
    color: Color,
    scale: f32,
) {
    if let Some(image_size) = renderer.texture_size(texture_id) {
        let frame = Rect::new(
            Vec2::new(center.x - 15.0 * scale, center.y - 15.0 * scale),
            Vec2::new(30.0 * scale, 30.0 * scale),
        );
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(frame, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(center.x - 7.0 * scale, center.y - 7.0 * scale),
            Vec2::new(14.0 * scale, 14.0 * scale),
        ),
        color,
    );
    draw_border(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(center.x - 10.0 * scale, center.y - 10.0 * scale),
            Vec2::new(20.0 * scale, 20.0 * scale),
        ),
        1.0 * scale,
        Color::rgba(color.r, color.g, color.b, 0.48),
    );
}

fn draw_header_text_group(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    panel: Rect,
    label: &TextSprite,
    value: &TextSprite,
    value_color: Color,
    scale: f32,
) {
    let x = panel.origin.x + 24.0 * scale;
    let text_gap = 1.0 * scale;
    let group_height = label.size.y + text_gap + value.size.y;
    let group_top = panel.origin.y + (panel.size.y - group_height) * 0.5 - 1.0 * scale;
    draw_text(renderer, label, viewport, x, group_top, color::TEXT_MUTED);
    draw_text_strong(
        renderer,
        value,
        viewport,
        x,
        group_top + label.size.y + text_gap,
        value_color,
        scale,
    );
}

fn draw_two_line_menu_text(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    rect: Rect,
    label: &TextSprite,
    sublabel: &TextSprite,
    label_color: Color,
    sublabel_color: Color,
    scale: f32,
    y_offset: f32,
    strong_shadow: bool,
) {
    let gap = 6.0 * scale;
    let text_padding = 8.0;
    let label_visual_height = (label.size.y - text_padding * 2.0).max(0.0);
    let sublabel_visual_height = (sublabel.size.y - text_padding * 2.0).max(0.0);
    let group_height = label_visual_height + gap + sublabel_visual_height;
    let visual_top = rect.origin.y + (rect.size.y - group_height) * 0.5 + y_offset;
    let label_y = visual_top - text_padding;
    let sublabel_y = visual_top + label_visual_height + gap - text_padding;
    let sublabel_x = rect.origin.x + 2.0 * scale;
    if strong_shadow {
        draw_text_strong(
            renderer,
            label,
            viewport,
            rect.origin.x,
            label_y,
            label_color,
            scale,
        );
    } else {
        draw_text(
            renderer,
            label,
            viewport,
            rect.origin.x,
            label_y,
            label_color,
        );
    }
    draw_text(
        renderer,
        sublabel,
        viewport,
        sublabel_x,
        sublabel_y,
        sublabel_color,
    );
}

fn centered_text_y(rect: Rect, text: &TextSprite, y_offset: f32) -> f32 {
    let text_padding = 8.0;
    let visual_height = (text.size.y - text_padding * 2.0).max(0.0);
    rect.origin.y + (rect.size.y - visual_height) * 0.5 + y_offset - text_padding
}

fn activity_objective_panel_rect(layout: &MenuLayout) -> Rect {
    let content = layout.content_body();
    Rect::new(
        content.origin,
        Vec2::new(content.size.x, 166.0 * layout.scale),
    )
}

fn activity_log_panel_rect(layout: &MenuLayout) -> Rect {
    let objective_panel = activity_objective_panel_rect(layout);
    let top = objective_panel.bottom() + 18.0 * layout.scale;
    let bottom = layout.bottom.origin.y - 12.0 * layout.scale;
    Rect::new(
        Vec2::new(layout.content_body().origin.x, top),
        Vec2::new(layout.content_body().size.x, (bottom - top).max(0.0)),
    )
}

fn activity_log_row_area_rect(log_panel: Rect, scale: f32) -> Rect {
    let top = log_panel.origin.y + ACTIVITY_LOG_HEADER_HEIGHT * scale;
    let bottom = log_panel.bottom() - ACTIVITY_LOG_BOTTOM_PADDING * scale;
    Rect::new(
        Vec2::new(log_panel.origin.x + 18.0 * scale, top),
        Vec2::new(
            (log_panel.size.x - 36.0 * scale).max(0.0),
            (bottom - top).max(0.0),
        ),
    )
}

fn activity_log_visible_capacity(log_panel: Rect, scale: f32) -> usize {
    let rows = activity_log_row_area_rect(log_panel, scale);
    let row_height = ACTIVITY_LOG_ROW_HEIGHT * scale;
    let row_gap = ACTIVITY_LOG_ROW_GAP * scale;
    if rows.size.y < row_height {
        return 1;
    }

    ((rows.size.y + row_gap) / (row_height + row_gap))
        .floor()
        .max(1.0) as usize
}

fn activity_log_visible_range(
    total_rows: usize,
    scroll: usize,
    visible_rows: usize,
) -> Range<usize> {
    let visible_rows = visible_rows.max(1);
    let start = scroll.min(total_rows.saturating_sub(visible_rows));
    let end = (start + visible_rows).min(total_rows);
    start..end
}

fn activity_log_scrollbar_track_rect(log_panel: Rect, scale: f32) -> Rect {
    Rect::new(
        Vec2::new(
            log_panel.right() - 18.0 * scale,
            log_panel.origin.y + 58.0 * scale,
        ),
        Vec2::new(
            4.0 * scale,
            (log_panel.size.y - 76.0 * scale).max(24.0 * scale),
        ),
    )
}

fn draw_activity_log_scrollbar(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    log_panel: Rect,
    total_rows: usize,
    scroll: usize,
    visible_rows: usize,
    scale: f32,
) {
    let track = activity_log_scrollbar_track_rect(log_panel, scale);
    draw_screen_rect(
        renderer,
        viewport,
        track,
        Color::rgba(0.035, 0.082, 0.096, 0.86),
    );

    let visible_ratio = (visible_rows.max(1) as f32 / total_rows.max(1) as f32).min(1.0);
    let thumb_height = (track.size.y * visible_ratio).max(24.0 * scale);
    let max_scroll = total_rows.saturating_sub(visible_rows.max(1));
    let thumb_travel = (track.size.y - thumb_height).max(0.0);
    let scroll_ratio = if max_scroll == 0 {
        0.0
    } else {
        scroll.min(max_scroll) as f32 / max_scroll as f32
    };
    let thumb = Rect::new(
        Vec2::new(track.origin.x, track.origin.y + thumb_travel * scroll_ratio),
        Vec2::new(track.size.x, thumb_height),
    );
    draw_screen_rect(
        renderer,
        viewport,
        thumb,
        Color::rgba(0.40, 0.94, 1.0, 0.92),
    );
}

fn game_menu_toast_rect(layout: &MenuLayout) -> Rect {
    let width = (MENU_TOAST_WIDTH * layout.scale).min(layout.root.size.x - 48.0 * layout.scale);
    Rect::new(
        Vec2::new(
            layout.root.origin.x + (layout.root.size.x - width) * 0.5,
            layout.bottom.origin.y - (MENU_TOAST_HEIGHT + 12.0) * layout.scale,
        ),
        Vec2::new(width, MENU_TOAST_HEIGHT * layout.scale),
    )
}

fn game_menu_toast_fill(tone: GameMenuToastTone) -> Color {
    match tone {
        GameMenuToastTone::Info => Color::rgba(0.015, 0.052, 0.066, 0.94),
        GameMenuToastTone::Success => Color::rgba(0.018, 0.090, 0.062, 0.95),
        GameMenuToastTone::Error => Color::rgba(0.126, 0.032, 0.034, 0.95),
    }
}

fn game_menu_toast_border(tone: GameMenuToastTone) -> Color {
    match tone {
        GameMenuToastTone::Info => Color::rgba(0.34, 0.90, 1.0, 0.88),
        GameMenuToastTone::Success => Color::rgba(0.42, 1.0, 0.72, 0.90),
        GameMenuToastTone::Error => Color::rgba(1.0, 0.42, 0.42, 0.92),
    }
}

fn game_menu_toast_text(tone: GameMenuToastTone) -> Color {
    match tone {
        GameMenuToastTone::Info => Color::rgba(0.82, 0.96, 1.0, 1.0),
        GameMenuToastTone::Success => Color::rgba(0.82, 1.0, 0.90, 1.0),
        GameMenuToastTone::Error => Color::rgba(1.0, 0.82, 0.82, 1.0),
    }
}

fn game_menu_tab_jump_message(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已切到背包页",
        Language::English => "Inventory page opened",
    }
}

fn game_menu_log_jump_message(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已切到外勤日志",
        Language::English => "Field log opened",
    }
}

fn game_menu_action_pending_message(language: Language) -> &'static str {
    match language {
        Language::Chinese => "这个动作还在接入中",
        Language::English => "This action is still being wired in",
    }
}

fn game_menu_save_success_message(language: Language, save_path: &Path) -> String {
    let target = save_target_label(language, save_path);
    match language {
        Language::Chinese => format!("已手动保存到 {target}"),
        Language::English => format!("Saved manually to {target}"),
    }
}

fn game_menu_save_error_message(language: Language, detail: &str) -> String {
    match language {
        Language::Chinese => format!("手动保存失败：{}", short_menu_error_detail(detail)),
        Language::English => format!("Manual save failed: {}", short_menu_error_detail(detail)),
    }
}

fn save_target_label(language: Language, save_path: &Path) -> String {
    if let Some(slot_index) = save_slot_index_from_path(save_path) {
        return match language {
            Language::Chinese => format!("槽位 {}", slot_index + 1),
            Language::English => format!("Slot {}", slot_index + 1),
        };
    }

    save_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("save")
        .to_owned()
}

fn save_slot_index_from_path(save_path: &Path) -> Option<usize> {
    let stem = save_path.file_stem()?.to_str()?;
    let number = stem.strip_prefix("profile_")?.parse::<usize>().ok()?;
    number.checked_sub(1)
}

fn short_menu_error_detail(detail: &str) -> &str {
    const MAX_CHARS: usize = 42;
    let trimmed = detail.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed;
    }

    let cut = trimmed
        .char_indices()
        .nth(MAX_CHARS)
        .map_or(trimmed.len(), |(index, _)| index);
    &trimmed[..cut]
}

fn draw_header_resources(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    panel: Rect,
    crystals: &TextSprite,
    credits: &TextSprite,
    scale: f32,
) {
    let icon_size = 30.0 * scale;
    let icon_gap = 5.0 * scale;
    let group_gap = 24.0 * scale;
    let crystal_width = icon_size + icon_gap + crystals.size.x;
    let credit_width = icon_size + icon_gap + credits.size.x;
    let total_width = crystal_width + group_gap + credit_width;
    let min_start_x = panel.origin.x + 22.0 * scale;
    let start_x = (panel.origin.x + (panel.size.x - total_width) * 0.5).max(min_start_x);
    let center_y = panel.origin.y + panel.size.y * 0.5;
    let text_y = center_y - crystals.size.y * 0.5;

    draw_header_resource_group(
        renderer,
        viewport,
        Vec2::new(start_x, center_y),
        icon::RESOURCE_CRYSTAL,
        Color::rgba(0.24, 0.78, 1.0, 0.96),
        crystals,
        text_y,
        scale,
    );
    draw_header_resource_group(
        renderer,
        viewport,
        Vec2::new(start_x + crystal_width + group_gap, center_y),
        icon::RESOURCE_COIN,
        Color::rgba(0.95, 0.67, 0.22, 0.96),
        credits,
        text_y,
        scale,
    );
}

fn draw_header_resource_group(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    origin: Vec2,
    texture_id: &str,
    icon_color: Color,
    value: &TextSprite,
    text_y: f32,
    scale: f32,
) {
    let icon_size = 30.0 * scale;
    let icon_gap = 5.0 * scale;
    draw_resource_marker(
        renderer,
        viewport,
        Vec2::new(origin.x + icon_size * 0.5, origin.y),
        texture_id,
        icon_color,
        scale,
    );
    draw_text(
        renderer,
        value,
        viewport,
        origin.x + icon_size + icon_gap,
        text_y,
        Color::rgba(0.88, 1.0, 0.98, 1.0),
    );
}

fn draw_attribute_icon(
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

fn draw_status_card(
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

fn draw_inventory_slot(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    slot: Rect,
    item: Option<inventory_scene::InventoryItemView>,
    count: Option<&TextSprite>,
    selected: bool,
    scale: f32,
) {
    draw_screen_rect(
        renderer,
        viewport,
        slot,
        if selected {
            Color::rgba(0.050, 0.18, 0.22, 0.68)
        } else if item.is_some() {
            Color::rgba(0.020, 0.060, 0.074, 0.46)
        } else {
            Color::rgba(0.012, 0.026, 0.034, 0.34)
        },
    );
    draw_border(
        renderer,
        viewport,
        slot,
        1.0 * scale,
        if selected {
            Color::rgba(0.54, 0.94, 1.0, 0.76)
        } else if item.is_some() {
            Color::rgba(0.17, 0.44, 0.52, 0.54)
        } else {
            Color::rgba(0.10, 0.23, 0.29, 0.40)
        },
    );
    if let Some(item) = item {
        renderer.draw_image(
            item.texture_id,
            screen_rect(viewport, inset_rect(slot, 9.0 * scale)),
            Color::rgba(1.0, 1.0, 1.0, if item.locked { 0.72 } else { 1.0 }),
        );
        if let Some(count) = count {
            draw_text(
                renderer,
                count,
                viewport,
                slot.right() - count.size.x - 5.0 * scale,
                slot.bottom() - count.size.y + 4.0 * scale,
                Color::rgba(0.92, 1.0, 0.98, 1.0),
            );
        }
    }
}

fn draw_codex_entry_card(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    entry: &CodexEntryCardText,
    selected: bool,
    index: usize,
    scale: f32,
) {
    draw_inner_panel(renderer, viewport, card, scale);
    if selected {
        draw_border(
            renderer,
            viewport,
            card,
            2.0 * scale,
            Color::rgba(0.58, 0.98, 1.0, 0.96),
        );
    }

    let image_size = 58.0 * scale;
    let image_rect = Rect::new(
        Vec2::new(
            card.right() - image_size - 16.0 * scale,
            card.origin.y + 14.0 * scale,
        ),
        Vec2::new(image_size, image_size),
    );
    draw_codex_glyph(renderer, viewport, image_rect, index, scale);
    if !entry.unlocked {
        draw_screen_rect(
            renderer,
            viewport,
            image_rect,
            Color::rgba(0.0, 0.0, 0.0, 0.46),
        );
    }

    let text_x = card.origin.x + 18.0 * scale;
    let title_color = if entry.unlocked {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_MUTED
    };
    draw_text_strong(
        renderer,
        &entry.title,
        viewport,
        text_x,
        card.origin.y + 10.0 * scale,
        title_color,
        scale,
    );
    draw_text(
        renderer,
        &entry.category,
        viewport,
        text_x,
        card.origin.y + 34.0 * scale,
        Color::rgba(0.46, 0.88, 0.96, 0.96),
    );
    draw_text(
        renderer,
        &entry.status,
        viewport,
        card.right() - image_size - 16.0 * scale,
        image_rect.bottom() + 4.0 * scale,
        if entry.unlocked {
            color::TEXT_GREEN
        } else {
            color::TEXT_DIM
        },
    );

    for (line_index, line) in entry.description_lines.iter().enumerate() {
        draw_text(
            renderer,
            line,
            viewport,
            text_x,
            card.origin.y + (56.0 + line_index as f32 * 17.0) * scale,
            if entry.unlocked {
                color::TEXT_SECONDARY
            } else {
                color::TEXT_DIM
            },
        );
    }
}

fn draw_codex_discovery_card(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    index: usize,
    label: &TextSprite,
    value: &TextSprite,
    ratio: f32,
    scale: f32,
) {
    let content = inset_rect(card, 4.0 * scale);
    let image = Rect::new(
        content.origin,
        Vec2::new(content.size.x, content.size.y - 84.0 * scale),
    );
    let info_top = image.bottom() + 2.0 * scale;
    draw_codex_glyph(renderer, viewport, image, index, scale);
    draw_text_centered(
        renderer,
        label,
        viewport,
        content.origin.x + content.size.x * 0.5,
        info_top,
        color::TEXT_PRIMARY,
    );
    draw_text_centered(
        renderer,
        value,
        viewport,
        content.origin.x + content.size.x * 0.5,
        info_top + 25.0 * scale,
        color::TEXT_SECONDARY,
    );
    draw_bar(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(content.origin.x + 10.0 * scale, info_top + 49.0 * scale),
            Vec2::new(content.size.x - 20.0 * scale, 6.0 * scale),
        ),
        ratio,
        scale,
    );
}

fn draw_nav_icon(
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

fn draw_bottom_icon(
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

fn draw_return_icon(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, scale: f32) {
    if let Some(image_size) = renderer.texture_size("menu.action_return") {
        let frame = Rect::new(
            Vec2::new(rect.origin.x + 32.0 * scale, rect.origin.y + 15.0 * scale),
            Vec2::new(42.0 * scale, 42.0 * scale),
        );
        renderer.draw_image(
            "menu.action_return",
            screen_rect(viewport, contain_rect(frame, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    let color = Color::rgba(1.0, 0.72, 0.30, 0.96);
    let origin = Vec2::new(rect.origin.x + 36.0 * scale, rect.origin.y + 18.0 * scale);
    draw_border(
        renderer,
        viewport,
        Rect::new(origin, Vec2::new(30.0 * scale, 30.0 * scale)),
        3.0 * scale,
        color,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(origin.x - 8.0 * scale, origin.y + 13.0 * scale),
            Vec2::new(26.0 * scale, 5.0 * scale),
        ),
        color,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(origin.x - 8.0 * scale, origin.y + 8.0 * scale),
            Vec2::new(9.0 * scale, 5.0 * scale),
        ),
        color,
    );
    draw_screen_rect(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(origin.x - 8.0 * scale, origin.y + 18.0 * scale),
            Vec2::new(9.0 * scale, 5.0 * scale),
        ),
        color,
    );
}

fn draw_compact_stat_bar(
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

fn draw_codex_glyph(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    index: usize,
    scale: f32,
) {
    let texture_id = codex_thumbnail_texture_id(index);
    if let Some(image_size) = renderer.texture_size(texture_id) {
        let frame = inset_rect(card, 2.0 * scale);
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(frame, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    let color = match index {
        0 => Color::rgba(0.56, 0.42, 0.95, 0.92),
        1 => Color::rgba(0.34, 0.88, 1.0, 0.92),
        2 => Color::rgba(0.58, 0.64, 1.0, 0.92),
        _ => Color::rgba(0.36, 0.92, 1.0, 0.92),
    };

    if card.size.y < 160.0 * scale && card.size.x > 240.0 * scale {
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
        return;
    }

    let center = Vec2::new(
        card.origin.x + card.size.x * 0.5,
        card.origin.y + card.size.y * 0.5,
    );
    match index {
        0 => {
            let body = Rect::new(
                Vec2::new(center.x - 26.0 * scale, center.y - 20.0 * scale),
                Vec2::new(52.0 * scale, 34.0 * scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                body,
                Color::rgba(0.28, 0.19, 0.42, 0.95),
            );
            draw_border(renderer, viewport, body, 2.0 * scale, color);
            for eye in [-1.0, 1.0] {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(center.x + eye * 11.0 * scale, center.y - 8.0 * scale),
                        Vec2::new(6.0 * scale, 6.0 * scale),
                    ),
                    Color::rgba(0.70, 0.98, 1.0, 0.96),
                );
            }
            for leg in 0..4 {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - 28.0 * scale + leg as f32 * 18.0 * scale,
                            center.y + 16.0 * scale,
                        ),
                        Vec2::new(7.0 * scale, 20.0 * scale),
                    ),
                    color,
                );
            }
        }
        1 => {
            for tier in 0..4 {
                let width = (64.0 - tier as f32 * 12.0) * scale;
                let height = (18.0 + tier as f32 * 8.0) * scale;
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - width * 0.5,
                            center.y + 38.0 * scale - tier as f32 * 25.0 * scale,
                        ),
                        Vec2::new(width, height),
                    ),
                    Color::rgba(0.10, 0.32, 0.42, 0.92),
                );
                draw_border(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - width * 0.5,
                            center.y + 38.0 * scale - tier as f32 * 25.0 * scale,
                        ),
                        Vec2::new(width, height),
                    ),
                    1.0 * scale,
                    color,
                );
            }
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(center.x - 5.0 * scale, center.y - 62.0 * scale),
                    Vec2::new(10.0 * scale, 28.0 * scale),
                ),
                color,
            );
        }
        2 => {
            for band in 0..5 {
                let width = (78.0 - (band as f32 - 2.0).abs() * 13.0) * scale;
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - width * 0.5,
                            center.y - 34.0 * scale + band as f32 * 14.0 * scale,
                        ),
                        Vec2::new(width, 10.0 * scale),
                    ),
                    if band == 2 {
                        Color::rgba(0.34, 0.88, 1.0, 0.92)
                    } else {
                        Color::rgba(0.24, 0.22, 0.48, 0.86)
                    },
                );
            }
            draw_border(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(center.x - 48.0 * scale, center.y - 42.0 * scale),
                    Vec2::new(96.0 * scale, 84.0 * scale),
                ),
                1.0 * scale,
                color,
            );
        }
        _ => {
            let tablet = Rect::new(
                Vec2::new(center.x - 34.0 * scale, center.y - 48.0 * scale),
                Vec2::new(68.0 * scale, 96.0 * scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                tablet,
                Color::rgba(0.06, 0.20, 0.28, 0.94),
            );
            draw_border(renderer, viewport, tablet, 2.0 * scale, color);
            for row in 0..4 {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            tablet.origin.x + 14.0 * scale,
                            tablet.origin.y + (18.0 + row as f32 * 16.0) * scale,
                        ),
                        Vec2::new(40.0 * scale, 3.0 * scale),
                    ),
                    Color::rgba(0.55, 0.95, 1.0, 0.78),
                );
            }
        }
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
    fn game_menu_tab_labels_exist_for_supported_languages() {
        for language in Language::SUPPORTED {
            assert!(!menu_status(language).is_empty());
            assert!(!close_hint(language).is_empty());
            assert!(!activity_log_header(language).is_empty());
            assert!(!activity_log_empty(language).is_empty());
            assert!(!activity_category_label("scan", language).is_empty());
            assert!(!language_setting_label(language).is_empty());
            assert!(!settings_hint(language).is_empty());
            assert!(!top_location_value(language).is_empty());
            assert!(!top_status_value(language).is_empty());
            assert!(!codex_discoveries_title(language).is_empty());

            for tab in GameMenuTab::ALL {
                assert!(!tab_label(tab, language).is_empty());
                assert!(!tab_sublabel(tab, language).is_empty());
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
        assert!(layout.bottom.origin.y > layout.content.bottom());
        assert!(layout.bottom.bottom() <= layout.root.bottom());
        assert!(layout.return_button().right() <= layout.bottom.right());
    }

    #[test]
    fn game_menu_save_feedback_names_fixed_slots() {
        assert_eq!(
            save_target_label(Language::Chinese, Path::new("saves/profile_02.ron")),
            "槽位 2"
        );
        assert!(
            game_menu_save_success_message(Language::English, Path::new("saves/profile_03.ron"))
                .contains("Slot 3")
        );
        assert!(game_menu_save_error_message(Language::Chinese, "disk is full").contains("失败"));
    }

    #[test]
    fn activity_log_visible_range_clamps_to_scrollable_window() {
        assert_eq!(activity_log_visible_range(3, 0, 5), 0..3);
        assert_eq!(activity_log_visible_range(12, 0, 3), 0..3);
        assert_eq!(activity_log_visible_range(12, 4, 3), 4..7);
        assert_eq!(activity_log_visible_range(12, 99, 3), 9..12);
    }

    #[test]
    fn activity_log_capacity_fits_only_whole_rows_inside_panel() {
        let panel = Rect::new(Vec2::ZERO, Vec2::new(500.0, 235.0));

        assert_eq!(activity_log_visible_capacity(panel, 1.0), 2);
    }

    #[test]
    fn activity_log_rows_stay_inside_frame_at_screenshot_size() {
        let layout = MenuLayout::new(Vec2::new(1534.0, 800.0));
        let log_panel = activity_log_panel_rect(&layout);
        let row_area = activity_log_row_area_rect(log_panel, layout.scale);
        let visible_rows = activity_log_visible_capacity(log_panel, layout.scale);
        let row_height = ACTIVITY_LOG_ROW_HEIGHT * layout.scale;
        let row_gap = ACTIVITY_LOG_ROW_GAP * layout.scale;
        let last_row_bottom = row_area.origin.y
            + visible_rows as f32 * row_height
            + visible_rows.saturating_sub(1) as f32 * row_gap;

        assert!(visible_rows > 0);
        assert!(last_row_bottom <= log_panel.bottom() - ACTIVITY_LOG_BOTTOM_PADDING * layout.scale);
        assert!(log_panel.bottom() < layout.bottom.origin.y);
    }

    #[test]
    fn game_menu_nav_items_fit_inside_sidebar() {
        let layout = MenuLayout::new(Vec2::new(1600.0, 900.0));
        let last = layout.nav_item(GameMenuTab::ALL.len() - 1);

        assert!(last.bottom() <= layout.nav.bottom());
    }

    #[test]
    fn game_menu_reads_shared_profile_and_inventory_views() {
        let save = crate::save::SaveData::default();
        let profile = profile_scene::profile_overview(Language::Chinese, &save.profile);
        let inventory_slots = inventory_scene::inventory_slots(&save.inventory);

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
    fn game_menu_texture_paths_exist() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for texture in menu_style::TEXTURES {
            assert!(
                project_root.join(texture.path).exists(),
                "{} should exist",
                texture.path
            );
        }
    }

    #[test]
    fn inventory_menu_selection_moves_inside_grid_bounds() {
        assert_eq!(move_inventory_slot(0, -1, 0), 0);
        assert_eq!(move_inventory_slot(0, 1, 0), 1);
        assert_eq!(move_inventory_slot(0, 0, 1), grid::INVENTORY_COLUMNS);
        assert_eq!(
            move_inventory_slot(grid::INVENTORY_SLOTS - 1, 1, 0),
            grid::INVENTORY_SLOTS - 1
        );
        assert_eq!(
            move_inventory_slot(grid::INVENTORY_SLOTS - 1, 0, 1),
            grid::INVENTORY_SLOTS - 1
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

    #[test]
    fn codex_snapshot_tracks_unlocked_entries() {
        let mut database = content::CodexDatabase::new("Overworld");
        database.entries.push(content::CodexEntry {
            id: "codex.test.flora".to_owned(),
            category: "Flora".to_owned(),
            title: "Test Flora".to_owned(),
            description: "A plant used by the menu tests.".to_owned(),
            scan_time: Some(1.25),
            unlock_tags: vec!["flora".to_owned()],
            image: None,
        });
        database.entries.push(content::CodexEntry {
            id: "codex.test.ruin".to_owned(),
            category: "Ruins".to_owned(),
            title: "Test Ruin".to_owned(),
            description: "A ruin used by the menu tests.".to_owned(),
            scan_time: Some(1.25),
            unlock_tags: vec!["ruin".to_owned()],
            image: None,
        });
        database.reindex();

        let mut ctx = GameContext {
            codex_database: database,
            ..GameContext::default()
        };
        ctx.scanned_codex_ids.insert("codex.test.flora".to_owned());

        let snapshot = CodexMenuSnapshot::from_context(&ctx);
        let summaries = codex_summary_views(&snapshot, Language::English);

        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(snapshot.unlocked_count(), 1);
        assert!(snapshot.entries[0].unlocked);
        assert!(summaries.iter().any(|summary| summary.label == "Flora"));
    }
}
