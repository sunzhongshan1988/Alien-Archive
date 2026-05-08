use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::Font;

use crate::save::{InventorySave, ItemStackSave};
use crate::ui::text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text};

use super::{GameContext, Language, RenderContext, Scene, SceneId};

const BACKPACK_COLUMNS: usize = 6;
const BACKPACK_ROWS: usize = 4;
const BACKPACK_SLOTS: usize = BACKPACK_COLUMNS * BACKPACK_ROWS;
const QUICKBAR_SLOTS: usize = 6;
const SLOT_SIZE: f32 = 56.0;
const SLOT_GAP: f32 = 10.0;
const QUICKBAR_SLOT_SIZE: f32 = 44.0;
const QUICKBAR_SLOT_GAP: f32 = 12.0;
const INVENTORY_WIDTH: f32 = 1008.0;
const INVENTORY_HEIGHT: f32 = 550.0;
const HEADER_HEIGHT: f32 = 70.0;
const OUTER_PADDING: f32 = 24.0;
const CONTENT_GAP: f32 = 18.0;
const CATEGORY_PANEL_SIZE: Vec2 = Vec2::new(166.0, 350.0);
const BACKPACK_PANEL_SIZE: Vec2 = Vec2::new(456.0, 350.0);
const DETAILS_PANEL_SIZE: Vec2 = Vec2::new(302.0, 350.0);
const QUICKBAR_PANEL_SIZE: Vec2 = Vec2::new(776.0, 58.0);

const UI_TEXTURES: &[(&str, &str)] = &[
    (
        "ui_inventory_slot_empty",
        "assets/images/ui/inventory/slot_empty.png",
    ),
    (
        "ui_inventory_slot_selected",
        "assets/images/ui/inventory/slot_selected.png",
    ),
    (
        "ui_inventory_slot_quickbar",
        "assets/images/ui/inventory/slot_quickbar.png",
    ),
    (
        "ui_inventory_slot_locked",
        "assets/images/ui/inventory/slot_locked.png",
    ),
    (
        "ui_inventory_panel_backpack",
        "assets/images/ui/inventory/panel_backpack.png",
    ),
    (
        "ui_inventory_panel_details",
        "assets/images/ui/inventory/panel_details.png",
    ),
    (
        "ui_inventory_panel_quickbar",
        "assets/images/ui/inventory/panel_quickbar.png",
    ),
    (
        "ui_inventory_tab_active",
        "assets/images/ui/inventory/tab_active.png",
    ),
    (
        "ui_inventory_tab_inactive",
        "assets/images/ui/inventory/tab_inactive.png",
    ),
    (
        "ui_inventory_badge_count",
        "assets/images/ui/inventory/badge_count.png",
    ),
    (
        "ui_inventory_rarity_common",
        "assets/images/ui/inventory/rarity_common.png",
    ),
    (
        "ui_inventory_rarity_uncommon",
        "assets/images/ui/inventory/rarity_uncommon.png",
    ),
    (
        "ui_inventory_rarity_rare",
        "assets/images/ui/inventory/rarity_rare.png",
    ),
    (
        "ui_inventory_rarity_artifact",
        "assets/images/ui/inventory/rarity_artifact.png",
    ),
    (
        "ui_inventory_cat_samples",
        "assets/images/ui/inventory/cat_samples.png",
    ),
    (
        "ui_inventory_cat_components",
        "assets/images/ui/inventory/cat_components.png",
    ),
    (
        "ui_inventory_cat_artifacts",
        "assets/images/ui/inventory/cat_artifacts.png",
    ),
    (
        "ui_inventory_cat_consumables",
        "assets/images/ui/inventory/cat_consumables.png",
    ),
    (
        "ui_inventory_cat_tools",
        "assets/images/ui/inventory/cat_tools.png",
    ),
    (
        "ui_inventory_cat_data",
        "assets/images/ui/inventory/cat_data.png",
    ),
];

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

