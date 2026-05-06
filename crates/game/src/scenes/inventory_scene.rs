use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::Font;

use crate::ui::text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text};

use super::{GameContext, RenderContext, Scene, SceneId};

const BACKPACK_COLUMNS: usize = 6;
const BACKPACK_ROWS: usize = 4;
const BACKPACK_SLOTS: usize = BACKPACK_COLUMNS * BACKPACK_ROWS;
const QUICKBAR_SLOTS: usize = 6;
const SLOT_SIZE: f32 = 64.0;
const SLOT_GAP: f32 = 8.0;
const TAB_WIDTH: f32 = 86.0;
const TAB_HEIGHT: f32 = 36.0;
const TAB_GAP: f32 = 4.0;
const BACKPACK_PANEL_SIZE: Vec2 = Vec2::new(448.0, 352.0);
const DETAILS_PANEL_SIZE: Vec2 = Vec2::new(320.0, 352.0);
const QUICKBAR_PANEL_SIZE: Vec2 = Vec2::new(448.0, 88.0);
const CONTENT_GAP: f32 = 24.0;

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

const ITEM_DEFINITIONS: &[ItemDefinition] = &[
    ItemDefinition {
        id: "alien_crystal_sample",
        name: "Crystal Sample",
        category: ItemCategory::Samples,
        texture_id: "item_alien_crystal_sample",
        icon_path: "assets/images/ui/inventory/items/item_alien_crystal_sample.png",
        max_stack: 10,
        rarity: Rarity::Rare,
        research: 42,
    },
    ItemDefinition {
        id: "bio_sample_vial",
        name: "Bio Sample",
        category: ItemCategory::Samples,
        texture_id: "item_bio_sample_vial",
        icon_path: "assets/images/ui/inventory/items/item_bio_sample_vial.png",
        max_stack: 10,
        rarity: Rarity::Uncommon,
        research: 18,
    },
    ItemDefinition {
        id: "data_shard",
        name: "Data Shard",
        category: ItemCategory::Data,
        texture_id: "item_data_shard",
        icon_path: "assets/images/ui/inventory/items/item_data_shard.png",
        max_stack: 99,
        rarity: Rarity::Rare,
        research: 66,
    },
    ItemDefinition {
        id: "energy_cell",
        name: "Energy Cell",
        category: ItemCategory::Components,
        texture_id: "item_energy_cell",
        icon_path: "assets/images/ui/inventory/items/item_energy_cell.png",
        max_stack: 20,
        rarity: Rarity::Uncommon,
        research: 35,
    },
    ItemDefinition {
        id: "scrap_part",
        name: "Scrap Part",
        category: ItemCategory::Components,
        texture_id: "item_scrap_part",
        icon_path: "assets/images/ui/inventory/items/item_scrap_part.png",
        max_stack: 99,
        rarity: Rarity::Common,
        research: 8,
    },
    ItemDefinition {
        id: "ruin_key",
        name: "Ruin Key",
        category: ItemCategory::Artifacts,
        texture_id: "item_ruin_key",
        icon_path: "assets/images/ui/inventory/items/item_ruin_key.png",
        max_stack: 1,
        rarity: Rarity::Artifact,
        research: 91,
    },
    ItemDefinition {
        id: "scanner_tool",
        name: "Scanner",
        category: ItemCategory::Tools,
        texture_id: "item_scanner_tool",
        icon_path: "assets/images/ui/inventory/items/item_scanner_tool.png",
        max_stack: 1,
        rarity: Rarity::Uncommon,
        research: 100,
    },
    ItemDefinition {
        id: "med_injector",
        name: "Med Injector",
        category: ItemCategory::Consumables,
        texture_id: "item_med_injector",
        icon_path: "assets/images/ui/inventory/items/item_med_injector.png",
        max_stack: 5,
        rarity: Rarity::Common,
        research: 0,
    },
    ItemDefinition {
        id: "coolant_canister",
        name: "Coolant Canister",
        category: ItemCategory::Components,
        texture_id: "item_coolant_canister",
        icon_path: "assets/images/ui/inventory/items/item_coolant_canister.png",
        max_stack: 10,
        rarity: Rarity::Uncommon,
        research: 24,
    },
    ItemDefinition {
        id: "metal_fragment",
        name: "Metal Fragment",
        category: ItemCategory::Components,
        texture_id: "item_metal_fragment",
        icon_path: "assets/images/ui/inventory/items/item_metal_fragment.png",
        max_stack: 99,
        rarity: Rarity::Common,
        research: 12,
    },
    ItemDefinition {
        id: "glow_fungus_sample",
        name: "Glow Fungus",
        category: ItemCategory::Samples,
        texture_id: "item_glow_fungus_sample",
        icon_path: "assets/images/ui/inventory/items/item_glow_fungus_sample.png",
        max_stack: 10,
        rarity: Rarity::Uncommon,
        research: 54,
    },
    ItemDefinition {
        id: "artifact_core",
        name: "Artifact Core",
        category: ItemCategory::Artifacts,
        texture_id: "item_artifact_core",
        icon_path: "assets/images/ui/inventory/items/item_artifact_core.png",
        max_stack: 1,
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

    fn label(self) -> &'static str {
        match self {
            Self::Samples => "Samples",
            Self::Components => "Components",
            Self::Artifacts => "Artifacts",
            Self::Consumables => "Consumables",
            Self::Tools => "Tools",
            Self::Data => "Data",
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
    fn label(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Artifact => "Artifact",
        }
    }

    fn texture_id(self) -> &'static str {
        match self {
            Self::Common => "ui_inventory_rarity_common",
            Self::Uncommon => "ui_inventory_rarity_uncommon",
            Self::Rare => "ui_inventory_rarity_rare",
            Self::Artifact => "ui_inventory_rarity_artifact",
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
    name: &'static str,
    category: ItemCategory,
    texture_id: &'static str,
    icon_path: &'static str,
    max_stack: u32,
    rarity: Rarity,
    research: u32,
}

#[derive(Default)]
struct InventoryText {
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
    state: InventoryState,
    text: InventoryText,
}

impl InventoryScene {
    pub fn new() -> Self {
        Self {
            state: InventoryState::new_mvp(),
            text: InventoryText::default(),
        }
    }

    fn draw_inventory(&mut self, ctx: &mut RenderContext<'_>) {
        let viewport = ctx.renderer.screen_size();
        let layout = InventoryLayout::new(viewport);

        ctx.renderer.draw_rect(
            screen_rect(viewport, 0.0, 0.0, viewport.x, viewport.y),
            Color::rgba(0.0, 0.0, 0.0, 0.86),
        );

        self.draw_tabs(ctx.renderer, viewport, &layout);
        self.draw_backpack(ctx.renderer, viewport, &layout);
        self.draw_quickbar(ctx.renderer, viewport, &layout);
        self.draw_details(ctx.renderer, viewport, &layout);
    }

    fn draw_tabs(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        for (index, category) in ItemCategory::ALL.iter().copied().enumerate() {
            let rect = layout.tab_rect(index);
            let active = category == self.state.active_category;
            renderer.draw_image(
                if active {
                    "ui_inventory_tab_active"
                } else {
                    "ui_inventory_tab_inactive"
                },
                rect,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );

            let icon_size = 22.0 * layout.scale;
            renderer.draw_image(
                category.icon_texture_id(),
                Rect::new(
                    Vec2::new(
                        rect.origin.x + 8.0 * layout.scale,
                        rect.origin.y + 7.0 * layout.scale,
                    ),
                    Vec2::new(icon_size, icon_size),
                ),
                Color::rgba(1.0, 1.0, 1.0, if active { 1.0 } else { 0.64 }),
            );

            if let Some(label) = self.text.tab_labels.get(&category) {
                draw_text(
                    renderer,
                    label,
                    viewport,
                    screen_x(viewport, rect.origin.x + 34.0 * layout.scale),
                    screen_y(viewport, rect.origin.y + 6.0 * layout.scale),
                    if active {
                        Color::rgba(0.88, 1.0, 0.98, 1.0)
                    } else {
                        Color::rgba(0.50, 0.64, 0.70, 0.92)
                    },
                );
            }
        }
    }

    fn draw_backpack(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        renderer.draw_image(
            "ui_inventory_panel_backpack",
            layout.backpack_panel,
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );

        if let Some(title) = &self.text.title {
            draw_text(
                renderer,
                title,
                viewport,
                screen_x(
                    viewport,
                    layout.backpack_panel.origin.x + 26.0 * layout.scale,
                ),
                screen_y(
                    viewport,
                    layout.backpack_panel.origin.y + 16.0 * layout.scale,
                ),
                Color::rgba(0.72, 0.94, 1.0, 1.0),
            );
        }

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
        renderer.draw_image(
            "ui_inventory_panel_quickbar",
            layout.quickbar_panel,
            Color::rgba(1.0, 1.0, 1.0, 1.0),
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
                    screen_x(viewport, rect.origin.x + 5.0 * layout.scale),
                    screen_y(viewport, rect.origin.y + 3.0 * layout.scale),
                    Color::rgba(0.62, 0.80, 0.86, 0.95),
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
        let base_texture = if stack.is_some_and(|stack| stack.locked) {
            "ui_inventory_slot_locked"
        } else if quickbar {
            "ui_inventory_slot_quickbar"
        } else {
            "ui_inventory_slot_empty"
        };

        renderer.draw_image(base_texture, rect, Color::rgba(1.0, 1.0, 1.0, 1.0));

        if let Some(stack) = stack {
            if let Some(definition) = item_definition(stack.item_id) {
                let inset = 8.0 * layout.scale;
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
                        rect.origin.x + rect.size.x - 30.0 * layout.scale,
                        rect.origin.y + rect.size.y - 20.0 * layout.scale,
                    ),
                    Vec2::new(28.0 * layout.scale, 18.0 * layout.scale),
                );
                renderer.draw_image(
                    "ui_inventory_badge_count",
                    badge,
                    Color::rgba(1.0, 1.0, 1.0, 0.96),
                );

                if let Some(count) = self.text.count_badges.get(&slot_index) {
                    draw_text_centered(
                        renderer,
                        count,
                        viewport,
                        screen_x(viewport, badge.origin.x + badge.size.x * 0.5),
                        screen_y(viewport, badge.origin.y - 1.0 * layout.scale),
                        Color::rgba(0.92, 1.0, 0.96, 1.0),
                    );
                }
            }
        }

        if slot_index == self.state.selected_slot {
            renderer.draw_image(
                "ui_inventory_slot_selected",
                rect,
                Color::rgba(0.72, 1.0, 1.0, 1.0),
            );
        }
    }

    fn draw_details(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &InventoryLayout) {
        renderer.draw_image(
            "ui_inventory_panel_details",
            layout.details_panel,
            Color::rgba(1.0, 1.0, 1.0, 1.0),
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
                    screen_x(viewport, origin.x + 38.0 * layout.scale),
                    screen_y(viewport, origin.y + 146.0 * layout.scale),
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
                origin.x + 32.0 * layout.scale,
                origin.y + 38.0 * layout.scale,
            ),
            Vec2::new(86.0 * layout.scale, 86.0 * layout.scale),
        );
        renderer.draw_image(
            definition.texture_id,
            icon_rect,
            Color::rgba(1.0, 1.0, 1.0, if stack.locked { 0.78 } else { 1.0 }),
        );
        renderer.draw_image(
            definition.rarity.texture_id(),
            Rect::new(
                Vec2::new(
                    origin.x + 34.0 * layout.scale,
                    origin.y + 128.0 * layout.scale,
                ),
                Vec2::new(96.0 * layout.scale, 8.0 * layout.scale),
            ),
            Color::rgba(1.0, 1.0, 1.0, 0.95),
        );

        let Some(detail) = self.text.item_details.get(definition.id) else {
            return;
        };

        let text_x = screen_x(viewport, origin.x + 134.0 * layout.scale);
        draw_text(
            renderer,
            &detail.name,
            viewport,
            text_x,
            screen_y(viewport, origin.y + 40.0 * layout.scale),
            Color::rgba(0.90, 1.0, 0.98, 1.0),
        );
        draw_text(
            renderer,
            &detail.category,
            viewport,
            text_x,
            screen_y(viewport, origin.y + 80.0 * layout.scale),
            Color::rgba(0.52, 0.76, 0.84, 1.0),
        );
        draw_text(
            renderer,
            &detail.rarity,
            viewport,
            text_x,
            screen_y(viewport, origin.y + 108.0 * layout.scale),
            rarity_color(definition.rarity),
        );

        let line_x = screen_x(viewport, origin.x + 38.0 * layout.scale);
        let mut line_y = origin.y + 174.0 * layout.scale;
        for line in [&detail.quantity, &detail.stack_limit, &detail.status] {
            draw_text(
                renderer,
                line,
                viewport,
                line_x,
                screen_y(viewport, line_y),
                Color::rgba(0.70, 0.88, 0.92, 0.96),
            );
            line_y += 32.0 * layout.scale;
        }

        if let Some(lock_state) = &detail.lock_state {
            draw_text(
                renderer,
                lock_state,
                viewport,
                line_x,
                screen_y(viewport, line_y + 12.0 * layout.scale),
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
                world_rect_to_screen(viewport, layout.tab_rect(index)),
            ) {
                self.select_first_in_category(category);
                return;
            }
        }
    }

    fn upload_textures(&mut self, renderer: &mut dyn Renderer, font: &Font<'static>) -> Result<()> {
        self.text.title = Some(upload_text(
            renderer,
            font,
            "inventory_text_title",
            "Inventory",
            26.0,
        )?);
        self.text.empty_detail = Some(upload_text(
            renderer,
            font,
            "inventory_text_empty_detail",
            "Empty slot",
            24.0,
        )?);

        for category in ItemCategory::ALL {
            self.text.tab_labels.insert(
                category,
                upload_text(
                    renderer,
                    font,
                    &format!(
                        "inventory_text_tab_{}",
                        category.label().to_ascii_lowercase()
                    ),
                    category.label(),
                    14.0,
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
                            12.0,
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
                    12.0,
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
                        definition.name,
                        25.0,
                    )?,
                    category: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_category_{}", definition.id),
                        definition.category.label(),
                        17.0,
                    )?,
                    quantity: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_quantity_{}", definition.id),
                        &format!(
                            "Quantity: {}",
                            quantity_for_item(&self.state, definition.id)
                        ),
                        17.0,
                    )?,
                    rarity: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_rarity_{}", definition.id),
                        definition.rarity.label(),
                        17.0,
                    )?,
                    stack_limit: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_stack_{}", definition.id),
                        &format!("Max Stack: {}", definition.max_stack),
                        17.0,
                    )?,
                    status: upload_text(
                        renderer,
                        font,
                        &format!("inventory_text_status_{}", definition.id),
                        &format!("Research: {}%", definition.research),
                        17.0,
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
                                "Locked Item",
                                17.0,
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

        for definition in ITEM_DEFINITIONS {
            if renderer.texture_size(definition.texture_id).is_none() {
                renderer
                    .load_texture(definition.texture_id, Path::new(definition.icon_path))
                    .with_context(|| format!("failed to load inventory item {}", definition.id))?;
            }
        }

        let font = load_ui_font()?;
        self.upload_textures(renderer, &font)
    }

    fn update(
        &mut self,
        _ctx: &mut GameContext,
        _dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if input.just_pressed(Button::Pause) || input.just_pressed(Button::Inventory) {
            return Ok(SceneCommand::Pop);
        }

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

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.draw_inventory(ctx);
        Ok(())
    }
}

