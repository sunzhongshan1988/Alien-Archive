use std::{fs, path::Path};

use anyhow::{Context, Result};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MAP_ID: &str = "overworld_landing_site";
pub const DEFAULT_MAP_PATH: &str = "assets/data/maps/overworld_landing_site.ron";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

    pub fn zh_label(self) -> &'static str {
        match self {
            Self::Ground => "地表",
            Self::Decals => "贴花",
            Self::Objects => "物件",
            Self::Entities => "实体",
            Self::Zones => "区域",
            Self::Collision => "碰撞",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

    pub fn place_decal(&mut self, asset: &str, x: f32, y: f32) -> String {
        let id = next_object_id("decal", &self.layers.decals);
        self.layers.decals.push(ObjectInstance {
            id: id.clone(),
            asset: asset.to_owned(),
            x,
            y,
            scale_x: 1.0,
            scale_y: 1.0,
            z_index: 0,
            collision_rect: None,
            depth_rect: None,
            flip_x: false,
            rotation: 0,
        });
        id
    }

    pub fn place_object(&mut self, asset: &str, x: f32, y: f32) -> String {
        let id = next_object_id("obj", &self.layers.objects);
        self.layers.objects.push(ObjectInstance {
            id: id.clone(),
            asset: asset.to_owned(),
            x,
            y,
            scale_x: 1.0,
            scale_y: 1.0,
            z_index: 0,
            collision_rect: None,
            depth_rect: None,
            flip_x: false,
            rotation: 0,
        });
        id
    }

    pub fn place_entity(&mut self, asset: &str, entity_type: &str, x: f32, y: f32) -> String {
        let id = next_entity_id("ent", &self.layers.entities);
        self.layers.entities.push(EntityInstance {
            id: id.clone(),
            asset: asset.to_owned(),
            entity_type: entity_type.to_owned(),
            x,
            y,
            scale_x: 1.0,
            scale_y: 1.0,
            z_index: 0,
            collision_rect: None,
            depth_rect: None,
            interaction_rect: None,
            unlock: None,
            transition: None,
            flip_x: false,
            rotation: 0,
        });
        id
    }

    pub fn place_collision(&mut self, x: i32, y: i32) {
        if let Some(cell) = self
            .layers
            .collision
            .iter_mut()
            .find(|cell| cell.is_full_cell_at(x, y))
        {
            cell.solid = true;
            return;
        }

        self.layers.collision.push(CollisionCell::solid_cell(x, y));
    }

    pub fn place_collision_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let rect = CollisionCell::solid_rect(x, y, w, h);
        if let Some(cell) = self.layers.collision.iter_mut().find(|cell| {
            cell.solid
                && floats_close(
                    cell.x as f32 + cell.offset[0],
                    rect.x as f32 + rect.offset[0],
                )
                && floats_close(
                    cell.y as f32 + cell.offset[1],
                    rect.y as f32 + rect.offset[1],
                )
                && floats_close(cell.size[0], rect.size[0])
                && floats_close(cell.size[1], rect.size[1])
        }) {
            *cell = rect;
            return;
        }

        self.layers.collision.push(rect);
    }

    pub fn erase_collision_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let target = CollisionBounds::new(x, y, w, h);
        self.layers
            .collision
            .retain(|cell| !cell.solid || !cell.bounds().intersects(target));
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
            LayerKind::Zones => self
                .layers
                .zones
                .retain(|zone| !zone_contains_cell(zone, x, y)),
            LayerKind::Collision => self.layers.collision.retain(|cell| {
                !cell
                    .bounds()
                    .intersects(CollisionBounds::new(x as f32, y as f32, 1.0, 1.0))
            }),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MapLayers {
    pub ground: Vec<TileInstance>,
    pub decals: Vec<ObjectInstance>,
    pub objects: Vec<ObjectInstance>,
    pub entities: Vec<EntityInstance>,
    pub zones: Vec<ZoneInstance>,
    pub collision: Vec<CollisionCell>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectInstance {
    pub id: String,
    pub asset: String,
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_instance_scale", skip_serializing_if = "is_one_f32")]
    pub scale_x: f32,
    #[serde(default = "default_instance_scale", skip_serializing_if = "is_one_f32")]
    pub scale_y: f32,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub z_index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collision_rect: Option<InstanceRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_rect: Option<InstanceRect>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub rotation: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EntityInstance {
    pub id: String,
    pub asset: String,
    pub entity_type: String,
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_instance_scale", skip_serializing_if = "is_one_f32")]
    pub scale_x: f32,
    #[serde(default = "default_instance_scale", skip_serializing_if = "is_one_f32")]
    pub scale_y: f32,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub z_index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collision_rect: Option<InstanceRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_rect: Option<InstanceRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_rect: Option<InstanceRect>,
    #[serde(default, skip_serializing_if = "option_unlock_rule_is_none_or_empty")]
    pub unlock: Option<UnlockRule>,
    #[serde(
        default,
        skip_serializing_if = "option_transition_target_is_none_or_empty"
    )]
    pub transition: Option<TransitionTarget>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_x: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub rotation: i32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UnlockRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_codex_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_message: Option<String>,
}

