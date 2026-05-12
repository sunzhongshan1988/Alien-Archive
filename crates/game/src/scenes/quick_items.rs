use content::items;

use crate::save::{InventorySave, PlayerProfileSave};

#[derive(Clone, Debug, PartialEq)]
pub enum QuickItemUseResult {
    Empty,
    NotUsable {
        item_id: String,
    },
    AlreadyFull {
        item_id: String,
        meter_id: String,
    },
    Used {
        item_id: String,
        meter_id: &'static str,
        amount: u32,
    },
}

pub(super) fn select_quickbar_slot(inventory: &mut InventorySave, quick_index: usize) -> bool {
    let Some(slot_count) = (!inventory.slots.is_empty()).then_some(inventory.slots.len()) else {
        return false;
    };
    let slot_index = quickbar_slot_index(inventory, quick_index);
    if slot_index >= slot_count || inventory.selected_slot == slot_index {
        return false;
    }

    inventory.selected_slot = slot_index;
    true
}

pub(super) fn use_selected_quickbar_item(
    inventory: &mut InventorySave,
    profile: &mut PlayerProfileSave,
) -> QuickItemUseResult {
    let slot_index = inventory.selected_slot;
    let Some(stack) = inventory
        .slots
        .get(slot_index)
        .and_then(|slot| slot.as_ref())
    else {
        return QuickItemUseResult::Empty;
    };
    let item_id = stack.item_id.clone();

    let Some(effect) = items::consumable_effect(&item_id) else {
        return QuickItemUseResult::NotUsable { item_id };
    };
    let meter_id = effect.meter_id.as_str();
    if profile_meter_value(profile, meter_id) >= profile_meter_max(profile, meter_id) {
        return QuickItemUseResult::AlreadyFull {
            item_id,
            meter_id: meter_id.to_owned(),
        };
    }

    profile.add_meter_delta(meter_id, effect.amount as i32);
    inventory.consume_slot(slot_index, 1);
    QuickItemUseResult::Used {
        item_id,
        meter_id,
        amount: effect.amount,
    }
}

pub(super) fn quickbar_slot_index(inventory: &InventorySave, quick_index: usize) -> usize {
    inventory
        .quickbar
        .get(quick_index)
        .and_then(|slot| *slot)
        .unwrap_or(quick_index)
}

fn profile_meter_value(profile: &PlayerProfileSave, id: &str) -> u32 {
    profile.meter(id).map_or(0, |meter| meter.value)
}

fn profile_meter_max(profile: &PlayerProfileSave, id: &str) -> u32 {
    profile.meter(id).map_or(0, |meter| meter.max)
}

#[cfg(test)]
mod tests {
    use content::semantics;

    use super::*;

    #[test]
    fn quickbar_selection_uses_saved_mapping_and_rejects_invalid_slots() {
        let mut inventory = InventorySave {
            quickbar: vec![Some(7), None, Some(99)],
            selected_slot: 0,
            ..InventorySave::default()
        };

        assert!(select_quickbar_slot(&mut inventory, 0));
        assert_eq!(inventory.selected_slot, 7);
        assert!(!select_quickbar_slot(&mut inventory, 0));
        assert!(select_quickbar_slot(&mut inventory, 1));
        assert_eq!(inventory.selected_slot, 1);
        assert!(!select_quickbar_slot(&mut inventory, 2));
    }

    #[test]
    fn usable_quick_item_updates_profile_and_consumes_stack() {
        let mut inventory = InventorySave::default();
        let mut profile = PlayerProfileSave::default();
        inventory.selected_slot = 7;
        profile.set_meter_value(semantics::METER_HEALTH, 50);

        let result = use_selected_quickbar_item(&mut inventory, &mut profile);

        assert_eq!(
            result,
            QuickItemUseResult::Used {
                item_id: "med_injector".to_owned(),
                meter_id: semantics::METER_HEALTH,
                amount: 35,
            }
        );
        assert_eq!(
            profile
                .meter(semantics::METER_HEALTH)
                .map(|meter| meter.value),
            Some(85)
        );
        assert_eq!(
            inventory.slots[7].as_ref().map(|stack| stack.quantity),
            Some(1)
        );
    }

    #[test]
    fn full_meter_does_not_spend_quick_item() {
        let mut inventory = InventorySave::default();
        let mut profile = PlayerProfileSave::default();
        inventory.selected_slot = 7;
        profile.set_meter_value(semantics::METER_HEALTH, 100);

        let result = use_selected_quickbar_item(&mut inventory, &mut profile);

        assert_eq!(
            result,
            QuickItemUseResult::AlreadyFull {
                item_id: "med_injector".to_owned(),
                meter_id: semantics::METER_HEALTH.to_owned(),
            }
        );
        assert_eq!(
            inventory.slots[7].as_ref().map(|stack| stack.quantity),
            Some(2)
        );
    }
}
