mod player;
mod scenes;
mod ui;
mod world;

use anyhow::Result;
use runtime::{Camera2d, Game, InputState, Renderer, run};
use scenes::{GameContext, SceneStack};

struct AlienArchiveApp {
    scenes: SceneStack,
    context: GameContext,
}

impl AlienArchiveApp {
    fn new() -> Result<Self> {
        Ok(Self {
            scenes: SceneStack::new_main_menu(),
            context: GameContext::default(),
        })
    }
}

impl Game for AlienArchiveApp {
    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        self.scenes.setup_current(renderer)
    }

    fn update(&mut self, dt: f32, input: &InputState) -> Result<()> {
        self.scenes.update(&mut self.context, dt, input)
    }

    fn render(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        self.scenes.render(renderer)
    }

    fn camera(&self) -> Camera2d {
        self.scenes.camera()
    }

    fn should_exit(&self) -> bool {
        self.context.should_quit
    }
}

fn main() -> Result<()> {
    run("Alien Archive", AlienArchiveApp::new()?)
}
