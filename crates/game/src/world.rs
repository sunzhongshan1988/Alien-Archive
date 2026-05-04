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

    #[allow(dead_code)]
    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.map.solid_rects()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_overworld_map() {
        let world = World::load(
            "assets/data/maps/overworld_landing_site.ron",
            Some("player_start"),
        )
        .expect("overworld map should load");

        assert!(
            world
                .first_entity(MapEntityKind::FacilityEntrance)
                .is_some()
        );
    }

    #[test]
    fn still_loads_legacy_overworld_map() {
        let world = World::load("assets/data/maps/overworld.ron", Some("player_start"))
            .expect("legacy overworld map should load");

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
}
