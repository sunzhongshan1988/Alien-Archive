use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use runtime::{Color, Rect, Renderer, Vec2};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Map {
    tiles: Vec<MapTile>,
    sprites: Vec<MapSprite>,
    entities: Vec<MapEntity>,
    collision_rects: Vec<Rect>,
    textures: HashMap<String, PathBuf>,
}

impl Map {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = resolve_asset_path(path.as_ref());
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read map file {}", path.display()))?;
        let file = parse_map_file(&source)
            .with_context(|| format!("failed to parse map file {}", path.display()))?;

        Self::from_file(file).with_context(|| format!("invalid map {}", path.display()))
    }

    pub fn entities(&self) -> &[MapEntity] {
        &self.entities
    }

    pub fn entity_by_id(&self, id: &str) -> Option<&MapEntity> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        for tile in &self.tiles {
            match &tile.visual {
                MapVisual::Color(color) => renderer.draw_rect(tile.rect, *color),
                MapVisual::Texture(texture_id) => {
                    renderer.draw_image_transformed(
                        texture_id,
                        tile.rect,
                        Color::rgba(1.0, 1.0, 1.0, 1.0),
                        tile.flip_x,
                        tile.rotation,
                    );
                }
            }
        }

        for sprite in &self.sprites {
            renderer.draw_image_transformed(
                &sprite.texture_id,
                sprite.rect,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
                sprite.flip_x,
                sprite.rotation,
            );
        }

        for entity in &self.entities {
            if entity.kind != MapEntityKind::PlayerSpawn {
                if let Some(texture_id) = &entity.texture_id {
                    renderer.draw_image_transformed(
                        texture_id,
                        entity.sprite_rect,
                        Color::rgba(1.0, 1.0, 1.0, 1.0),
                        entity.flip_x,
                        entity.rotation,
                    );
                } else {
                    renderer.draw_rect(entity.rect, entity.color);
                }
            }
        }
    }

    pub fn load_assets(&self, renderer: &mut dyn Renderer) -> Result<()> {
        for (texture_id, path) in &self.textures {
            if renderer.texture_size(texture_id).is_none() {
                renderer.load_texture(texture_id, path)?;
            }
        }

        Ok(())
    }

    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.tiles
            .iter()
            .filter(|tile| tile.solid)
            .map(|tile| tile.rect)
            .chain(self.collision_rects.iter().copied())
            .chain(
                self.entities
                    .iter()
                    .filter(|entity| entity.solid)
                    .map(|entity| entity.rect),
            )
    }

    fn from_file(file: AnyMapFile) -> Result<Self> {
        match file {
            AnyMapFile::Editor(file) => Self::from_editor_file(file),
            AnyMapFile::Legacy(file) => Self::from_legacy_file(file),
        }
    }

    fn from_legacy_file(file: LegacyMapFile) -> Result<Self> {
        if file.tile_size <= 0.0 {
            bail!("tile_size must be greater than zero");
        }

        let palette = file
            .palette
            .into_iter()
            .map(|entry| (entry.glyph, entry))
            .collect::<HashMap<_, _>>();
        let origin = Vec2::new(file.origin[0], file.origin[1]);
        let mut tiles = Vec::new();

        for (row_index, row) in file.tiles.iter().enumerate() {
            for (column_index, glyph) in row.chars().enumerate() {
                let Some(entry) = palette.get(&glyph) else {
                    bail!("unknown tile glyph '{glyph}' at row {row_index}, column {column_index}");
                };

                if entry.empty {
                    continue;
                }

                let position =
                    legacy_grid_to_world(origin, file.tile_size, column_index, row_index);
                tiles.push(MapTile {
                    rect: Rect::new(position, Vec2::new(file.tile_size, file.tile_size)),
                    visual: MapVisual::Color(color_from(entry.color)),
                    solid: entry.solid,
                    flip_x: false,
                    rotation: 0,
                });
            }
        }

        let entities = file
            .entities
            .into_iter()
            .map(|entity| {
                let position = legacy_grid_to_world(
                    origin,
                    file.tile_size,
                    entity.position[0] as usize,
                    entity.position[1] as usize,
                );
                let size = Vec2::new(
                    file.tile_size * entity.size[0] as f32,
                    file.tile_size * entity.size[1] as f32,
                );

                MapEntity {
                    id: entity.id,
                    kind: entity.kind,
                    rect: Rect::new(position, size),
                    sprite_rect: Rect::new(position, size),
                    color: color_from(entity.color),
                    solid: entity.solid,
                    codex_id: entity.codex_id,
                    texture_id: None,
                    flip_x: false,
                    rotation: 0,
                }
            })
            .collect();

        Ok(Self {
            tiles,
            sprites: Vec::new(),
            entities,
            collision_rects: Vec::new(),
            textures: HashMap::new(),
        })
    }

    fn from_editor_file(file: EditorMapFile) -> Result<Self> {
        if file.tile_size == 0 {
            bail!("tile_size must be greater than zero");
        }

        if file.mode != "Overworld" {
            bail!("expected Overworld map mode, got {}", file.mode);
        }

        let registry = scan_overworld_assets()?;
        let tile_size = file.tile_size as f32;
        let origin = Vec2::new(
            -(file.width as f32 * tile_size) * 0.5,
            -(file.height as f32 * tile_size) * 0.5,
        );
        let mut textures = HashMap::new();

        let mut tiles = Vec::new();
        for tile in file.layers.ground {
            let asset = registry
                .get(&tile.asset)
                .with_context(|| format!("unknown tile asset {}", tile.asset))?;
            textures.insert(asset.id.clone(), asset.path.clone());
            let position = grid_to_world(origin, tile_size, tile.x, tile.y);
            let size = Vec2::new(
                tile_size * tile.w.max(1) as f32,
                tile_size * tile.h.max(1) as f32,
            );

            tiles.push(MapTile {
                rect: Rect::new(position, size),
                visual: MapVisual::Texture(asset.id.clone()),
                solid: false,
                flip_x: tile.flip_x,
                rotation: tile.rotation,
            });
        }

        let mut sprites = Vec::new();
        for decal in file.layers.decals {
            push_sprite(
                &registry,
                &mut textures,
                &mut sprites,
                origin,
                tile_size,
                decal,
            )?;
        }
        for object in file.layers.objects {
            push_sprite(
                &registry,
                &mut textures,
                &mut sprites,
                origin,
                tile_size,
                object,
            )?;
        }

        let mut entities = Vec::new();
        for spawn in file.spawns {
            let position = object_anchor_to_world(origin, tile_size, spawn.x, spawn.y);
            entities.push(MapEntity {
                id: spawn.id,
                kind: MapEntityKind::PlayerSpawn,
                rect: centered_rect(position, Vec2::new(tile_size, tile_size)),
                sprite_rect: centered_rect(position, Vec2::new(tile_size, tile_size)),
                color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                solid: false,
                codex_id: None,
                texture_id: None,
                flip_x: false,
                rotation: 0,
            });
        }

        for entity in file.layers.entities {
            let asset = registry
                .get(&entity.asset)
                .with_context(|| format!("unknown entity asset {}", entity.asset))?;
            textures.insert(asset.id.clone(), asset.path.clone());
            let anchor = object_anchor_to_world(origin, tile_size, entity.x, entity.y);
            let sprite_size = asset.default_size;
            let sprite_rect = bottom_centered_rect(anchor, sprite_size);
            let hit_rect = centered_rect(anchor, Vec2::new(tile_size, tile_size));

            entities.push(MapEntity {
                id: entity.id,
                kind: map_entity_kind(&entity.entity_type),
                rect: hit_rect,
                sprite_rect,
                color: Color::rgba(0.65, 0.35, 1.0, 0.75),
                solid: false,
                codex_id: scan_codex_id(&entity.asset),
                texture_id: Some(asset.id.clone()),
                flip_x: entity.flip_x,
                rotation: entity.rotation,
            });
        }

        let collision_rects = file
            .layers
            .collision
            .into_iter()
            .filter(|cell| cell.solid)
            .map(|cell| {
                Rect::new(
                    grid_to_world(origin, tile_size, cell.x, cell.y),
                    Vec2::new(tile_size, tile_size),
                )
            })
            .collect();

        Ok(Self {
            tiles,
            sprites,
            entities,
            collision_rects,
            textures,
        })
    }
}