const ITEM_DEFINITIONS: &[ItemDefinition] = &[
    ItemDefinition {
        id: "alien_crystal_sample",
        name: LocalizedText::new("Crystal Sample", "晶体样本"),
        category: ItemCategory::Samples,
        texture_id: "item_alien_crystal_sample",
        icon_path: "assets/images/ui/inventory/items/item_alien_crystal_sample.png",
        max_stack: 10,
        weight: 2,
        rarity: Rarity::Rare,
        research: 42,
    },
    ItemDefinition {
        id: "bio_sample_vial",
        name: LocalizedText::new("Bio Sample", "生物样本"),
        category: ItemCategory::Samples,
        texture_id: "item_bio_sample_vial",
        icon_path: "assets/images/ui/inventory/items/item_bio_sample_vial.png",
        max_stack: 10,
        weight: 1,
        rarity: Rarity::Uncommon,
        research: 18,
    },
    ItemDefinition {
        id: "data_shard",
        name: LocalizedText::new("Data Shard", "数据碎片"),
        category: ItemCategory::Data,
        texture_id: "item_data_shard",
        icon_path: "assets/images/ui/inventory/items/item_data_shard.png",
        max_stack: 99,
        weight: 0,
        rarity: Rarity::Rare,
        research: 66,
    },
    ItemDefinition {
        id: "energy_cell",
        name: LocalizedText::new("Energy Cell", "能量电池"),
        category: ItemCategory::Consumables,
        texture_id: "item_energy_cell",
        icon_path: "assets/images/ui/inventory/items/item_energy_cell.png",
        max_stack: 20,
        weight: 1,
        rarity: Rarity::Uncommon,
        research: 35,
    },
    ItemDefinition {
        id: "scrap_part",
        name: LocalizedText::new("Scrap Part", "废料零件"),
        category: ItemCategory::Components,
        texture_id: "item_scrap_part",
        icon_path: "assets/images/ui/inventory/items/item_scrap_part.png",
        max_stack: 99,
        weight: 1,
        rarity: Rarity::Common,
        research: 8,
    },
    ItemDefinition {
        id: "ruin_key",
        name: LocalizedText::new("Ruin Key", "遗迹钥匙"),
        category: ItemCategory::Artifacts,
        texture_id: "item_ruin_key",
        icon_path: "assets/images/ui/inventory/items/item_ruin_key.png",
        max_stack: 1,
        weight: 1,
        rarity: Rarity::Artifact,
        research: 91,
    },
    ItemDefinition {
        id: "scanner_tool",
        name: LocalizedText::new("Scanner", "扫描器"),
        category: ItemCategory::Tools,
        texture_id: "item_scanner_tool",
        icon_path: "assets/images/ui/inventory/items/item_scanner_tool.png",
        max_stack: 1,
        weight: 3,
        rarity: Rarity::Uncommon,
        research: 100,
    },
    ItemDefinition {
        id: "med_injector",
        name: LocalizedText::new("Med Injector", "医疗注射器"),
        category: ItemCategory::Consumables,
        texture_id: "item_med_injector",
        icon_path: "assets/images/ui/inventory/items/item_med_injector.png",
        max_stack: 5,
        weight: 1,
        rarity: Rarity::Common,
        research: 0,
    },
    ItemDefinition {
        id: "coolant_canister",
        name: LocalizedText::new("Coolant Canister", "冷却罐"),
        category: ItemCategory::Consumables,
        texture_id: "item_coolant_canister",
        icon_path: "assets/images/ui/inventory/items/item_coolant_canister.png",
        max_stack: 10,
        weight: 3,
        rarity: Rarity::Uncommon,
        research: 24,
    },
    ItemDefinition {
        id: "metal_fragment",
        name: LocalizedText::new("Metal Fragment", "金属碎片"),
        category: ItemCategory::Components,
        texture_id: "item_metal_fragment",
        icon_path: "assets/images/ui/inventory/items/item_metal_fragment.png",
        max_stack: 99,
        weight: 1,
        rarity: Rarity::Common,
        research: 12,
    },
    ItemDefinition {
        id: "glow_fungus_sample",
        name: LocalizedText::new("Glow Fungus", "发光菌样本"),
        category: ItemCategory::Samples,
        texture_id: "item_glow_fungus_sample",
        icon_path: "assets/images/ui/inventory/items/item_glow_fungus_sample.png",
        max_stack: 10,
        weight: 1,
        rarity: Rarity::Uncommon,
        research: 54,
    },
    ItemDefinition {
        id: "artifact_core",
        name: LocalizedText::new("Artifact Core", "遗物核心"),
        category: ItemCategory::Artifacts,
        texture_id: "item_artifact_core",
        icon_path: "assets/images/ui/inventory/items/item_artifact_core.png",
        max_stack: 1,
        weight: 4,
        rarity: Rarity::Artifact,
        research: 73,
    },
];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ItemCategory {
    Samples,
    Components,
    Artifacts,
    Consumables,
    Tools,
    Data,
}

impl ItemCategory {
    const ALL: [Self; 6] = [
        Self::Samples,
        Self::Components,
        Self::Artifacts,
        Self::Consumables,
        Self::Tools,
        Self::Data,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Samples => "samples",
            Self::Components => "components",
            Self::Artifacts => "artifacts",
            Self::Consumables => "consumables",
            Self::Tools => "tools",
            Self::Data => "data",
        }
    }

    fn from_key(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|category| category.key() == value)
    }

    fn label(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => match self {
                Self::Samples => "样本",
                Self::Components => "组件",
                Self::Artifacts => "遗物",
                Self::Consumables => "消耗品",
                Self::Tools => "工具",
                Self::Data => "数据",
            },
            Language::English => match self {
                Self::Samples => "Samples",
                Self::Components => "Components",
                Self::Artifacts => "Artifacts",
                Self::Consumables => "Consumables",
                Self::Tools => "Tools",
                Self::Data => "Data",
            },
        }
    }

    fn icon_texture_id(self) -> &'static str {
        match self {
            Self::Samples => "ui_inventory_cat_samples",
            Self::Components => "ui_inventory_cat_components",
            Self::Artifacts => "ui_inventory_cat_artifacts",
            Self::Consumables => "ui_inventory_cat_consumables",
            Self::Tools => "ui_inventory_cat_tools",
            Self::Data => "ui_inventory_cat_data",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rarity {
    Common,
    Uncommon,
    Rare,
    Artifact,
}

impl Rarity {
    fn label(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => match self {
                Self::Common => "普通",
                Self::Uncommon => "优秀",
                Self::Rare => "稀有",
                Self::Artifact => "遗物",
            },
            Language::English => match self {
                Self::Common => "Common",
                Self::Uncommon => "Uncommon",
                Self::Rare => "Rare",
                Self::Artifact => "Artifact",
            },
        }
    }
}

#[derive(Clone)]
struct InventoryState {
    slots: Vec<Option<ItemStack>>,
    quickbar: [Option<usize>; QUICKBAR_SLOTS],
    selected_slot: usize,
    active_category: ItemCategory,
}

#[derive(Clone)]
struct ItemStack {
    item_id: &'static str,
    quantity: u32,
    locked: bool,
}

#[derive(Clone, Copy)]
struct ItemDefinition {
    id: &'static str,
    name: LocalizedText,
    category: ItemCategory,
    texture_id: &'static str,
    icon_path: &'static str,
    max_stack: u32,
    weight: u32,
    rarity: Rarity,
    research: u32,
}

#[derive(Clone, Copy)]
pub(super) struct InventoryItemView {
    pub(super) texture_id: &'static str,
    pub(super) quantity: u32,
    pub(super) max_stack: u32,
    pub(super) research: u32,
    pub(super) locked: bool,
    pub(super) rarity_color: Color,
    name_english: &'static str,
    name_chinese: &'static str,
    category_english: &'static str,
    category_chinese: &'static str,
    rarity_english: &'static str,
    rarity_chinese: &'static str,
}

impl InventoryItemView {
    pub(super) fn name(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.name_chinese,
            Language::English => self.name_english,
        }
    }

    pub(super) fn category(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.category_chinese,
            Language::English => self.category_english,
        }
    }

    pub(super) fn rarity(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.rarity_chinese,
            Language::English => self.rarity_english,
        }
    }
}

#[derive(Default)]
struct InventoryText {
    language: Option<Language>,
    title: Option<TextSprite>,
    empty_detail: Option<TextSprite>,
    tab_labels: HashMap<ItemCategory, TextSprite>,
    item_details: HashMap<&'static str, ItemDetailText>,
    count_badges: HashMap<usize, TextSprite>,
    quickbar_numbers: Vec<TextSprite>,
}

