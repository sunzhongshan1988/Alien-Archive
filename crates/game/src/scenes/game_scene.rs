use anyhow::Result;
use runtime::{Button, Camera2d, InputState, SceneCommand};

use crate::{player::Player, world::World};

use super::{GameContext, RenderContext, Scene, SceneId};

pub struct GameScene {
    player: Player,
    world: World,
    camera: Camera2d,
}

impl GameScene {
    pub fn new() -> Result<Self> {
        let world = World::load_demo()?;
        let player = Player::new(world.player_spawn());

        Ok(Self {
            camera: Camera2d::follow(player.position),
            player,
            world,
        })
    }
}

impl Scene for GameScene {
    fn id(&self) -> SceneId {
        SceneId::Game
    }

    fn name(&self) -> &str {
        "GameScene"
    }

    fn update(
        &mut self,
        _ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if input.just_pressed(Button::Pause) {
            return Ok(SceneCommand::Push(SceneId::Pause));
        }

        self.player.update(dt, input);
        self.camera.position = self.player.position;

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.world.draw(ctx.renderer);
        self.player.draw(ctx.renderer);
        Ok(())
    }

    fn camera(&self) -> Camera2d {
        self.camera
    }
}