struct InventoryLayout {
    scale: f32,
    tabs_origin: Vec2,
    backpack_panel: Rect,
    details_panel: Rect,
    quickbar_panel: Rect,
}

impl InventoryLayout {
    fn new(viewport: Vec2) -> Self {
        let total_width = BACKPACK_PANEL_SIZE.x + CONTENT_GAP + DETAILS_PANEL_SIZE.x;
        let total_height = TAB_HEIGHT + 12.0 + BACKPACK_PANEL_SIZE.y + 16.0 + QUICKBAR_PANEL_SIZE.y;
        let scale = ((viewport.x - 48.0) / total_width)
            .min((viewport.y - 48.0) / total_height)
            .min(1.0)
            .max(0.64);
        let origin = Vec2::new(-total_width * scale * 0.5, -total_height * scale * 0.5);
        let tabs_origin = origin;
        let backpack_origin = Vec2::new(origin.x, origin.y + (TAB_HEIGHT + 12.0) * scale);
        let details_origin = Vec2::new(
            backpack_origin.x + (BACKPACK_PANEL_SIZE.x + CONTENT_GAP) * scale,
            backpack_origin.y,
        );
        let quickbar_origin = Vec2::new(
            backpack_origin.x,
            backpack_origin.y + (BACKPACK_PANEL_SIZE.y + 16.0) * scale,
        );

        Self {
            scale,
            tabs_origin,
            backpack_panel: Rect::new(backpack_origin, BACKPACK_PANEL_SIZE * scale),
            details_panel: Rect::new(details_origin, DETAILS_PANEL_SIZE * scale),
            quickbar_panel: Rect::new(quickbar_origin, QUICKBAR_PANEL_SIZE * scale),
        }
    }