struct ItemDetailText {
    name: TextSprite,
    category: TextSprite,
    quantity: TextSprite,
    rarity: TextSprite,
    stack_limit: TextSprite,
    status: TextSprite,
    lock_state: Option<TextSprite>,
}

pub struct InventoryScene {
    language: Language,
    state: InventoryState,
    text: InventoryText,
}

impl InventoryScene {
    pub fn new(ctx: &GameContext) -> Self {
        Self {
            language: ctx.language,
            state: InventoryState::from_save(&ctx.save_data.inventory),
            text: InventoryText::default(),
        }
    }

    fn draw_inventory(&mut self, ctx: &mut RenderContext<'_>) {
        let viewport = ctx.renderer.screen_size();
        let layout = InventoryLayout::new(viewport);

        ctx.renderer.draw_rect(
            screen_rect(viewport, 0.0, 0.0, viewport.x, viewport.y),
            Color::rgba(0.0, 0.0, 0.0, 0.78),
        );

        self.draw_shell(ctx.renderer, viewport, &layout);
        self.draw_category_nav(ctx.renderer, viewport, &layout);
        self.draw_backpack(ctx.renderer, viewport, &layout);
        self.draw_quickbar(ctx.renderer, viewport, &layout);
        self.draw_details(ctx.renderer, viewport, &layout);
    }

    fn draw_shell(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        draw_panel(
            renderer,
            layout.root_panel,
            Color::rgba(0.014, 0.023, 0.031, 0.98),
            Color::rgba(0.25, 0.38, 0.50, 0.82),
            layout.scale,
        );
        draw_corner_brackets(
            renderer,
            layout.root_panel,
            22.0 * layout.scale,
            2.0 * layout.scale,
            Color::rgba(0.28, 0.88, 1.0, 0.95),
        );

        renderer.draw_rect(
            Rect::new(
                layout.header.origin,
                Vec2::new(layout.header.size.x, 1.0 * layout.scale),
            ),
            Color::rgba(0.31, 0.92, 1.0, 0.82),
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.header.origin.x + 22.0 * layout.scale,
                    layout.header.bottom() - 12.0 * layout.scale,
                ),
                Vec2::new(160.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.90, 0.70, 0.32, 0.90),
        );

