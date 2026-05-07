use anyhow::Result;
use runtime::{Button, Camera2d, InputState, SceneCommand, Vec2, collision::rects_overlap};

use crate::{
    player::Player,
    world::{MapEntity, MapEntityKind, World},
};

use super::{
    GameContext, GameMenuTab, RenderContext, Scene, SceneId,
    notice_system::NoticeState,
    rewards,
    scan_system::{ScanState, nearby_scan_target},
};

const FACILITY_MAP: &str = "assets/data/maps/facility_ruin_01.ron";
const OVERWORLD_RETURN_SPAWN: &str = "facility_return";
const RUN_SPEED: f32 = 260.0;
const GRAVITY: f32 = 980.0;
const JUMP_SPEED: f32 = 430.0;

pub struct FacilityScene {
    player: Player,
    world: World,
    map_path: String,
    camera: Camera2d,
    velocity: Vec2,
    floor_y: f32,
    grounded: bool,
    scan: ScanState,
    notice: NoticeState,
}

impl FacilityScene {
    pub fn new(ctx: &GameContext) -> Result<Self> {
        let map_path = ctx
            .facility_map_path
            .as_deref()
            .unwrap_or(FACILITY_MAP)
            .to_owned();
        let mut world = World::load(&map_path, ctx.facility_spawn_id.as_deref())?;
        world.remove_entities_by_id(&ctx.collected_entity_ids_for_map(&map_path));
        let player = Player::new(
            ctx.facility_player_position
                .unwrap_or_else(|| world.player_spawn()),
        );

        Ok(Self {
            camera: Camera2d::follow(player.position),
            floor_y: player.position.y,
            player,
            world,
            map_path,
            velocity: Vec2::ZERO,
            grounded: true,
            scan: ScanState::default(),
            notice: NoticeState::default(),
        })
    }

    fn overlapping_entity(&self, kind: MapEntityKind) -> Option<&MapEntity> {
        self.world
            .entities(kind)
            .find(|entity| rects_overlap(self.player.rect(), entity.rect))
    }

    fn overlapping_pickup(&self) -> Option<&MapEntity> {
        self.world.all_entities().find(|entity| {
            rewards::pickup_reward_for_entity(entity).is_some()
                && rects_overlap(self.player.rect(), entity.rect)
        })
    }

    fn try_collect_pickup(&mut self, ctx: &mut GameContext) -> bool {
        let Some(entity) = self.overlapping_pickup().cloned() else {
            return false;
        };
        let Some(reward) = rewards::pickup_reward_for_entity(&entity) else {
            return false;
        };

        if ctx.is_entity_collected(&self.map_path, &entity.id) {
            return true;
        }

        let added = ctx.add_inventory_item(reward.item_id, reward.quantity, reward.locked);
        if added == 0 {
            self.notice.push_inventory_full(ctx.language);
            return true;
        }

        ctx.collect_entity(&self.map_path, &entity.id);
        self.world
            .remove_entities_by_id(&ctx.collected_entity_ids_for_map(&self.map_path));
        self.notice.push_pickup(ctx.language, reward.item_id, added);
        true
    }

    fn update_sideview_player(&mut self, dt: f32, input: &InputState, scan_jump_blocked: bool) {
        let horizontal = match (input.is_down(Button::Left), input.is_down(Button::Right)) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };

        self.velocity.x = horizontal * RUN_SPEED;

        if self.grounded
            && (input.just_pressed(Button::Up)
                || (input.just_pressed(Button::Scan) && !scan_jump_blocked))
        {
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
        self.notice.update(dt);

        if input.just_pressed(Button::Pause) {
            return Ok(SceneCommand::Push(SceneId::GameMenu));
        }

        if input.just_pressed(Button::Inventory) {
            ctx.game_menu_tab = GameMenuTab::Inventory;
            return Ok(SceneCommand::Push(SceneId::GameMenu));
        }

        if input.just_pressed(Button::Profile) {
            ctx.game_menu_tab = GameMenuTab::Profile;
            return Ok(SceneCommand::Push(SceneId::GameMenu));
        }

        if input.just_pressed(Button::Interact) {
            if self.try_collect_pickup(ctx) {
                return Ok(SceneCommand::None);
            }

            if let Some(exit) = self.overlapping_entity(MapEntityKind::FacilityExit) {
                let unlock = exit.unlock.clone();
                if !ctx.is_unlock_rule_satisfied(unlock.as_ref()) {
                    self.notice.push_locked_unlock_rule(
                        ctx.language,
                        unlock.as_ref(),
                        &ctx.codex_database,
                    );
                    return Ok(SceneCommand::None);
                }

                ctx.overworld_spawn_id = Some(OVERWORLD_RETURN_SPAWN.to_owned());
                ctx.overworld_player_position = None;
                ctx.request_save();
                return Ok(SceneCommand::Switch(SceneId::Overworld));
            }
        }

        let scan_target = nearby_scan_target(&self.world, self.player.rect());
        let scan_update = self
            .scan
            .update(ctx, dt, input.is_down(Button::Scan), scan_target);
        if let Some(codex_id) = scan_update.completed_codex_id {
            self.notice
                .push_scan_complete(ctx.language, &codex_id, &ctx.codex_database);
        }

        self.update_sideview_player(dt, input, self.scan.should_capture_scan_button());
        self.camera.position = self.player.position;
        ctx.record_world_location(SceneId::Facility, &self.map_path, self.player.position);

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.world.draw(ctx.renderer);
        self.player.draw(ctx.renderer);
        self.scan.draw(ctx.renderer)?;
        self.notice.draw(ctx.renderer)?;
        Ok(())
    }

    fn camera(&self) -> Camera2d {
        self.camera
    }
}