#[derive(Clone, Debug)]
struct MapTile {
    rect: Rect,
    visual: MapVisual,
    solid: bool,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Debug)]
enum MapVisual {
    Color(Color),
    Texture(String),
}

#[derive(Clone, Debug)]
struct MapSprite {
    texture_id: String,
    rect: Rect,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MapEntity {
    pub id: String,
    pub kind: MapEntityKind,
    pub rect: Rect,
    sprite_rect: Rect,
    pub color: Color,
    pub solid: bool,
    pub codex_id: Option<String>,
    texture_id: Option<String>,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MapEntityKind {
    PlayerSpawn,
    FacilityEntrance,
    FacilityExit,
    ScanTarget,
    Door,
    Decoration,
}

#[derive(Debug)]
enum AnyMapFile {
    Editor(EditorMapFile),
    Legacy(LegacyMapFile),
}

#[derive(Debug, Deserialize)]
struct LegacyMapFile {
    tile_size: f32,
    origin: [f32; 2],
    palette: Vec<TilePaletteEntry>,
    tiles: Vec<String>,
    entities: Vec<MapEntityDef>,
}

#[derive(Debug, Deserialize)]
struct TilePaletteEntry {
    glyph: char,
    color: [f32; 4],
    solid: bool,
    empty: bool,
}

#[derive(Debug, Deserialize)]
struct MapEntityDef {
    id: String,
    kind: MapEntityKind,
    position: [i32; 2],
    size: [i32; 2],
    color: [f32; 4],
    solid: bool,
    codex_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EditorMapFile {
    #[allow(dead_code)]
    id: String,
    mode: String,
    tile_size: u32,
    width: u32,
    height: u32,
    layers: EditorMapLayers,
    spawns: Vec<EditorSpawnPoint>,
}

#[derive(Debug, Deserialize)]
struct EditorMapLayers {
    ground: Vec<EditorTileInstance>,
    decals: Vec<EditorObjectInstance>,
    objects: Vec<EditorObjectInstance>,
    entities: Vec<EditorEntityInstance>,
    #[allow(dead_code)]
    zones: Vec<EditorZoneInstance>,
    collision: Vec<EditorCollisionCell>,
}

#[derive(Debug, Deserialize)]
struct EditorTileInstance {
    asset: String,
    x: i32,
    y: i32,
    #[serde(default = "default_tile_extent")]
    w: i32,
    #[serde(default = "default_tile_extent")]
    h: i32,
    #[serde(default)]
    flip_x: bool,
    #[serde(default)]
    rotation: i32,
}

#[derive(Debug, Deserialize)]
struct EditorObjectInstance {
    #[allow(dead_code)]
    id: String,
    asset: String,
    x: f32,
    y: f32,
    #[serde(default)]
    flip_x: bool,
    #[serde(default)]
    rotation: i32,
}

#[derive(Debug, Deserialize)]
struct EditorEntityInstance {
    id: String,
    asset: String,
    entity_type: String,
    x: f32,
    y: f32,
    #[serde(default)]
    flip_x: bool,
    #[serde(default)]
    rotation: i32,
}

#[derive(Debug, Deserialize)]
struct EditorZoneInstance {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    zone_type: String,
    #[allow(dead_code)]
    points: Vec<[f32; 2]>,
}

#[derive(Debug, Deserialize)]
struct EditorCollisionCell {
    x: i32,
    y: i32,
    solid: bool,
}

#[derive(Debug, Deserialize)]
struct EditorSpawnPoint {
    id: String,
    x: f32,
    y: f32,
}

#[derive(Clone, Debug)]
struct OverworldAsset {
    id: String,
    path: PathBuf,
    default_size: Vec2,
}

fn grid_to_world(origin: Vec2, tile_size: f32, column: i32, row: i32) -> Vec2 {
    Vec2::new(
        origin.x + column as f32 * tile_size,
        origin.y + row as f32 * tile_size,
    )
}

fn legacy_grid_to_world(origin: Vec2, tile_size: f32, column: usize, row: usize) -> Vec2 {
    grid_to_world(origin, tile_size, column as i32, row as i32)
}

fn push_sprite(
    registry: &HashMap<String, OverworldAsset>,
    textures: &mut HashMap<String, PathBuf>,
    sprites: &mut Vec<MapSprite>,
    origin: Vec2,
    tile_size: f32,
    instance: EditorObjectInstance,
) -> Result<()> {
    let asset = registry
        .get(&instance.asset)
        .with_context(|| format!("unknown object asset {}", instance.asset))?;
    textures.insert(asset.id.clone(), asset.path.clone());
    let anchor = object_anchor_to_world(origin, tile_size, instance.x, instance.y);

    sprites.push(MapSprite {
        texture_id: asset.id.clone(),
        rect: bottom_centered_rect(anchor, asset.default_size),
        flip_x: instance.flip_x,
        rotation: instance.rotation,
    });

    Ok(())
}

fn object_anchor_to_world(origin: Vec2, tile_size: f32, x: f32, y: f32) -> Vec2 {
    Vec2::new(
        origin.x + (x + 0.5) * tile_size,
        origin.y + (y + 1.0) * tile_size,
    )
}

fn centered_rect(center: Vec2, size: Vec2) -> Rect {
    Rect::new(
        Vec2::new(center.x - size.x * 0.5, center.y - size.y * 0.5),
        size,
    )
}

fn bottom_centered_rect(anchor: Vec2, size: Vec2) -> Rect {
    Rect::new(Vec2::new(anchor.x - size.x * 0.5, anchor.y - size.y), size)
}

fn map_entity_kind(value: &str) -> MapEntityKind {
    match value {
        "PlayerSpawn" => MapEntityKind::PlayerSpawn,
        "FacilityEntrance" | "Entrance" => MapEntityKind::FacilityEntrance,
        "FacilityExit" | "Exit" => MapEntityKind::FacilityExit,
        "ScanTarget" => MapEntityKind::ScanTarget,
        "Door" => MapEntityKind::Door,
        _ => MapEntityKind::Decoration,
    }
}

fn scan_codex_id(asset: &str) -> Option<String> {
    if asset.starts_with("ow_flora_") {
        Some(format!(
            "codex.flora.{}",
            asset.trim_start_matches("ow_flora_")
        ))
    } else if asset.starts_with("ow_ruin_") {
        Some(format!(
            "codex.ruin.{}",
            asset.trim_start_matches("ow_ruin_")
        ))
    } else if asset.starts_with("ow_interact_") {
        Some(format!(
            "codex.interact.{}",
            asset.trim_start_matches("ow_interact_")
        ))
    } else {
        None
    }
}

fn scan_overworld_assets() -> Result<HashMap<String, OverworldAsset>> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sprites_root = resolve_asset_path(&project_root.join("assets").join("sprites"));
    let mut assets = HashMap::new();
    let mut seen_paths = HashSet::new();

    for category in OVERWORLD_CATEGORIES {
        let category_dir = sprites_root.join(category).join("overworld");
        if !category_dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&category_dir)
            .with_context(|| format!("failed to scan {}", category_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
            {
                continue;
            }
            if !seen_paths.insert(path.clone()) {
                continue;
            }

            let Some(id) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
            else {
                continue;
            };

            assets.insert(
                id.clone(),
                OverworldAsset {
                    id,
                    default_size: default_asset_size(
                        category,
                        image::image_dimensions(&path).with_context(|| {
                            format!("failed to read image size {}", path.display())
                        })?,
                    ),
                    path,
                },
            );
        }
    }

    Ok(assets)
}

const OVERWORLD_CATEGORIES: &[&str] = &[
    "tiles",
    "decals",
    "props",
    "flora",
    "fauna",
    "structures",
    "ruins",
    "interactables",
    "pickups",
    "zones",
];

fn default_asset_size(category: &str, source_size: (u32, u32)) -> Vec2 {
    let target_height = match category {
        "tiles" => 32.0,
        "decals" | "zones" => 48.0,
        "ruins" | "structures" => 128.0,
        "interactables" | "fauna" | "pickups" => 72.0,
        _ => 72.0,
    };
    let width = source_size.0.max(1) as f32;
    let height = source_size.1.max(1) as f32;
    let scale = target_height / height;

    Vec2::new(width * scale, target_height)
}

fn default_tile_extent() -> i32 {
    1
}

fn parse_map_file(source: &str) -> Result<AnyMapFile> {
    match ron::from_str::<EditorMapFile>(source) {
        Ok(file) => Ok(AnyMapFile::Editor(file)),
        Err(editor_error) => match ron::from_str::<LegacyMapFile>(source) {
            Ok(file) => Ok(AnyMapFile::Legacy(file)),
            Err(legacy_error) => {
                bail!("not an editor map ({editor_error}); not a legacy map ({legacy_error})");
            }
        },
    }
}

fn color_from(color: [f32; 4]) -> Color {
    Color::rgba(color[0], color[1], color[2], color[3])
}

fn resolve_asset_path(path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_path_buf();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}