        if let Some(title) = &self.text.title {
            draw_text(
                renderer,
                title,
                viewport,
                screen_x(viewport, layout.header.origin.x + 24.0 * layout.scale),
                screen_y(viewport, layout.header.origin.y + 16.0 * layout.scale),
                Color::rgba(0.90, 1.0, 0.98, 1.0),
            );
        }
    }

    fn draw_category_nav(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &InventoryLayout,
    ) {
        draw_panel(
            renderer,
            layout.category_panel,
            Color::rgba(0.020, 0.034, 0.043, 0.92),
            Color::rgba(0.20, 0.30, 0.40, 0.75),
            layout.scale,
        );

        for (index, category) in ItemCategory::ALL.iter().copied().enumerate() {
            let rect = layout.category_rect(index);
            let active = category == self.state.active_category;

            renderer.draw_rect(
                rect,
                if active {
                    Color::rgba(0.05, 0.24, 0.29, 0.98)
                } else {
                    Color::rgba(0.025, 0.046, 0.058, 0.88)
                },
            );
            draw_border(
                renderer,
                rect,
                1.0 * layout.scale,
                if active {
                    Color::rgba(0.29, 0.90, 1.0, 0.92)
                } else {
                    Color::rgba(0.14, 0.22, 0.30, 0.82)
                },
            );
            if active {
                renderer.draw_rect(
                    Rect::new(rect.origin, Vec2::new(3.0 * layout.scale, rect.size.y)),
                    Color::rgba(0.95, 0.70, 0.30, 0.98),
                );
            }

            let icon_size = 22.0 * layout.scale;
            renderer.draw_image(
                category.icon_texture_id(),
                Rect::new(
                    Vec2::new(
                        rect.origin.x + 12.0 * layout.scale,
                        rect.origin.y + (rect.size.y - icon_size) * 0.5,
                    ),
                    Vec2::new(icon_size, icon_size),
                ),
                Color::rgba(1.0, 1.0, 1.0, if active { 1.0 } else { 0.58 }),
            );

            if let Some(label) = self.text.tab_labels.get(&category) {
                draw_text(
                    renderer,
                    label,
                    viewport,
                    screen_x(viewport, rect.origin.x + 46.0 * layout.scale),
                    screen_y(viewport, rect.origin.y + 9.0 * layout.scale),
                    if active {
                        Color::rgba(0.88, 1.0, 0.98, 1.0)
                    } else {
                        Color::rgba(0.54, 0.68, 0.72, 0.95)
                    },
                );
            }
        }
    }

    fn draw_backpack(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        draw_panel(
            renderer,
            layout.backpack_panel,
            Color::rgba(0.017, 0.027, 0.035, 0.94),
            Color::rgba(0.18, 0.29, 0.39, 0.78),
            layout.scale,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.backpack_panel.origin.x + 20.0 * layout.scale,
                    layout.backpack_panel.origin.y + 22.0 * layout.scale,
                ),
                Vec2::new(96.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.28, 0.88, 1.0, 0.78),
        );

        for slot_index in 0..BACKPACK_SLOTS {
            self.draw_slot(
                renderer,
                viewport,
                layout,
                slot_index,
                layout.backpack_slot_rect(slot_index),
                false,
            );
        }
    }

    fn draw_quickbar(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        draw_panel(
            renderer,
            layout.quickbar_panel,
            Color::rgba(0.020, 0.030, 0.036, 0.94),
            Color::rgba(0.31, 0.25, 0.16, 0.84),
            layout.scale,
        );
        renderer.draw_rect(
            Rect::new(
                layout.quickbar_panel.origin,
                Vec2::new(3.0 * layout.scale, layout.quickbar_panel.size.y),
            ),
            Color::rgba(0.90, 0.67, 0.30, 0.95),
        );

        for quick_index in 0..QUICKBAR_SLOTS {
            let rect = layout.quickbar_slot_rect(quick_index);
            let slot_index = self.state.quickbar[quick_index].unwrap_or(quick_index);
            self.draw_slot(renderer, viewport, layout, slot_index, rect, true);

            if let Some(label) = self.text.quickbar_numbers.get(quick_index) {
                draw_text(
                    renderer,
                    label,
                    viewport,
                    screen_x(viewport, rect.origin.x + 4.0 * layout.scale),
                    screen_y(viewport, rect.origin.y + 2.0 * layout.scale),
                    Color::rgba(0.84, 0.73, 0.55, 0.95),
                );
            }
        }
    }

    fn draw_slot(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        layout: &InventoryLayout,
        slot_index: usize,
        rect: Rect,
        quickbar: bool,
    ) {
        let stack = self.state.slots.get(slot_index).and_then(Option::as_ref);
        let selected = slot_index == self.state.selected_slot;
        let locked = stack.is_some_and(|stack| stack.locked);

        renderer.draw_rect(
            rect,
            if quickbar {
                Color::rgba(0.012, 0.020, 0.026, 0.96)
            } else {
                Color::rgba(0.007, 0.014, 0.018, 0.98)
            },
        );
        draw_border(
            renderer,
            rect,
            1.0 * layout.scale,
            if selected {
                Color::rgba(0.32, 0.94, 1.0, 0.96)
            } else if locked {
                Color::rgba(0.70, 0.54, 0.35, 0.78)
            } else {
                Color::rgba(0.15, 0.26, 0.34, 0.86)
            },
        );
        renderer.draw_rect(
            inset_rect(rect, 5.0 * layout.scale),
            if stack.is_some() {
                Color::rgba(0.032, 0.060, 0.074, 0.72)
            } else {
                Color::rgba(0.000, 0.004, 0.008, 0.52)
            },
        );

        if selected {
            draw_corner_brackets(
                renderer,
                rect,
                12.0 * layout.scale,
                2.0 * layout.scale,
                Color::rgba(0.40, 0.96, 1.0, 1.0),
            );
            renderer.draw_rect(
                Rect::new(
                    Vec2::new(
                        rect.origin.x + 8.0 * layout.scale,
                        rect.bottom() - 4.0 * layout.scale,
                    ),
                    Vec2::new(rect.size.x - 16.0 * layout.scale, 2.0 * layout.scale),
                ),
                Color::rgba(0.95, 0.70, 0.30, 0.95),
            );
        }

        if let Some(stack) = stack {
            if let Some(definition) = item_definition(stack.item_id) {
                let inset = if quickbar { 6.0 } else { 7.0 } * layout.scale;
                renderer.draw_rect(
                    inset_rect(rect, 10.0 * layout.scale),
                    rarity_glow_color(definition.rarity, if selected { 0.30 } else { 0.16 }),
                );
                renderer.draw_image(
                    definition.texture_id,
                    Rect::new(
                        Vec2::new(rect.origin.x + inset, rect.origin.y + inset),
                        Vec2::new(rect.size.x - inset * 2.0, rect.size.y - inset * 2.0),
                    ),
                    Color::rgba(1.0, 1.0, 1.0, if stack.locked { 0.72 } else { 1.0 }),
                );
            }

            if stack.quantity > 1 {
                let badge = Rect::new(
                    Vec2::new(
                        rect.origin.x + rect.size.x - 31.0 * layout.scale,
                        rect.origin.y + rect.size.y - 20.0 * layout.scale,
                    ),
                    Vec2::new(28.0 * layout.scale, 18.0 * layout.scale),
                );
                renderer.draw_rect(badge, Color::rgba(0.018, 0.036, 0.045, 0.96));
                draw_border(
                    renderer,
                    badge,
                    1.0 * layout.scale,
                    Color::rgba(0.36, 0.88, 1.0, 0.82),
                );

                if let Some(count) = self.text.count_badges.get(&slot_index) {
                    draw_text_centered(
                        renderer,
                        count,
                        viewport,
                        screen_x(viewport, badge.origin.x + badge.size.x * 0.5),
                        screen_y(viewport, badge.origin.y - 3.0 * layout.scale),
                        Color::rgba(0.92, 1.0, 0.96, 1.0),
                    );
                }
            }
        }
    }

    fn draw_details(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        draw_panel(
            renderer,
            layout.details_panel,
            Color::rgba(0.017, 0.026, 0.033, 0.95),
            Color::rgba(0.18, 0.29, 0.39, 0.78),
            layout.scale,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.details_panel.origin.x + 18.0 * layout.scale,
                    layout.details_panel.origin.y + 22.0 * layout.scale,
                ),
                Vec2::new(72.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.90, 0.70, 0.32, 0.88),
        );

        let origin = layout.details_panel.origin;
        let selected = self
            .state
            .slots
            .get(self.state.selected_slot)
            .and_then(Option::as_ref);
        let Some(stack) = selected else {
            if let Some(empty) = &self.text.empty_detail {
                draw_text(
                    renderer,
                    empty,
                    viewport,
                    screen_x(viewport, origin.x + 34.0 * layout.scale),
                    screen_y(viewport, origin.y + 150.0 * layout.scale),
                    Color::rgba(0.46, 0.64, 0.70, 0.92),
                );
            }
            return;
        };

        let Some(definition) = item_definition(stack.item_id) else {
            return;
        };

        let icon_rect = Rect::new(
            Vec2::new(
                origin.x + (DETAILS_PANEL_SIZE.x * 0.5 - 46.0) * layout.scale,
                origin.y + 38.0 * layout.scale,
            ),
            Vec2::new(92.0 * layout.scale, 92.0 * layout.scale),
        );
        renderer.draw_rect(
            inset_rect(icon_rect, -5.0 * layout.scale),
            rarity_glow_color(definition.rarity, 0.18),
        );
        draw_border(
            renderer,
            inset_rect(icon_rect, -5.0 * layout.scale),
            1.0 * layout.scale,
            rarity_color(definition.rarity),
        );
        renderer.draw_image(
            definition.texture_id,
            icon_rect,
            Color::rgba(1.0, 1.0, 1.0, if stack.locked { 0.78 } else { 1.0 }),
        );

        let Some(detail) = self.text.item_details.get(definition.id) else {
            return;
        };

        let center_x = screen_x(viewport, origin.x + layout.details_panel.size.x * 0.5);
        draw_text_centered(
            renderer,
            &detail.name,
            viewport,
            center_x,
            screen_y(viewport, origin.y + 138.0 * layout.scale),
            Color::rgba(0.90, 1.0, 0.98, 1.0),
        );
        draw_text_centered(
            renderer,
            &detail.category,
            viewport,
            center_x,
            screen_y(viewport, origin.y + 176.0 * layout.scale),
            Color::rgba(0.52, 0.76, 0.84, 1.0),
        );
        draw_text_centered(
            renderer,
            &detail.rarity,
            viewport,
            center_x,
            screen_y(viewport, origin.y + 204.0 * layout.scale),
            rarity_color(definition.rarity),
        );

        let progress_track = Rect::new(
            Vec2::new(
                origin.x + 34.0 * layout.scale,
                origin.y + 238.0 * layout.scale,
            ),
            Vec2::new(234.0 * layout.scale, 8.0 * layout.scale),
        );
        renderer.draw_rect(progress_track, Color::rgba(0.05, 0.08, 0.09, 0.95));
        renderer.draw_rect(
            Rect::new(
                progress_track.origin,
                Vec2::new(
                    progress_track.size.x * (definition.research as f32 / 100.0),
                    progress_track.size.y,
                ),
            ),
            rarity_color(definition.rarity),
        );

        let line_x = screen_x(viewport, origin.x + 34.0 * layout.scale);
        let mut line_y = origin.y + 258.0 * layout.scale;
        for line in [&detail.quantity, &detail.stack_limit, &detail.status] {
            draw_text(
                renderer,
                line,
                viewport,
                line_x,
                screen_y(viewport, line_y),
                Color::rgba(0.70, 0.88, 0.92, 0.96),
            );
            line_y += 29.0 * layout.scale;
        }

        if let Some(lock_state) = &detail.lock_state {
            draw_text(
                renderer,
                lock_state,
                viewport,
                line_x,
                screen_y(viewport, line_y),
                Color::rgba(0.95, 0.72, 0.42, 1.0),
            );
        }
    }

    fn move_selection(&mut self, dx: i32, dy: i32) {
        let column = self.state.selected_slot % BACKPACK_COLUMNS;
        let row = self.state.selected_slot / BACKPACK_COLUMNS;
        let next_column = (column as i32 + dx).clamp(0, BACKPACK_COLUMNS as i32 - 1) as usize;
        let next_row = (row as i32 + dy).clamp(0, BACKPACK_ROWS as i32 - 1) as usize;
        self.select_slot(next_row * BACKPACK_COLUMNS + next_column);
    }

    fn select_slot(&mut self, slot_index: usize) {
        self.state.selected_slot = slot_index.min(BACKPACK_SLOTS - 1);
        if let Some(stack) = self
            .state
            .slots
            .get(self.state.selected_slot)
            .and_then(Option::as_ref)
        {
            if let Some(definition) = item_definition(stack.item_id) {
                self.state.active_category = definition.category;
            }
        }
    }

    fn select_first_in_category(&mut self, category: ItemCategory) {
        self.state.active_category = category;
        if let Some(slot_index) = self.state.slots.iter().position(|slot| {
            slot.as_ref()
                .and_then(|stack| item_definition(stack.item_id))
                .is_some_and(|definition| definition.category == category)
        }) {
            self.state.selected_slot = slot_index;
        }
    }

    fn handle_mouse(&mut self, input: &InputState) {
        if !input.mouse_left_just_pressed() {
            return;
        }

        let Some(position) = input.cursor_position() else {
            return;
        };

        let viewport = input.screen_size();
        let layout = InventoryLayout::new(viewport);

        for index in 0..BACKPACK_SLOTS {
            if screen_point_in_rect(
                position,
                world_rect_to_screen(viewport, layout.backpack_slot_rect(index)),
            ) {
                self.select_slot(index);
                return;
            }
        }

        for index in 0..QUICKBAR_SLOTS {
            let slot_index = self.state.quickbar[index].unwrap_or(index);
            if screen_point_in_rect(
                position,
                world_rect_to_screen(viewport, layout.quickbar_slot_rect(index)),
            ) {
                self.select_slot(slot_index);
                return;
            }
        }

        for (index, category) in ItemCategory::ALL.iter().copied().enumerate() {
            if screen_point_in_rect(
                position,
                world_rect_to_screen(viewport, layout.category_rect(index)),
            ) {
                self.select_first_in_category(category);
                return;
            }
        }
    }

    fn upload_textures(&mut self, renderer: &mut dyn Renderer, font: &Font<'static>) -> Result<()> {
        let language = self.language;
        self.text = InventoryText {
            language: Some(language),
            ..InventoryText::default()
        };

        self.text.title = Some(upload_text(
            renderer,
            font,
            "inventory_text_title",
            inventory_title(language),
            match language {
                Language::Chinese => 34.0,
                Language::English => 30.0,
            },
        )?);
        self.text.empty_detail = Some(upload_text(
            renderer,
            font,
            "inventory_text_empty_detail",
            empty_slot_text(language),
            22.0,
        )?);

        for category in ItemCategory::ALL {
            self.text.tab_labels.insert(
                category,
                upload_text(
                    renderer,
                    font,
                    &format!("inventory_text_tab_{}", category.key()),
                    category.label(language),
                    match language {
                        Language::Chinese => 19.0,
                        Language::English => 16.0,
                    },
                )?,
            );
        }

        for slot_index in 0..BACKPACK_SLOTS {
            if let Some(stack) = self.state.slots[slot_index].as_ref() {
                if stack.quantity > 1 {
                    self.text.count_badges.insert(
                        slot_index,
                        upload_text(
                            renderer,
                            font,
                            &format!("inventory_text_count_{slot_index}"),
                            &stack.quantity.to_string(),
                            13.0,
                        )?,
                    );
                }
            }
        }

        self.text.quickbar_numbers = (1..=QUICKBAR_SLOTS)
            .map(|number| {
                upload_text(
                    renderer,
                    font,
                    &format!("inventory_text_quickbar_{number}"),
                    &number.to_string(),
                    13.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        for definition in ITEM_DEFINITIONS {
            self.text.item_details.insert(
                definition.id,
                ItemDetailText {
                    name: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_name_{}", definition.id),
                        definition.name.get(language),
                        match language {
                            Language::Chinese => 28.0,
                            Language::English => 26.0,
                        },
                    )?,
                    category: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_category_{}", definition.id),
                        definition.category.label(language),
                        18.0,
                    )?,
                    quantity: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_quantity_{}", definition.id),
                        &quantity_text(language, quantity_for_item(&self.state, definition.id)),
                        18.0,
                    )?,
                    rarity: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_rarity_{}", definition.id),
                        definition.rarity.label(language),
                        18.0,
                    )?,
                    stack_limit: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_stack_{}", definition.id),
                        &stack_limit_text(language, definition.max_stack),
                        18.0,
                    )?,
                    status: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_status_{}", definition.id),
                        &research_text(language, definition.research),
                        18.0,
                    )?,
                    lock_state: self
                        .state
                        .slots
                        .iter()
                        .flatten()
                        .find(|stack| stack.item_id == definition.id && stack.locked)
                        .map(|_| {
                            upload_text(
                                renderer,
                                font,
                                &format!("inventory_text_locked_{}", definition.id),
                                locked_item_text(language),
                                18.0,
                            )
                        })
                        .transpose()?,
                },
            );
        }

        Ok(())
    }
}

