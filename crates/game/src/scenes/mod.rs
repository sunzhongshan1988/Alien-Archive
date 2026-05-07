mod facility_scene;
mod game_menu_scene;
mod inventory_scene;
mod main_menu;
mod overworld_scene;
mod pause_scene;
mod profile_scene;
mod scan_system;

use anyhow::Result;
use runtime::{Camera2d, InputState, Renderer, SceneCommand};
use std::collections::HashSet;

use facility_scene::FacilityScene;
use game_menu_scene::GameMenuScene;
use inventory_scene::InventoryScene;
use main_menu::MainMenuScene;
use overworld_scene::OverworldScene;
use pause_scene::PauseScene;
use profile_scene::ProfileScene;

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
    pub overworld_spawn_id: Option<String>,
    pub facility_spawn_id: Option<String>,
    pub scanned_codex_ids: HashSet<String>,
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
        SceneId::Overworld => Ok(Box::new(OverworldScene::new(
            ctx.overworld_spawn_id.as_deref(),
        )?)),
        SceneId::Facility => Ok(Box::new(FacilityScene::new(
            ctx.facility_spawn_id.as_deref(),
        )?)),
        SceneId::GameMenu => Ok(Box::new(GameMenuScene::new(
            ctx.language,
            ctx.game_menu_tab,
        ))),
        SceneId::Inventory => Ok(Box::new(InventoryScene::new(ctx.language))),
        SceneId::Profile => Ok(Box::new(ProfileScene::new(ctx.language))),
        SceneId::Pause | SceneId::Codex => Ok(Box::new(PauseScene::new())),
    }
}