    fn tab_rect(&self, index: usize) -> Rect {
        Rect::new(
            Vec2::new(
                self.tabs_origin.x + index as f32 * (TAB_WIDTH + TAB_GAP) * self.scale,
                self.tabs_origin.y,
            ),
            Vec2::new(TAB_WIDTH * self.scale, TAB_HEIGHT * self.scale),
        )
    }

    fn backpack_slot_rect(&self, slot_index: usize) -> Rect {
        let column = slot_index % BACKPACK_COLUMNS;
        let row = slot_index / BACKPACK_COLUMNS;
        self.slot_rect(
            self.backpack_panel.origin + Vec2::new(16.0 * self.scale, 64.0 * self.scale),
            column,
            row,
        )
    }

    fn quickbar_slot_rect(&self, quick_index: usize) -> Rect {
        self.slot_rect(
            self.quickbar_panel.origin + Vec2::new(16.0 * self.scale, 12.0 * self.scale),
            quick_index,
            0,
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

fn item_definition(item_id: &str) -> Option<&'static ItemDefinition> {
    ITEM_DEFINITIONS
        .iter()
        .find(|definition| definition.id == item_id)
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

fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::Common => Color::rgba(0.72, 0.82, 0.86, 1.0),
        Rarity::Uncommon => Color::rgba(0.58, 0.95, 0.72, 1.0),
        Rarity::Rare => Color::rgba(0.42, 0.78, 1.0, 1.0),
        Rarity::Artifact => Color::rgba(0.98, 0.78, 0.42, 1.0),
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
}
