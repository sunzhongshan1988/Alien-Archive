use anyhow::Result;
use content::items;
use runtime::{Color, Rect, Renderer, Vec2};
use rusttype::Font;

use crate::{
    save::InventorySave,
    ui::{
        game_menu_content::{
            category_label, locked_label, quantity_label, rarity_label, research_label,
            stack_limit_label,
        },
        menu_style::inset_rect,
        menu_widgets::{draw_border, draw_screen_rect, screen_rect},
        text::{TextSprite, draw_text, upload_text},
    },
};

use super::{Language, inventory_scene};

pub(super) struct InventoryDetailText {
    pub name: TextSprite,
    pub category: TextSprite,
    pub quantity: TextSprite,
    pub rarity: TextSprite,
    pub stack_limit: TextSprite,
    pub research: TextSprite,
    pub lock_state: Option<TextSprite>,
}

pub(super) fn inventory_capacity(inventory: &InventorySave) -> (usize, usize) {
    let slots = inventory_scene::inventory_slots(inventory);
    let used = slots.iter().filter(|slot| slot.is_some()).count();
    (used, slots.len())
}

pub(super) fn upload_inventory_slot_counts(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    inventory: &InventorySave,
) -> Result<Vec<Option<TextSprite>>> {
    inventory_scene::inventory_slots(inventory)
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
        .collect()
}

pub(super) fn upload_inventory_slot_details(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    language: Language,
    inventory: &InventorySave,
) -> Result<Vec<Option<InventoryDetailText>>> {
    let slots = inventory_scene::inventory_slots(inventory);
    let mut details = Vec::with_capacity(slots.len());
    for (index, item) in slots.into_iter().enumerate() {
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

pub(super) fn equipment_module_slots(inventory: &InventorySave) -> Vec<usize> {
    inventory
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, stack)| {
            let stack = stack.as_ref()?;
            items::is_equipment_module(&stack.item_id, stack.locked).then_some(index)
        })
        .take(4)
        .collect()
}

pub(super) fn draw_inventory_slot(
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

#[cfg(test)]
mod tests {
    use crate::save::ItemStackSave;

    use super::*;

    #[test]
    fn inventory_capacity_counts_visible_slots() {
        let inventory = InventorySave::default();

        assert_eq!(inventory_capacity(&inventory), (12, 24));
    }

    #[test]
    fn equipment_module_slots_include_locked_items_and_limit_to_four() {
        let inventory = InventorySave {
            slots: vec![
                Some(ItemStackSave::new("med_injector", 1, false)),
                Some(ItemStackSave::new("locked_module_a", 1, true)),
                Some(ItemStackSave::new("locked_module_b", 1, true)),
                Some(ItemStackSave::new("locked_module_c", 1, true)),
                Some(ItemStackSave::new("locked_module_d", 1, true)),
                Some(ItemStackSave::new("locked_module_e", 1, true)),
            ],
            ..InventorySave::default()
        };

        assert_eq!(equipment_module_slots(&inventory), vec![1, 2, 3, 4]);
    }
}
