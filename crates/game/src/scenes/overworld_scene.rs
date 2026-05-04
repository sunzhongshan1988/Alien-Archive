use anyhow::Result;
use runtime::{Button, Camera2d, InputState, Renderer, SceneCommand, collision::rects_overlap};

use crate::{
    player::Player,
    world::{MapEntityKind, World},
};

use super::{GameContext, RenderContext, Scene, SceneId};

const OVERWORLD_MAP: &str = "assets/data/maps/overworld_landing_site.ron";
const FACILITY_ENTRY_SPAWN: &str = "entry";
const OVERWORLD_CAMERA_ZOOM: f32 = 2.0;

pub struct OverworldScene {
    player: Player,
    world: World,
    camera: Camera2d,
}

impl OverworldScene {
    pub fn new(spawn_id: Option<&str>) -> Result<Self> {
        let world = World::load(OVERWORLD_MAP, spawn_id)?;
        let player = Player::new(world.player_spawn());

        Ok(Self {
            camera: Camera2d::follow_with_zoom(player.position, OVERWORLD_CAMERA_ZOOM),
            player,
            world,
        })
    }

    fn player_overlaps(&self, kind: MapEntityKind) -> bool {
        self.world
            .first_entity(kind)
            .is_some_and(|entity| rects_overlap(self.player.rect(), entity.rect))
    }
}

impl Scene for OverworldScene {
    fn id(&self) -> SceneId {
        SceneId::Overworld
    }

    fn name(&self) -> &str {
        "OverworldScene"
    }

    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        self.world.load_assets(renderer)?;
        Player::load_topdown_assets(renderer)
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

        if input.just_pressed(Button::Interact)
            && self.player_overlaps(MapEntityKind::FacilityEntrance)
        {
            ctx.facility_spawn_id = Some(FACILITY_ENTRY_SPAWN.to_owned());
            return Ok(SceneCommand::Switch(SceneId::Facility));
        }

        self.player.update(dt, input);
        self.camera.position = self.player.position;

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.world.draw(ctx.renderer);
        self.player.draw_topdown(ctx.renderer);
        Ok(())
    }

    fn camera(&self) -> Camera2d {
        self.camera
    }
}