impl UnlockRule {
    pub fn is_empty(&self) -> bool {
        self.requires_codex_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            && self
                .requires_item_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            && self
                .locked_message
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TransitionTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_id: Option<String>,
}

impl TransitionTarget {
    pub fn is_empty(&self) -> bool {
        self.scene
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            && self
                .map_path
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            && self
                .spawn_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct InstanceRect {
    pub offset: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ZoneInstance {
    pub id: String,
    pub zone_type: String,
    pub points: Vec<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<WalkSurfaceRule>,
    #[serde(default, skip_serializing_if = "option_unlock_rule_is_none_or_empty")]
    pub unlock: Option<UnlockRule>,
    #[serde(
        default,
        skip_serializing_if = "option_transition_target_is_none_or_empty"
    )]
    pub transition: Option<TransitionTarget>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WalkSurfaceRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_default_walk_surface_kind")]
    pub kind: WalkSurfaceKind,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub constrain_movement: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub z_index: i32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub depth_offset: f32,
}

impl Default for WalkSurfaceRule {
    fn default() -> Self {
        Self {
            surface_id: None,
            kind: WalkSurfaceKind::Platform,
            constrain_movement: true,
            z_index: 64,
            depth_offset: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WalkSurfaceKind {
    Platform,
    Ramp,
}

impl Default for WalkSurfaceKind {
    fn default() -> Self {
        Self::Platform
    }
}

impl WalkSurfaceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Platform => "Platform",
            Self::Ramp => "Ramp",
        }
    }

    pub fn zh_label(self) -> &'static str {
        match self {
            Self::Platform => "台面",
            Self::Ramp => "斜坡入口",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CollisionCell {
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "is_zero_vec2")]
    pub offset: [f32; 2],
    #[serde(
        default = "default_collision_size",
        skip_serializing_if = "is_unit_vec2"
    )]
    pub size: [f32; 2],
    pub solid: bool,
}

impl CollisionCell {
    pub fn solid_cell(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            offset: [0.0, 0.0],
            size: [1.0, 1.0],
            solid: true,
        }
    }

    pub fn solid_rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        let min_x = x.min(x + w);
        let min_y = y.min(y + h);
        let width = w.abs().max(0.05);
        let height = h.abs().max(0.05);
        let cell_x = min_x.floor() as i32;
        let cell_y = min_y.floor() as i32;
        Self {
            x: cell_x,
            y: cell_y,
            offset: [min_x - cell_x as f32, min_y - cell_y as f32],
            size: [width, height],
            solid: true,
        }
    }

    pub fn bounds(&self) -> CollisionBounds {
        CollisionBounds::new(
            self.x as f32 + self.offset[0],
            self.y as f32 + self.offset[1],
            self.size[0],
            self.size[1],
        )
    }

    fn is_full_cell_at(&self, x: i32, y: i32) -> bool {
        self.x == x
            && self.y == y
            && floats_close(self.offset[0], 0.0)
            && floats_close(self.offset[1], 0.0)
            && floats_close(self.size[0], 1.0)
            && floats_close(self.size[1], 1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionBounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl CollisionBounds {
    fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w: w.max(0.0),
            h: h.max(0.0),
        }
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn contains_point(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    pub fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

fn zone_contains_cell(zone: &ZoneInstance, x: i32, y: i32) -> bool {
    if zone.points.is_empty() {
        return false;
    }
    let min_x = zone
        .points
        .iter()
        .map(|point| point[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = zone
        .points
        .iter()
        .map(|point| point[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = zone
        .points
        .iter()
        .map(|point| point[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = zone
        .points
        .iter()
        .map(|point| point[1])
        .fold(f32::NEG_INFINITY, f32::max);

    let x = x as f32 + 0.5;
    let y = y as f32 + 0.5;
    x >= min_x && x <= max_x && y >= min_y && y <= max_y
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_true() -> bool {
    true
}

fn default_collision_size() -> [f32; 2] {
    [1.0, 1.0]
}

fn is_zero_vec2(value: &[f32; 2]) -> bool {
    floats_close(value[0], 0.0) && floats_close(value[1], 0.0)
}

fn is_unit_vec2(value: &[f32; 2]) -> bool {
    floats_close(value[0], 1.0) && floats_close(value[1], 1.0)
}

fn floats_close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.0001
}

fn option_unlock_rule_is_none_or_empty(value: &Option<UnlockRule>) -> bool {
    value.as_ref().is_none_or(UnlockRule::is_empty)
}

fn option_transition_target_is_none_or_empty(value: &Option<TransitionTarget>) -> bool {
    value.as_ref().is_none_or(TransitionTarget::is_empty)
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

fn is_zero_f32(value: &f32) -> bool {
    floats_close(*value, 0.0)
}

fn is_default_walk_surface_kind(value: &WalkSurfaceKind) -> bool {
    *value == WalkSurfaceKind::Platform
}

fn is_one_i32(value: &i32) -> bool {
    *value == 1
}

fn is_one_f32(value: &f32) -> bool {
    (*value - 1.0).abs() < f32::EPSILON
}

fn default_tile_extent() -> i32 {
    1
}

fn default_instance_scale() -> f32 {
    1.0
}
