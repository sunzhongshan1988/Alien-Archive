use crate::semantics;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalizedTextDef {
    pub english: &'static str,
    pub chinese: &'static str,
}

impl LocalizedTextDef {
    pub const fn new(english: &'static str, chinese: &'static str) -> Self {
        Self { english, chinese }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ItemCategory {
    Samples,
    Components,
    Artifacts,
    Consumables,
    Tools,
    Data,
}

impl ItemCategory {
    pub const ALL: [Self; 6] = [
        Self::Samples,
        Self::Components,
        Self::Artifacts,
        Self::Consumables,
        Self::Tools,
        Self::Data,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Samples => "samples",
            Self::Components => "components",
            Self::Artifacts => "artifacts",
            Self::Consumables => "consumables",
            Self::Tools => "tools",
            Self::Data => "data",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|category| category.key() == value)
    }

    pub const fn label(self) -> LocalizedTextDef {
        match self {
            Self::Samples => LocalizedTextDef::new("Samples", "样本"),
            Self::Components => LocalizedTextDef::new("Components", "组件"),
            Self::Artifacts => LocalizedTextDef::new("Artifacts", "遗物"),
            Self::Consumables => LocalizedTextDef::new("Consumables", "消耗品"),
            Self::Tools => LocalizedTextDef::new("Tools", "工具"),
            Self::Data => LocalizedTextDef::new("Data", "数据"),
        }
    }

    pub const fn icon_texture_id(self) -> &'static str {
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
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Artifact,
}

impl ItemRarity {
    pub const ALL: [Self; 4] = [Self::Common, Self::Uncommon, Self::Rare, Self::Artifact];