impl InventoryState {
    fn new_mvp() -> Self {
        let mut slots = vec![None; BACKPACK_SLOTS];
        for (index, stack) in [
            ItemStack::new("alien_crystal_sample", 3, false),
            ItemStack::new("bio_sample_vial", 2, false),
            ItemStack::new("data_shard", 8, false),
            ItemStack::new("energy_cell", 4, false),
            ItemStack::new("scrap_part", 17, false),
            ItemStack::new("ruin_key", 1, true),
            ItemStack::new("scanner_tool", 1, true),
            ItemStack::new("med_injector", 2, false),
            ItemStack::new("coolant_canister", 1, false),
            ItemStack::new("metal_fragment", 9, false),
            ItemStack::new("glow_fungus_sample", 2, false),
            ItemStack::new("artifact_core", 1, true),
        ]
        .into_iter()
        .enumerate()
        {
            slots[index] = Some(stack);
        }

        Self {
            slots,
            quickbar: [Some(6), Some(7), Some(3), Some(2), Some(5), Some(11)],
            selected_slot: 0,
            active_category: ItemCategory::Samples,
        }
    }

    fn from_save(save: &InventorySave) -> Self {
        let mut state = Self::new_mvp();
        state.slots = vec![None; BACKPACK_SLOTS];
        for (index, saved_stack) in save.slots.iter().take(BACKPACK_SLOTS).enumerate() {
            let Some(saved_stack) = saved_stack else {
                continue;
            };
            let Some(definition) = item_definition(&saved_stack.item_id) else {
                continue;
            };

            state.slots[index] = Some(ItemStack::new(
                definition.id,
                saved_stack.quantity.clamp(1, definition.max_stack.max(1)),
                saved_stack.locked,
            ));
        }

        state.quickbar = [None; QUICKBAR_SLOTS];
        for (index, slot_index) in save.quickbar.iter().take(QUICKBAR_SLOTS).enumerate() {
            state.quickbar[index] =
                (*slot_index).and_then(|slot| (slot < BACKPACK_SLOTS).then_some(slot));
        }

        state.selected_slot = save.selected_slot.min(BACKPACK_SLOTS - 1);
        state.active_category =
            ItemCategory::from_key(&save.active_category).unwrap_or_else(|| {
                state
                    .slots
                    .get(state.selected_slot)
                    .and_then(Option::as_ref)
                    .and_then(|stack| item_definition(stack.item_id))
                    .map_or(ItemCategory::Samples, |definition| definition.category)
            });

        state
    }

