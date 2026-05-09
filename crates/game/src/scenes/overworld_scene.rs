use anyhow::Result;
use runtime::{Button, Camera2d, InputState, Renderer, SceneCommand, collision::rects_overlap};

use crate::{
    player::Player,
    world::{MapEntity, MapEntityKind, MapZone, World},
};

use super::{
    FieldActivity, FieldEnvironment, GameContext, GameMenuTab, RenderContext, Scene,
    SceneDebugSnapshot, SceneId,
    notice_system::NoticeState,
    rewards,
    scan_system::{ScanState, nearby_scan_target},
};

const OVERWORLD_MAP: &str = "assets/data/maps/overworld_landing_site.ron";
const FACILITY_MAP: &str = "assets/data/maps/facility_ruin_01.ron";
const FACILITY_ENTRY_SPAWN: &str = "entry";
const OVERWORLD_CAMERA_ZOOM: f32 = 1.5;

pub struct OverworldScene {
    player: Player,
    world: World,
    map_path: String,
    camera: Camera2d,
    scan: ScanState,
    notice: NoticeState,
}

impl OverworldScene {
    pub fn new(ctx: &GameContext) -> Result<Self> {
        let map_path = ctx
            .overworld_map_path
            .as_deref()
            .unwrap_or(OVERWORLD_MAP)
            .to_owned();
        let mut world = World::load(&map_path, ctx.overworld_spawn_id.as_deref())?;
        world.remove_entities_by_id(&ctx.collected_entity_ids_for_map(&map_path));
        let player = Player::new(
            ctx.overworld_player_position
                .unwrap_or_else(|| world.player_spawn()),
        );

        Ok(Self {
            camera: Camera2d::follow_with_zoom(player.position, OVERWORLD_CAMERA_ZOOM),
            player,
            world,
            map_path,
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

    fn overlapping_transition_zone(&self) -> Option<&MapZone> {
        self.world
            .zones("MapTransition")
            .find(|zone| rects_overlap(self.player.rect(), zone.bounds))
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
            ctx.log_inventory_full();
            return true;
        }

        ctx.collect_entity(&self.map_path, &entity.id);
        self.world
            .remove_entities_by_id(&ctx.collected_entity_ids_for_map(&self.map_path));
        self.notice.push_pickup(ctx.language, reward.item_id, added);
        true
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

        if input.just_pressed(Button::UseQuickItem) {
            let result = ctx.use_selected_quickbar_item();
            self.notice
                .push_quick_item_use_result(ctx.language, &result);
            return Ok(SceneCommand::None);
        }

        if input.just_pressed(Button::Interact) {
            if self.try_collect_pickup(ctx) {
                return Ok(SceneCommand::None);
            }

            if let Some(entrance) = self.overlapping_entity(MapEntityKind::FacilityEntrance) {
                let unlock = entrance.unlock.clone();
                if !ctx.is_unlock_rule_satisfied(unlock.as_ref()) {
                    self.notice.push_locked_unlock_rule(
                        ctx.language,
                        unlock.as_ref(),
                        &ctx.codex_database,
                    );
                    ctx.log_locked_unlock_rule(unlock.as_ref());
                    return Ok(SceneCommand::None);
                }

                let scene_id = ctx.apply_map_transition(
                    entrance.transition.as_ref(),
                    SceneId::Facility,
                    FACILITY_MAP,
                    FACILITY_ENTRY_SPAWN,
                );
                return Ok(SceneCommand::Switch(scene_id));
            }
        }

        if let Some(zone) = self.overlapping_transition_zone() {
            if ctx.is_unlock_rule_satisfied(zone.unlock.as_ref()) {
                let scene_id = ctx.apply_map_transition(
                    zone.transition.as_ref(),
                    SceneId::Facility,
                    FACILITY_MAP,
                    FACILITY_ENTRY_SPAWN,
                );
                return Ok(SceneCommand::Switch(scene_id));
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

        let status = ctx.update_field_status(
            dt,
            FieldActivity {
                moving: input.movement().length_squared() > f32::EPSILON,
                scanning: input.is_down(Button::Scan),
                jumped: false,
                environment: FieldEnvironment::Overworld,
            },
        );
        self.player.update_topdown_with_speed(
            dt,
            input,
            self.world.solid_rects(),
            status.movement_speed_multiplier,
        );
        self.camera.position = self.player.position;
        ctx.record_world_location(SceneId::Overworld, &self.map_path, self.player.position);

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.world.load_visible_ground_assets(ctx.renderer)?;
        let surface = self
            .world
            .walk_surface_at(self.player.topdown_feet_position());
        let actor_depth_y =
            self.player.topdown_depth_y() + surface.map_or(0.0, |surface| surface.depth_offset);
        let actor_z_index = surface.map_or(0, |surface| surface.z_index);
        self.world.draw_with_actor_at_depth(
            ctx.renderer,
            actor_depth_y,
            actor_z_index,
            |renderer| {
                self.player.draw_topdown(renderer);
            },
        );
        self.scan.draw(ctx.renderer)?;
        self.notice.draw(ctx.renderer)?;
        Ok(())
    }

    fn camera(&self) -> Camera2d {
        self.camera
    }

    fn debug_snapshot(&self, _ctx: &GameContext) -> SceneDebugSnapshot {
        SceneDebugSnapshot::new(self.id(), self.name()).with_field_state(
            &self.map_path,
            self.player.position,
            self.world.solid_rects().count(),
            nearby_scan_target(&self.world, self.player.rect()).map(debug_scan_target_label),
        )
    }
}

fn debug_scan_target_label(entity: &MapEntity) -> String {
    entity
        .codex_id
        .as_deref()
        .map(|codex_id| format!("{codex_id} [{}]", entity.id))
        .unwrap_or_else(|| entity.id.clone())
}
