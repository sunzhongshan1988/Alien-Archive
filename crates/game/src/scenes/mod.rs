mod debug_overlay;
mod facility_scene;
mod field_hud;
mod game_menu_scene;
mod inventory_scene;
mod main_menu;
mod notice_system;
mod overworld_scene;
mod pause_scene;
mod profile_scene;
mod rewards;
mod scan_system;
mod zone_system;

use anyhow::Result;
use content::CodexDatabase;
use runtime::{Button, Camera2d, InputState, Renderer, SceneCommand, Vec2};
use std::{collections::HashSet, path::PathBuf};

use crate::objectives::{ObjectiveDatabase, ObjectiveMenuRow};
use crate::save::{CodexSave, InventorySave, MeterSave, SaveData, SaveVec2, WorldSave};
use crate::ui::localization;
use crate::world::{MapObjectiveRule, MapPromptRule, MapTransitionTarget, MapUnlockRule};

use facility_scene::FacilityScene;
use game_menu_scene::GameMenuScene;
use inventory_scene::InventoryScene;
use main_menu::MainMenuScene;
use overworld_scene::OverworldScene;
use pause_scene::PauseScene;
use profile_scene::ProfileScene;

use debug_overlay::{DebugOverlay, SceneDebugSnapshot};
use field_hud::{FieldHud, quickbar_slot_at_position};

const AUTOSAVE_INTERVAL: f32 = 5.0;
const STAMINA_MOVE_DRAIN_PER_SECOND: f32 = 2.0;
const STAMINA_SCAN_DRAIN_PER_SECOND: f32 = 1.0;
const STAMINA_IDLE_RECOVER_PER_SECOND: f32 = 5.0;
const STAMINA_JUMP_COST: i32 = 8;
const FACILITY_SUIT_DRAIN_PER_SECOND: f32 = 0.06;
const FACILITY_OXYGEN_DRAIN_PER_SECOND: f32 = 0.16;
const FACILITY_RADIATION_DRAIN_PER_SECOND: f32 = 0.05;
const FACILITY_SPORE_DRAIN_PER_SECOND: f32 = 0.03;
const OVERWORLD_RECOVERY_PER_SECOND: f32 = 0.08;
const CRITICAL_HEALTH_DRAIN_PER_SECOND: f32 = 0.35;
const FIELD_CLOCK_MINUTES_PER_SECOND: f32 = 1.0;
const LOG_CATEGORY_PICKUP: &str = "pickup";
const LOG_CATEGORY_SCAN: &str = "scan";
const LOG_CATEGORY_UNLOCK: &str = "unlock";
const LOG_CATEGORY_STATUS: &str = "status";
const LOG_CATEGORY_ITEM: &str = "item";
const LOG_CATEGORY_ZONE: &str = "zone";
const LOG_CATEGORY_OBJECTIVE: &str = "objective";

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneId {
    Boot,
    MainMenu,
    Overworld,
    Facility,
    GameMenu,
    Inventory,
    Profile,
    Codex,
    Pause,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    Chinese,
    English,
}

impl Language {
    pub const SUPPORTED: [Self; 2] = [Self::Chinese, Self::English];

