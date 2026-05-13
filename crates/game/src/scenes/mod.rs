mod activity_log;
mod debug_overlay;
mod facility_scene;
mod field_hud;
mod game_menu_activity;
mod game_menu_art;
mod game_menu_codex;
mod game_menu_feedback;
mod game_menu_inventory;
mod game_menu_map;
mod game_menu_profile;
mod game_menu_scene;
mod inventory_scene;
mod main_menu;
mod notice_system;
mod overworld_scene;
mod pause_scene;
mod profile_derived;
mod profile_scene;
mod profile_status;
mod quick_items;
mod rewards;
mod scan_system;
mod world_runtime;
mod zone_system;

use anyhow::Result;
use content::{CodexDatabase, semantics};
use runtime::{Button, Camera2d, InputState, Renderer, SceneCommand, Vec2};
use std::{collections::HashSet, path::PathBuf};

use crate::objectives::{ObjectiveDatabase, ObjectiveMenuRow};
use crate::save::{CodexSave, InventorySave, SaveData};
use crate::world::{MapObjectiveRule, MapPromptRule, MapTransitionTarget, MapUnlockRule};

use activity_log::ActivityLogEvent;
use facility_scene::FacilityScene;
use game_menu_scene::GameMenuScene;
use inventory_scene::InventoryScene;
use main_menu::MainMenuScene;
use overworld_scene::OverworldScene;
use pause_scene::PauseScene;
use profile_scene::ProfileScene;

use debug_overlay::{DebugOverlay, SceneDebugSnapshot};
use field_hud::{FieldHud, quickbar_slot_at_position};
pub use profile_derived::{DerivedMeterValue, DerivedScoreValue, ProfileDerivedState};
use profile_derived::{ProfileDerivationInput, derive_profile_state, inventory_load_units};
pub use profile_status::{FieldActivity, FieldEnvironment, FieldStatusEffects};
use profile_status::{ProfileStatusRuntime, STAMINA_JUMP_COST, StatusAlert, StatusSnapshot};
pub use quick_items::QuickItemUseResult;
use world_runtime::{entity_progress_key, zone_progress_key};