    pub const fn label(self) -> LocalizedTextDef {
        match self {
            Self::Common => LocalizedTextDef::new("Common", "普通"),
            Self::Uncommon => LocalizedTextDef::new("Uncommon", "优秀"),
            Self::Rare => LocalizedTextDef::new("Rare", "稀有"),
            Self::Artifact => LocalizedTextDef::new("Artifact", "遗物"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumableEffectDef {
    pub meter_id: &'static str,
    pub amount: u32,
}

impl ConsumableEffectDef {
    pub const fn new(meter_id: &'static str, amount: u32) -> Self {
        Self { meter_id, amount }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemDef {
    pub id: &'static str,
    pub name: LocalizedTextDef,
    pub category: ItemCategory,
    pub texture_id: &'static str,
    pub icon_path: &'static str,
    pub max_stack: u32,
    pub weight: u32,
    pub rarity: ItemRarity,
    pub research: u32,
    pub equipment_module: bool,
    pub consumable: Option<ConsumableEffectDef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemRewardDef {
    pub item_id: &'static str,
    pub quantity: u32,
    pub locked: bool,
}

impl ItemRewardDef {
    pub const fn new(item_id: &'static str, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            locked: false,
        }
    }

    pub const fn locked(item_id: &'static str, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            locked: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PickupRewardRule {
    pub asset_id: &'static str,
    pub reward: ItemRewardDef,
}

pub const ITEM_DEFS: &[ItemDef] = &[
    ItemDef {
        id: semantics::ITEM_ALIEN_CRYSTAL_SAMPLE,
        name: LocalizedTextDef::new("Crystal Sample", "晶体样本"),
        category: ItemCategory::Samples,
        texture_id: "item_alien_crystal_sample",
        icon_path: "assets/images/ui/inventory/items/item_alien_crystal_sample.png",
        max_stack: 10,
        weight: 2,
        rarity: ItemRarity::Rare,
        research: 42,
        equipment_module: false,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_BIO_SAMPLE_VIAL,
        name: LocalizedTextDef::new("Bio Sample", "生物样本"),
        category: ItemCategory::Samples,
        texture_id: "item_bio_sample_vial",
        icon_path: "assets/images/ui/inventory/items/item_bio_sample_vial.png",
        max_stack: 10,
        weight: 1,
        rarity: ItemRarity::Uncommon,
        research: 18,
        equipment_module: false,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_DATA_SHARD,
        name: LocalizedTextDef::new("Data Shard", "数据碎片"),
        category: ItemCategory::Data,
        texture_id: "item_data_shard",
        icon_path: "assets/images/ui/inventory/items/item_data_shard.png",
        max_stack: 99,
        weight: 0,
        rarity: ItemRarity::Rare,
        research: 66,
        equipment_module: false,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_ENERGY_CELL,
        name: LocalizedTextDef::new("Energy Cell", "能量电池"),
        category: ItemCategory::Consumables,
        texture_id: "item_energy_cell",
        icon_path: "assets/images/ui/inventory/items/item_energy_cell.png",
        max_stack: 20,
        weight: 1,
        rarity: ItemRarity::Uncommon,
        research: 35,
        equipment_module: false,
        consumable: Some(ConsumableEffectDef::new(semantics::METER_STAMINA, 35)),
    },
    ItemDef {
        id: semantics::ITEM_SCRAP_PART,
        name: LocalizedTextDef::new("Scrap Part", "废料零件"),
        category: ItemCategory::Components,
        texture_id: "item_scrap_part",
        icon_path: "assets/images/ui/inventory/items/item_scrap_part.png",
        max_stack: 99,
        weight: 1,
        rarity: ItemRarity::Common,
        research: 8,
        equipment_module: false,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_RUIN_KEY,
        name: LocalizedTextDef::new("Ruin Key", "遗迹钥匙"),
        category: ItemCategory::Artifacts,
        texture_id: "item_ruin_key",
        icon_path: "assets/images/ui/inventory/items/item_ruin_key.png",
        max_stack: 1,
        weight: 1,
        rarity: ItemRarity::Artifact,
        research: 91,
        equipment_module: true,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_SCANNER_TOOL,
        name: LocalizedTextDef::new("Scanner", "扫描器"),
        category: ItemCategory::Tools,
        texture_id: "item_scanner_tool",
        icon_path: "assets/images/ui/inventory/items/item_scanner_tool.png",
        max_stack: 1,
        weight: 3,
        rarity: ItemRarity::Uncommon,
        research: 100,
        equipment_module: true,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_MED_INJECTOR,
        name: LocalizedTextDef::new("Med Injector", "医疗注射器"),
        category: ItemCategory::Consumables,
        texture_id: "item_med_injector",
        icon_path: "assets/images/ui/inventory/items/item_med_injector.png",
        max_stack: 5,
        weight: 1,
        rarity: ItemRarity::Common,
        research: 0,
        equipment_module: false,
        consumable: Some(ConsumableEffectDef::new(semantics::METER_HEALTH, 35)),
    },
    ItemDef {
        id: semantics::ITEM_COOLANT_CANISTER,
        name: LocalizedTextDef::new("Coolant Canister", "冷却罐"),
        category: ItemCategory::Consumables,
        texture_id: "item_coolant_canister",
        icon_path: "assets/images/ui/inventory/items/item_coolant_canister.png",
        max_stack: 10,
        weight: 3,
        rarity: ItemRarity::Uncommon,
        research: 24,
        equipment_module: false,
        consumable: Some(ConsumableEffectDef::new(semantics::METER_SUIT, 30)),
    },
    ItemDef {
        id: semantics::ITEM_METAL_FRAGMENT,
        name: LocalizedTextDef::new("Metal Fragment", "金属碎片"),
        category: ItemCategory::Components,
        texture_id: "item_metal_fragment",
        icon_path: "assets/images/ui/inventory/items/item_metal_fragment.png",
        max_stack: 99,
        weight: 1,
        rarity: ItemRarity::Common,
        research: 12,
        equipment_module: false,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_GLOW_FUNGUS_SAMPLE,
        name: LocalizedTextDef::new("Glow Fungus", "发光菌样本"),
        category: ItemCategory::Samples,
        texture_id: "item_glow_fungus_sample",
        icon_path: "assets/images/ui/inventory/items/item_glow_fungus_sample.png",
        max_stack: 10,
        weight: 1,
        rarity: ItemRarity::Uncommon,
        research: 54,
        equipment_module: false,
        consumable: None,
    },
    ItemDef {
        id: semantics::ITEM_ARTIFACT_CORE,
        name: LocalizedTextDef::new("Artifact Core", "遗物核心"),
        category: ItemCategory::Artifacts,
        texture_id: "item_artifact_core",
        icon_path: "assets/images/ui/inventory/items/item_artifact_core.png",
        max_stack: 1,
        weight: 4,
        rarity: ItemRarity::Artifact,
        research: 73,
        equipment_module: true,
        consumable: None,
    },
];

pub const DEFAULT_INVENTORY_STACKS: &[ItemRewardDef] = &[
    ItemRewardDef::new(semantics::ITEM_ALIEN_CRYSTAL_SAMPLE, 3),
    ItemRewardDef::new(semantics::ITEM_BIO_SAMPLE_VIAL, 2),
    ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 8),
    ItemRewardDef::new(semantics::ITEM_ENERGY_CELL, 4),
    ItemRewardDef::new(semantics::ITEM_SCRAP_PART, 17),
    ItemRewardDef::locked(semantics::ITEM_RUIN_KEY, 1),
    ItemRewardDef::locked(semantics::ITEM_SCANNER_TOOL, 1),
    ItemRewardDef::new(semantics::ITEM_MED_INJECTOR, 2),
    ItemRewardDef::new(semantics::ITEM_COOLANT_CANISTER, 1),
    ItemRewardDef::new(semantics::ITEM_METAL_FRAGMENT, 9),
    ItemRewardDef::new(semantics::ITEM_GLOW_FUNGUS_SAMPLE, 2),
    ItemRewardDef::locked(semantics::ITEM_ARTIFACT_CORE, 1),
];

pub const DEFAULT_QUICKBAR_SLOTS: &[Option<usize>] =
    &[Some(6), Some(7), Some(3), Some(2), Some(5), Some(11)];

pub const PICKUP_REWARD_RULES: &[PickupRewardRule] = &[
    PickupRewardRule {
        asset_id: "ow_pickup_bio_sample",
        reward: ItemRewardDef::new(semantics::ITEM_BIO_SAMPLE_VIAL, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_crystal_sample",
        reward: ItemRewardDef::new(semantics::ITEM_ALIEN_CRYSTAL_SAMPLE, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_data_shard",
        reward: ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_energy_cell",
        reward: ItemRewardDef::new(semantics::ITEM_ENERGY_CELL, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_ruin_key",
        reward: ItemRewardDef::locked(semantics::ITEM_RUIN_KEY, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_scrap_part",
        reward: ItemRewardDef::new(semantics::ITEM_SCRAP_PART, 3),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_signal_chip",
        reward: ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_bio_sample_vial",
        reward: ItemRewardDef::new(semantics::ITEM_BIO_SAMPLE_VIAL, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_crystal_sample_cluster",
        reward: ItemRewardDef::new(semantics::ITEM_ALIEN_CRYSTAL_SAMPLE, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_data_shard_cluster",
        reward: ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_energy_cell",
        reward: ItemRewardDef::new(semantics::ITEM_ENERGY_CELL, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_ruin_key_tablet",
        reward: ItemRewardDef::locked(semantics::ITEM_RUIN_KEY, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_scrap_part_pile",
        reward: ItemRewardDef::new(semantics::ITEM_SCRAP_PART, 3),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_gen_ls_signal_chip_pad",
        reward: ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_exp2_data_shard_cluster",
        reward: ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_exp2_energy_cell_canister",
        reward: ItemRewardDef::new(semantics::ITEM_ENERGY_CELL, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_exp2_medical_injector_case",
        reward: ItemRewardDef::new(semantics::ITEM_MED_INJECTOR, 1),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_exp2_mineral_sample_container",
        reward: ItemRewardDef::new(semantics::ITEM_METAL_FRAGMENT, 2),
    },
    PickupRewardRule {
        asset_id: "ow_pickup_exp2_signal_chip_pad",
        reward: ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 2),
    },
];

pub fn item_def(item_id: &str) -> Option<&'static ItemDef> {
    ITEM_DEFS.iter().find(|definition| definition.id == item_id)
}

pub fn item_max_stack(item_id: &str) -> Option<u32> {
    item_def(item_id).map(|definition| definition.max_stack)
}

pub fn item_weight(item_id: &str) -> u32 {
    item_def(item_id).map_or(1, |definition| definition.weight)
}

pub fn item_name(item_id: &str) -> Option<LocalizedTextDef> {
    item_def(item_id).map(|definition| definition.name)
}

pub fn is_equipment_module(item_id: &str, locked: bool) -> bool {
    locked || item_def(item_id).is_some_and(|definition| definition.equipment_module)
}

pub fn consumable_effect(item_id: &str) -> Option<ConsumableEffectDef> {
    item_def(item_id).and_then(|definition| definition.consumable)
}

pub fn pickup_reward_for_asset(asset_id: &str) -> Option<ItemRewardDef> {
    PICKUP_REWARD_RULES
        .iter()
        .find(|rule| rule.asset_id == asset_id)
        .map(|rule| rule.reward)
}

pub fn scan_reward_for_codex(codex_id: &str) -> Option<ItemRewardDef> {
    match codex_id {
        "codex.flora.glowfungus" => Some(ItemRewardDef::new(semantics::ITEM_GLOW_FUNGUS_SAMPLE, 1)),
        id if id.starts_with("codex.flora.") => {
            Some(ItemRewardDef::new(semantics::ITEM_BIO_SAMPLE_VIAL, 1))
        }
        id if id.contains("generator") || id.contains("power_node") => {
            Some(ItemRewardDef::new(semantics::ITEM_ENERGY_CELL, 1))
        }
        id if id.contains("terminal") || id.contains("data") || id.contains("signal") => {
            Some(ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 1))
        }
        id if id.contains("locked_door") || id.contains("gate") => {
            Some(ItemRewardDef::locked(semantics::ITEM_RUIN_KEY, 1))
        }
        id if id.starts_with("codex.ruin.") || id.starts_with("ruin.") => {
            Some(ItemRewardDef::new(semantics::ITEM_DATA_SHARD, 1))
        }
        _ => None,
    }
}

pub fn research_meter_for_codex(codex_id: &str) -> &'static str {
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
    fn core_item_tables_reference_known_item_defs() {
        for stack in DEFAULT_INVENTORY_STACKS {
            assert!(item_def(stack.item_id).is_some(), "{}", stack.item_id);
            assert!(stack.quantity > 0);
        }

        for rule in PICKUP_REWARD_RULES {
            assert!(
                item_def(rule.reward.item_id).is_some(),
                "{} maps to unknown item {}",
                rule.asset_id,
                rule.reward.item_id
            );
        }
    }

    #[test]
    fn consumable_effects_are_declared_on_items() {
        assert_eq!(
            consumable_effect(semantics::ITEM_MED_INJECTOR),
            Some(ConsumableEffectDef::new(semantics::METER_HEALTH, 35))
        );
        assert_eq!(
            consumable_effect(semantics::ITEM_ENERGY_CELL),
            Some(ConsumableEffectDef::new(semantics::METER_STAMINA, 35))
        );
        assert_eq!(consumable_effect(semantics::ITEM_RUIN_KEY), None);
    }
}
