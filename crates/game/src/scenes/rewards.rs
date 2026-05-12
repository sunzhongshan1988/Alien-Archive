use content::items;

use crate::world::MapEntity;

pub(super) type ItemReward = items::ItemRewardDef;

pub(super) fn pickup_reward_for_entity(entity: &MapEntity) -> Option<ItemReward> {
    let asset_id = entity.asset_id.as_deref()?;
    pickup_reward_for_asset(asset_id)
}

pub(super) fn pickup_reward_for_asset(asset_id: &str) -> Option<ItemReward> {
    items::pickup_reward_for_asset(asset_id)
}

pub(super) fn scan_reward_for_codex(codex_id: &str) -> Option<ItemReward> {
    items::scan_reward_for_codex(codex_id)
}

pub(super) fn research_meter_for_codex(codex_id: &str) -> &'static str {
    items::research_meter_for_codex(codex_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    use content::{AssetDatabase, AssetKind};

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
        assert_eq!(
            pickup_reward_for_asset("ow_pickup_gen_ls_energy_cell").map(|reward| reward.item_id),
            Some("energy_cell")
        );
        assert_eq!(
            pickup_reward_for_asset("ow_pickup_exp2_medical_injector_case")
                .map(|reward| reward.item_id),
            Some("med_injector")
        );
    }

    #[test]
    fn bundled_pickup_assets_have_real_inventory_rewards() {
        let database =
            AssetDatabase::load(&workspace_root().join("assets/data/assets/overworld_assets.ron"))
                .expect("asset database should load");
        let pickup_assets = database
            .assets()
            .iter()
            .filter(|asset| {
                asset.kind == AssetKind::Entity
                    && (asset.category == "pickups" || asset.tags.iter().any(|tag| tag == "pickup"))
            })
            .collect::<Vec<_>>();

        assert!(!pickup_assets.is_empty());
        for asset in pickup_assets {
            let reward = pickup_reward_for_asset(&asset.id)
                .unwrap_or_else(|| panic!("pickup asset {} has no reward mapping", asset.id));
            assert!(
                items::item_max_stack(reward.item_id).is_some(),
                "pickup asset {} maps to unknown item {}",
                asset.id,
                reward.item_id
            );
            assert!(reward.quantity > 0);
        }
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("game crate should live under workspace/crates/game")
            .to_path_buf()
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
