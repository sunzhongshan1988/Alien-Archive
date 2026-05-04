use anyhow::Result;
use runtime::{Button, Camera2d, InputState, SceneCommand, Vec2, collision::rects_overlap};

use crate::{
    player::Player,
    world::{MapEntityKind, World},
};

use super::{GameContext, RenderContext, Scene, SceneId};

const FACILITY_MAP: &str = "assets/data/maps/facility_ruin_01.ron";
const OVERWORLD_RETURN_SPAWN: &str = "facility_return";
const RUN_SPEED: f32 = 260.0;
const GRAVITY: f32 = 980.0;
const JUMP_SPEED: f32 = 430.0;

pub struct FacilityScene {
    player: Player,
    world: World,
    camera: Camera2d,
    velocity: Vec2,
    floor_y: f32,
    grounded: bool,
}

impl FacilityScene {
    pub fn new(spawn_id: Option<&str>) -> Result<Self> {
        let world = World::load(FACILITY_MAP, spawn_id)?;
        let player = Player::new(world.player_spawn());

        Ok(Self {
            camera: Camera2d::follow(player.position),
            floor_y: player.position.y,
            player,
            world,
            velocity: Vec2::ZERO,
            grounded: true,
        })
    }

    fn player_overlaps(&self, kind: MapEntityKind) -> bool {
        self.world
            .first_entity(kind)
            .is_some_and(|entity| rects_overlap(self.player.rect(), entity.rect))
    }

    fn update_sideview_player(&mut self, dt: f32, input: &InputState) {
        let horizontal = match (input.is_down(Button::Left), input.is_down(Button::Right)) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };

        self.velocity.x = horizontal * RUN_SPEED;

        if self.grounded && (input.just_pressed(Button::Up) || input.just_pressed(Button::Scan)) {
            self.velocity.y = -JUMP_SPEED;
            self.grounded = false;
        }

        self.velocity.y += GRAVITY * dt;
        self.player.position += self.velocity * dt;

        if self.player.position.y >= self.floor_y {
            self.player.position.y = self.floor_y;
            self.velocity.y = 0.0;
            self.grounded = true;
        }
    }
}

impl Scene for FacilityScene {
    fn id(&self) -> SceneId {
        SceneId::Facility
    }

    fn name(&self) -> &str {
        "FacilityScene"
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if input.just_pressed(Button::Pause) {
            return Ok(SceneCommand::Push(SceneId::Pause));
        }

        if input.just_pressed(Button::Interact) && self.player_overlaps(MapEntityKind::FacilityExit)
        {
            ctx.overworld_spawn_id = Some(OVERWORLD_RETURN_SPAWN.to_owned());
            return Ok(SceneCommand::Switch(SceneId::Overworld));
        }

        self.update_sideview_player(dt, input);
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
