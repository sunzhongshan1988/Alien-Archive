use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use runtime::{Color, Rect, Renderer, Vec2};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Map {
    tiles: Vec<MapTile>,
    entities: Vec<MapEntity>,
}

impl Map {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read map file {}", path.display()))?;
        let file: MapFile = ron::from_str(&source)
            .with_context(|| format!("failed to parse map file {}", path.display()))?;

        Self::from_file(file).with_context(|| format!("invalid map {}", path.display()))
    }

    pub fn entities(&self) -> &[MapEntity] {
        &self.entities
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        for tile in &self.tiles {
            renderer.draw_rect(tile.rect, tile.color);
        }

        for entity in &self.entities {
            if entity.kind != MapEntityKind::PlayerSpawn {
                renderer.draw_rect(entity.rect, entity.color);
            }
        }
    }

    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.tiles
            .iter()
            .filter(|tile| tile.solid)
            .map(|tile| tile.rect)
            .chain(
                self.entities
                    .iter()
                    .filter(|entity| entity.solid)
                    .map(|entity| entity.rect),
            )
    }

    fn from_file(file: MapFile) -> Result<Self> {
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

                let position = grid_to_world(origin, file.tile_size, column_index, row_index);
                tiles.push(MapTile {
                    rect: Rect::new(position, Vec2::new(file.tile_size, file.tile_size)),
                    color: color_from(entry.color),
                    solid: entry.solid,
                });
            }
        }

        let entities = file
            .entities
            .into_iter()
            .map(|entity| {
                let position = grid_to_world(
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
                    color: color_from(entity.color),
                    solid: entity.solid,
                    codex_id: entity.codex_id,
                }
            })
            .collect();

        Ok(Self { tiles, entities })
    }
}

#[derive(Clone, Debug)]
struct MapTile {
    rect: Rect,
    color: Color,
    solid: bool,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MapEntity {
    pub id: String,
    pub kind: MapEntityKind,
    pub rect: Rect,
    pub color: Color,
    pub solid: bool,
    pub codex_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MapEntityKind {
    PlayerSpawn,
    ScanTarget,
    Door,
    Decoration,
}

#[derive(Debug, Deserialize)]
struct MapFile {
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

fn grid_to_world(origin: Vec2, tile_size: f32, column: usize, row: usize) -> Vec2 {
    Vec2::new(
        origin.x + column as f32 * tile_size,
        origin.y + row as f32 * tile_size,
    )
}

fn color_from(color: [f32; 4]) -> Color {
    Color::rgba(color[0], color[1], color[2], color[3])
}
