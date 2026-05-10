mod map;

use anyhow::Result;
use runtime::{Rect, Renderer, Vec2};

pub use map::{
    MapEntity, MapEntityKind, MapHazardRule, MapObjectiveRule, MapPromptRule, MapTransitionTarget,
    MapUnlockRule, MapWalkSurface, MapZone,
};

use map::Map;

pub struct World {
    map: Map,
    player_spawn: Vec2,
}

impl World {
    pub fn load(path: &str, spawn_id: Option<&str>) -> Result<Self> {
        let map = Map::load(path)?;
        let player_spawn = spawn_id
            .and_then(|id| map.entity_by_id(id))
            .or_else(|| {
                map.entities()
                    .iter()
                    .find(|entity| entity.kind == MapEntityKind::PlayerSpawn)
            })
            .map(|entity| entity.rect.origin)
            .unwrap_or(Vec2::ZERO);

        Ok(Self { map, player_spawn })
    }

    pub fn player_spawn(&self) -> Vec2 {
        self.player_spawn
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        self.map.draw(renderer);
    }

    #[allow(dead_code)]
    pub fn draw_with_actor(
        &self,
        renderer: &mut dyn Renderer,
        actor_depth_y: f32,
        draw_actor: impl FnOnce(&mut dyn Renderer),
    ) {
        self.map
            .draw_with_actor(renderer, actor_depth_y, draw_actor);
    }

    pub fn draw_with_actor_at_depth(
        &self,
        renderer: &mut dyn Renderer,
        actor_depth_y: f32,
        actor_z_index: i32,
        draw_actor: impl FnOnce(&mut dyn Renderer),
    ) {
        self.map
            .draw_with_actor_at_depth(renderer, actor_depth_y, actor_z_index, draw_actor);
    }

    pub fn draw_debug_geometry(
        &self,
        renderer: &mut dyn Renderer,
        active_scan_target_id: Option<&str>,
        player_rect: Option<Rect>,
    ) {
        self.map
            .draw_debug_geometry(renderer, active_scan_target_id, player_rect);
    }

    pub fn load_assets(&self, renderer: &mut dyn Renderer) -> Result<()> {
        self.map.load_assets(renderer)
    }

    pub fn load_visible_ground_assets(&self, renderer: &mut dyn Renderer) -> Result<()> {
        self.map.load_visible_ground_assets(renderer)
    }

    #[allow(dead_code)]
    pub fn first_entity(&self, kind: MapEntityKind) -> Option<&MapEntity> {
        self.map
            .entities()
            .iter()
            .find(|entity| entity.kind == kind)
    }

    pub fn all_entities(&self) -> impl Iterator<Item = &MapEntity> + '_ {
        self.map.entities().iter()
    }

    pub fn entities(&self, kind: MapEntityKind) -> impl Iterator<Item = &MapEntity> + '_ {
        self.map
            .entities()
            .iter()
            .filter(move |entity| entity.kind == kind)
    }

    pub fn zones<'a>(&'a self, zone_type: &'a str) -> impl Iterator<Item = &'a MapZone> + 'a {
        self.map
            .zones()
            .iter()
            .filter(move |zone| zone.zone_type == zone_type)
    }

    pub fn overlapping_zones(&self, rect: Rect) -> impl Iterator<Item = &MapZone> + '_ {
        self.map
            .zones()
            .iter()
            .filter(move |zone| runtime::collision::rects_overlap(rect, zone.bounds))
    }

    pub fn walk_surface_at(&self, point: Vec2) -> Option<MapWalkSurface> {
        self.map.walk_surface_at(point)
    }

    pub fn walk_surface_entry_at(&self, point: Vec2) -> Option<MapWalkSurface> {
        self.map.walk_surface_entry_at(point)
    }

    pub fn walk_surface_for_id_at(&self, surface_id: &str, point: Vec2) -> Option<MapWalkSurface> {
        self.map.walk_surface_for_id_at(surface_id, point)
    }

    pub fn walk_surface_contains(&self, surface_id: &str, point: Vec2) -> bool {
        self.map.walk_surface_contains(surface_id, point)
    }

    pub fn walk_surface_allows_movement(
        &self,
        surface_id: &str,
        previous: Vec2,
        next: Vec2,
    ) -> bool {
        self.map
            .walk_surface_allows_movement(surface_id, previous, next)
    }

    pub fn codex_entities(&self) -> impl Iterator<Item = &MapEntity> + '_ {
        self.map
            .entities()
            .iter()
            .filter(|entity| entity.codex_id.is_some())
    }

    #[allow(dead_code)]
    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.map.solid_rects()
    }

    pub fn remove_entities_by_id(&mut self, ids: &std::collections::BTreeSet<String>) {
        self.map.remove_entities_by_id(ids);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn loads_overworld_map() {
        let world = World::load(
            "assets/data/maps/overworld_landing_site.ron",
            Some("player_start"),
        )
        .expect("overworld map should load");

        assert!(world.first_entity(MapEntityKind::PlayerSpawn).is_some());
    }

    #[test]
    fn still_loads_legacy_overworld_map() {
        let path = write_legacy_overworld_fixture();
        let world = World::load(
            path.to_str().expect("fixture path should be utf-8"),
            Some("player_start"),
        )
        .expect("legacy overworld map should load");
        let _ = fs::remove_file(path);

        assert!(
            world
                .first_entity(MapEntityKind::FacilityEntrance)
                .is_some()
        );
        let entrance = world
            .first_entity(MapEntityKind::FacilityEntrance)
            .expect("legacy entrance should load");
        assert_eq!(
            entrance
                .unlock
                .as_ref()
                .and_then(|unlock| unlock.requires_codex_id.as_deref()),
            Some("ruin.entrance_01")
        );
        assert_eq!(
            entrance
                .transition
                .as_ref()
                .and_then(|transition| transition.spawn_id.as_deref()),
            Some("entry")
        );
    }

    #[test]
    fn loads_facility_map() {
        let world = World::load("assets/data/maps/facility_ruin_01.ron", Some("entry"))
            .expect("facility map should load");

        assert!(world.first_entity(MapEntityKind::FacilityExit).is_some());
    }

    fn write_legacy_overworld_fixture() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "alien_archive_legacy_overworld_{}.ron",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"(
    tile_size: 48.0,
    origin: (0.0, 0.0),
    palette: [
        (glyph: '.', color: (0.0, 0.0, 0.0, 0.0), solid: false, empty: true),
        (glyph: 'g', color: (0.070, 0.140, 0.120, 1.0), solid: false, empty: false),
    ],
    tiles: [
        "gg",
        "gg",
    ],
    entities: [
        (
            id: "player_start",
            kind: PlayerSpawn,
            position: (0, 0),
            size: (1, 1),
            color: (0.0, 0.0, 0.0, 0.0),
            solid: false,
            codex_id: None,
        ),
        (
            id: "ruin_entrance_01",
            kind: FacilityEntrance,
            position: (1, 1),
            size: (1, 1),
            color: (0.660, 0.360, 1.000, 0.82),
            solid: false,
            codex_id: Some("ruin.entrance_01"),
            transition: Some((
                scene: Some("Facility"),
                map_path: Some("assets/data/maps/facility_ruin_01.ron"),
                spawn_id: Some("entry"),
            )),
        ),
    ],
)"#,
        )
        .expect("legacy overworld fixture should be written");

        path
    }
}