    fn to_save(&self) -> InventorySave {
        InventorySave {
            slots: self
                .slots
                .iter()
                .map(|slot| {
                    slot.as_ref().map(|stack| {
                        ItemStackSave::new(stack.item_id, stack.quantity, stack.locked)
                    })
                })
                .collect(),
            quickbar: self.quickbar.to_vec(),
            selected_slot: self.selected_slot,
            active_category: self.active_category.key().to_owned(),
        }
    }
}

impl ItemStack {
    fn new(item_id: &'static str, quantity: u32, locked: bool) -> Self {
        Self {
            item_id,
            quantity,
            locked,
        }
    }
}

impl Scene for InventoryScene {
    fn id(&self) -> SceneId {
        SceneId::Inventory
    }

    fn name(&self) -> &str {
        "InventoryScene"
    }

    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        for (texture_id, path) in UI_TEXTURES {
            if renderer.texture_size(texture_id).is_none() {
                renderer.load_texture(texture_id, Path::new(path))?;
            }
        }

        load_inventory_item_icons(renderer)?;

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
            self.text = InventoryText::default();
        }

        if input.just_pressed(Button::Pause) || input.just_pressed(Button::Inventory) {
            return Ok(SceneCommand::Pop);
        }

        let before = self.state.to_save();

        self.handle_mouse(input);

        if input.just_pressed(Button::Left) {
            self.move_selection(-1, 0);
        }
        if input.just_pressed(Button::Right) {
            self.move_selection(1, 0);
        }
        if input.just_pressed(Button::Up) {
            self.move_selection(0, -1);
        }
        if input.just_pressed(Button::Down) {
            self.move_selection(0, 1);
        }

        let after = self.state.to_save();
        if after != before {
            ctx.set_inventory_save(after);
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        if self.text.language != Some(self.language) {
            let font = load_ui_font()?;
            self.upload_textures(ctx.renderer, &font)?;
        }

        self.draw_inventory(ctx);
        Ok(())
    }
}

struct InventoryLayout {
    scale: f32,
    root_panel: Rect,
    header: Rect,
    category_panel: Rect,
    backpack_panel: Rect,
    details_panel: Rect,
    quickbar_panel: Rect,
}

