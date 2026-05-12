use content::semantics;

use crate::world::MapEntity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ItemReward {
    pub(super) item_id: &'static str,
    pub(super) quantity: u32,
    pub(super) locked: bool,
}

impl ItemReward {
    const fn new(item_id: &'static str, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            locked: false,
        }
    }

    const fn locked(item_id: &'static str, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            locked: true,
        }
    }
}

pub(super) fn pickup_reward_for_entity(entity: &MapEntity) -> Option<ItemReward> {
    let asset_id = entity.asset_id.as_deref()?;
    pickup_reward_for_asset(asset_id)
}

pub(super) fn pickup_reward_for_asset(asset_id: &str) -> Option<ItemReward> {
    match asset_id {
        "ow_pickup_bio_sample" => Some(ItemReward::new(semantics::ITEM_BIO_SAMPLE_VIAL, 1)),
        "ow_pickup_crystal_sample" => {
            Some(ItemReward::new(semantics::ITEM_ALIEN_CRYSTAL_SAMPLE, 1))
        }
        "ow_pickup_data_shard" => Some(ItemReward::new(semantics::ITEM_DATA_SHARD, 1)),
        "ow_pickup_energy_cell" => Some(ItemReward::new(semantics::ITEM_ENERGY_CELL, 1)),
        "ow_pickup_ruin_key" => Some(ItemReward::locked(semantics::ITEM_RUIN_KEY, 1)),
        "ow_pickup_scrap_part" => Some(ItemReward::new(semantics::ITEM_SCRAP_PART, 3)),
        "ow_pickup_signal_chip" => Some(ItemReward::new(semantics::ITEM_DATA_SHARD, 2)),
        _ => None,
    }
}

pub(super) fn scan_reward_for_codex(codex_id: &str) -> Option<ItemReward> {
    match codex_id {
        "codex.flora.glowfungus" => Some(ItemReward::new(semantics::ITEM_GLOW_FUNGUS_SAMPLE, 1)),
        id if id.starts_with("codex.flora.") => {
            Some(ItemReward::new(semantics::ITEM_BIO_SAMPLE_VIAL, 1))
        }
        id if id.contains("generator") || id.contains("power_node") => {
            Some(ItemReward::new(semantics::ITEM_ENERGY_CELL, 1))
        }
        id if id.contains("terminal") || id.contains("data") || id.contains("signal") => {
            Some(ItemReward::new(semantics::ITEM_DATA_SHARD, 1))
        }
        id if id.contains("locked_door") || id.contains("gate") => {
            Some(ItemReward::locked(semantics::ITEM_RUIN_KEY, 1))
        }
        id if id.starts_with("codex.ruin.") || id.starts_with("ruin.") => {
            Some(ItemReward::new(semantics::ITEM_DATA_SHARD, 1))
        }
        _ => None,
    }
}

pub(super) fn research_meter_for_codex(codex_id: &str) -> &'static str {
    if codex_id.starts_with("codex.flora.") {
        semantics::METER_BIO
    } else if codex_id.contains("mineral") {
        semantics::METER_MINERAL
    } else if codex_id.starts_with("codex.ruin.") || codex_id.starts_with("ruin.") {
        semantics::METER_RUIN
    } else {
        semantics::METER_DATA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pickup_assets_map_to_inventory_items() {
        assert_eq!(
            pickup_reward_for_asset("ow_pickup_bio_sample").map(|reward| reward.item_id),
            Some("bio_sample_vial")
        );
        assert_eq!(
            pickup_reward_for_asset("ow_pickup_ruin_key"),
            Some(ItemReward::locked("ruin_key", 1))
        );
    }

    #[test]
    fn scan_codex_ids_map_to_research_and_rewards() {
        assert_eq!(research_meter_for_codex("codex.flora.glowfungus"), "bio");
        assert_eq!(research_meter_for_codex("codex.ruin.terminal"), "ruin");
        assert_eq!(
            scan_reward_for_codex("codex.interact.generator").map(|reward| reward.item_id),
            Some("energy_cell")
        );
    }
}
