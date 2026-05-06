mod facility_scene;
mod inventory_scene;
mod main_menu;
mod overworld_scene;
mod pause_scene;

use anyhow::Result;
use runtime::{Camera2d, InputState, Renderer, SceneCommand};

use facility_scene::FacilityScene;
use inventory_scene::InventoryScene;
use main_menu::MainMenuScene;
use overworld_scene::OverworldScene;
use pause_scene::PauseScene;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneId {
    Boot,
    MainMenu,
    Overworld,
    Facility,
    Inventory,
    Codex,
    Pause,
}

#[derive(Default)]
pub struct GameContext {
    pub should_quit: bool,
    pub overworld_spawn_id: Option<String>,
    pub facility_spawn_id: Option<String>,
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
        self.setup_current(renderer)?;

        let Some(current) = self.stack.last_mut() else {
            return Ok(());
        };

        current.scene.render(&mut RenderContext { renderer })
    }

    pub fn camera(&self) -> Camera2d {
        self.stack
            .last()
            .map(|scene| scene.scene.camera())
            .unwrap_or_default()
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

fn create_scene(scene_id: SceneId, ctx: &GameContext) -> Result<Box<dyn Scene>> {
    match scene_id {
        SceneId::Boot | SceneId::MainMenu => Ok(Box::new(MainMenuScene::new())),
        SceneId::Overworld => Ok(Box::new(OverworldScene::new(
            ctx.overworld_spawn_id.as_deref(),
        )?)),
        SceneId::Facility => Ok(Box::new(FacilityScene::new(
            ctx.facility_spawn_id.as_deref(),
        )?)),
        SceneId::Inventory => Ok(Box::new(InventoryScene::new())),
        SceneId::Pause | SceneId::Codex => Ok(Box::new(PauseScene::new())),
    }
}