impl InventoryLayout {
    fn new(viewport: Vec2) -> Self {
        let scale = ((viewport.x - 48.0) / INVENTORY_WIDTH)
            .min((viewport.y - 48.0) / INVENTORY_HEIGHT)
            .min(1.0)
            .max(0.64);
        let root_panel = Rect::new(
            Vec2::new(
                -INVENTORY_WIDTH * scale * 0.5,
                -INVENTORY_HEIGHT * scale * 0.5,
            ),
            Vec2::new(INVENTORY_WIDTH * scale, INVENTORY_HEIGHT * scale),
        );
        let header = Rect::new(
            root_panel.origin,
            Vec2::new(root_panel.size.x, HEADER_HEIGHT * scale),
        );
        let content_y = root_panel.origin.y + (HEADER_HEIGHT + 16.0) * scale;
        let category_origin = Vec2::new(root_panel.origin.x + OUTER_PADDING * scale, content_y);
        let backpack_origin = Vec2::new(
            category_origin.x + (CATEGORY_PANEL_SIZE.x + CONTENT_GAP) * scale,
            content_y,
        );
        let details_origin = Vec2::new(
            backpack_origin.x + (BACKPACK_PANEL_SIZE.x + CONTENT_GAP) * scale,
            content_y,
        );
        let quickbar_origin = Vec2::new(
            backpack_origin.x,
            root_panel.origin.y
                + (INVENTORY_HEIGHT - OUTER_PADDING - QUICKBAR_PANEL_SIZE.y) * scale,
        );

        Self {
            scale,
            root_panel,
            header,
            category_panel: Rect::new(category_origin, CATEGORY_PANEL_SIZE * scale),
            backpack_panel: Rect::new(backpack_origin, BACKPACK_PANEL_SIZE * scale),
            details_panel: Rect::new(details_origin, DETAILS_PANEL_SIZE * scale),
            quickbar_panel: Rect::new(quickbar_origin, QUICKBAR_PANEL_SIZE * scale),
        }
    }

    fn category_rect(&self, index: usize) -> Rect {
        let row_height = 46.0 * self.scale;
        let row_gap = 9.0 * self.scale;
        let row_width = self.category_panel.size.x - 24.0 * self.scale;
        Rect::new(
            Vec2::new(
                self.category_panel.origin.x + 12.0 * self.scale,
                self.category_panel.origin.y
                    + 16.0 * self.scale
                    + index as f32 * (row_height + row_gap),
            ),
            Vec2::new(row_width, row_height),
        )
    }

    fn backpack_slot_rect(&self, slot_index: usize) -> Rect {
        let column = slot_index % BACKPACK_COLUMNS;
        let row = slot_index / BACKPACK_COLUMNS;
        let grid_width =
            BACKPACK_COLUMNS as f32 * SLOT_SIZE + (BACKPACK_COLUMNS - 1) as f32 * SLOT_GAP;
        self.slot_rect(
            self.backpack_panel.origin
                + Vec2::new(
                    ((BACKPACK_PANEL_SIZE.x - grid_width) * 0.5) * self.scale,
                    56.0 * self.scale,
                ),
            column,
            row,
        )
    }

    fn quickbar_slot_rect(&self, quick_index: usize) -> Rect {
        let strip_width = QUICKBAR_SLOTS as f32 * QUICKBAR_SLOT_SIZE
            + (QUICKBAR_SLOTS - 1) as f32 * QUICKBAR_SLOT_GAP;
        let stride = (QUICKBAR_SLOT_SIZE + QUICKBAR_SLOT_GAP) * self.scale;
        Rect::new(
            Vec2::new(
                self.quickbar_panel.origin.x
                    + ((QUICKBAR_PANEL_SIZE.x - strip_width) * 0.5) * self.scale
                    + quick_index as f32 * stride,
                self.quickbar_panel.origin.y + 4.0 * self.scale,
            ),
            Vec2::new(
                QUICKBAR_SLOT_SIZE * self.scale,
                QUICKBAR_SLOT_SIZE * self.scale,
            ),
        )
    }

    fn slot_rect(&self, origin: Vec2, column: usize, row: usize) -> Rect {
        let stride = (SLOT_SIZE + SLOT_GAP) * self.scale;
        Rect::new(
            Vec2::new(
                origin.x + column as f32 * stride,
                origin.y + row as f32 * stride,
            ),
            Vec2::new(SLOT_SIZE * self.scale, SLOT_SIZE * self.scale),
        )
    }
}

fn draw_panel(renderer: &mut dyn Renderer, rect: Rect, fill: Color, border: Color, scale: f32) {
    renderer.draw_rect(rect, fill);
    draw_border(renderer, rect, 1.0 * scale, border);
    draw_border(
        renderer,
        inset_rect(rect, 4.0 * scale),
        1.0 * scale,
        Color::rgba(0.08, 0.16, 0.21, 0.60),
    );
}

fn draw_border(renderer: &mut dyn Renderer, rect: Rect, thickness: f32, color: Color) {
    let thickness = thickness.max(1.0);
    renderer.draw_rect(
        Rect::new(rect.origin, Vec2::new(rect.size.x, thickness)),
        color,
    );
    renderer.draw_rect(
        Rect::new(
            Vec2::new(rect.origin.x, rect.bottom() - thickness),
            Vec2::new(rect.size.x, thickness),
        ),
        color,
    );
    renderer.draw_rect(
        Rect::new(rect.origin, Vec2::new(thickness, rect.size.y)),
        color,
    );
    renderer.draw_rect(
        Rect::new(
            Vec2::new(rect.right() - thickness, rect.origin.y),
            Vec2::new(thickness, rect.size.y),
        ),
        color,
    );
}

fn draw_corner_brackets(
    renderer: &mut dyn Renderer,
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
        renderer.draw_rect(
            Rect::new(Vec2::new(x, y), Vec2::new(length, thickness)),
            color,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(horizontal_x, vertical_y),
                Vec2::new(thickness, length),
            ),
            color,
        );
    }
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

fn item_definition(item_id: &str) -> Option<&'static ItemDefinition> {
    ITEM_DEFINITIONS
        .iter()
        .find(|definition| definition.id == item_id)
}

pub(super) fn load_inventory_item_icons(renderer: &mut dyn Renderer) -> Result<()> {
    for definition in ITEM_DEFINITIONS {
        if renderer.texture_size(definition.texture_id).is_none() {
            renderer
                .load_texture(definition.texture_id, Path::new(definition.icon_path))
                .with_context(|| format!("failed to load inventory item {}", definition.id))?;
        }
    }

    Ok(())
}

pub(super) fn inventory_slots(save: &InventorySave) -> Vec<Option<InventoryItemView>> {
    InventoryState::from_save(save)
        .slots
        .iter()
        .map(|slot| slot.as_ref().and_then(inventory_item_view))
        .collect()
}

pub(super) fn inventory_item_max_stack(item_id: &str) -> Option<u32> {
    item_definition(item_id).map(|definition| definition.max_stack)
}

pub(super) fn inventory_item_weight(item_id: &str) -> u32 {
    item_definition(item_id).map_or(1, |definition| definition.weight)
}

pub(super) fn inventory_item_name(item_id: &str, language: Language) -> String {
    item_definition(item_id)
        .map(|definition| definition.name.get(language).to_owned())
        .unwrap_or_else(|| item_id.to_owned())
}

