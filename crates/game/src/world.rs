mod map;

use anyhow::Result;
use runtime::{Rect, Renderer, Vec2};

use map::{Map, MapEntityKind};

pub struct World {
    map: Map,
    player_spawn: Vec2,
}

impl World {
    pub fn load_demo() -> Result<Self> {
        let map = Map::load("assets/data/maps/demo.ron")?;
        let player_spawn = map
            .entities()
            .iter()
            .find(|entity| entity.kind == MapEntityKind::PlayerSpawn)
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
    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.map.solid_rects()
    }
}
