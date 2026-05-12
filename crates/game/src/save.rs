use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use content::{items, semantics};
use runtime::Vec2;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SAVE_PATH: &str = "saves/profile_01.ron";
pub const SAVE_SCHEMA_VERSION: u32 = 1;
pub const SAVE_SLOT_COUNT: usize = 3;
pub const ACTIVITY_LOG_LIMIT: usize = 32;
const DEFAULT_INVENTORY_SLOTS: usize = 24;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    pub profile: PlayerProfileSave,
    pub inventory: InventorySave,
    pub world: WorldSave,
    pub codex: CodexSave,
    pub objectives: ObjectivesSave,
    pub activity_log: ActivityLogSave,
    pub settings: SettingsSave,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_SCHEMA_VERSION,
            profile: PlayerProfileSave::default(),
            inventory: InventorySave::default(),
            world: WorldSave::default(),
            codex: CodexSave::default(),
            objectives: ObjectivesSave::default(),
            activity_log: ActivityLogSave::default(),
            settings: SettingsSave::default(),
        }
    }
}

impl SaveData {
    pub fn load_or_default(path: &Path) -> Self {
        Self::load(path).unwrap_or_default()
    }

    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read save file {}", path.display()))?;
        let mut data: Self = ron::from_str(&source)
            .with_context(|| format!("failed to parse save file {}", path.display()))?;
        data.version = SAVE_SCHEMA_VERSION;
        data.normalize();
        Ok(data)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create save directory {}", parent.display()))?;
        }

        let pretty = ron::ser::PrettyConfig::new();
        let source =
            ron::ser::to_string_pretty(self, pretty).context("failed to serialize save")?;
        let temp_path = path.with_extension("ron.tmp");
        fs::write(&temp_path, source)
            .with_context(|| format!("failed to write temporary save {}", temp_path.display()))?;
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove old save {}", path.display()))?;
        }
        fs::rename(&temp_path, path)
            .with_context(|| format!("failed to replace save file {}", path.display()))?;
        Ok(())
    }

    pub fn normalize(&mut self) {
        if self.profile.vitals.is_empty() {
            self.profile.vitals = PlayerProfileSave::default().vitals;
        }
        if self.profile.attributes.is_empty() {
            self.profile.attributes = PlayerProfileSave::default().attributes;
        }
        if self.profile.research.is_empty() {
            self.profile.research = PlayerProfileSave::default().research;
        }
        if self.profile.resistances.is_empty() {
            self.profile.resistances = PlayerProfileSave::default().resistances;
        }
        if self.inventory.slots.is_empty() {
            self.inventory = InventorySave::default();
        }
        self.world.field_time_minutes %= 24 * 60;
        if self.world.weather.trim().is_empty() {
            self.world.weather = "clear".to_owned();
        }
        self.activity_log.normalize();
        self.objectives.normalize();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerProfileSave {
    pub callsign: String,
    pub role: String,
    pub field_id: String,
    pub level: u32,
    pub xp: u32,
    pub xp_next: u32,
    pub vitals: Vec<MeterSave>,
    pub attributes: Vec<ScoreSave>,
    pub research: Vec<MeterSave>,
    pub resistances: Vec<MeterSave>,
}

impl Default for PlayerProfileSave {
    fn default() -> Self {
        Self {
            callsign: "Stardust Surveyor".to_owned(),
            role: "Forward Explorer / Sample Analysis".to_owned(),
            field_id: "AA-01".to_owned(),
            level: 1,
            xp: 0,
            xp_next: 1_000,
            vitals: vec![
                MeterSave::new(semantics::METER_HEALTH, 100, 100),
                MeterSave::new(semantics::METER_STAMINA, 100, 100),
                MeterSave::new(semantics::METER_SUIT, 100, 100),
                MeterSave::new(semantics::METER_LOAD, 0, 60),
            ],
            attributes: vec![
                ScoreSave::new("survival", 0, 10),
                ScoreSave::new("mobility", 0, 10),
                ScoreSave::new("scanning", 0, 10),
                ScoreSave::new("harvesting", 0, 10),
                ScoreSave::new("analysis", 0, 10),
            ],
            research: vec![
                MeterSave::new(semantics::METER_BIO, 0, 100),
                MeterSave::new(semantics::METER_MINERAL, 0, 100),
                MeterSave::new(semantics::METER_RUIN, 0, 100),
                MeterSave::new(semantics::METER_DATA, 0, 100),
            ],
            resistances: vec![
                MeterSave::new(semantics::METER_SPORES, 100, 100),
                MeterSave::new(semantics::METER_HEAT, 100, 100),
                MeterSave::new(semantics::METER_RADIATION, 100, 100),
                MeterSave::new(semantics::METER_OXYGEN, 100, 100),
            ],
        }
    }
}

impl PlayerProfileSave {
    pub fn meter(&self, id: &str) -> Option<&MeterSave> {
        self.vitals
            .iter()
            .chain(self.research.iter())
            .chain(self.resistances.iter())
            .find(|stat| stat.id == id)
    }

    pub fn meter_mut(&mut self, id: &str) -> Option<&mut MeterSave> {
        self.vitals
            .iter_mut()
            .chain(self.research.iter_mut())
            .chain(self.resistances.iter_mut())
            .find(|stat| stat.id == id)
    }

    pub fn set_meter_value(&mut self, id: &str, value: u32) -> bool {
        let Some(meter) = self.meter_mut(id) else {
            return false;
        };
        let next_value = value.min(meter.max);
        if meter.value == next_value {
            return false;
        }

        meter.value = next_value;
        true
    }

    pub fn add_meter_delta(&mut self, id: &str, delta: i32) -> bool {
        let Some(meter) = self.meter_mut(id) else {
            return false;
        };

        let current = meter.value as i32;
        let max = meter.max as i32;
        let next_value = (current + delta).clamp(0, max) as u32;
        if meter.value == next_value {
            return false;
        }

        meter.value = next_value;
        true
    }

    pub fn score(&self, id: &str) -> Option<&ScoreSave> {
        self.attributes.iter().find(|stat| stat.id == id)
    }

    pub fn set_score_value(&mut self, id: &str, value: u32) -> bool {
        let Some(score) = self.attributes.iter_mut().find(|stat| stat.id == id) else {
            return false;
        };
        let next_value = value.min(score.max);
        if score.value == next_value {
            return false;
        }

        score.value = next_value;
        true
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeterSave {
    pub id: String,
    pub value: u32,
    pub max: u32,
}

impl MeterSave {
    pub fn new(id: impl Into<String>, value: u32, max: u32) -> Self {
        Self {
            id: id.into(),
            value,
            max,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreSave {
    pub id: String,
    pub value: u32,
    pub max: u32,
}

impl ScoreSave {
    pub fn new(id: impl Into<String>, value: u32, max: u32) -> Self {
        Self {
            id: id.into(),
            value,
            max,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InventorySave {
    pub slots: Vec<Option<ItemStackSave>>,
    pub quickbar: Vec<Option<usize>>,
    pub selected_slot: usize,
    pub active_category: String,
}

impl InventorySave {
    pub fn item_quantity(&self, item_id: &str) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|stack| stack.item_id == item_id)
            .map(|stack| stack.quantity)
            .sum()
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.item_quantity(item_id) > 0
    }

    pub fn add_item(
        &mut self,
        item_id: impl Into<String>,
        quantity: u32,
        max_stack: u32,
        locked: bool,
    ) -> u32 {
        let item_id = item_id.into();
        if item_id.trim().is_empty() || quantity == 0 {
            return 0;
        }

        let max_stack = max_stack.max(1);
        let mut remaining = quantity;
        for stack in self.slots.iter_mut().flatten() {
            if stack.item_id != item_id || stack.quantity >= max_stack {
                continue;
            }

            let added = remaining.min(max_stack - stack.quantity);
            stack.quantity += added;
            remaining -= added;
            if remaining == 0 {
                return quantity;
            }
        }

        for slot in &mut self.slots {
            if slot.is_some() {
                continue;
            }

            let added = remaining.min(max_stack);
            *slot = Some(ItemStackSave {
                item_id: item_id.clone(),
                quantity: added,
                locked,
            });
            remaining -= added;
            if remaining == 0 {
                return quantity;
            }
        }

        quantity - remaining
    }

    pub fn consume_slot(&mut self, slot_index: usize, quantity: u32) -> bool {
        let Some(slot) = self.slots.get_mut(slot_index) else {
            return false;
        };
        let Some(stack) = slot else {
            return false;
        };
        if quantity == 0 {
            return false;
        }

        if stack.quantity > quantity {
            stack.quantity -= quantity;
        } else {
            *slot = None;
        }
        true
    }
}

impl Default for InventorySave {
    fn default() -> Self {
        let mut slots = vec![None; DEFAULT_INVENTORY_SLOTS];
        for (index, stack) in items::DEFAULT_INVENTORY_STACKS.iter().enumerate() {
            slots[index] = Some(ItemStackSave::new(
                stack.item_id,
                stack.quantity,
                stack.locked,
            ));
        }

        Self {
            slots,
            quickbar: items::DEFAULT_QUICKBAR_SLOTS.to_vec(),
            selected_slot: 0,
            active_category: items::ItemCategory::Samples.key().to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemStackSave {
    pub item_id: String,
    pub quantity: u32,
    pub locked: bool,
}

impl Default for ItemStackSave {
    fn default() -> Self {
        Self {
            item_id: String::new(),
            quantity: 1,
            locked: false,
        }
    }
}

impl ItemStackSave {
    pub fn new(item_id: impl Into<String>, quantity: u32, locked: bool) -> Self {
        Self {
            item_id: item_id.into(),
            quantity,
            locked,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldSave {
    pub current_scene: String,
    pub map_path: String,
    pub spawn_id: Option<String>,
    pub player_position: Option<SaveVec2>,
    pub collected_entities: BTreeSet<String>,
    pub triggered_zones: BTreeSet<String>,
    pub field_time_minutes: u32,
    pub weather: String,
}

impl Default for WorldSave {
    fn default() -> Self {
        Self {
            current_scene: "Overworld".to_owned(),
            map_path: "assets/data/maps/overworld_landing_site.ron".to_owned(),
            spawn_id: Some("player_start".to_owned()),
            player_position: None,
            collected_entities: BTreeSet::new(),
            triggered_zones: BTreeSet::new(),
            field_time_minutes: 8 * 60,
            weather: "clear".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CodexSave {
    pub scanned_ids: BTreeSet<String>,
}

impl CodexSave {
    pub fn from_runtime(scanned_ids: &HashSet<String>) -> Self {
        Self {
            scanned_ids: scanned_ids.iter().cloned().collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObjectivesSave {
    pub states: BTreeMap<String, ObjectiveStateSave>,
}

impl ObjectivesSave {
    pub fn normalize(&mut self) {
        self.states.retain(|id, state| {
            let keep = !id.trim().is_empty();
            state.normalize();
            keep
        });
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObjectiveStateSave {
    pub status: String,
    pub completed_checkpoints: BTreeSet<String>,
}

impl Default for ObjectiveStateSave {
    fn default() -> Self {
        Self {
            status: "inactive".to_owned(),
            completed_checkpoints: BTreeSet::new(),
        }
    }
}

impl ObjectiveStateSave {
    pub fn new(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            completed_checkpoints: BTreeSet::new(),
        }
    }

    fn normalize(&mut self) {
        let status = self.status.trim().to_ascii_lowercase();
        self.status = match status.as_str() {
            "active" | "started" | "tracked" => "active",
            "completed" | "complete" | "done" => "completed",
            _ => "inactive",
        }
        .to_owned();
        self.completed_checkpoints
            .retain(|checkpoint| !checkpoint.trim().is_empty());
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityLogSave {
    pub entries: Vec<ActivityLogEntrySave>,
    pub next_sequence: u64,
}

impl Default for ActivityLogSave {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_sequence: 1,
        }
    }
}

impl ActivityLogSave {
    pub fn push(
        &mut self,
        category: impl Into<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
        scene: impl Into<String>,
        map_path: impl Into<String>,
    ) {
        let sequence = self.next_sequence.max(1);
        self.next_sequence = sequence + 1;
        self.entries.push(ActivityLogEntrySave {
            sequence,
            category: category.into(),
            title: title.into(),
            detail: detail.into(),
            scene: scene.into(),
            map_path: map_path.into(),
        });
        self.trim_to_limit();
    }

    pub fn normalize(&mut self) {
        let max_sequence = self
            .entries
            .iter()
            .map(|entry| entry.sequence)
            .max()
            .unwrap_or(0);
        self.next_sequence = self.next_sequence.max(max_sequence + 1).max(1);
        self.trim_to_limit();
    }

    fn trim_to_limit(&mut self) {
        let overflow = self.entries.len().saturating_sub(ACTIVITY_LOG_LIMIT);
        if overflow > 0 {
            self.entries.drain(0..overflow);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityLogEntrySave {
    pub sequence: u64,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub scene: String,
    pub map_path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsSave {
    pub language: String,
}

impl Default for SettingsSave {
    fn default() -> Self {
        Self {
            language: "Chinese".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveVec2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for SaveVec2 {
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<SaveVec2> for Vec2 {
    fn from(value: SaveVec2) -> Self {
        Vec2::new(value.x, value.y)
    }
}

pub fn save_slot_path(slot_index: usize) -> PathBuf {
    PathBuf::from(format!("saves/profile_{:02}.ron", slot_index + 1))
}

pub fn delete_save_file(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("failed to delete save file {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_save_contains_core_runtime_state() {
        let save = SaveData::default();

        assert_eq!(save.version, SAVE_SCHEMA_VERSION);
        assert!(!save.profile.vitals.is_empty());
        assert!(!save.inventory.slots.is_empty());
        assert_eq!(save.world.current_scene, "Overworld");
        assert_eq!(save.world.field_time_minutes, 8 * 60);
        assert_eq!(save.world.weather, "clear");
        assert!(save.world.triggered_zones.is_empty());
        assert!(save.objectives.states.is_empty());
        assert!(save.activity_log.entries.is_empty());
    }

    #[test]
    fn save_slot_paths_are_stable() {
        assert_eq!(save_slot_path(0), PathBuf::from("saves/profile_01.ron"));
        assert_eq!(save_slot_path(2), PathBuf::from("saves/profile_03.ron"));
    }

    #[test]
    fn codex_save_sorts_runtime_ids() {
        let scanned = HashSet::from(["codex.b".to_owned(), "codex.a".to_owned()]);
        let save = CodexSave::from_runtime(&scanned);

        assert_eq!(
            save.scanned_ids.into_iter().collect::<Vec<_>>(),
            vec!["codex.a".to_owned(), "codex.b".to_owned()]
        );
    }

    #[test]
    fn inventory_add_item_stacks_then_uses_empty_slots() {
        let mut inventory = InventorySave {
            slots: vec![
                Some(ItemStackSave::new(
                    semantics::ITEM_BIO_SAMPLE_VIAL,
                    9,
                    false,
                )),
                None,
            ],
            quickbar: Vec::new(),
            selected_slot: 0,
            active_category: "samples".to_owned(),
        };

        let added = inventory.add_item(semantics::ITEM_BIO_SAMPLE_VIAL, 3, 10, false);

        assert_eq!(added, 3);
        assert_eq!(
            inventory.slots[0].as_ref().map(|stack| stack.quantity),
            Some(10)
        );
        assert_eq!(
            inventory.slots[1].as_ref().map(|stack| stack.quantity),
            Some(2)
        );
    }

    #[test]
    fn profile_meters_clamp_runtime_changes() {
        let mut profile = PlayerProfileSave::default();

        assert!(profile.add_meter_delta(semantics::METER_STAMINA, -999));
        assert_eq!(
            profile
                .meter(semantics::METER_STAMINA)
                .map(|meter| meter.value),
            Some(0)
        );
        assert!(profile.add_meter_delta(semantics::METER_STAMINA, 999));
        assert_eq!(
            profile
                .meter(semantics::METER_STAMINA)
                .map(|meter| (meter.value, meter.max)),
            Some((100, 100))
        );
        assert!(profile.set_meter_value(semantics::METER_LOAD, 999));
        assert_eq!(
            profile
                .meter(semantics::METER_LOAD)
                .map(|meter| (meter.value, meter.max)),
            Some((60, 60))
        );
    }

    #[test]
    fn activity_log_keeps_recent_entries_only() {
        let mut log = ActivityLogSave::default();

        for index in 0..(ACTIVITY_LOG_LIMIT + 4) {
            log.push(
                "status",
                format!("event {index}"),
                "detail",
                "Overworld",
                "assets/data/maps/overworld_landing_site.ron",
            );
        }

        assert_eq!(log.entries.len(), ACTIVITY_LOG_LIMIT);
        assert_eq!(log.entries.first().map(|entry| entry.sequence), Some(5));
        assert_eq!(
            log.entries.last().map(|entry| entry.sequence),
            Some((ACTIVITY_LOG_LIMIT + 4) as u64)
        );
    }

    #[test]
    fn save_file_can_be_written_more_than_once() {
        let path = std::env::temp_dir().join(format!(
            "alien_archive_save_test_{}.ron",
            std::process::id()
        ));
        let mut save = SaveData::default();

        save.save(&path).expect("first save should write");
        save.profile.level += 1;
        save.save(&path).expect("second save should replace");

        let loaded = SaveData::load(&path).expect("save should load");
        assert_eq!(loaded.profile.level, save.profile.level);

        let _ = fs::remove_file(path);
    }
}