const AUTOSAVE_INTERVAL: f32 = 5.0;
const CRITICAL_HEALTH_DRAIN_PER_SECOND: f32 = 0.35;

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
        ctx.sync_derived_profile_state();
        ctx.apply_world_save();
        ctx
    }

    pub fn objective_menu_rows(&self) -> Vec<ObjectiveMenuRow> {
        self.objective_database
            .menu_rows(&self.save_data.objectives, self.language)
    }

    pub fn resume_scene_id(&self) -> SceneId {
        world_runtime::scene_id_from_save_key(&self.save_data.world.current_scene)
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
            self.add_inventory_item(&reward.item_id, reward.quantity, reward.locked);
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
        self.profile_meter_value(semantics::METER_STAMINA) as i32 >= STAMINA_JUMP_COST
    }

    pub fn update_field_status(&mut self, dt: f32, activity: FieldActivity) -> FieldStatusEffects {
        self.sync_inventory_load_meter();

        if activity.jumped {
            self.change_profile_meter(semantics::METER_STAMINA, -STAMINA_JUMP_COST);
        }

        self.advance_field_clock(dt);
        let load_ratio = self.profile_meter_ratio(semantics::METER_LOAD);
        for delta in self
            .profile_status_runtime
            .field_meter_deltas(dt, activity, load_ratio)
        {
            self.change_profile_meter(delta.meter_id, delta.delta);
        }

        if self.profile_meter_value(semantics::METER_SUIT) == 0
            || self.profile_meter_value(semantics::METER_OXYGEN) == 0
        {
            self.accumulate_profile_delta(
                semantics::METER_HEALTH,
                -CRITICAL_HEALTH_DRAIN_PER_SECOND * dt,
            );
        }
        self.update_activity_status_alerts();

        FieldStatusEffects {
            movement_speed_multiplier: self.profile_movement_speed_multiplier(),
        }
    }

    pub fn sync_derived_profile_state(&mut self) -> bool {
        let mut changed = self.sync_inventory_load_meter();
        let derived = self.profile_derived_state();
        changed |= self.apply_profile_derived_state(&derived);

        if changed {
            self.mark_save_dirty();
        }
        changed
    }

    pub fn profile_derived_state(&self) -> ProfileDerivedState {
        derive_profile_state(ProfileDerivationInput {
            profile: &self.save_data.profile,
            inventory: &self.save_data.inventory,
            scanned_codex_ids: &self.scanned_codex_ids,
            codex_entry_count: self.codex_database.entries().len(),
            collected_entity_count: self.save_data.world.collected_entities.len(),
        })
    }

    pub fn profile_meter_value(&self, id: &str) -> u32 {
        self.save_data
            .profile
            .meter(id)
            .map_or(0, |meter| meter.value)
    }

    pub fn select_quickbar_slot(&mut self, quick_index: usize) -> bool {
        if !quick_items::select_quickbar_slot(&mut self.save_data.inventory, quick_index) {
            return false;
        }

        self.request_save();
        true
    }

    pub fn use_selected_quickbar_item(&mut self) -> QuickItemUseResult {
        let result = quick_items::use_selected_quickbar_item(
            &mut self.save_data.inventory,
            &mut self.save_data.profile,
        );
        if let QuickItemUseResult::Used {
            item_id,
            meter_id,
            amount,
        } = &result
        {
            self.sync_inventory_load_meter();
            self.log_item_used(item_id, meter_id, *amount);
            self.request_save();
        }
        result
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
        self.push_activity_event(activity_log::inventory_full(self.language));
    }

    pub fn log_stamina_low(&mut self) {
        self.push_activity_event(activity_log::stamina_low(self.language));
    }

    pub fn log_locked_unlock_rule(&mut self, unlock: Option<&MapUnlockRule>) {
        let detail = self.locked_rule_log_detail(unlock);
        self.push_activity_event(activity_log::access_blocked(self.language, detail));
    }

    pub fn log_zone_prompt(&mut self, prompt: &MapPromptRule, fallback: &str) {
        self.push_activity_event(activity_log::zone_prompt(self.language, prompt, fallback));
    }

    pub fn log_zone_hazard(&mut self, zone_id: &str, detail: String) {
        self.push_activity_event(activity_log::zone_hazard(self.language, zone_id, detail));
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
        self.push_activity_event(activity_log::objective(
            objective
                .log_title
                .clone()
                .unwrap_or_else(|| event.log_title.clone()),
            objective
                .log_detail
                .clone()
                .unwrap_or_else(|| event.log_detail.clone()),
        ));
        Some(event)
    }

    pub fn log_scene_transition(&mut self, scene_id: SceneId, map_path: &str) {
        self.push_activity_event(activity_log::scene_transition(
            self.language,
            scene_id,
            map_path,
        ));
    }

    pub fn apply_map_transition(
        &mut self,
        transition: Option<&MapTransitionTarget>,
        default_scene: SceneId,
        default_map: &str,
        default_spawn: &str,
    ) -> SceneId {
        let destination = world_runtime::resolve_map_transition(
            transition,
            default_scene,
            default_map,
            default_spawn,
        );

        match destination.scene_id {
            SceneId::Facility => {
                self.facility_map_path = Some(destination.map_path.clone());
                self.facility_spawn_id = Some(destination.spawn_id.clone());
                self.facility_player_position = None;
            }
            _ => {
                self.overworld_map_path = Some(destination.map_path.clone());
                self.overworld_spawn_id = Some(destination.spawn_id.clone());
                self.overworld_player_position = None;
            }
        }

        self.log_scene_transition(destination.scene_id, &destination.map_path);
        self.request_save();
        destination.scene_id
    }

    fn log_item_used(&mut self, item_id: &str, meter_id: &str, amount: u32) {
        let item_name = inventory_scene::inventory_item_name(item_id, self.language);
        let meter_name = profile_meter_label(meter_id, self.language);
        self.push_activity_event(activity_log::item_used(
            self.language,
            &item_name,
            meter_name,
            amount,
        ));
    }

    fn push_activity_event(&mut self, event: ActivityLogEvent) {
        self.save_data.activity_log.push(
            event.category,
            event.title,
            event.detail,
            self.save_data.world.current_scene.clone(),
            self.save_data.world.map_path.clone(),
        );
        self.request_save();
    }

    pub fn collected_entity_ids_for_map(
        &self,
        map_path: &str,
    ) -> std::collections::BTreeSet<String> {
        world_runtime::collected_entity_ids_for_map(
            &self.save_data.world.collected_entities,
            map_path,
        )
    }

    pub fn mark_save_dirty(&mut self) {
        self.save_dirty = true;
    }

    pub fn record_world_location(&mut self, scene_id: SceneId, map_path: &str, position: Vec2) {
        let spawn_id = match scene_id {
            SceneId::Facility => self.facility_spawn_id.clone(),
            _ => self.overworld_spawn_id.clone(),
        };
        let update = world_runtime::make_world_location_update(
            &self.save_data.world,
            scene_id,
            map_path,
            spawn_id,
            position,
        );
        self.save_data.world = update.world;

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

        if update.changed {
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
        match world_runtime::scene_id_from_save_key(&world.current_scene) {
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
        self.sync_derived_profile_state();
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
        let _ = codex_id;
        self.sync_derived_profile_state();
    }

    fn sync_inventory_load_meter(&mut self) -> bool {
        let load = inventory_load_units(&self.save_data.inventory);
        self.set_profile_meter_value(semantics::METER_LOAD, load)
    }

    fn sync_inventory_load_meter_silent(&mut self) {
        let load = inventory_load_units(&self.save_data.inventory);
        self.save_data
            .profile
            .set_meter_value(semantics::METER_LOAD, load);
    }

    fn apply_profile_derived_state(&mut self, derived: &ProfileDerivedState) -> bool {
        let profile = &mut self.save_data.profile;
        let mut changed = profile.level != derived.level
            || profile.xp != derived.xp
            || profile.xp_next != derived.xp_next;
        if changed {
            profile.level = derived.level;
            profile.xp = derived.xp;
            profile.xp_next = derived.xp_next;
        }

        for meter in &mut profile.research {
            let Some(derived_meter) = derived.research.iter().find(|entry| entry.id == meter.id)
            else {
                continue;
            };
            let next_value = derived_meter.value.min(meter.max);
            if meter.value != next_value {
                meter.value = next_value;
                changed = true;
            }
        }

        for score in &derived.attributes {
            changed |= profile.set_score_value(&score.id, score.value);
        }
        changed
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
        let Some(whole_delta) = self.profile_status_runtime.accumulated_delta(id, delta) else {
            return false;
        };
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
        self.profile_derived_state().movement_speed_multiplier
    }

    fn advance_field_clock(&mut self, dt: f32) -> bool {
        let whole_minutes = self.profile_status_runtime.advance_field_clock(dt);
        if whole_minutes == 0 {
            return false;
        }

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
        self.push_activity_event(activity_log::item_added(
            self.language,
            &item_name,
            quantity,
        ));
    }

    fn log_codex_scan(&mut self, codex_id: &str) {
        let entry_title = self
            .codex_database
            .get(codex_id)
            .map(|entry| entry.title.trim())
            .filter(|title| !title.is_empty())
            .unwrap_or(codex_id);
        let research_meter = rewards::research_meter_for_codex(codex_id);
        self.push_activity_event(activity_log::codex_scan(
            self.language,
            entry_title,
            research_meter,
        ));
    }

    fn locked_rule_log_detail(&self, unlock: Option<&MapUnlockRule>) -> String {
        let Some(unlock) = unlock else {
            return activity_log::access_unavailable(self.language);
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
        activity_log::access_blocked_detail(self.language, None, codex_title, item_name.as_deref())
    }

    fn update_activity_status_alerts(&mut self) {
        let snapshot = StatusSnapshot {
            stamina_ratio: self.profile_meter_ratio(semantics::METER_STAMINA),
            load_ratio: self.profile_meter_ratio(semantics::METER_LOAD),
            suit_ratio: self.profile_meter_ratio(semantics::METER_SUIT),
            oxygen_ratio: self.profile_meter_ratio(semantics::METER_OXYGEN),
            health_ratio: self.profile_meter_ratio(semantics::METER_HEALTH),
        };
        let alerts = self.profile_status_runtime.status_alerts(snapshot);
        for alert in alerts {
            let event = match alert {
                StatusAlert::StaminaLow => activity_log::status_stamina_low(self.language),
                StatusAlert::HeavyLoad => activity_log::status_load_high(self.language),
                StatusAlert::SuitCritical => activity_log::status_suit_low(self.language),
                StatusAlert::OxygenCritical => activity_log::status_oxygen_low(self.language),
                StatusAlert::HealthCritical => activity_log::status_health_critical(self.language),
            };
            self.push_activity_event(event);
        }
    }
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

fn profile_meter_label(id: &str, language: Language) -> &'static str {
    if let Some(meter) = semantics::meter_def(id) {
        match language {
            Language::Chinese => meter.zh_label,
            Language::English => meter.english_label,
        }
    } else {
        "Status"
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
    use content::CodexEntry;

    fn codex_database_with_entries(ids: &[&str]) -> CodexDatabase {
        let mut database = CodexDatabase::new("Test");
        database.entries = ids
            .iter()
            .map(|id| CodexEntry {
                id: (*id).to_owned(),
                category: "Test".to_owned(),
                title: (*id).to_owned(),
                description: String::new(),
                scan_time: Some(1.25),
                unlock_tags: Vec::new(),
                image: None,
            })
            .collect();
        database.reindex();
        database
    }

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
    fn derived_profile_state_uses_real_progress_sources() {
        let database =
            codex_database_with_entries(&["codex.flora.glowfungus", "codex.ruin.terminal"]);
        let mut ctx = GameContext::from_save(PathBuf::new(), SaveData::default(), database);

        ctx.scanned_codex_ids
            .insert("codex.flora.glowfungus".to_owned());
        ctx.save_data
            .world
            .collected_entities
            .insert("assets/data/maps/a.ron::pickup_001".to_owned());
        let derived = ctx.profile_derived_state();
        ctx.sync_derived_profile_state();

        assert_eq!(derived.level, 1);
        assert_eq!(derived.xp, 120);
        assert_eq!(
            derived
                .research
                .iter()
                .find(|meter| meter.id == "bio")
                .map(|meter| meter.value),
            Some(6)
        );
        assert_eq!(ctx.save_data.profile.level, 1);
        assert_eq!(ctx.save_data.profile.xp, 120);
        assert_eq!(
            ctx.save_data.profile.meter("bio").map(|meter| meter.value),
            Some(6)
        );
        assert_eq!(
            ctx.save_data
                .profile
                .score("scanning")
                .map(|score| score.value),
            Some(5)
        );
        assert!(
            ctx.save_data
                .profile
                .score("harvesting")
                .is_some_and(|score| score.value > 0)
        );
        assert!(
            ctx.save_data
                .profile
                .score("analysis")
                .is_some_and(|score| score.value > 0)
        );
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
                .any(|entry| entry.category == activity_log::LOG_CATEGORY_ITEM)
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
        assert!(categories.contains(&activity_log::LOG_CATEGORY_SCAN));
        assert!(categories.contains(&activity_log::LOG_CATEGORY_PICKUP));
        assert!(categories.contains(&activity_log::LOG_CATEGORY_STATUS));
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
                .any(|entry| entry.category == activity_log::LOG_CATEGORY_OBJECTIVE)
        );
        assert!(
            ctx.objective_menu_rows()
                .iter()
                .any(|row| row.id == "secure_landing_site" && row.progress == 50)
        );
    }
}