fn inventory_item_view(stack: &ItemStack) -> Option<InventoryItemView> {
    let definition = item_definition(stack.item_id)?;

    Some(InventoryItemView {
        texture_id: definition.texture_id,
        quantity: stack.quantity,
        max_stack: definition.max_stack,
        research: definition.research,
        locked: stack.locked,
        rarity_color: rarity_color(definition.rarity),
        name_english: definition.name.get(Language::English),
        name_chinese: definition.name.get(Language::Chinese),
        category_english: definition.category.label(Language::English),
        category_chinese: definition.category.label(Language::Chinese),
        rarity_english: definition.rarity.label(Language::English),
        rarity_chinese: definition.rarity.label(Language::Chinese),
    })
}

fn quantity_for_item(state: &InventoryState, item_id: &str) -> u32 {
    state
        .slots
        .iter()
        .flatten()
        .find(|stack| stack.item_id == item_id)
        .map(|stack| stack.quantity)
        .unwrap_or(0)
}

fn inventory_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "外勤背包",
        Language::English => "EXOSUIT PACK",
    }
}

fn empty_slot_text(language: Language) -> &'static str {
    match language {
        Language::Chinese => "空槽位",
        Language::English => "Empty slot",
    }
}

fn quantity_text(language: Language, quantity: u32) -> String {
    match language {
        Language::Chinese => format!("数量: {quantity}"),
        Language::English => format!("Quantity: {quantity}"),
    }
}

fn stack_limit_text(language: Language, max_stack: u32) -> String {
    match language {
        Language::Chinese => format!("最大堆叠: {max_stack}"),
        Language::English => format!("Max Stack: {max_stack}"),
    }
}

fn research_text(language: Language, research: u32) -> String {
    match language {
        Language::Chinese => format!("研究进度: {research}%"),
        Language::English => format!("Research: {research}%"),
    }
}

fn locked_item_text(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已锁定",
        Language::English => "Locked Item",
    }
}

fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::Common => Color::rgba(0.72, 0.82, 0.86, 1.0),
        Rarity::Uncommon => Color::rgba(0.58, 0.95, 0.72, 1.0),
        Rarity::Rare => Color::rgba(0.42, 0.78, 1.0, 1.0),
        Rarity::Artifact => Color::rgba(0.98, 0.78, 0.42, 1.0),
    }
}

fn rarity_glow_color(rarity: Rarity, alpha: f32) -> Color {
    match rarity {
        Rarity::Common => Color::rgba(0.42, 0.52, 0.56, alpha),
        Rarity::Uncommon => Color::rgba(0.20, 0.72, 0.44, alpha),
        Rarity::Rare => Color::rgba(0.16, 0.58, 0.92, alpha),
        Rarity::Artifact => Color::rgba(0.90, 0.56, 0.18, alpha),
    }
}

fn screen_rect(viewport: Vec2, x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(
        Vec2::new(-viewport.x * 0.5 + x, -viewport.y * 0.5 + y),
        Vec2::new(width, height),
    )
}

fn world_rect_to_screen(viewport: Vec2, rect: Rect) -> Rect {
    Rect::new(
        Vec2::new(
            rect.origin.x + viewport.x * 0.5,
            rect.origin.y + viewport.y * 0.5,
        ),
        rect.size,
    )
}

fn screen_x(viewport: Vec2, world_x: f32) -> f32 {
    world_x + viewport.x * 0.5
}

fn screen_y(viewport: Vec2, world_y: f32) -> f32 {
    world_y + viewport.y * 0.5
}

fn screen_point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_inventory_has_expected_slot_counts() {
        let state = InventoryState::new_mvp();

        assert_eq!(state.slots.len(), BACKPACK_SLOTS);
        assert_eq!(state.quickbar.len(), QUICKBAR_SLOTS);
        assert_eq!(state.slots.iter().filter(|slot| slot.is_some()).count(), 12);
        assert!(state.slots.iter().flatten().any(|stack| stack.locked));
    }

    #[test]
    fn mvp_inventory_items_have_definitions() {
        let state = InventoryState::new_mvp();

        for stack in state.slots.iter().flatten() {
            assert!(
                item_definition(stack.item_id).is_some(),
                "{} should have an item definition",
                stack.item_id
            );
        }
    }

    #[test]
    fn inventory_state_round_trips_save_selection_and_items() {
        let mut save = InventorySave::default();
        save.selected_slot = 3;
        save.active_category = "components".to_owned();
        save.slots[3] = Some(ItemStackSave::new("energy_cell", 99, false));

        let state = InventoryState::from_save(&save);
        let saved_again = state.to_save();

        assert_eq!(state.selected_slot, 3);
        assert_eq!(state.active_category, ItemCategory::Components);
        assert_eq!(saved_again.selected_slot, 3);
        assert_eq!(saved_again.active_category, "components");
        assert_eq!(
            saved_again.slots[3].as_ref().map(|stack| stack.quantity),
            Some(20)
        );
    }

    #[test]
    fn inventory_texture_paths_exist() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

        for (_, path) in UI_TEXTURES {
            assert!(project_root.join(path).exists(), "{path} should exist");
        }

        for definition in ITEM_DEFINITIONS {
            assert!(
                project_root.join(definition.icon_path).exists(),
                "{} should exist",
                definition.icon_path
            );
        }
    }

    #[test]
    fn inventory_text_has_chinese_and_english_strings() {
        for language in Language::SUPPORTED {
            assert!(!inventory_title(language).is_empty());
            assert!(!empty_slot_text(language).is_empty());
            assert!(!quantity_text(language, 3).is_empty());
            assert!(!stack_limit_text(language, 10).is_empty());
            assert!(!research_text(language, 42).is_empty());
            assert!(!locked_item_text(language).is_empty());

            for category in ItemCategory::ALL {
                assert!(!category.label(language).is_empty());
            }

            for rarity in [
                Rarity::Common,
                Rarity::Uncommon,
                Rarity::Rare,
                Rarity::Artifact,
            ] {
                assert!(!rarity.label(language).is_empty());
            }

            for definition in ITEM_DEFINITIONS {
                assert!(!definition.name.get(language).is_empty());
                assert_eq!(inventory_item_weight(definition.id), definition.weight);
            }
        }
    }
}