    pub fn next(self) -> Self {
        match self {
            Self::Chinese => Self::English,
            Self::English => Self::Chinese,
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::SUPPORTED[0]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameMenuTab {
    Profile,
    Inventory,
    Codex,
    Map,
    Quests,
    Settings,
}

impl GameMenuTab {
    pub const ALL: [Self; 6] = [
        Self::Profile,
        Self::Inventory,
        Self::Codex,
        Self::Map,
        Self::Quests,
        Self::Settings,
    ];

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .unwrap_or_default();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .unwrap_or_default();
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

impl Default for GameMenuTab {
    fn default() -> Self {
        Self::Profile
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldEnvironment {
    Overworld,
    Facility,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldActivity {
    pub moving: bool,
    pub scanning: bool,
    pub jumped: bool,
    pub environment: FieldEnvironment,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldStatusEffects {
    pub movement_speed_multiplier: f32,
}

impl Default for FieldStatusEffects {
    fn default() -> Self {
        Self {
            movement_speed_multiplier: 1.0,
        }
    }
}

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

#[derive(Default)]
struct ProfileStatusRuntime {
    stamina: f32,
    health: f32,
    suit: f32,
    spores: f32,
    radiation: f32,
    oxygen: f32,
    field_clock_minutes: f32,
    low_stamina_logged: bool,
    heavy_load_logged: bool,
    suit_critical_logged: bool,
    oxygen_critical_logged: bool,
    health_critical_logged: bool,
}

#[derive(Default)]
pub struct GameContext {
    pub language: Language,
    pub game_menu_tab: GameMenuTab,
    pub should_quit: bool,
    pub overworld_map_path: Option<String>,
    pub overworld_spawn_id: Option<String>,
    pub overworld_player_position: Option<Vec2>,
    pub facility_map_path: Option<String>,
    pub facility_spawn_id: Option<String>,
    pub facility_player_position: Option<Vec2>,
    pub codex_database: CodexDatabase,
    pub objective_database: ObjectiveDatabase,
    pub scanned_codex_ids: HashSet<String>,
    pub save_path: PathBuf,
    pub save_data: SaveData,
    save_dirty: bool,
    save_requested: bool,
    save_timer: f32,
    profile_status_runtime: ProfileStatusRuntime,
}

impl GameContext {
    pub fn from_save(
        save_path: PathBuf,
        save_data: SaveData,
        codex_database: CodexDatabase,
    ) -> Self {
        Self::from_save_with_objectives(
            save_path,
            save_data,
            codex_database,
            ObjectiveDatabase::load_default(),
        )
    }

    pub fn from_save_with_objectives(
        save_path: PathBuf,
        save_data: SaveData,
        codex_database: CodexDatabase,
        objective_database: ObjectiveDatabase,
    ) -> Self {
        let language = Language::from_save_key(&save_data.settings.language);
        let scanned_codex_ids = save_data.codex.scanned_ids.iter().cloned().collect();
        let mut ctx = Self {
            language,
            codex_database,
            objective_database,
            scanned_codex_ids,
            save_path,
            save_data,
            ..Self::default()
        };
        ctx.sync_inventory_load_meter_silent();
        ctx.apply_world_save();
        ctx
    }

    pub fn objective_menu_rows(&self) -> Vec<ObjectiveMenuRow> {
        self.objective_database
            .menu_rows(&self.save_data.objectives, self.language)
    }

    pub fn resume_scene_id(&self) -> SceneId {
        scene_id_from_save_key(&self.save_data.world.current_scene)
    }

    pub fn load_save_file_or_default(&mut self, save_path: PathBuf) {
        let save_data = if save_path.exists() {
            SaveData::load_or_default(&save_path)
        } else {
            new_save_data_for_language(self.language)
        };
        self.replace_save_data(save_path, save_data);
    }

    pub fn start_new_save(&mut self, save_path: PathBuf) {
        self.replace_save_data(save_path, new_save_data_for_language(self.language));
        self.request_save();
    }

    pub fn reset_to_empty_save_slot(&mut self, save_path: PathBuf) {
        self.replace_save_data(save_path, new_save_data_for_language(self.language));
    }

    pub fn request_save(&mut self) {
        self.save_dirty = true;
        self.save_requested = true;
    }

    pub fn complete_codex_scan(&mut self, codex_id: &str) -> bool {
        if !self.scanned_codex_ids.insert(codex_id.to_owned()) {
            return false;
        }

        self.apply_scan_profile_progress(codex_id);
        if let Some(reward) = rewards::scan_reward_for_codex(codex_id) {
            self.add_inventory_item(reward.item_id, reward.quantity, reward.locked);
        }
        self.log_codex_scan(codex_id);
        self.request_save();
        true
    }

    pub fn add_inventory_item(&mut self, item_id: &str, quantity: u32, locked: bool) -> u32 {
        let max_stack = inventory_scene::inventory_item_max_stack(item_id).unwrap_or(99);
        let added = self
            .save_data
            .inventory
            .add_item(item_id, quantity, max_stack, locked);
        if added > 0 {
            self.sync_inventory_load_meter();
            self.log_item_added(item_id, added);
            self.request_save();
        }
        added
    }

    pub fn set_inventory_save(&mut self, inventory: InventorySave) {
        if self.save_data.inventory == inventory {
            return;
        }

        self.save_data.inventory = inventory;
        self.sync_inventory_load_meter();
        self.request_save();
    }

    pub fn can_start_jump(&self) -> bool {
        self.profile_meter_value("stamina") as i32 >= STAMINA_JUMP_COST
    }

    pub fn update_field_status(&mut self, dt: f32, activity: FieldActivity) -> FieldStatusEffects {
        self.sync_inventory_load_meter();

        if activity.jumped {
            self.change_profile_meter("stamina", -STAMINA_JUMP_COST);
        }
        self.advance_field_clock(dt);

        let mut stamina_rate = if activity.moving {
            -STAMINA_MOVE_DRAIN_PER_SECOND
        } else {
            STAMINA_IDLE_RECOVER_PER_SECOND
        };
        if activity.scanning {
            stamina_rate -= STAMINA_SCAN_DRAIN_PER_SECOND;
        }
        if self.profile_meter_ratio("load") >= 0.85 && activity.moving {
            stamina_rate -= 0.75;
        }
        self.accumulate_profile_delta("stamina", stamina_rate * dt);

        match activity.environment {
            FieldEnvironment::Overworld => {
                self.accumulate_profile_delta("suit", OVERWORLD_RECOVERY_PER_SECOND * dt);
                self.accumulate_profile_delta("oxygen", OVERWORLD_RECOVERY_PER_SECOND * dt);
                self.accumulate_profile_delta("spores", OVERWORLD_RECOVERY_PER_SECOND * dt);
                self.accumulate_profile_delta("radiation", OVERWORLD_RECOVERY_PER_SECOND * dt);
            }
            FieldEnvironment::Facility => {
                self.accumulate_profile_delta("suit", -FACILITY_SUIT_DRAIN_PER_SECOND * dt);
                self.accumulate_profile_delta("oxygen", -FACILITY_OXYGEN_DRAIN_PER_SECOND * dt);
                self.accumulate_profile_delta(
                    "radiation",
                    -FACILITY_RADIATION_DRAIN_PER_SECOND * dt,
                );
                self.accumulate_profile_delta("spores", -FACILITY_SPORE_DRAIN_PER_SECOND * dt);
            }
        }

        if self.profile_meter_value("suit") == 0 || self.profile_meter_value("oxygen") == 0 {
            self.accumulate_profile_delta("health", -CRITICAL_HEALTH_DRAIN_PER_SECOND * dt);
        }
        self.update_activity_status_alerts();

        FieldStatusEffects {
            movement_speed_multiplier: self.profile_movement_speed_multiplier(),
        }
    }

    pub fn profile_meter_value(&self, id: &str) -> u32 {
        self.save_data
            .profile
            .meter(id)
            .map_or(0, |meter| meter.value)
    }

    pub fn profile_meter_max(&self, id: &str) -> u32 {
        self.save_data
            .profile
            .meter(id)
            .map_or(0, |meter| meter.max)
    }

    pub fn select_quickbar_slot(&mut self, quick_index: usize) -> bool {
        let Some(slot_count) = (!self.save_data.inventory.slots.is_empty())
            .then_some(self.save_data.inventory.slots.len())
        else {
            return false;
        };
        let slot_index = self
            .save_data
            .inventory
            .quickbar
            .get(quick_index)
            .and_then(|slot| *slot)
            .unwrap_or(quick_index);
        if slot_index >= slot_count || self.save_data.inventory.selected_slot == slot_index {
            return false;
        }

        self.save_data.inventory.selected_slot = slot_index;
        self.request_save();
        true
    }

    pub fn use_selected_quickbar_item(&mut self) -> QuickItemUseResult {
        let slot_index = self.save_data.inventory.selected_slot;
        let Some(stack) = self
            .save_data
            .inventory
            .slots
            .get(slot_index)
            .and_then(|slot| slot.as_ref())
        else {
            return QuickItemUseResult::Empty;
        };
        let item_id = stack.item_id.clone();

        let Some(effect) = consumable_effect_for_item(&item_id) else {
            return QuickItemUseResult::NotUsable { item_id };
        };
        if self.profile_meter_value(effect.meter_id) >= self.profile_meter_max(effect.meter_id) {
            return QuickItemUseResult::AlreadyFull {
                item_id,
                meter_id: effect.meter_id.to_owned(),
            };
        }

        self.change_profile_meter(effect.meter_id, effect.amount as i32);
        self.save_data.inventory.consume_slot(slot_index, 1);
        self.sync_inventory_load_meter();
        self.log_item_used(&item_id, effect.meter_id, effect.amount);
        self.request_save();
        QuickItemUseResult::Used {
            item_id,
            meter_id: effect.meter_id,
            amount: effect.amount,
        }
    }

    pub fn is_unlock_rule_satisfied(&self, unlock: Option<&MapUnlockRule>) -> bool {
        let Some(unlock) = unlock else {
            return true;
        };

        let codex_ok = unlock
            .requires_codex_id
            .as_deref()
            .is_none_or(|id| self.scanned_codex_ids.contains(id));
        let item_ok = unlock
            .requires_item_id
            .as_deref()
            .is_none_or(|id| self.save_data.inventory.has_item(id));
        codex_ok && item_ok
    }

    pub fn is_entity_collected(&self, map_path: &str, entity_id: &str) -> bool {
        self.save_data
            .world
            .collected_entities
            .contains(&entity_progress_key(map_path, entity_id))
    }

    pub fn collect_entity(&mut self, map_path: &str, entity_id: &str) -> bool {
        let inserted = self
            .save_data
            .world
            .collected_entities
            .insert(entity_progress_key(map_path, entity_id));
        if inserted {
            self.request_save();
        }
        inserted
    }

    pub fn is_zone_triggered(&self, map_path: &str, zone_id: &str) -> bool {
        self.save_data
            .world
            .triggered_zones
            .contains(&zone_progress_key(map_path, zone_id))
    }

    pub fn mark_zone_triggered(&mut self, map_path: &str, zone_id: &str) -> bool {
        let inserted = self
            .save_data
            .world
            .triggered_zones
            .insert(zone_progress_key(map_path, zone_id));
        if inserted {
            self.request_save();
        }
        inserted
    }

    pub fn apply_zone_meter_effect(
        &mut self,
        meter_id: &str,
        rate_per_second: f32,
        dt: f32,
    ) -> bool {
        if dt <= 0.0 || rate_per_second.abs() <= f32::EPSILON {
            return false;
        }

        self.accumulate_profile_delta(meter_id, rate_per_second * dt)
    }

    pub fn update_zone_status_alerts(&mut self) {
        self.update_activity_status_alerts();
    }

    pub fn log_inventory_full(&mut self) {
        let title = localization::text(
            self.language,
            "activity.event.inventory_full.title",
            "Inventory full",
            "背包已满",
        );
        let detail = localization::text(
            self.language,
            "activity.event.inventory_full.detail",
            "No empty slot was available for the pickup",
            "没有空槽位，物品未收入背包",
        );
        self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
    }

    pub fn log_stamina_low(&mut self) {
        let title = localization::text(
            self.language,
            "activity.event.stamina_blocked.title",
            "Stamina too low",
            "体力不足",
        );
        let detail = localization::text(
            self.language,
            "activity.event.stamina_blocked.detail",
            "The action was cancelled; pause to recover stamina",
            "本次动作被取消，先停止移动恢复体力",
        );
        self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
    }

    pub fn log_locked_unlock_rule(&mut self, unlock: Option<&MapUnlockRule>) {
        let detail = self.locked_rule_log_detail(unlock);
        let title = localization::text(
            self.language,
            "activity.event.access_blocked.title",
            "Access blocked",
            "入口受限",
        );
        self.push_activity_log(LOG_CATEGORY_UNLOCK, title, detail);
    }

    pub fn log_zone_prompt(&mut self, prompt: &MapPromptRule, fallback: &str) {
        let title = prompt.log_title.clone().unwrap_or_else(|| {
            localization::text(
                self.language,
                "activity.event.zone_prompt.title",
                "Area note",
                "区域提示",
            )
            .into_owned()
        });
        let detail = prompt
            .log_detail
            .clone()
            .unwrap_or_else(|| fallback.to_owned());
        self.push_activity_log(LOG_CATEGORY_ZONE, title, detail);
    }

    pub fn log_zone_hazard(&mut self, zone_id: &str, detail: String) {
        let title = localization::text(
            self.language,
            "activity.event.zone_hazard.title",
            "Hazard zone",
            "危险区域",
        );
        let detail = localization::format_text(
            self.language,
            "activity.event.zone_hazard.detail",
            "{zone}: {detail}",
            "{zone}：{detail}",
            &[("zone", zone_id.to_owned()), ("detail", detail)],
        );
        self.push_activity_log(LOG_CATEGORY_ZONE, title, detail);
    }

    pub fn apply_objective_zone(
        &mut self,
        objective: &MapObjectiveRule,
    ) -> Option<crate::objectives::ObjectiveAdvanceEvent> {
        let event = self.objective_database.apply_rule(
            &mut self.save_data.objectives,
            self.language,
            &objective.objective_id,
            objective.checkpoint_id.as_deref(),
            objective.complete_objective,
        )?;
        self.push_activity_log(
            LOG_CATEGORY_OBJECTIVE,
            objective
                .log_title
                .clone()
                .unwrap_or_else(|| event.log_title.clone()),
            objective
                .log_detail
                .clone()
                .unwrap_or_else(|| event.log_detail.clone()),
        );
        Some(event)
    }

    pub fn log_scene_transition(&mut self, scene_id: SceneId, map_path: &str) {
        let title = match scene_id {
            SceneId::Facility => localization::text(
                self.language,
                "activity.event.transition.facility.title",
                "Entered facility",
                "进入设施",
            ),
            _ => localization::text(
                self.language,
                "activity.event.transition.overworld.title",
                "Returned to overworld",
                "返回外部区域",
            ),
        };
        let detail = localization::format_text(
            self.language,
            "activity.event.transition.detail",
            "Destination: {map}",
            "目的地：{map}",
            &[("map", map_path.to_owned())],
        );
        self.push_activity_log(LOG_CATEGORY_UNLOCK, title, detail);
    }

    pub fn apply_map_transition(
        &mut self,
        transition: Option<&MapTransitionTarget>,
        default_scene: SceneId,
        default_map: &str,
        default_spawn: &str,
    ) -> SceneId {
        let scene_id = transition
            .and_then(|transition| transition.scene.as_deref())
            .map(scene_id_from_transition_key)
            .unwrap_or(default_scene);
        let map_path = transition
            .and_then(|transition| transition.map_path.clone())
            .unwrap_or_else(|| default_map.to_owned());
        let spawn_id = transition
            .and_then(|transition| transition.spawn_id.clone())
            .unwrap_or_else(|| default_spawn.to_owned());

        match scene_id {
            SceneId::Facility => {
                self.facility_map_path = Some(map_path.clone());
                self.facility_spawn_id = Some(spawn_id);
                self.facility_player_position = None;
            }
            _ => {
                self.overworld_map_path = Some(map_path.clone());
                self.overworld_spawn_id = Some(spawn_id);
                self.overworld_player_position = None;
            }
        }

        self.log_scene_transition(scene_id, &map_path);
        self.request_save();
        scene_id
    }

    fn log_item_used(&mut self, item_id: &str, meter_id: &str, amount: u32) {
        let item_name = inventory_scene::inventory_item_name(item_id, self.language);
        let meter_name = profile_meter_label(meter_id, self.language);
        let title = localization::format_text(
            self.language,
            "activity.event.item_used.title",
            "Used {item}",
            "使用 {item}",
            &[("item", item_name.to_owned())],
        );
        let detail = localization::format_text(
            self.language,
            "activity.event.item_used.detail",
            "Restored {meter} by {amount}",
            "{meter} 恢复 {amount}",
            &[
                ("meter", meter_name.to_owned()),
                ("amount", amount.to_string()),
            ],
        );
        self.push_activity_log(LOG_CATEGORY_ITEM, title, detail);
    }

    fn push_activity_log(
        &mut self,
        category: &str,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) {
        self.save_data.activity_log.push(
            category,
            title,
            detail,
            self.save_data.world.current_scene.clone(),
            self.save_data.world.map_path.clone(),
        );
        self.request_save();
    }

    pub fn collected_entity_ids_for_map(
        &self,
        map_path: &str,
    ) -> std::collections::BTreeSet<String> {
        let prefix = format!("{map_path}::");
        self.save_data
            .world
            .collected_entities
            .iter()
            .filter_map(|key| key.strip_prefix(&prefix).map(str::to_owned))
            .collect()
    }

    pub fn mark_save_dirty(&mut self) {
        self.save_dirty = true;
    }

    pub fn record_world_location(&mut self, scene_id: SceneId, map_path: &str, position: Vec2) {
        let scene_key = scene_id.save_key();
        let changed = self.save_data.world.current_scene != scene_key
            || self.save_data.world.map_path != map_path
            || position_changed(self.save_data.world.player_position, position);

        self.save_data.world = WorldSave {
            current_scene: scene_key.to_owned(),
            map_path: map_path.to_owned(),
            spawn_id: match scene_id {
                SceneId::Facility => self.facility_spawn_id.clone(),
                _ => self.overworld_spawn_id.clone(),
            },
            player_position: Some(position.into()),
            collected_entities: self.save_data.world.collected_entities.clone(),
            triggered_zones: self.save_data.world.triggered_zones.clone(),
            field_time_minutes: self.save_data.world.field_time_minutes,
            weather: self.save_data.world.weather.clone(),
        };

        match scene_id {
            SceneId::Facility => {
                self.facility_map_path = Some(map_path.to_owned());
                self.facility_player_position = Some(position);
            }
            _ => {
                self.overworld_map_path = Some(map_path.to_owned());
                self.overworld_player_position = Some(position);
            }
        }

        if changed {
            self.mark_save_dirty();
        }
    }

    pub fn update_save(&mut self, dt: f32) -> Result<()> {
        if !self.save_dirty {
            return Ok(());
        }

        self.save_timer += dt;
        if self.save_requested || self.save_timer >= AUTOSAVE_INTERVAL || self.should_quit {
            self.save_now()?;
        }

        Ok(())
    }

    pub fn save_now(&mut self) -> Result<()> {
        self.sync_runtime_to_save();
        self.save_data.save(&self.save_path)?;
        self.save_dirty = false;
        self.save_requested = false;
        self.save_timer = 0.0;
        Ok(())
    }

    fn apply_world_save(&mut self) {
        let world = &self.save_data.world;
        let position = world.player_position.map(Into::into);
        match scene_id_from_save_key(&world.current_scene) {
            SceneId::Facility => {
                self.facility_map_path = Some(world.map_path.clone());
                self.facility_spawn_id = world.spawn_id.clone();
                self.facility_player_position = position;
            }
            _ => {
                self.overworld_map_path = Some(world.map_path.clone());
                self.overworld_spawn_id = world.spawn_id.clone();
                self.overworld_player_position = position;
            }
        }
    }

    fn sync_runtime_to_save(&mut self) {
        self.sync_inventory_load_meter();
        self.save_data.settings.language = self.language.save_key().to_owned();
        self.save_data.codex = CodexSave::from_runtime(&self.scanned_codex_ids);
    }

    fn replace_save_data(&mut self, save_path: PathBuf, save_data: SaveData) {
        let codex_database = self.codex_database.clone();
        let objective_database = self.objective_database.clone();
        *self = Self::from_save_with_objectives(
            save_path,
            save_data,
            codex_database,
            objective_database,
        );
    }

    fn apply_scan_profile_progress(&mut self, codex_id: &str) {
        let research_meter = rewards::research_meter_for_codex(codex_id);
        bump_meter(&mut self.save_data.profile.research, research_meter, 6);
        self.save_data.profile.xp += 120;
        while self.save_data.profile.xp_next > 0
            && self.save_data.profile.xp >= self.save_data.profile.xp_next
        {
            self.save_data.profile.xp -= self.save_data.profile.xp_next;
            self.save_data.profile.level += 1;
            self.save_data.profile.xp_next += 2_500;
        }
    }

    fn sync_inventory_load_meter(&mut self) -> bool {
        let load = inventory_load_units(&self.save_data.inventory);
        self.set_profile_meter_value("load", load)
    }

    fn sync_inventory_load_meter_silent(&mut self) {
        let load = inventory_load_units(&self.save_data.inventory);
        self.save_data.profile.set_meter_value("load", load);
    }

    fn set_profile_meter_value(&mut self, id: &str, value: u32) -> bool {
        let changed = self.save_data.profile.set_meter_value(id, value);
        if changed {
            self.mark_save_dirty();
        }
        changed
    }

    fn change_profile_meter(&mut self, id: &str, delta: i32) -> bool {
        if delta == 0 {
            return false;
        }

        let changed = self.save_data.profile.add_meter_delta(id, delta);
        if changed {
            self.mark_save_dirty();
        }
        changed
    }

    fn accumulate_profile_delta(&mut self, id: &str, delta: f32) -> bool {
        let accumulator = match id {
            "health" => &mut self.profile_status_runtime.health,
            "stamina" => &mut self.profile_status_runtime.stamina,
            "suit" => &mut self.profile_status_runtime.suit,
            "spores" => &mut self.profile_status_runtime.spores,
            "radiation" => &mut self.profile_status_runtime.radiation,
            "oxygen" => &mut self.profile_status_runtime.oxygen,
            _ => return false,
        };
        let whole_delta = accumulated_integer_delta(accumulator, delta);
        self.change_profile_meter(id, whole_delta)
    }

    fn profile_meter_ratio(&self, id: &str) -> f32 {
        self.save_data.profile.meter(id).map_or(0.0, |meter| {
            if meter.max == 0 {
                0.0
            } else {
                meter.value as f32 / meter.max as f32
            }
        })
    }

    fn profile_movement_speed_multiplier(&self) -> f32 {
        let stamina = self.profile_meter_ratio("stamina");
        let load = self.profile_meter_ratio("load");
        let stamina_factor = if self.profile_meter_value("stamina") == 0 {
            0.55
        } else if stamina < 0.20 {
            0.78
        } else {
            1.0
        };
        let load_factor = if load >= 0.95 {
            0.70
        } else if load >= 0.80 {
            0.86
        } else {
            1.0
        };

        stamina_factor * load_factor
    }

    fn advance_field_clock(&mut self, dt: f32) -> bool {
        if dt <= 0.0 {
            return false;
        }

        self.profile_status_runtime.field_clock_minutes += dt * FIELD_CLOCK_MINUTES_PER_SECOND;
        let whole_minutes = self.profile_status_runtime.field_clock_minutes.floor() as u32;
        if whole_minutes == 0 {
            return false;
        }

        self.profile_status_runtime.field_clock_minutes -= whole_minutes as f32;
        let minutes_per_day = 24 * 60;
        self.save_data.world.field_time_minutes =
            (self.save_data.world.field_time_minutes + whole_minutes) % minutes_per_day;
        let next_weather = weather_key_for_time(self.save_data.world.field_time_minutes);
        if self.save_data.world.weather != next_weather {
            self.save_data.world.weather = next_weather.to_owned();
        }
        self.mark_save_dirty();
        true
    }

    fn log_item_added(&mut self, item_id: &str, quantity: u32) {
        let item_name = inventory_scene::inventory_item_name(item_id, self.language);
        let title = localization::text(
            self.language,
            "activity.event.item_added.title",
            "Item acquired",
            "获得物品",
        );
        let detail = localization::format_text(
            self.language,
            "activity.event.item_added.detail",
            "{item} x{quantity} added to inventory",
            "{item} x{quantity} 已写入背包",
            &[
                ("item", item_name.to_owned()),
                ("quantity", quantity.to_string()),
            ],
        );
        self.push_activity_log(LOG_CATEGORY_PICKUP, title, detail);
    }

    fn log_codex_scan(&mut self, codex_id: &str) {
        let entry_title = self
            .codex_database
            .get(codex_id)
            .map(|entry| entry.title.trim())
            .filter(|title| !title.is_empty())
            .unwrap_or(codex_id);
        let research_meter = rewards::research_meter_for_codex(codex_id);
        let title = localization::text(
            self.language,
            "activity.event.scan_recorded.title",
            "Scan recorded",
            "扫描完成",
        );
        let detail = localization::format_text(
            self.language,
            "activity.event.scan_recorded.detail",
            "{title} · {meter} research +6 · XP +120",
            "{title} · {meter} 研究 +6 · XP +120",
            &[
                ("title", entry_title.to_owned()),
                ("meter", research_meter.to_owned()),
            ],
        );
        self.push_activity_log(LOG_CATEGORY_SCAN, title, detail);
    }

    fn locked_rule_log_detail(&self, unlock: Option<&MapUnlockRule>) -> String {
        let Some(unlock) = unlock else {
            return localization::text(
                self.language,
                "activity.event.access_blocked.unavailable",
                "The entrance is currently unavailable",
                "入口当前不可用",
            )
            .into_owned();
        };

        if let Some(message) = unlock.locked_message.as_deref() {
            return message.to_owned();
        }

        let codex_title = unlock
            .requires_codex_id
            .as_deref()
            .and_then(|id| self.codex_database.get(id).map(|entry| entry.title.trim()))
            .filter(|title| !title.is_empty());
        let item_name = unlock
            .requires_item_id
            .as_deref()
            .map(|id| inventory_scene::inventory_item_name(id, self.language));
        match (codex_title, item_name.as_deref()) {
            (Some(codex), Some(item)) => localization::format_text(
                self.language,
                "activity.event.access_blocked.scan_and_item",
                "Requires scan: {codex}; item: {item}",
                "需要先扫描 {codex}，并携带 {item}",
                &[("codex", codex.to_owned()), ("item", item.to_owned())],
            ),
            (Some(codex), None) => localization::format_text(
                self.language,
                "activity.event.access_blocked.scan",
                "Requires scan: {codex}",
                "需要先扫描 {codex}",
                &[("codex", codex.to_owned())],
            ),
            (None, Some(item)) => localization::format_text(
                self.language,
                "activity.event.access_blocked.item",
                "Requires item: {item}",
                "需要携带 {item}",
                &[("item", item.to_owned())],
            ),
            (None, None) => localization::text(
                self.language,
                "activity.event.access_blocked.missing_requirement",
                "Missing access requirement",
                "缺少进入条件",
            )
            .into_owned(),
        }
    }

    fn update_activity_status_alerts(&mut self) {
        if self.should_log_meter_below("stamina", 0.18, 0.35, StatusAlert::StaminaLow) {
            let title = localization::text(
                self.language,
                "activity.event.status.stamina_low.title",
                "Stamina low",
                "体力偏低",
            );
            let detail = localization::text(
                self.language,
                "activity.event.status.stamina_low.detail",
                "Movement, scans, and jumps are limited",
                "移动、扫描和跳跃会受到限制",
            );
            self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
        }
        if self.should_log_meter_above("load", 0.85, 0.70, StatusAlert::HeavyLoad) {
            let title = localization::text(
                self.language,
                "activity.event.status.load_high.title",
                "Load high",
                "负重过高",
            );
            let detail = localization::text(
                self.language,
                "activity.event.status.load_high.detail",
                "Inventory weight is slowing movement",
                "背包重量已经开始拖慢移动速度",
            );
            self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
        }
        if self.should_log_meter_below("suit", 0.25, 0.45, StatusAlert::SuitCritical) {
            let title = localization::text(
                self.language,
                "activity.event.status.suit_low.title",
                "Suit integrity low",
                "外骨骼受损",
            );
            let detail = localization::text(
                self.language,
                "activity.event.status.suit_low.detail",
                "Continued exposure increases health risk",
                "继续暴露会提高生命风险",
            );
            self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
        }
        if self.should_log_meter_below("oxygen", 0.25, 0.45, StatusAlert::OxygenCritical) {
            let title = localization::text(
                self.language,
                "activity.event.status.oxygen_low.title",
                "Oxygen low",
                "氧气偏低",
            );
            let detail = localization::text(
                self.language,
                "activity.event.status.oxygen_low.detail",
                "Health will fall once oxygen is depleted",
                "氧气耗尽后生命值会下降",
            );
            self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
        }
        if self.should_log_meter_below("health", 0.35, 0.55, StatusAlert::HealthCritical) {
            let title = localization::text(
                self.language,
                "activity.event.status.health_critical.title",
                "Health critical",
                "生命值危险",
            );
            let detail = localization::text(
                self.language,
                "activity.event.status.health_critical.detail",
                "Extract or use recovery supplies soon",
                "建议尽快撤离或使用恢复道具",
            );
            self.push_activity_log(LOG_CATEGORY_STATUS, title, detail);
        }
    }

    fn should_log_meter_below(
        &mut self,
        id: &str,
        trigger: f32,
        reset: f32,
        alert: StatusAlert,
    ) -> bool {
        let ratio = self.profile_meter_ratio(id);
        let active = self.status_alert_flag(alert);
        let should_log = !*active && ratio <= trigger;
        if ratio >= reset {
            *active = false;
        } else if should_log {
            *active = true;
        }
        should_log
    }

    fn should_log_meter_above(
        &mut self,
        id: &str,
        trigger: f32,
        reset: f32,
        alert: StatusAlert,
    ) -> bool {
        let ratio = self.profile_meter_ratio(id);
        let active = self.status_alert_flag(alert);
        let should_log = !*active && ratio >= trigger;
        if ratio <= reset {
            *active = false;
        } else if should_log {
            *active = true;
        }
        should_log
    }

    fn status_alert_flag(&mut self, alert: StatusAlert) -> &mut bool {
        match alert {
            StatusAlert::StaminaLow => &mut self.profile_status_runtime.low_stamina_logged,
            StatusAlert::HeavyLoad => &mut self.profile_status_runtime.heavy_load_logged,
            StatusAlert::SuitCritical => &mut self.profile_status_runtime.suit_critical_logged,
            StatusAlert::OxygenCritical => &mut self.profile_status_runtime.oxygen_critical_logged,
            StatusAlert::HealthCritical => &mut self.profile_status_runtime.health_critical_logged,
        }
    }
}

#[derive(Clone, Copy)]
enum StatusAlert {
    StaminaLow,
    HeavyLoad,
    SuitCritical,
    OxygenCritical,
    HealthCritical,
}

pub struct RenderContext<'a> {
    pub renderer: &'a mut dyn Renderer,
}

#[allow(dead_code)]
pub trait Scene {
    fn id(&self) -> SceneId;
    fn name(&self) -> &str;

    fn setup(&mut self, _renderer: &mut dyn Renderer) -> Result<()> {
        Ok(())
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>>;

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()>;

    fn render_debug_geometry(&self, _renderer: &mut dyn Renderer) {}

    fn camera(&self) -> Camera2d {
        Camera2d::default()
    }

    fn debug_snapshot(&self, _ctx: &GameContext) -> SceneDebugSnapshot {
        SceneDebugSnapshot::new(self.id(), self.name())
    }
}

struct ManagedScene {
    scene: Box<dyn Scene>,
    setup_complete: bool,
}

impl ManagedScene {
    fn new(scene: Box<dyn Scene>) -> Self {
        Self {
            scene,
            setup_complete: false,
        }
    }
}

pub struct SceneStack {
    stack: Vec<ManagedScene>,
    debug_overlay: DebugOverlay,
    field_hud: FieldHud,
}

impl SceneStack {
    pub fn new_main_menu() -> Self {
        Self {
            stack: vec![ManagedScene::new(Box::new(MainMenuScene::new()))],
            debug_overlay: DebugOverlay::default(),
            field_hud: FieldHud::default(),
        }
    }

    pub fn new_scene(scene_id: SceneId, ctx: &GameContext) -> Result<Self> {
        Ok(Self {
            stack: vec![ManagedScene::new(create_scene(scene_id, ctx)?)],
            debug_overlay: DebugOverlay::default(),
            field_hud: FieldHud::default(),
        })
    }

    pub fn setup_current(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        let Some(current) = self.stack.last_mut() else {
            return Ok(());
        };

        if !current.setup_complete {
            current.scene.setup(renderer)?;
            current.setup_complete = true;
        }

        Ok(())
    }

    pub fn update(&mut self, ctx: &mut GameContext, dt: f32, input: &InputState) -> Result<()> {
        if input.just_pressed(Button::DebugOverlay) {
            self.debug_overlay.toggle();
        }
        if self.current_field_scene_id().is_some() {
            handle_quickbar_input(ctx, input);
        }

        let Some(current) = self.stack.last_mut() else {
            ctx.should_quit = true;
            return Ok(());
        };

        let command = current.scene.update(ctx, dt, input)?;
        self.apply_command(ctx, command)
    }

    pub fn render(&mut self, ctx: &GameContext, renderer: &mut dyn Renderer) -> Result<()> {
        if self.stack.is_empty() {
            return Ok(());
        }

        let top_index = self.stack.len() - 1;
        if top_index > 0 && is_overlay_scene(self.stack[top_index].scene.id()) {
            let (base_scenes, overlay_scenes) = self.stack.split_at_mut(top_index);
            let base = &mut base_scenes[top_index - 1];
            let overlay = &mut overlay_scenes[0];

            if !base.setup_complete {
                base.scene.setup(renderer)?;
                base.setup_complete = true;
            }
            if !overlay.setup_complete {
                overlay.scene.setup(renderer)?;
                overlay.setup_complete = true;
            }

            renderer.set_camera(base.scene.camera());
            base.scene.render(&mut RenderContext { renderer })?;
            if self.debug_overlay.is_visible() {
                renderer.set_camera(base.scene.camera());
                base.scene.render_debug_geometry(renderer);
            }
            if is_field_scene(base.scene.id()) {
                self.field_hud.draw(renderer, ctx, base.scene.id())?;
            }
            renderer.set_camera(overlay.scene.camera());
            overlay.scene.render(&mut RenderContext { renderer })?;
        } else {
            let current = &mut self.stack[top_index];
            if !current.setup_complete {
                current.scene.setup(renderer)?;
                current.setup_complete = true;
            }
            renderer.set_camera(current.scene.camera());
            current.scene.render(&mut RenderContext { renderer })?;
            if self.debug_overlay.is_visible() {
                renderer.set_camera(current.scene.camera());
                current.scene.render_debug_geometry(renderer);
            }
            if is_field_scene(current.scene.id()) {
                self.field_hud.draw(renderer, ctx, current.scene.id())?;
            }
        }

        let snapshot = self.debug_snapshot(ctx);
        self.debug_overlay.draw(renderer, ctx, &snapshot)
    }

    pub fn camera(&self) -> Camera2d {
        let Some(current) = self.stack.last() else {
            return Camera2d::default();
        };
        if self.stack.len() > 1 && is_overlay_scene(current.scene.id()) {
            return self.stack[self.stack.len() - 2].scene.camera();
        }

        current.scene.camera()
    }

    fn debug_snapshot(&self, ctx: &GameContext) -> SceneDebugSnapshot {
        let Some(current) = self.stack.last() else {
            return SceneDebugSnapshot::new(SceneId::Boot, "EmptySceneStack");
        };

        if self.stack.len() > 1 && is_overlay_scene(current.scene.id()) {
            let mut snapshot = self.stack[self.stack.len() - 2].scene.debug_snapshot(ctx);
            snapshot.overlay_scene_name = Some(current.scene.name().to_owned());
            return snapshot;
        }

        current.scene.debug_snapshot(ctx)
    }

    fn current_field_scene_id(&self) -> Option<SceneId> {
        let current = self.stack.last()?;
        if self.stack.len() > 1 && is_overlay_scene(current.scene.id()) {
            return None;
        }
        is_field_scene(current.scene.id()).then_some(current.scene.id())
    }

    fn apply_command(
        &mut self,
        ctx: &mut GameContext,
        command: SceneCommand<SceneId>,
    ) -> Result<()> {
        match command {
            SceneCommand::None => {}
            SceneCommand::Switch(scene_id) => {
                self.stack.clear();
                self.stack
                    .push(ManagedScene::new(create_scene(scene_id, ctx)?));
            }
            SceneCommand::Push(scene_id) => {
                self.stack
                    .push(ManagedScene::new(create_scene(scene_id, ctx)?));
            }
            SceneCommand::Pop => {
                self.stack.pop();
                if self.stack.is_empty() {
                    ctx.should_quit = true;
                }
            }
            SceneCommand::Quit => {
                ctx.should_quit = true;
            }
        }

        Ok(())
    }
}

fn is_overlay_scene(scene_id: SceneId) -> bool {
    matches!(scene_id, SceneId::GameMenu | SceneId::Pause)
}

fn is_field_scene(scene_id: SceneId) -> bool {
    matches!(scene_id, SceneId::Overworld | SceneId::Facility)
}

fn handle_quickbar_input(ctx: &mut GameContext, input: &InputState) {
    if input.mouse_left_just_pressed() {
        if let Some(position) = input.cursor_position() {
            if let Some(index) = quickbar_slot_at_position(input.screen_size(), position) {
                ctx.select_quickbar_slot(index);
                return;
            }
        }
    }

    for (index, button) in [
        Button::QuickSlot1,
        Button::QuickSlot2,
        Button::QuickSlot3,
        Button::QuickSlot4,
        Button::QuickSlot5,
        Button::QuickSlot6,
    ]
    .into_iter()
    .enumerate()
    {
        if input.just_pressed(button) {
            ctx.select_quickbar_slot(index);
        }
    }
}

fn create_scene(scene_id: SceneId, ctx: &GameContext) -> Result<Box<dyn Scene>> {
    match scene_id {
        SceneId::Boot | SceneId::MainMenu => Ok(Box::new(MainMenuScene::new())),
        SceneId::Overworld => Ok(Box::new(OverworldScene::new(ctx)?)),
        SceneId::Facility => Ok(Box::new(FacilityScene::new(ctx)?)),
        SceneId::GameMenu => Ok(Box::new(GameMenuScene::new(ctx))),
        SceneId::Inventory => Ok(Box::new(InventoryScene::new(ctx))),
        SceneId::Profile => Ok(Box::new(ProfileScene::new(ctx))),
        SceneId::Pause | SceneId::Codex => Ok(Box::new(PauseScene::new())),
    }
}

impl Language {
    pub fn save_key(self) -> &'static str {
        match self {
            Self::Chinese => "Chinese",
            Self::English => "English",
        }
    }

    pub fn from_save_key(value: &str) -> Self {
        match value {
            "English" | "english" | "en" => Self::English,
            _ => Self::Chinese,
        }
    }
}

impl SceneId {
    fn save_key(self) -> &'static str {
        match self {
            SceneId::Facility => "Facility",
            _ => "Overworld",
        }
    }
}

fn scene_id_from_save_key(value: &str) -> SceneId {
    match value {
        "Facility" | "facility" => SceneId::Facility,
        _ => SceneId::Overworld,
    }
}

fn scene_id_from_transition_key(value: &str) -> SceneId {
    match value.trim().to_ascii_lowercase().as_str() {
        "facility" => SceneId::Facility,
        _ => SceneId::Overworld,
    }
}

fn position_changed(previous: Option<SaveVec2>, current: Vec2) -> bool {
    let Some(previous) = previous else {
        return true;
    };

    let dx = previous.x - current.x;
    let dy = previous.y - current.y;
    dx * dx + dy * dy > 16.0
}

fn inventory_load_units(inventory: &InventorySave) -> u32 {
    inventory
        .slots
        .iter()
        .flatten()
        .map(|stack| inventory_scene::inventory_item_weight(&stack.item_id) * stack.quantity)
        .sum()
}

struct ConsumableEffect {
    meter_id: &'static str,
    amount: u32,
}

fn consumable_effect_for_item(item_id: &str) -> Option<ConsumableEffect> {
    let effect = match item_id {
        "med_injector" => ConsumableEffect {
            meter_id: "health",
            amount: 35,
        },
        "energy_cell" => ConsumableEffect {
            meter_id: "stamina",
            amount: 35,
        },
        "coolant_canister" => ConsumableEffect {
            meter_id: "suit",
            amount: 30,
        },
        _ => return None,
    };
    Some(effect)
}

fn profile_meter_label(id: &str, language: Language) -> &'static str {
    match (id, language) {
        ("health", Language::Chinese) => "生命",
        ("stamina", Language::Chinese) => "体力",
        ("suit", Language::Chinese) => "外骨骼",
        ("load", Language::Chinese) => "负重",
        ("oxygen", Language::Chinese) => "氧气",
        ("radiation", Language::Chinese) => "辐射抗性",
        ("spores", Language::Chinese) => "孢子抗性",
        ("health", Language::English) => "Health",
        ("stamina", Language::English) => "Stamina",
        ("suit", Language::English) => "Suit",
        ("load", Language::English) => "Load",
        ("oxygen", Language::English) => "Oxygen",
        ("radiation", Language::English) => "Radiation",
        ("spores", Language::English) => "Spore resistance",
        _ => "Status",
    }
}

fn accumulated_integer_delta(accumulator: &mut f32, delta: f32) -> i32 {
    *accumulator += delta;
    let whole = if *accumulator >= 1.0 {
        (*accumulator).floor()
    } else if *accumulator <= -1.0 {
        (*accumulator).ceil()
    } else {
        0.0
    };
    *accumulator -= whole;
    whole as i32
}

fn entity_progress_key(map_path: &str, entity_id: &str) -> String {
    format!("{map_path}::{entity_id}")
}

fn zone_progress_key(map_path: &str, zone_id: &str) -> String {
    format!("{map_path}::{zone_id}")
}

fn bump_meter(meters: &mut [MeterSave], id: &str, amount: u32) {
    if let Some(meter) = meters.iter_mut().find(|meter| meter.id == id) {
        meter.value = (meter.value + amount).min(meter.max);
    }
}

fn new_save_data_for_language(language: Language) -> SaveData {
    let mut save_data = SaveData::default();
    save_data.settings.language = language.save_key().to_owned();
    save_data
}

fn weather_key_for_time(field_time_minutes: u32) -> &'static str {
    match (field_time_minutes % (24 * 60)) / 60 {
        0..=4 => "cold_mist",
        5..=10 => "clear",
        11..=16 => "ion_wind",
        17..=20 => "spore_drift",
        _ => "cold_mist",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completing_codex_scan_awards_profile_progress_once() {
        let mut ctx = GameContext::default();
        let starting_xp = ctx.save_data.profile.xp;
        let starting_bio = ctx
            .save_data
            .profile
            .meter("bio")
            .expect("bio research should exist")
            .value;

        assert!(ctx.complete_codex_scan("codex.flora.glowfungus"));
        assert!(!ctx.complete_codex_scan("codex.flora.glowfungus"));

        assert_eq!(ctx.save_data.profile.xp, starting_xp + 120);
        assert_eq!(
            ctx.save_data
                .profile
                .meter("bio")
                .expect("bio research should exist")
                .value,
            starting_bio + 6
        );
        assert!(ctx.scanned_codex_ids.contains("codex.flora.glowfungus"));
    }

    #[test]
    fn collected_entity_ids_are_scoped_to_map_path() {
        let mut ctx = GameContext::default();

        assert!(ctx.collect_entity("assets/data/maps/a.ron", "pickup_001"));
        assert!(!ctx.collect_entity("assets/data/maps/a.ron", "pickup_001"));
        assert!(ctx.is_entity_collected("assets/data/maps/a.ron", "pickup_001"));
        assert!(!ctx.is_entity_collected("assets/data/maps/b.ron", "pickup_001"));
        assert_eq!(
            ctx.collected_entity_ids_for_map("assets/data/maps/a.ron")
                .into_iter()
                .collect::<Vec<_>>(),
            vec!["pickup_001".to_owned()]
        );
    }

    #[test]
    fn triggered_zone_ids_are_scoped_to_map_path() {
        let mut ctx = GameContext::default();

        assert!(ctx.mark_zone_triggered("assets/data/maps/a.ron", "prompt_001"));
        assert!(!ctx.mark_zone_triggered("assets/data/maps/a.ron", "prompt_001"));
        assert!(ctx.is_zone_triggered("assets/data/maps/a.ron", "prompt_001"));
        assert!(!ctx.is_zone_triggered("assets/data/maps/b.ron", "prompt_001"));
        assert!(
            ctx.save_data
                .world
                .triggered_zones
                .contains("assets/data/maps/a.ron::prompt_001")
        );
    }

    #[test]
    fn unlock_rules_check_scans_and_inventory() {
        let mut ctx = GameContext::default();
        let codex_rule = MapUnlockRule {
            requires_codex_id: Some("codex.ruin.door".to_owned()),
            ..MapUnlockRule::default()
        };
        let item_rule = MapUnlockRule {
            requires_item_id: Some("ruin_key".to_owned()),
            ..MapUnlockRule::default()
        };
        let missing_item_rule = MapUnlockRule {
            requires_item_id: Some("missing_relic".to_owned()),
            ..MapUnlockRule::default()
        };

        assert!(ctx.is_unlock_rule_satisfied(None));
        assert!(!ctx.is_unlock_rule_satisfied(Some(&codex_rule)));
        ctx.scanned_codex_ids.insert("codex.ruin.door".to_owned());
        assert!(ctx.is_unlock_rule_satisfied(Some(&codex_rule)));
        assert!(ctx.is_unlock_rule_satisfied(Some(&item_rule)));
        assert!(!ctx.is_unlock_rule_satisfied(Some(&missing_item_rule)));
    }

    #[test]
    fn map_transition_target_updates_runtime_destination() {
        let mut ctx = GameContext::default();
        let transition = MapTransitionTarget {
            scene: Some("Facility".to_owned()),
            map_path: Some("assets/data/maps/facility_custom.ron".to_owned()),
            spawn_id: Some("east_entry".to_owned()),
        };

        let scene_id = ctx.apply_map_transition(
            Some(&transition),
            SceneId::Overworld,
            "assets/data/maps/overworld_landing_site.ron",
            "player_start",
        );

        assert_eq!(scene_id, SceneId::Facility);
        assert_eq!(
            ctx.facility_map_path.as_deref(),
            Some("assets/data/maps/facility_custom.ron")
        );
        assert_eq!(ctx.facility_spawn_id.as_deref(), Some("east_entry"));
        assert!(ctx.facility_player_position.is_none());
    }

    #[test]
    fn inventory_load_updates_profile_meter() {
        let mut save = SaveData::default();
        save.inventory.slots = vec![
            Some(crate::save::ItemStackSave::new(
                "alien_crystal_sample",
                3,
                false,
            )),
            None,
        ];
        let mut ctx = GameContext::from_save(PathBuf::new(), save, CodexDatabase::default());

        assert_eq!(ctx.profile_meter_value("load"), 6);
        ctx.add_inventory_item("scrap_part", 4, false);
        assert_eq!(ctx.profile_meter_value("load"), 10);
    }

    #[test]
    fn field_status_consumes_and_recovers_stamina() {
        let mut ctx = GameContext::default();
        ctx.save_data.inventory.slots.clear();
        ctx.sync_inventory_load_meter();
        ctx.save_data.profile.set_meter_value("stamina", 10);

        ctx.update_field_status(
            1.0,
            FieldActivity {
                moving: true,
                scanning: true,
                jumped: false,
                environment: FieldEnvironment::Overworld,
            },
        );
        assert_eq!(ctx.profile_meter_value("stamina"), 7);

        ctx.update_field_status(
            1.0,
            FieldActivity {
                moving: false,
                scanning: false,
                jumped: false,
                environment: FieldEnvironment::Overworld,
            },
        );
        assert_eq!(ctx.profile_meter_value("stamina"), 12);
    }

    #[test]
    fn zone_meter_effects_accumulate_into_profile_meters() {
        let mut ctx = GameContext::default();
        ctx.save_data.profile.set_meter_value("oxygen", 62);

        assert!(ctx.apply_zone_meter_effect("oxygen", -2.0, 1.0));

        assert_eq!(ctx.profile_meter_value("oxygen"), 60);
        assert!(ctx.save_dirty);
    }

    #[test]
    fn selected_quickbar_consumable_restores_meter_and_consumes_stack() {
        let mut ctx = GameContext::default();
        ctx.save_data.inventory.selected_slot = 7;
        ctx.save_data.profile.set_meter_value("health", 50);

        let result = ctx.use_selected_quickbar_item();

        assert_eq!(
            result,
            QuickItemUseResult::Used {
                item_id: "med_injector".to_owned(),
                meter_id: "health",
                amount: 35,
            }
        );
        assert_eq!(ctx.profile_meter_value("health"), 85);
        assert_eq!(
            ctx.save_data.inventory.slots[7]
                .as_ref()
                .map(|stack| stack.quantity),
            Some(1)
        );
        assert!(ctx.save_dirty);
        assert!(
            ctx.save_data
                .activity_log
                .entries
                .iter()
                .any(|entry| entry.category == LOG_CATEGORY_ITEM)
        );
    }

    #[test]
    fn selected_quickbar_consumable_is_not_spent_when_meter_is_full() {
        let mut ctx = GameContext::default();
        ctx.save_data.inventory.selected_slot = 7;
        ctx.save_data.profile.set_meter_value("health", 100);

        let result = ctx.use_selected_quickbar_item();

        assert_eq!(
            result,
            QuickItemUseResult::AlreadyFull {
                item_id: "med_injector".to_owned(),
                meter_id: "health".to_owned(),
            }
        );
        assert_eq!(
            ctx.save_data.inventory.slots[7]
                .as_ref()
                .map(|stack| stack.quantity),
            Some(2)
        );
    }

    #[test]
    fn facility_environment_changes_persistent_condition_meters() {
        let mut ctx = GameContext::default();
        let starting_oxygen = ctx.profile_meter_value("oxygen");

        ctx.update_field_status(
            10.0,
            FieldActivity {
                moving: false,
                scanning: false,
                jumped: false,
                environment: FieldEnvironment::Facility,
            },
        );

        assert!(ctx.profile_meter_value("oxygen") < starting_oxygen);
        assert!(ctx.save_dirty);
    }

    #[test]
    fn activity_log_records_scans_inventory_and_status() {
        let mut ctx = GameContext::default();

        assert!(ctx.complete_codex_scan("codex.flora.glowfungus"));
        ctx.log_inventory_full();
        ctx.save_data.profile.set_meter_value("stamina", 12);
        ctx.update_field_status(
            0.0,
            FieldActivity {
                moving: false,
                scanning: false,
                jumped: false,
                environment: FieldEnvironment::Overworld,
            },
        );

        let categories = ctx
            .save_data
            .activity_log
            .entries
            .iter()
            .map(|entry| entry.category.as_str())
            .collect::<Vec<_>>();
        assert!(categories.contains(&LOG_CATEGORY_SCAN));
        assert!(categories.contains(&LOG_CATEGORY_PICKUP));
        assert!(categories.contains(&LOG_CATEGORY_STATUS));
    }

    #[test]
    fn objective_zone_updates_save_and_activity_log() {
        let mut ctx = GameContext::default();
        let rule = MapObjectiveRule {
            objective_id: "secure_landing_site".to_owned(),
            checkpoint_id: Some("landing_perimeter".to_owned()),
            complete_objective: false,
            message: None,
            log_title: None,
            log_detail: None,
            once: true,
        };

        let event = ctx
            .apply_objective_zone(&rule)
            .expect("checkpoint should update objective");

        assert_eq!(event.objective_id, "secure_landing_site");
        let state = ctx
            .save_data
            .objectives
            .states
            .get("secure_landing_site")
            .expect("objective state should be saved");
        assert_eq!(state.status, "active");
        assert!(state.completed_checkpoints.contains("landing_perimeter"));
        assert!(
            ctx.save_data
                .activity_log
                .entries
                .iter()
                .any(|entry| entry.category == LOG_CATEGORY_OBJECTIVE)
        );
        assert!(
            ctx.objective_menu_rows()
                .iter()
                .any(|row| row.id == "secure_landing_site" && row.progress == 50)
        );
    }
}
