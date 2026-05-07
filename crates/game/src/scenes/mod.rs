mod facility_scene;
mod game_menu_scene;
mod inventory_scene;
mod main_menu;
mod notice_system;
mod overworld_scene;
mod pause_scene;
mod profile_scene;
mod rewards;
mod scan_system;

use anyhow::Result;
use content::CodexDatabase;
use runtime::{Camera2d, InputState, Renderer, SceneCommand, Vec2};
use std::{collections::HashSet, path::PathBuf};

use crate::save::{CodexSave, MeterSave, SaveData, SaveVec2, WorldSave};
use crate::world::MapUnlockRule;

use facility_scene::FacilityScene;
use game_menu_scene::GameMenuScene;
use inventory_scene::InventoryScene;
use main_menu::MainMenuScene;
use overworld_scene::OverworldScene;
use pause_scene::PauseScene;
use profile_scene::ProfileScene;

const AUTOSAVE_INTERVAL: f32 = 5.0;

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
    pub scanned_codex_ids: HashSet<String>,
    pub save_path: PathBuf,
    pub save_data: SaveData,
    save_dirty: bool,
    save_requested: bool,
    save_timer: f32,
}

impl GameContext {
    pub fn from_save(
        save_path: PathBuf,
        save_data: SaveData,
        codex_database: CodexDatabase,
    ) -> Self {
        let language = Language::from_save_key(&save_data.settings.language);
        let scanned_codex_ids = save_data.codex.scanned_ids.iter().cloned().collect();
        let mut ctx = Self {
            language,
            codex_database,
            scanned_codex_ids,
            save_path,
            save_data,
            ..Self::default()
        };
        ctx.apply_world_save();
        ctx
    }

    pub fn resume_scene_id(&self) -> SceneId {
        scene_id_from_save_key(&self.save_data.world.current_scene)
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
            self.request_save();
        }
        added
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
        self.save_data.settings.language = self.language.save_key().to_owned();
        self.save_data.codex = CodexSave::from_runtime(&self.scanned_codex_ids);
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

    fn camera(&self) -> Camera2d {
        Camera2d::default()
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
}

impl SceneStack {
    pub fn new_main_menu() -> Self {
        Self {
            stack: vec![ManagedScene::new(Box::new(MainMenuScene::new()))],
        }
    }

    pub fn new_scene(scene_id: SceneId, ctx: &GameContext) -> Result<Self> {
        Ok(Self {
            stack: vec![ManagedScene::new(create_scene(scene_id, ctx)?)],
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
        let Some(current) = self.stack.last_mut() else {
            ctx.should_quit = true;
            return Ok(());
        };

        let command = current.scene.update(ctx, dt, input)?;
        self.apply_command(ctx, command)
    }

    pub fn render(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
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
            renderer.set_camera(overlay.scene.camera());
            overlay.scene.render(&mut RenderContext { renderer })?;
            return Ok(());
        }

        let current = &mut self.stack[top_index];
        if !current.setup_complete {
            current.scene.setup(renderer)?;
            current.setup_complete = true;
        }
        renderer.set_camera(current.scene.camera());
        current.scene.render(&mut RenderContext { renderer })
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

fn position_changed(previous: Option<SaveVec2>, current: Vec2) -> bool {
    let Some(previous) = previous else {
        return true;
    };

    let dx = previous.x - current.x;
    let dy = previous.y - current.y;
    dx * dx + dy * dy > 16.0
}

fn entity_progress_key(map_path: &str, entity_id: &str) -> String {
    format!("{map_path}::{entity_id}")
}

fn bump_meter(meters: &mut [MeterSave], id: &str, amount: u32) {
    if let Some(meter) = meters.iter_mut().find(|meter| meter.id == id) {
        meter.value = (meter.value + amount).min(meter.max);
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
}
