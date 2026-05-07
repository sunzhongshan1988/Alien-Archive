mod map;

use anyhow::Result;
use runtime::{Rect, Renderer, Vec2};

pub use map::{MapEntity, MapEntityKind};

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

    pub fn load_assets(&self, renderer: &mut dyn Renderer) -> Result<()> {
        self.map.load_assets(renderer)
    }

    pub fn first_entity(&self, kind: MapEntityKind) -> Option<&MapEntity> {
        self.map
            .entities()
            .iter()
            .find(|entity| entity.kind == kind)
    }

    pub fn entities(&self, kind: MapEntityKind) -> impl Iterator<Item = &MapEntity> + '_ {
        self.map
            .entities()
            .iter()
            .filter(move |entity| entity.kind == kind)
    }

    #[allow(dead_code)]
    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.map.solid_rects()
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
        ),
    ],
)"#,
        )
        .expect("legacy overworld fixture should be written");

        path
    }
}
