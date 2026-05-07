use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use content::{
    AnchorKind, AssetDatabase, DEFAULT_ASSET_DB_PATH, InstanceRect, MapDocument as EditorMapFile,
    ObjectInstance as EditorObjectInstance, UnlockRule,
};
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

    pub fn remove_entities_by_id(&mut self, ids: &std::collections::BTreeSet<String>) {
        self.entities.retain(|entity| !ids.contains(&entity.id));
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

        let mut sprites = self.sprites.iter().collect::<Vec<_>>();
        sprites.sort_by(|left, right| {
            left.z_index
                .cmp(&right.z_index)
                .then_with(|| left.rect.bottom().total_cmp(&right.rect.bottom()))
        });
        for sprite in sprites {
            renderer.draw_image_transformed(
                &sprite.texture_id,
                sprite.rect,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
                sprite.flip_x,
                sprite.rotation,
            );
        }

        let mut entities = self.entities.iter().collect::<Vec<_>>();
        entities.sort_by(|left, right| {
            left.z_index.cmp(&right.z_index).then_with(|| {
                left.sprite_rect
                    .bottom()
                    .total_cmp(&right.sprite_rect.bottom())
            })
        });
        for entity in entities {
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
                    .map(|entity| entity.collision_rect.unwrap_or(entity.rect)),
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

                let codex_id = entity.codex_id;
                let unlock = legacy_unlock_for_kind(entity.kind, codex_id.as_deref());

                MapEntity {
                    id: entity.id,
                    kind: entity.kind,
                    rect: Rect::new(position, size),
                    collision_rect: None,
                    sprite_rect: Rect::new(position, size),
                    color: color_from(entity.color),
                    solid: entity.solid,
                    z_index: 0,
                    asset_id: None,
                    codex_id,
                    unlock,
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
            let position = object_anchor_to_world(
                origin,
                tile_size,
                spawn.x,
                spawn.y,
                AnchorKind::BottomCenter,
            );
            entities.push(MapEntity {
                id: spawn.id,
                kind: MapEntityKind::PlayerSpawn,
                rect: centered_rect(position, Vec2::new(tile_size, tile_size)),
                collision_rect: None,
                sprite_rect: centered_rect(position, Vec2::new(tile_size, tile_size)),
                color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                solid: false,
                z_index: 0,
                asset_id: None,
                codex_id: None,
                unlock: None,
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
            let anchor =
                object_anchor_to_world(origin, tile_size, entity.x, entity.y, asset.anchor);
            let sprite_size = scaled_size(asset.default_size, entity.scale_x, entity.scale_y);
            let sprite_rect = anchored_rect(anchor, sprite_size, asset.anchor);
            let default_rect = centered_rect(anchor, Vec2::new(tile_size, tile_size));
            let hit_rect = entity
                .interaction_rect
                .map(|rect| instance_rect_to_world(origin, tile_size, entity.x, entity.y, rect))
                .unwrap_or(default_rect);
            let collision_rect = entity
                .collision_rect
                .map(|rect| instance_rect_to_world(origin, tile_size, entity.x, entity.y, rect));
            let kind = map_entity_kind(&entity.entity_type);
            let codex_id = asset.codex_id.clone();
            let unlock = MapUnlockRule::from_content(entity.unlock.clone())
                .or_else(|| legacy_unlock_for_kind(kind, codex_id.as_deref()));

            entities.push(MapEntity {
                id: entity.id,
                kind,
                rect: hit_rect,
                collision_rect,
                sprite_rect,
                color: Color::rgba(0.65, 0.35, 1.0, 0.75),
                solid: false,
                z_index: entity.z_index,
                asset_id: Some(asset.id.clone()),
                codex_id,
                unlock,
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
    z_index: i32,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MapEntity {
    pub id: String,
    pub kind: MapEntityKind,
    pub rect: Rect,
    pub collision_rect: Option<Rect>,
    sprite_rect: Rect,
    pub color: Color,
    pub solid: bool,
    z_index: i32,
    pub asset_id: Option<String>,
    pub codex_id: Option<String>,
    pub unlock: Option<MapUnlockRule>,
    texture_id: Option<String>,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MapUnlockRule {
    pub requires_codex_id: Option<String>,
    pub requires_item_id: Option<String>,
    pub locked_message: Option<String>,
}

impl MapUnlockRule {
    fn from_content(rule: Option<UnlockRule>) -> Option<Self> {
        let rule = rule?;
        let unlock = Self {
            requires_codex_id: clean_optional_string(rule.requires_codex_id),
            requires_item_id: clean_optional_string(rule.requires_item_id),
            locked_message: clean_optional_string(rule.locked_message),
        };
        (!unlock.is_empty()).then_some(unlock)
    }

    pub fn is_empty(&self) -> bool {
        self.requires_codex_id.is_none()
            && self.requires_item_id.is_none()
            && self.locked_message.is_none()
    }
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

#[derive(Clone, Debug)]
struct OverworldAsset {
    id: String,
    path: PathBuf,
    default_size: Vec2,
    anchor: AnchorKind,
    codex_id: Option<String>,
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
    let anchor = object_anchor_to_world(origin, tile_size, instance.x, instance.y, asset.anchor);

    sprites.push(MapSprite {
        texture_id: asset.id.clone(),
        rect: anchored_rect(
            anchor,
            scaled_size(asset.default_size, instance.scale_x, instance.scale_y),
            asset.anchor,
        ),
        z_index: instance.z_index,
        flip_x: instance.flip_x,
        rotation: instance.rotation,
    });

    Ok(())
}

fn object_anchor_to_world(
    origin: Vec2,
    tile_size: f32,
    x: f32,
    y: f32,
    anchor: AnchorKind,
) -> Vec2 {
    match anchor {
        AnchorKind::TopLeft => Vec2::new(origin.x + x * tile_size, origin.y + y * tile_size),
        AnchorKind::Center => Vec2::new(
            origin.x + (x + 0.5) * tile_size,
            origin.y + (y + 0.5) * tile_size,
        ),
        AnchorKind::BottomCenter => Vec2::new(
            origin.x + (x + 0.5) * tile_size,
            origin.y + (y + 1.0) * tile_size,
        ),
    }
}

fn scaled_size(default_size: Vec2, scale_x: f32, scale_y: f32) -> Vec2 {
    Vec2::new(
        default_size.x * scale_x.max(0.05),
        default_size.y * scale_y.max(0.05),
    )
}

fn centered_rect(center: Vec2, size: Vec2) -> Rect {
    Rect::new(
        Vec2::new(center.x - size.x * 0.5, center.y - size.y * 0.5),
        size,
    )
}

fn anchored_rect(anchor: Vec2, size: Vec2, anchor_kind: AnchorKind) -> Rect {
    let origin = match anchor_kind {
        AnchorKind::TopLeft => anchor,
        AnchorKind::Center => Vec2::new(anchor.x - size.x * 0.5, anchor.y - size.y * 0.5),
        AnchorKind::BottomCenter => Vec2::new(anchor.x - size.x * 0.5, anchor.y - size.y),
    };
    Rect::new(origin, size)
}

fn instance_rect_to_world(
    origin: Vec2,
    tile_size: f32,
    x: f32,
    y: f32,
    rect: InstanceRect,
) -> Rect {
    Rect::new(
        Vec2::new(
            origin.x + (x + rect.offset[0]) * tile_size,
            origin.y + (y + rect.offset[1]) * tile_size,
        ),
        Vec2::new(rect.size[0] * tile_size, rect.size[1] * tile_size),
    )
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

fn legacy_unlock_for_kind(kind: MapEntityKind, codex_id: Option<&str>) -> Option<MapUnlockRule> {
    if !matches!(kind, MapEntityKind::FacilityEntrance | MapEntityKind::Door) {
        return None;
    }

    clean_optional_string(codex_id.map(str::to_owned)).map(|requires_codex_id| MapUnlockRule {
        requires_codex_id: Some(requires_codex_id),
        ..MapUnlockRule::default()
    })
}

fn clean_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn scan_overworld_assets() -> Result<HashMap<String, OverworldAsset>> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let database = AssetDatabase::load(&resolve_asset_path(
        &project_root.join(DEFAULT_ASSET_DB_PATH),
    ))?;

    Ok(database
        .assets
        .into_iter()
        .map(|asset| {
            (
                asset.id.clone(),
                OverworldAsset {
                    id: asset.id,
                    path: resolve_asset_path(&project_root.join(asset.path)),
                    default_size: Vec2::new(asset.default_size[0], asset.default_size[1]),
                    anchor: asset.anchor,
                    codex_id: asset.codex_id,
                },
            )
        })
        .collect())
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
