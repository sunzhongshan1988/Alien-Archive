use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::semantics;

pub const DEFAULT_ITEM_DB_PATH: &str = "crates/content/data/items.ron";

const BUNDLED_ITEM_DB: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/items.ron"));

static ITEM_DATABASE: OnceLock<ItemDatabase> = OnceLock::new();

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct LocalizedTextDef {
    pub english: String,
    pub chinese: String,
}

impl LocalizedTextDef {
    pub fn new(english: impl Into<String>, chinese: impl Into<String>) -> Self {
        Self {
            english: english.into(),
            chinese: chinese.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticLocalizedTextDef {
    pub english: &'static str,
    pub chinese: &'static str,
}

impl StaticLocalizedTextDef {
    pub const fn new(english: &'static str, chinese: &'static str) -> Self {
        Self { english, chinese }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

    pub const fn label(self) -> StaticLocalizedTextDef {
        match self {
            Self::Samples => StaticLocalizedTextDef::new("Samples", "样本"),
            Self::Components => StaticLocalizedTextDef::new("Components", "组件"),
            Self::Artifacts => StaticLocalizedTextDef::new("Artifacts", "遗物"),
            Self::Consumables => StaticLocalizedTextDef::new("Consumables", "消耗品"),
            Self::Tools => StaticLocalizedTextDef::new("Tools", "工具"),
            Self::Data => StaticLocalizedTextDef::new("Data", "数据"),
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Artifact,
}

impl ItemRarity {
    pub const ALL: [Self; 4] = [Self::Common, Self::Uncommon, Self::Rare, Self::Artifact];

    pub const fn label(self) -> StaticLocalizedTextDef {
        match self {
            Self::Common => StaticLocalizedTextDef::new("Common", "普通"),
            Self::Uncommon => StaticLocalizedTextDef::new("Uncommon", "优秀"),
            Self::Rare => StaticLocalizedTextDef::new("Rare", "稀有"),
            Self::Artifact => StaticLocalizedTextDef::new("Artifact", "遗物"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConsumableEffectDef {
    pub meter_id: String,
    pub amount: u32,
}

impl ConsumableEffectDef {
    pub fn new(meter_id: impl Into<String>, amount: u32) -> Self {
        Self {
            meter_id: meter_id.into(),
            amount,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ItemDef {
    pub id: String,
    pub name: LocalizedTextDef,
    pub category: ItemCategory,
    pub texture_id: String,
    pub icon_path: String,
    pub max_stack: u32,
    pub weight: u32,
    pub rarity: ItemRarity,
    pub research: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub equipment_module: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumable: Option<ConsumableEffectDef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ItemRewardDef {
    pub item_id: String,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
}

impl ItemRewardDef {
    pub fn new(item_id: impl Into<String>, quantity: u32) -> Self {
        Self {
            item_id: item_id.into(),
            quantity,
            locked: false,
        }
    }

    pub fn locked(item_id: impl Into<String>, quantity: u32) -> Self {
        Self {
            item_id: item_id.into(),
            quantity,
            locked: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PickupRewardRule {
    pub asset_id: String,
    pub reward: ItemRewardDef,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ItemDatabase {
    pub mode: String,
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub default_inventory: Vec<ItemRewardDef>,
    #[serde(default)]
    pub default_quickbar: Vec<Option<usize>>,
    #[serde(default)]
    pub pickup_rewards: Vec<PickupRewardRule>,
    #[serde(skip)]
    by_id: HashMap<String, usize>,
}

impl ItemDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read item database {}", path.display()))?;
        Self::from_ron(&source)
            .with_context(|| format!("failed to parse item database {}", path.display()))
    }

    pub fn from_ron(source: &str) -> Result<Self> {
        let mut database: Self = ron::from_str(source).context("failed to parse item RON")?;
        database.reindex();
        Ok(database)
    }

    pub fn reindex(&mut self) {
        self.by_id = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id.clone(), index))
            .collect();
    }

    pub fn get(&self, item_id: &str) -> Option<&ItemDef> {
        self.by_id
            .get(item_id)
            .and_then(|index| self.items.get(*index))
            .or_else(|| self.items.iter().find(|item| item.id == item_id))
    }

    pub fn items(&self) -> &[ItemDef] {
        &self.items
    }

    pub fn default_inventory(&self) -> &[ItemRewardDef] {
        &self.default_inventory
    }

    pub fn default_quickbar(&self) -> &[Option<usize>] {
        &self.default_quickbar
    }

    pub fn pickup_rewards(&self) -> &[PickupRewardRule] {
        &self.pickup_rewards
    }
}

pub fn item_database() -> &'static ItemDatabase {
    ITEM_DATABASE.get_or_init(|| {
        ItemDatabase::load(Path::new(DEFAULT_ITEM_DB_PATH)).unwrap_or_else(|error| {
            eprintln!("item database load failed: {error:?}");
            bundled_item_database()
        })
    })
}

pub fn item_defs() -> &'static [ItemDef] {
    item_database().items()
}

pub fn default_inventory_stacks() -> &'static [ItemRewardDef] {
    item_database().default_inventory()
}

pub fn default_quickbar_slots() -> &'static [Option<usize>] {
    item_database().default_quickbar()
}

pub fn pickup_reward_rules() -> &'static [PickupRewardRule] {
    item_database().pickup_rewards()
}

pub fn item_def(item_id: &str) -> Option<&'static ItemDef> {
    item_database().get(item_id)
}

pub fn item_max_stack(item_id: &str) -> Option<u32> {
    item_def(item_id).map(|definition| definition.max_stack)
}

pub fn item_weight(item_id: &str) -> u32 {
    item_def(item_id).map_or(1, |definition| definition.weight)
}

pub fn item_name(item_id: &str) -> Option<&'static LocalizedTextDef> {
    item_def(item_id).map(|definition| &definition.name)
}

pub fn is_equipment_module(item_id: &str, locked: bool) -> bool {
    locked || item_def(item_id).is_some_and(|definition| definition.equipment_module)
}

pub fn consumable_effect(item_id: &str) -> Option<&'static ConsumableEffectDef> {
    item_def(item_id).and_then(|definition| definition.consumable.as_ref())
}

pub fn pickup_reward_for_asset(asset_id: &str) -> Option<ItemRewardDef> {
    pickup_reward_rules()
        .iter()
        .find(|rule| rule.asset_id == asset_id)
        .map(|rule| rule.reward.clone())
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

fn bundled_item_database() -> ItemDatabase {
    ItemDatabase::from_ron(BUNDLED_ITEM_DB).expect("bundled item database should parse")
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn parses_bundled_item_database() {
        let database = bundled_item_database();

        assert!(database.get(semantics::ITEM_MED_INJECTOR).is_some());
        assert!(!database.default_inventory().is_empty());
        assert!(!database.default_quickbar().is_empty());
        assert!(!database.pickup_rewards().is_empty());
    }

    #[test]
    fn loads_optional_workspace_item_file_when_present() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(DEFAULT_ITEM_DB_PATH);
        if !path.exists() {
            return;
        }

        let database = ItemDatabase::load(&path).expect("item file should load");

        assert!(database.get(semantics::ITEM_ENERGY_CELL).is_some());
    }

    #[test]
    fn core_item_tables_reference_known_item_defs() {
        let database = bundled_item_database();

        for stack in database.default_inventory() {
            assert!(database.get(&stack.item_id).is_some(), "{}", stack.item_id);
            assert!(stack.quantity > 0);
        }

        for rule in database.pickup_rewards() {
            assert!(
                database.get(&rule.reward.item_id).is_some(),
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
            Some(&ConsumableEffectDef::new(semantics::METER_HEALTH, 35))
        );
        assert_eq!(
            consumable_effect(semantics::ITEM_ENERGY_CELL),
            Some(&ConsumableEffectDef::new(semantics::METER_STAMINA, 35))
        );
        assert_eq!(consumable_effect(semantics::ITEM_RUIN_KEY), None);
    }
}
