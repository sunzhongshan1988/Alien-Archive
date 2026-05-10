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
    zone_system::ZoneRuntimeState,
};

const OVERWORLD_MAP: &str = "assets/data/maps/overworld_landing_site.ron";
const FACILITY_MAP: &str = "assets/data/maps/facility_ruin_01.ron";
const FACILITY_ENTRY_SPAWN: &str = "entry";
const OVERWORLD_CAMERA_ZOOM: f32 = 1.5;

pub struct OverworldScene {
    player: Player,
    world: World,
    map_path: String,
    active_walk_surface_id: Option<String>,
    camera: Camera2d,
    scan: ScanState,
    notice: NoticeState,
    zones: ZoneRuntimeState,
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
        let active_walk_surface_id = world
            .walk_surface_entry_at(player.topdown_feet_position())
            .map(|surface| surface.surface_id);

        Ok(Self {
            camera: Camera2d::follow_with_zoom(player.position, OVERWORLD_CAMERA_ZOOM),
            player,
            world,
            map_path,
            active_walk_surface_id,
            scan: ScanState::default(),
            notice: NoticeState::default(),
            zones: ZoneRuntimeState::default(),
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

    fn refresh_active_walk_surface(&mut self, ground_entry_from: Option<runtime::Vec2>) {
        let feet = self.player.topdown_feet_position();

        if let Some(surface_id) = self.active_walk_surface_id.clone() {
            if ground_entry_from
                .is_some_and(|previous| self.world.walk_surface_exits(&surface_id, previous, feet))
            {
                self.active_walk_surface_id = None;
            } else if self.world.walk_surface_contains(&surface_id, feet) {
                return;
            } else {
                self.active_walk_surface_id = None;
            }
        }

        let Some(previous) = ground_entry_from else {
            return;
        };

        if let Some(surface) = self.world.walk_surface_ground_entry(previous, feet) {
            self.active_walk_surface_id = Some(surface.surface_id);
        }
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
        self.zones.update(
            ctx,
            &mut self.notice,
            &self.world,
            &self.map_path,
            self.player.rect(),
            dt,
        );

        let status = ctx.update_field_status(
            dt,
            FieldActivity {
                moving: input.movement().length_squared() > f32::EPSILON,
                scanning: input.is_down(Button::Scan),
                jumped: false,
                environment: FieldEnvironment::Overworld,
            },
        );
        self.refresh_active_walk_surface(None);
        let previous_feet = self.player.topdown_feet_position();
        let active_surface_id = self.active_walk_surface_id.clone();
        let world = &self.world;
        let active_surface_constrained = active_surface_id
            .as_deref()
            .and_then(|surface_id| world.walk_surface_for_id_at(surface_id, previous_feet))
            .is_none_or(|surface| surface.constrain_movement);
        let mut solid_rects = self
            .world
            .solid_rects_without_zone_collision()
            .collect::<Vec<_>>();
        if let Some(surface_id) = active_surface_id.as_deref() {
            solid_rects.extend(self.world.surface_collision_rects(surface_id));
        }
        let zone_collision_rects = self.world.zone_collision_rects().collect::<Vec<_>>();

        self.player
            .update_topdown_with_speed_and_conditional_collision(
                dt,
                input,
                solid_rects,
                zone_collision_rects,
                status.movement_speed_multiplier,
                |feet| {
                    let Some(surface_id) = active_surface_id.as_deref() else {
                        return world.walk_surface_allows_ground_movement(previous_feet, feet);
                    };
                    if !active_surface_constrained {
                        return true;
                    }
                    world.walk_surface_allows_movement(surface_id, previous_feet, feet)
                },
                |previous, feet| {
                    let Some(surface_id) = active_surface_id.as_deref() else {
                        return !world.walk_surface_allows_ground_entry(previous, feet);
                    };
                    if !active_surface_constrained {
                        return false;
                    }
                    !world.walk_surface_allows_movement(surface_id, previous, feet)
                },
            );
        self.refresh_active_walk_surface(Some(previous_feet));
        self.camera.position = self.player.position;
        ctx.record_world_location(SceneId::Overworld, &self.map_path, self.player.position);

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.world.load_visible_ground_assets(ctx.renderer)?;
        let feet = self.player.topdown_feet_position();
        let surface = self
            .active_walk_surface_id
            .as_deref()
            .and_then(|surface_id| self.world.walk_surface_for_id_at(surface_id, feet));
        let actor_depth_y = self.player.topdown_depth_y()
            + surface.as_ref().map_or(0.0, |surface| surface.depth_offset);
        let actor_z_index = surface.as_ref().map_or(0, |surface| surface.z_index);
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

    fn render_debug_geometry(&self, renderer: &mut dyn Renderer) {
        self.world.draw_debug_geometry(
            renderer,
            nearby_scan_target(&self.world, self.player.rect()).map(|entity| entity.id.as_str()),
            Some(self.player.topdown_collision_rect()),
        );
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
