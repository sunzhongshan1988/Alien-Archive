use std::{fs, path::Path};

use anyhow::{Context, Result};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MAP_ID: &str = "overworld_landing_site";
pub const DEFAULT_MAP_PATH: &str = "assets/data/maps/overworld_landing_site.ron";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LayerKind {
    Ground,
    Decals,
    Objects,
    Entities,
    Zones,
    Collision,
}

impl LayerKind {
    pub const ALL: [Self; 6] = [
        Self::Ground,
        Self::Decals,
        Self::Objects,
        Self::Entities,
        Self::Zones,
        Self::Collision,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ground => "Ground",
            Self::Decals => "Decals",
            Self::Objects => "Objects",
            Self::Entities => "Entities",
            Self::Zones => "Zones",
            Self::Collision => "Collision",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapDocument {
    pub id: String,
    pub mode: String,
    pub tile_size: u32,
    pub width: u32,
    pub height: u32,
    pub layers: MapLayers,
    pub spawns: Vec<SpawnPoint>,
}

impl MapDocument {
    pub fn new_landing_site() -> Self {
        Self {
            id: DEFAULT_MAP_ID.to_owned(),
            mode: "Overworld".to_owned(),
            tile_size: 32,
            width: 80,
            height: 60,
            layers: MapLayers::default(),
            spawns: vec![SpawnPoint {
                id: "player_start".to_owned(),
                x: 8.0,
                y: 12.0,
            }],
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read map {}", path.display()))?;
        let document = ron::from_str(&source)
            .with_context(|| format!("failed to parse RON map {}", path.display()))?;

        Ok(document)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let pretty = PrettyConfig::new()
            .depth_limit(4)
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        let source = ron::ser::to_string_pretty(self, pretty).context("failed to serialize map")?;
        fs::write(path, source).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn place_tile(&mut self, asset: &str, x: i32, y: i32) {
        self.place_tile_sized(asset, x, y, 1, 1);
    }

    pub fn place_tile_sized(&mut self, asset: &str, x: i32, y: i32, w: i32, h: i32) {
        let max_width = (self.width as i32 - x).max(1);
        let max_height = (self.height as i32 - y).max(1);
        let w = w.clamp(1, max_width);
        let h = h.clamp(1, max_height);

        if let Some(tile) = self
            .layers
            .ground
            .iter_mut()
            .find(|tile| tile.x == x && tile.y == y)
        {
            tile.asset = asset.to_owned();
            tile.w = w;
            tile.h = h;
            return;
        }

        self.layers.ground.push(TileInstance {
            asset: asset.to_owned(),
            x,
            y,
            w,
            h,
            flip_x: false,
            rotation: 0,
        });
    }

    pub fn place_decal(&mut self, asset: &str, x: f32, y: f32) {
        let id = next_object_id("decal", &self.layers.decals);
        self.layers.decals.push(ObjectInstance {
            id,
            asset: asset.to_owned(),
            x,
            y,
            flip_x: false,
            rotation: 0,
        });
    }

    pub fn place_object(&mut self, asset: &str, x: f32, y: f32) {
        let id = next_object_id("obj", &self.layers.objects);
        self.layers.objects.push(ObjectInstance {
            id,
            asset: asset.to_owned(),
            x,
            y,
            flip_x: false,
            rotation: 0,
        });
    }

    pub fn place_entity(&mut self, asset: &str, x: f32, y: f32) {
        let id = next_entity_id("ent", &self.layers.entities);
        self.layers.entities.push(EntityInstance {
            id,
            asset: asset.to_owned(),
            entity_type: infer_entity_type(asset).to_owned(),
            x,
            y,
            flip_x: false,
            rotation: 0,
        });
    }

    pub fn place_collision(&mut self, x: i32, y: i32) {
        if let Some(cell) = self
            .layers
            .collision
            .iter_mut()
            .find(|cell| cell.x == x && cell.y == y)
        {
            cell.solid = true;
            return;
        }

        self.layers
            .collision
            .push(CollisionCell { x, y, solid: true });
    }

    pub fn erase_at(&mut self, layer: LayerKind, x: i32, y: i32) {
        match layer {
            LayerKind::Ground => self
                .layers
                .ground
                .retain(|tile| !tile_contains_cell(tile, x, y)),
            LayerKind::Decals => retain_outside_cell(&mut self.layers.decals, x, y),
            LayerKind::Objects => retain_outside_cell(&mut self.layers.objects, x, y),
            LayerKind::Entities => retain_entities_outside_cell(&mut self.layers.entities, x, y),
            LayerKind::Zones => {}
            LayerKind::Collision => self
                .layers
                .collision
                .retain(|cell| cell.x != x || cell.y != y),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MapLayers {
    pub ground: Vec<TileInstance>,
    pub decals: Vec<ObjectInstance>,
    pub objects: Vec<ObjectInstance>,
    pub entities: Vec<EntityInstance>,
    pub zones: Vec<ZoneInstance>,
    pub collision: Vec<CollisionCell>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TileInstance {
    pub asset: String,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_tile_extent", skip_serializing_if = "is_one_i32")]
    pub w: i32,
    #[serde(default = "default_tile_extent", skip_serializing_if = "is_one_i32")]
    pub h: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub rotation: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectInstance {
    pub id: String,
    pub asset: String,
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub rotation: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityInstance {
    pub id: String,
    pub asset: String,
    pub entity_type: String,
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub rotation: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ZoneInstance {
    pub id: String,
    pub zone_type: String,
    pub points: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CollisionCell {
    pub x: i32,
    pub y: i32,
    pub solid: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpawnPoint {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

fn next_id(prefix: &str, index: usize) -> String {
    format!("{prefix}_{:03}", index + 1)
}

fn next_object_id(prefix: &str, instances: &[ObjectInstance]) -> String {
    for index in 0.. {
        let candidate = next_id(prefix, index);
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded id scan should always find a candidate")
}

fn next_entity_id(prefix: &str, instances: &[EntityInstance]) -> String {
    for index in 0.. {
        let candidate = next_id(prefix, index);
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded id scan should always find a candidate")
}

fn retain_outside_cell(instances: &mut Vec<ObjectInstance>, x: i32, y: i32) {
    instances.retain(|instance| instance.x.floor() as i32 != x || instance.y.floor() as i32 != y);
}

fn retain_entities_outside_cell(instances: &mut Vec<EntityInstance>, x: i32, y: i32) {
    instances.retain(|instance| instance.x.floor() as i32 != x || instance.y.floor() as i32 != y);
}

fn tile_contains_cell(tile: &TileInstance, x: i32, y: i32) -> bool {
    let width = tile.w.max(1);
    let height = tile.h.max(1);
    x >= tile.x && x < tile.x + width && y >= tile.y && y < tile.y + height
}

fn infer_entity_type(asset: &str) -> &'static str {
    if asset.contains("gate") || asset.contains("door") || asset.contains("entrance") {
        "FacilityEntrance"
    } else if asset.contains("terminal") || asset.contains("signal") || asset.contains("scan") {
        "ScanTarget"
    } else {
        "Decoration"
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

fn is_one_i32(value: &i32) -> bool {
    *value == 1
}

fn default_tile_extent() -> i32 {
    1
}
