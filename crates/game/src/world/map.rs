use std::{
    cmp::Ordering,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use content::{
    AnchorKind, AssetDatabase, DEFAULT_ASSET_DB_PATH, HazardRule as EditorHazardRule, InstanceRect,
    MapDocument as EditorMapFile, ObjectInstance as EditorObjectInstance,
    ObjectiveRule as EditorObjectiveRule, PromptRule as EditorPromptRule,
    SurfaceGateRule as EditorSurfaceGateRule, TransitionTarget, UnlockRule,
    WalkSurfaceKind as EditorWalkSurfaceKind, WalkSurfaceRule as EditorWalkSurfaceRule,
    ZoneInstance as EditorZoneInstance,
};
use image::{RgbaImage, imageops};
use runtime::{Color, Rect, Renderer, Vec2, collision::rects_overlap};
use serde::Deserialize;

const GROUND_CACHE_CHUNK_TILES: u32 = 32;
const MAP_TEXTURE_ATLAS_WIDTH: u32 = 2048;
const MAP_TEXTURE_ATLAS_PADDING: u32 = 2;
const DEFAULT_OBJECT_DEPTH_INSET_TILES: f32 = 0.5;
const COLLISION_LINE_WIDTH_TILES: f32 = 0.25;
const COLLISION_LINE_SAMPLE_STEP_TILES: f32 = COLLISION_LINE_WIDTH_TILES * 0.5;
const WALK_SURFACE_EDGE_TOLERANCE: f32 = 4.0;
const WALK_SURFACE_GATE_SIDE_TOLERANCE_PIXELS: f32 = 0.25;
const WALK_SURFACE_GATE_INTERSECTION_TOLERANCE_PIXELS: f32 = 0.25;

#[derive(Clone, Debug)]
pub struct Map {
    tiles: Vec<MapTile>,
    ground_cache: Option<MapGroundCache>,
    sprites: Vec<MapSprite>,
    entities: Vec<MapEntity>,
    zones: Vec<MapZone>,
    surface_gates: Vec<MapSurfaceGate>,
    collision_rects: Vec<Rect>,
    zone_collision_rects: Vec<Rect>,
    surface_collision_rects: Vec<MapSurfaceCollisionRect>,
    textures: HashMap<String, PathBuf>,
    texture_atlas: Option<MapTextureAtlas>,
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

    pub fn zones(&self) -> &[MapZone] {
        &self.zones
    }

    pub fn remove_entities_by_id(&mut self, ids: &std::collections::BTreeSet<String>) {
        self.entities.retain(|entity| !ids.contains(&entity.id));
    }

    pub fn entity_by_id(&self, id: &str) -> Option<&MapEntity> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        let visible = renderer.visible_world_rect();
        self.draw_ground(renderer, visible);
        self.draw_decals(renderer, visible);

        for drawable in self.sorted_depth_drawables(visible) {
            drawable.draw(renderer);
        }
    }

    pub fn draw_debug_geometry(
        &self,
        renderer: &mut dyn Renderer,
        active_scan_target_id: Option<&str>,
        player_rect: Option<Rect>,
    ) {
        let visible = renderer.visible_world_rect();

        for zone in self
            .zones
            .iter()
            .filter(|zone| rects_overlap(zone.bounds, visible))
        {
            let (fill, border) = debug_zone_colors(zone);
            draw_debug_rect(renderer, zone.bounds, fill, border, 2.0);
        }

        for rect in self
            .solid_rects()
            .filter(|rect| rects_overlap(*rect, visible))
        {
            draw_debug_rect(
                renderer,
                rect,
                Color::rgba(1.0, 0.05, 0.02, 0.08),
                Color::rgba(1.0, 0.12, 0.08, 0.80),
                2.0,
            );
        }

        for entity in self
            .entities
            .iter()
            .filter(|entity| rects_overlap(entity.rect, visible))
        {
            let is_active_scan_target = active_scan_target_id == Some(entity.id.as_str());
            let (fill, border, thickness) = if is_active_scan_target {
                (
                    Color::rgba(1.0, 0.84, 0.10, 0.14),
                    Color::rgba(1.0, 0.88, 0.16, 0.95),
                    3.0,
                )
            } else if entity.codex_id.is_some() {
                (
                    Color::rgba(0.08, 0.90, 1.0, 0.08),
                    Color::rgba(0.16, 0.95, 1.0, 0.70),
                    2.0,
                )
            } else {
                (
                    Color::rgba(0.06, 0.42, 1.0, 0.06),
                    Color::rgba(0.18, 0.58, 1.0, 0.55),
                    1.5,
                )
            };
            draw_debug_rect(renderer, entity.rect, fill, border, thickness);

            if let Some(collision_rect) = entity.collision_rect {
                if rects_overlap(collision_rect, visible) {
                    draw_debug_rect(
                        renderer,
                        collision_rect,
                        Color::rgba(1.0, 0.45, 0.08, 0.07),
                        Color::rgba(1.0, 0.58, 0.16, 0.70),
                        1.5,
                    );
                }
            }
        }

        if let Some(rect) = player_rect.filter(|rect| rects_overlap(*rect, visible)) {
            draw_debug_rect(
                renderer,
                rect,
                Color::rgba(0.20, 1.0, 0.36, 0.11),
                Color::rgba(0.34, 1.0, 0.46, 0.92),
                2.0,
            );
        }
    }

    #[allow(dead_code)]
    pub fn draw_with_actor(
        &self,
        renderer: &mut dyn Renderer,
        actor_depth_y: f32,
        draw_actor: impl FnOnce(&mut dyn Renderer),
    ) {
        self.draw_with_actor_at_depth(renderer, actor_depth_y, 0, draw_actor);
    }

    pub fn draw_with_actor_at_depth(
        &self,
        renderer: &mut dyn Renderer,
        actor_depth_y: f32,
        actor_z_index: i32,
        draw_actor: impl FnOnce(&mut dyn Renderer),
    ) {
        let visible = renderer.visible_world_rect();
        self.draw_ground(renderer, visible);
        self.draw_decals(renderer, visible);

        let actor_key = DepthKey::new(actor_depth_y, actor_z_index);
        let mut draw_actor = Some(draw_actor);

        for drawable in self.sorted_depth_drawables(visible) {
            if draw_actor.is_some() && actor_key.cmp(&drawable.depth_key()) == Ordering::Less {
                draw_actor.take().expect("actor draw should exist")(renderer);
            }
            drawable.draw(renderer);
        }

        if let Some(draw_actor) = draw_actor {
            draw_actor(renderer);
        }
    }

    fn draw_ground(&self, renderer: &mut dyn Renderer, visible: Rect) {
        if let Some(cache) = &self.ground_cache {
            for chunk in cache.visible_chunks(visible) {
                renderer.draw_image(
                    &chunk.texture_id,
                    chunk.rect,
                    Color::rgba(1.0, 1.0, 1.0, 1.0),
                );
            }
            return;
        }

        for tile in self
            .tiles
            .iter()
            .filter(|tile| rects_overlap(tile.rect, visible))
        {
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
    }

    fn draw_decals(&self, renderer: &mut dyn Renderer, visible: Rect) {
        let mut sprites = self
            .sprites
            .iter()
            .filter(|sprite| {
                sprite.layer == MapSpriteLayer::Decal && rects_overlap(sprite.rect, visible)
            })
            .collect::<Vec<_>>();
        sprites.sort_by(|left, right| {
            left.z_index
                .cmp(&right.z_index)
                .then_with(|| left.rect.bottom().total_cmp(&right.rect.bottom()))
        });
        for sprite in sprites {
            draw_texture_region(
                renderer,
                &sprite.texture_id,
                sprite.source,
                sprite.rect,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
                sprite.flip_x,
                sprite.rotation,
            );
        }
    }

    fn sorted_depth_drawables(&self, visible: Rect) -> Vec<DepthDrawable<'_>> {
        let mut drawables = self
            .sprites
            .iter()
            .filter(|sprite| {
                sprite.layer == MapSpriteLayer::Object && rects_overlap(sprite.rect, visible)
            })
            .map(DepthDrawable::Sprite)
            .chain(
                self.entities
                    .iter()
                    .filter(|entity| {
                        entity.kind != MapEntityKind::PlayerSpawn
                            && rects_overlap(entity.sprite_rect, visible)
                    })
                    .map(DepthDrawable::Entity),
            )
            .collect::<Vec<_>>();
        drawables.sort_by(|left, right| left.depth_key().cmp(&right.depth_key()));
        drawables
    }

    pub fn load_assets(&self, renderer: &mut dyn Renderer) -> Result<()> {
        if let Some(atlas) = &self.texture_atlas {
            if renderer.texture_size(&atlas.texture_id).is_none() {
                renderer.load_texture_rgba(
                    &atlas.texture_id,
                    atlas.width,
                    atlas.height,
                    &atlas.rgba,
                )?;
            }
        }

        for (texture_id, path) in &self.textures {
            if renderer.texture_size(texture_id).is_none() {
                renderer.load_texture(texture_id, path)?;
            }
        }

        Ok(())
    }

    pub fn load_visible_ground_assets(&self, renderer: &mut dyn Renderer) -> Result<()> {
        let Some(cache) = &self.ground_cache else {
            return Ok(());
        };

        let visible = renderer.visible_world_rect();
        let mut source_cache = HashMap::<PathBuf, RgbaImage>::new();
        for chunk in cache.visible_chunks(visible) {
            if renderer.texture_size(&chunk.texture_id).is_some() {
                continue;
            }

            let rgba = chunk.render_rgba(cache.tile_size, &mut source_cache)?;
            renderer.load_texture_rgba(&chunk.texture_id, chunk.width, chunk.height, &rgba)?;
        }

        Ok(())
    }

    pub fn solid_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.solid_rects_without_zone_collision()
            .chain(self.zone_collision_rects.iter().copied())
            .chain(
                self.surface_collision_rects
                    .iter()
                    .map(|collision| collision.rect),
            )
    }

    pub fn solid_rects_without_zone_collision(&self) -> impl Iterator<Item = Rect> + '_ {
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

    pub fn zone_collision_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.zone_collision_rects.iter().copied()
    }

    pub fn surface_collision_rects<'a>(
        &'a self,
        surface_id: &'a str,
    ) -> impl Iterator<Item = Rect> + 'a {
        self.surface_collision_rects
            .iter()
            .filter(move |collision| collision.surface_id == surface_id)
            .map(|collision| collision.rect)
    }

    #[cfg(test)]
    fn walk_surface_at(&self, point: Vec2) -> Option<MapWalkSurface> {
        self.walk_surface_at_filtered(point, |_| true)
    }

    pub fn walk_surface_entry_at(&self, point: Vec2) -> Option<MapWalkSurface> {
        self.walk_surface_at_filtered(point, |surface| surface.kind == MapWalkSurfaceKind::Ramp)
    }

    pub fn walk_surface_for_id_at(&self, surface_id: &str, point: Vec2) -> Option<MapWalkSurface> {
        self.walk_surface_at_filtered(point, |surface| surface.surface_id == surface_id)
    }

    pub fn walk_surface_contains(&self, surface_id: &str, point: Vec2) -> bool {
        self.walk_surface_for_id_at(surface_id, point).is_some()
    }

    pub fn walk_surface_allows_ground_movement(&self, _previous: Vec2, _next: Vec2) -> bool {
        true
    }

    pub fn walk_surface_ground_entry(&self, previous: Vec2, next: Vec2) -> Option<MapWalkSurface> {
        let gate =
            self.walk_surface_gate_crossed(None, previous, next, SurfaceGateTraversal::Enter)?;
        self.walk_surface_for_id_at(&gate.surface_id, next)
            .filter(|surface| surface.kind == MapWalkSurfaceKind::Ramp)
    }

    pub fn walk_surface_allows_ground_entry(&self, previous: Vec2, next: Vec2) -> bool {
        self.walk_surface_ground_entry(previous, next).is_some()
    }

    pub fn walk_surface_allows_movement(
        &self,
        surface_id: &str,
        previous: Vec2,
        next: Vec2,
    ) -> bool {
        if self.walk_surface_exits(surface_id, previous, next) {
            return true;
        }
        if self.walk_surface_contains(surface_id, next) {
            return true;
        }
        false
    }

    pub fn walk_surface_exits(&self, surface_id: &str, previous: Vec2, next: Vec2) -> bool {
        self.walk_surface_gate_crossed(Some(surface_id), previous, next, SurfaceGateTraversal::Exit)
            .is_some()
    }

    fn walk_surface_at_filtered(
        &self,
        point: Vec2,
        mut include: impl FnMut(&MapWalkSurface) -> bool,
    ) -> Option<MapWalkSurface> {
        self.walk_surface_zone_at_filtered(point, &mut include)
            .map(|(_, surface)| surface.clone())
    }

    fn walk_surface_zone_at_filtered(
        &self,
        point: Vec2,
        include: &mut impl FnMut(&MapWalkSurface) -> bool,
    ) -> Option<(&MapZone, &MapWalkSurface)> {
        self.zones
            .iter()
            .filter_map(|zone| zone.surface.as_ref().map(|surface| (zone, surface)))
            .filter(|(_, surface)| include(surface))
            .filter(|(zone, _)| walk_surface_zone_contains_point(zone, point))
            .max_by(|(_, left), (_, right)| compare_walk_surfaces(left, right))
    }

    fn walk_surface_gate_crossed(
        &self,
        surface_id: Option<&str>,
        previous: Vec2,
        next: Vec2,
        traversal: SurfaceGateTraversal,
    ) -> Option<&MapSurfaceGate> {
        self.surface_gates
            .iter()
            .filter(|gate| {
                surface_id.is_none_or(|surface_id| gate.surface_id.as_str() == surface_id)
            })
            .find(|gate| self.surface_gate_allows_traversal(gate, previous, next, traversal))
    }

    fn surface_gate_allows_traversal(
        &self,
        gate: &MapSurfaceGate,
        previous: Vec2,
        next: Vec2,
        traversal: SurfaceGateTraversal,
    ) -> bool {
        if !segments_intersect(previous, next, gate.start, gate.end) {
            return false;
        }

        let gate_vector = gate.end - gate.start;
        let side_tolerance = WALK_SURFACE_GATE_SIDE_TOLERANCE_PIXELS * vec2_length(gate_vector);
        let Some(platform_side) = self.surface_gate_platform_side(gate) else {
            return false;
        };
        if platform_side.abs() <= side_tolerance {
            return false;
        }

        let side_sign = platform_side.signum();
        let previous_side = vec2_cross(gate_vector, previous - gate.start) * side_sign;
        let next_side = vec2_cross(gate_vector, next - gate.start) * side_sign;

        match traversal {
            SurfaceGateTraversal::Enter => {
                previous_side < -side_tolerance && next_side >= -side_tolerance
            }
            SurfaceGateTraversal::Exit => {
                previous_side >= side_tolerance && next_side <= side_tolerance
            }
        }
    }

    fn surface_gate_platform_side(&self, gate: &MapSurfaceGate) -> Option<f32> {
        let gate_center = gate.center();
        let platform_center = self.nearest_walk_surface_zone_center(
            &gate.surface_id,
            MapWalkSurfaceKind::Platform,
            gate_center,
        )?;
        Some(vec2_cross(
            gate.end - gate.start,
            platform_center - gate.start,
        ))
    }

    fn nearest_walk_surface_zone_center(
        &self,
        surface_id: &str,
        kind: MapWalkSurfaceKind,
        point: Vec2,
    ) -> Option<Vec2> {
        self.zones
            .iter()
            .filter_map(|zone| zone.surface.as_ref().map(|surface| (zone, surface)))
            .filter(|(_, surface)| surface.surface_id == surface_id && surface.kind == kind)
            .min_by(|(left, _), (right, _)| {
                vec2_length_squared(rect_center(left.bounds) - point)
                    .total_cmp(&vec2_length_squared(rect_center(right.bounds) - point))
            })
            .map(|(zone, _)| rect_center(zone.bounds))
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
                    depth_y: Rect::new(position, size).bottom(),
                    color: color_from(entity.color),
                    solid: entity.solid,
                    z_index: 0,
                    asset_id: None,
                    codex_id,
                    unlock,
                    transition: MapTransitionTarget::from_content(entity.transition),
                    texture_id: None,
                    source: None,
                    flip_x: false,
                    rotation: 0,
                }
            })
            .collect();

        Ok(Self {
            tiles,
            sprites: Vec::new(),
            entities,
            zones: Vec::new(),
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
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
        let mut ground_sources = Vec::new();

        let mut tiles = Vec::new();
        let mut collision_rects = Vec::new();
        for tile in file.layers.ground {
            let asset = registry
                .get(&tile.asset)
                .with_context(|| format!("unknown tile asset {}", tile.asset))?;
            let position = grid_to_world(origin, tile_size, tile.x, tile.y);
            let size = Vec2::new(
                tile_size * tile.w.max(1) as f32,
                tile_size * tile.h.max(1) as f32,
            );
            ground_sources.push(GroundTextureSource {
                path: asset.path.clone(),
                x: tile.x,
                y: tile.y,
                w: tile.w.max(1),
                h: tile.h.max(1),
                flip_x: tile.flip_x,
                rotation: tile.rotation,
            });

            tiles.push(MapTile {
                rect: Rect::new(position, size),
                visual: MapVisual::Texture(asset.id.clone()),
                solid: false,
                flip_x: tile.flip_x,
                rotation: tile.rotation,
            });

            if let Some(rect) = asset.default_collision_rect {
                collision_rects.push(instance_rect_to_world(
                    origin,
                    tile_size,
                    tile.x as f32,
                    tile.y as f32,
                    rect,
                ));
            }
        }
        let ground_cache = build_ground_cache(
            &file.id,
            &ground_sources,
            origin,
            file.width,
            file.height,
            file.tile_size,
        )?;

        let mut sprites = Vec::new();
        for decal in file.layers.decals {
            push_sprite(
                &registry,
                &mut textures,
                &mut sprites,
                &mut collision_rects,
                MapSpriteLayer::Decal,
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
                &mut collision_rects,
                MapSpriteLayer::Object,
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
                depth_y: position.y,
                color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                solid: false,
                z_index: 0,
                asset_id: None,
                codex_id: None,
                unlock: None,
                transition: None,
                texture_id: None,
                source: None,
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
                .or(asset.default_interaction_rect)
                .map(|rect| instance_rect_to_world(origin, tile_size, entity.x, entity.y, rect))
                .unwrap_or(default_rect);
            let collision_rect = entity
                .collision_rect
                .or(asset.default_collision_rect)
                .map(|rect| instance_rect_to_world(origin, tile_size, entity.x, entity.y, rect));
            let depth_y = entity
                .depth_rect
                .or(asset.default_depth_rect)
                .or(entity.collision_rect)
                .or(asset.default_collision_rect)
                .map(|rect| instance_rect_to_world(origin, tile_size, entity.x, entity.y, rect))
                .map(|rect| rect.bottom())
                .unwrap_or_else(|| fallback_object_depth_y(sprite_rect, tile_size));
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
                depth_y,
                color: Color::rgba(0.65, 0.35, 1.0, 0.75),
                solid: false,
                z_index: entity.z_index,
                asset_id: Some(asset.id.clone()),
                codex_id,
                unlock,
                transition: MapTransitionTarget::from_content(entity.transition),
                texture_id: Some(asset.id.clone()),
                source: None,
                flip_x: entity.flip_x,
                rotation: entity.rotation,
            });
        }

        let texture_atlas = build_texture_atlas(&file.id, &textures, &mut sprites, &mut entities)?;
        if texture_atlas.is_some() {
            textures.clear();
        }

        let (zone_collision_rects, surface_collision_rects) = collision_rects_from_zones(
            &file.layers.zones,
            origin,
            tile_size,
            file.width,
            file.height,
        );

        let mut surface_gates = Vec::new();
        let zones = file
            .layers
            .zones
            .into_iter()
            .map(|zone| {
                let points = zone
                    .points
                    .into_iter()
                    .map(|point| {
                        Vec2::new(
                            origin.x + point[0] * tile_size,
                            origin.y + point[1] * tile_size,
                        )
                    })
                    .collect::<Vec<_>>();
                let bounds = bounds_for_points(&points);
                if let Some(gate) =
                    MapSurfaceGate::from_content(&zone.id, &zone.zone_type, zone.gate, &points)
                {
                    surface_gates.push(gate);
                }
                let surface = MapWalkSurface::from_content(&zone.id, zone.surface);
                MapZone {
                    id: zone.id,
                    zone_type: zone.zone_type,
                    points,
                    bounds,
                    hazard: MapHazardRule::from_content(zone.hazard),
                    prompt: MapPromptRule::from_content(zone.prompt),
                    objective: MapObjectiveRule::from_content(zone.objective),
                    surface,
                    unlock: MapUnlockRule::from_content(zone.unlock),
                    transition: MapTransitionTarget::from_content(zone.transition),
                }
            })
            .collect();

        collision_rects.extend(
            file.layers
                .collision
                .into_iter()
                .filter(|cell| cell.solid)
                .map(|cell| {
                    let bounds = cell.bounds();
                    Rect::new(
                        Vec2::new(
                            origin.x + bounds.x * tile_size,
                            origin.y + bounds.y * tile_size,
                        ),
                        Vec2::new(bounds.w * tile_size, bounds.h * tile_size),
                    )
                }),
        );

        Ok(Self {
            tiles,
            sprites,
            entities,
            zones,
            surface_gates,
            collision_rects,
            zone_collision_rects,
            surface_collision_rects,
            ground_cache,
            textures,
            texture_atlas,
        })
    }
}

#[derive(Clone, Debug)]
struct MapTextureAtlas {
    texture_id: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

struct AtlasEntry {
    id: String,
    image: RgbaImage,
}

fn build_texture_atlas(
    map_id: &str,
    textures: &HashMap<String, PathBuf>,
    sprites: &mut [MapSprite],
    entities: &mut [MapEntity],
) -> Result<Option<MapTextureAtlas>> {
    if textures.len() <= 1 {
        return Ok(None);
    }

    let mut texture_paths = textures.iter().collect::<Vec<_>>();
    texture_paths.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut entries = Vec::with_capacity(texture_paths.len());
    for (id, path) in texture_paths {
        let image = image::ImageReader::open(path)
            .with_context(|| format!("failed to open map atlas image {}", path.display()))?
            .decode()
            .with_context(|| format!("failed to decode map atlas image {}", path.display()))?
            .to_rgba8();
        entries.push(AtlasEntry {
            id: id.clone(),
            image,
        });
    }

    let widest = entries
        .iter()
        .map(|entry| entry.image.width())
        .max()
        .unwrap_or(1);
    let atlas_width = MAP_TEXTURE_ATLAS_WIDTH.max(widest + MAP_TEXTURE_ATLAS_PADDING * 2);
    let placements = pack_atlas_entries(&entries, atlas_width);
    let atlas_height = placements
        .values()
        .map(|region| region.bottom().ceil() as u32 + MAP_TEXTURE_ATLAS_PADDING)
        .max()
        .unwrap_or(1)
        .max(1);

    let mut canvas = RgbaImage::new(atlas_width, atlas_height);
    for entry in &entries {
        let Some(region) = placements.get(&entry.id) else {
            continue;
        };
        imageops::overlay(
            &mut canvas,
            &entry.image,
            region.origin.x as i64,
            region.origin.y as i64,
        );
    }

    let atlas_id = format!("__map_texture_atlas_{}", texture_id_component(map_id));
    for sprite in sprites {
        if let Some(region) = placements.get(&sprite.texture_id) {
            sprite.texture_id = atlas_id.clone();
            sprite.source = Some(*region);
        }
    }
    for entity in entities {
        let Some(texture_id) = &mut entity.texture_id else {
            continue;
        };
        if let Some(region) = placements.get(texture_id) {
            *texture_id = atlas_id.clone();
            entity.source = Some(*region);
        }
    }

    Ok(Some(MapTextureAtlas {
        texture_id: atlas_id,
        width: atlas_width,
        height: atlas_height,
        rgba: canvas.into_raw(),
    }))
}

fn pack_atlas_entries(entries: &[AtlasEntry], atlas_width: u32) -> HashMap<String, Rect> {
    let mut placements = HashMap::new();
    let mut x = MAP_TEXTURE_ATLAS_PADDING;
    let mut y = MAP_TEXTURE_ATLAS_PADDING;
    let mut row_height = 0;

    for entry in entries {
        let width = entry.image.width();
        let height = entry.image.height();
        if x > MAP_TEXTURE_ATLAS_PADDING
            && x.saturating_add(width)
                .saturating_add(MAP_TEXTURE_ATLAS_PADDING)
                > atlas_width
        {
            x = MAP_TEXTURE_ATLAS_PADDING;
            y = y
                .saturating_add(row_height)
                .saturating_add(MAP_TEXTURE_ATLAS_PADDING);
            row_height = 0;
        }

        placements.insert(
            entry.id.clone(),
            Rect::new(
                Vec2::new(x as f32, y as f32),
                Vec2::new(width as f32, height as f32),
            ),
        );
        x = x
            .saturating_add(width)
            .saturating_add(MAP_TEXTURE_ATLAS_PADDING);
        row_height = row_height.max(height);
    }

    placements
}

#[derive(Clone, Debug)]
struct MapGroundCache {
    tile_size: u32,
    chunks: Vec<MapGroundChunkCache>,
}

impl MapGroundCache {
    fn visible_chunks(&self, visible: Rect) -> impl Iterator<Item = &MapGroundChunkCache> {
        self.chunks
            .iter()
            .filter(move |chunk| rects_overlap(chunk.rect, visible))
    }
}

#[derive(Clone, Debug)]
struct MapGroundChunkCache {
    texture_id: String,
    rect: Rect,
    width: u32,
    height: u32,
    tile_origin_x: u32,
    tile_origin_y: u32,
    sources: Vec<GroundTextureSource>,
}

impl MapGroundChunkCache {
    fn render_rgba(
        &self,
        tile_size: u32,
        source_cache: &mut HashMap<PathBuf, RgbaImage>,
    ) -> Result<Vec<u8>> {
        let mut canvas = RgbaImage::new(self.width, self.height);

        for source in &self.sources {
            let tile = if let Some(tile) = source_cache.get(&source.path) {
                tile.clone()
            } else {
                let tile = image::ImageReader::open(&source.path)
                    .with_context(|| {
                        format!("failed to open ground tile image {}", source.path.display())
                    })?
                    .decode()
                    .with_context(|| {
                        format!(
                            "failed to decode ground tile image {}",
                            source.path.display()
                        )
                    })?
                    .to_rgba8();
                source_cache.insert(source.path.clone(), tile.clone());
                tile
            };

            let target_width = (source.w.max(1) as u32).saturating_mul(tile_size).max(1);
            let target_height = (source.h.max(1) as u32).saturating_mul(tile_size).max(1);
            let tile = transform_ground_tile(
                &tile,
                target_width,
                target_height,
                source.flip_x,
                source.rotation,
            );
            imageops::overlay(
                &mut canvas,
                &tile,
                (source.x as i64 - self.tile_origin_x as i64) * tile_size as i64,
                (source.y as i64 - self.tile_origin_y as i64) * tile_size as i64,
            );
        }

        Ok(canvas.into_raw())
    }
}

#[derive(Clone, Debug)]
struct GroundTextureSource {
    path: PathBuf,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    flip_x: bool,
    rotation: i32,
}

fn build_ground_cache(
    map_id: &str,
    sources: &[GroundTextureSource],
    origin: Vec2,
    map_width: u32,
    map_height: u32,
    tile_size: u32,
) -> Result<Option<MapGroundCache>> {
    if sources.is_empty() || map_width == 0 || map_height == 0 {
        return Ok(None);
    }

    let chunk_tiles = GROUND_CACHE_CHUNK_TILES.max(1);
    let chunks_x = map_width.div_ceil(chunk_tiles);
    let chunks_y = map_height.div_ceil(chunk_tiles);
    let mut chunk_sources = HashMap::<(u32, u32), Vec<GroundTextureSource>>::new();

    let map_width = map_width as i32;
    let map_height = map_height as i32;
    for source in sources {
        let source_left = source.x;
        let source_top = source.y;
        let source_right = source.x.saturating_add(source.w.max(1));
        let source_bottom = source.y.saturating_add(source.h.max(1));
        if source_left >= map_width
            || source_top >= map_height
            || source_right <= 0
            || source_bottom <= 0
        {
            continue;
        }

        let min_chunk_x = (source_left.max(0) as u32) / chunk_tiles;
        let min_chunk_y = (source_top.max(0) as u32) / chunk_tiles;
        let max_chunk_x = ((source_right.min(map_width) - 1).max(0) as u32) / chunk_tiles;
        let max_chunk_y = ((source_bottom.min(map_height) - 1).max(0) as u32) / chunk_tiles;

        for chunk_y in min_chunk_y..=max_chunk_y.min(chunks_y.saturating_sub(1)) {
            for chunk_x in min_chunk_x..=max_chunk_x.min(chunks_x.saturating_sub(1)) {
                chunk_sources
                    .entry((chunk_x, chunk_y))
                    .or_default()
                    .push(source.clone());
            }
        }
    }

    let map_id = texture_id_component(map_id);
    let mut chunks = Vec::new();
    for chunk_y in 0..chunks_y {
        for chunk_x in 0..chunks_x {
            let Some(sources) = chunk_sources.remove(&(chunk_x, chunk_y)) else {
                continue;
            };

            let tile_origin_x = chunk_x * chunk_tiles;
            let tile_origin_y = chunk_y * chunk_tiles;
            let tile_width = (map_width as u32 - tile_origin_x).min(chunk_tiles);
            let tile_height = (map_height as u32 - tile_origin_y).min(chunk_tiles);
            chunks.push(MapGroundChunkCache {
                texture_id: format!("__map_ground_cache_{map_id}_{chunk_x}_{chunk_y}"),
                rect: Rect::new(
                    Vec2::new(
                        origin.x + tile_origin_x as f32 * tile_size as f32,
                        origin.y + tile_origin_y as f32 * tile_size as f32,
                    ),
                    Vec2::new(
                        tile_width as f32 * tile_size as f32,
                        tile_height as f32 * tile_size as f32,
                    ),
                ),
                width: tile_width.saturating_mul(tile_size).max(1),
                height: tile_height.saturating_mul(tile_size).max(1),
                tile_origin_x,
                tile_origin_y,
                sources,
            });
        }
    }

    Ok(Some(MapGroundCache { tile_size, chunks }))
}

fn texture_id_component(value: &str) -> String {
    let value = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    if value.is_empty() {
        "map".to_owned()
    } else {
        value
    }
}

fn transform_ground_tile(
    source: &RgbaImage,
    width: u32,
    height: u32,
    flip_x: bool,
    rotation: i32,
) -> RgbaImage {
    let mut tile = if source.width() == width && source.height() == height {
        source.clone()
    } else {
        imageops::resize(source, width, height, imageops::FilterType::Nearest)
    };

    if flip_x {
        tile = imageops::flip_horizontal(&tile);
    }

    tile = match rotation.rem_euclid(360) {
        90 => imageops::rotate90(&tile),
        180 => imageops::rotate180(&tile),
        270 => imageops::rotate270(&tile),
        _ => tile,
    };

    if tile.width() != width || tile.height() != height {
        imageops::resize(&tile, width, height, imageops::FilterType::Nearest)
    } else {
        tile
    }
}

fn draw_texture_region(
    renderer: &mut dyn Renderer,
    texture_id: &str,
    source: Option<Rect>,
    rect: Rect,
    tint: Color,
    flip_x: bool,
    rotation: i32,
) {
    if let Some(source) = source {
        renderer.draw_image_region_transformed(texture_id, rect, source, tint, flip_x, rotation);
    } else {
        renderer.draw_image_transformed(texture_id, rect, tint, flip_x, rotation);
    }
}

fn debug_zone_colors(zone: &MapZone) -> (Color, Color) {
    match zone.zone_type.as_str() {
        "CollisionArea" | "CollisionLine" => (
            Color::rgba(1.0, 0.02, 0.02, 0.06),
            Color::rgba(1.0, 0.10, 0.08, 0.62),
        ),
        "MapTransition" => (
            Color::rgba(1.0, 0.62, 0.08, 0.08),
            Color::rgba(1.0, 0.72, 0.16, 0.76),
        ),
        "WalkSurface" => (
            Color::rgba(0.12, 1.0, 0.42, 0.06),
            Color::rgba(0.24, 1.0, 0.52, 0.66),
        ),
        "SurfaceGate" => (
            Color::rgba(1.0, 0.74, 0.12, 0.08),
            Color::rgba(1.0, 0.80, 0.18, 0.76),
        ),
        "HazardZone" => (
            Color::rgba(1.0, 0.10, 0.04, 0.08),
            Color::rgba(1.0, 0.18, 0.10, 0.78),
        ),
        "PromptZone" => (
            Color::rgba(0.14, 0.52, 1.0, 0.07),
            Color::rgba(0.24, 0.62, 1.0, 0.70),
        ),
        "ObjectiveZone" | "Checkpoint" => (
            Color::rgba(0.16, 1.0, 0.84, 0.08),
            Color::rgba(0.36, 1.0, 0.88, 0.74),
        ),
        _ => (
            Color::rgba(0.72, 0.28, 1.0, 0.06),
            Color::rgba(0.78, 0.40, 1.0, 0.58),
        ),
    }
}

fn draw_debug_rect(
    renderer: &mut dyn Renderer,
    rect: Rect,
    fill: Color,
    border: Color,
    thickness: f32,
) {
    if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
        return;
    }
    if fill.a > 0.0 {
        renderer.draw_rect(rect, fill);
    }
    draw_world_border(renderer, rect, thickness, border);
}

fn draw_world_border(renderer: &mut dyn Renderer, rect: Rect, thickness: f32, color: Color) {
    let thickness = thickness.max(1.0);
    let horizontal = thickness.min(rect.size.y);
    let vertical = thickness.min(rect.size.x);

    renderer.draw_rect(
        Rect::new(rect.origin, Vec2::new(rect.size.x, horizontal)),
        color,
    );
    renderer.draw_rect(
        Rect::new(
            Vec2::new(rect.origin.x, rect.bottom() - horizontal),
            Vec2::new(rect.size.x, horizontal),
        ),
        color,
    );
    renderer.draw_rect(
        Rect::new(rect.origin, Vec2::new(vertical, rect.size.y)),
        color,
    );
    renderer.draw_rect(
        Rect::new(
            Vec2::new(rect.right() - vertical, rect.origin.y),
            Vec2::new(vertical, rect.size.y),
        ),
        color,
    );
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
    source: Option<Rect>,
    rect: Rect,
    z_index: i32,
    depth_y: f32,
    layer: MapSpriteLayer,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MapSpriteLayer {
    Decal,
    Object,
}

#[derive(Clone, Copy, Debug)]
enum DepthDrawable<'a> {
    Sprite(&'a MapSprite),
    Entity(&'a MapEntity),
}

impl DepthDrawable<'_> {
    fn depth_key(self) -> DepthKey {
        match self {
            Self::Sprite(sprite) => DepthKey::new(sprite.depth_y, sprite.z_index),
            Self::Entity(entity) => DepthKey::new(entity.depth_y, entity.z_index),
        }
    }

    fn draw(self, renderer: &mut dyn Renderer) {
        match self {
            Self::Sprite(sprite) => draw_texture_region(
                renderer,
                &sprite.texture_id,
                sprite.source,
                sprite.rect,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
                sprite.flip_x,
                sprite.rotation,
            ),
            Self::Entity(entity) => {
                if let Some(texture_id) = &entity.texture_id {
                    draw_texture_region(
                        renderer,
                        texture_id,
                        entity.source,
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DepthKey {
    y: f32,
    z_index: i32,
}

impl DepthKey {
    fn new(y: f32, z_index: i32) -> Self {
        Self { y, z_index }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        self.z_index
            .cmp(&other.z_index)
            .then_with(|| self.y.total_cmp(&other.y))
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MapEntity {
    pub id: String,
    pub kind: MapEntityKind,
    pub rect: Rect,
    pub collision_rect: Option<Rect>,
    sprite_rect: Rect,
    depth_y: f32,
    pub color: Color,
    pub solid: bool,
    z_index: i32,
    pub asset_id: Option<String>,
    pub codex_id: Option<String>,
    pub unlock: Option<MapUnlockRule>,
    pub transition: Option<MapTransitionTarget>,
    texture_id: Option<String>,
    source: Option<Rect>,
    flip_x: bool,
    rotation: i32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MapZone {
    pub id: String,
    pub zone_type: String,
    pub points: Vec<Vec2>,
    pub bounds: Rect,
    pub hazard: Option<MapHazardRule>,
    pub prompt: Option<MapPromptRule>,
    pub objective: Option<MapObjectiveRule>,
    pub surface: Option<MapWalkSurface>,
    pub unlock: Option<MapUnlockRule>,
    pub transition: Option<MapTransitionTarget>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapHazardRule {
    pub effects: Vec<MapHazardEffect>,
    pub message: Option<String>,
}

impl MapHazardRule {
    fn from_content(hazard: Option<EditorHazardRule>) -> Option<Self> {
        let hazard = hazard?;
        let effects = hazard
            .effects
            .into_iter()
            .filter_map(|effect| {
                let meter_id = effect.meter.trim().to_owned();
                (!meter_id.is_empty()).then_some(MapHazardEffect {
                    meter_id,
                    rate_per_second: effect.rate_per_second,
                })
            })
            .collect::<Vec<_>>();
        let message = clean_optional_string(hazard.message);
        (!effects.is_empty() || message.is_some()).then_some(Self { effects, message })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapHazardEffect {
    pub meter_id: String,
    pub rate_per_second: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapPromptRule {
    pub message: Option<String>,
    pub log_title: Option<String>,
    pub log_detail: Option<String>,
    pub once: bool,
}

impl Default for MapPromptRule {
    fn default() -> Self {
        Self {
            message: None,
            log_title: None,
            log_detail: None,
            once: true,
        }
    }
}

impl MapPromptRule {
    fn from_content(prompt: Option<EditorPromptRule>) -> Option<Self> {
        let prompt = prompt?;
        let message = clean_optional_string(prompt.message);
        let log_title = clean_optional_string(prompt.log_title);
        let log_detail = clean_optional_string(prompt.log_detail);
        (message.is_some() || log_title.is_some() || log_detail.is_some()).then_some(Self {
            message,
            log_title,
            log_detail,
            once: prompt.once,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapObjectiveRule {
    pub objective_id: String,
    pub checkpoint_id: Option<String>,
    pub complete_objective: bool,
    pub message: Option<String>,
    pub log_title: Option<String>,
    pub log_detail: Option<String>,
    pub once: bool,
}

impl MapObjectiveRule {
    fn from_content(objective: Option<EditorObjectiveRule>) -> Option<Self> {
        let objective = objective?;
        let objective_id = objective.objective_id.trim().to_owned();
        let checkpoint_id = clean_optional_string(objective.checkpoint_id);
        let message = clean_optional_string(objective.message);
        let log_title = clean_optional_string(objective.log_title);
        let log_detail = clean_optional_string(objective.log_detail);
        (!objective_id.is_empty()).then_some(Self {
            objective_id,
            checkpoint_id,
            complete_objective: objective.complete_objective,
            message,
            log_title,
            log_detail,
            once: objective.once,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapWalkSurface {
    pub surface_id: String,
    pub kind: MapWalkSurfaceKind,
    pub constrain_movement: bool,
    pub z_index: i32,
    pub depth_offset: f32,
}

impl MapWalkSurface {
    fn from_content(zone_id: &str, surface: Option<EditorWalkSurfaceRule>) -> Option<Self> {
        surface.map(|surface| Self {
            surface_id: surface
                .surface_id
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| zone_id.to_owned()),
            kind: MapWalkSurfaceKind::from_content(surface.kind),
            constrain_movement: surface.constrain_movement,
            z_index: surface.z_index,
            depth_offset: surface.depth_offset,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapSurfaceGate {
    pub id: String,
    pub surface_id: String,
    pub start: Vec2,
    pub end: Vec2,
}

impl MapSurfaceGate {
    fn from_content(
        zone_id: &str,
        zone_type: &str,
        gate: Option<EditorSurfaceGateRule>,
        points: &[Vec2],
    ) -> Option<Self> {
        if zone_type != "SurfaceGate" {
            return None;
        }
        let gate = gate?;
        let surface_id = clean_optional_string(gate.surface_id)?;
        let [start, end, ..] = points else {
            return None;
        };
        Some(Self {
            id: zone_id.to_owned(),
            surface_id,
            start: *start,
            end: *end,
        })
    }

    fn center(&self) -> Vec2 {
        Vec2::new(
            (self.start.x + self.end.x) * 0.5,
            (self.start.y + self.end.y) * 0.5,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapSurfaceCollisionRect {
    pub surface_id: String,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SurfaceGateTraversal {
    Enter,
    Exit,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MapWalkSurfaceKind {
    #[default]
    Platform,
    Ramp,
}

impl MapWalkSurfaceKind {
    fn from_content(kind: EditorWalkSurfaceKind) -> Self {
        match kind {
            EditorWalkSurfaceKind::Platform => Self::Platform,
            EditorWalkSurfaceKind::Ramp => Self::Ramp,
        }
    }
}

fn walk_surface_kind_priority(kind: MapWalkSurfaceKind) -> i32 {
    match kind {
        MapWalkSurfaceKind::Ramp => 0,
        MapWalkSurfaceKind::Platform => 1,
    }
}

fn compare_walk_surfaces(left: &MapWalkSurface, right: &MapWalkSurface) -> Ordering {
    left.z_index
        .cmp(&right.z_index)
        .then_with(|| {
            walk_surface_kind_priority(left.kind).cmp(&walk_surface_kind_priority(right.kind))
        })
        .then_with(|| left.depth_offset.total_cmp(&right.depth_offset))
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MapUnlockRule {
    pub requires_codex_id: Option<String>,
    pub requires_item_id: Option<String>,
    pub locked_message: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MapTransitionTarget {
    pub scene: Option<String>,
    pub map_path: Option<String>,
    pub spawn_id: Option<String>,
}

impl MapTransitionTarget {
    fn from_content(target: Option<TransitionTarget>) -> Option<Self> {
        let target = target?;
        let transition = Self {
            scene: clean_optional_string(target.scene),
            map_path: clean_optional_string(target.map_path),
            spawn_id: clean_optional_string(target.spawn_id),
        };
        (!transition.is_empty()).then_some(transition)
    }

    pub fn is_empty(&self) -> bool {
        self.scene.is_none() && self.map_path.is_none() && self.spawn_id.is_none()
    }
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
    #[serde(default)]
    transition: Option<TransitionTarget>,
}

#[derive(Clone, Debug)]
struct OverworldAsset {
    id: String,
    path: PathBuf,
    default_size: Vec2,
    default_collision_rect: Option<InstanceRect>,
    default_depth_rect: Option<InstanceRect>,
    default_interaction_rect: Option<InstanceRect>,
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
    collision_rects: &mut Vec<Rect>,
    layer: MapSpriteLayer,
    origin: Vec2,
    tile_size: f32,
    instance: EditorObjectInstance,
) -> Result<()> {
    let asset = registry
        .get(&instance.asset)
        .with_context(|| format!("unknown object asset {}", instance.asset))?;
    textures.insert(asset.id.clone(), asset.path.clone());
    let anchor = object_anchor_to_world(origin, tile_size, instance.x, instance.y, asset.anchor);
    let sprite_rect = anchored_rect(
        anchor,
        scaled_size(asset.default_size, instance.scale_x, instance.scale_y),
        asset.anchor,
    );
    let depth_y = instance
        .depth_rect
        .or(asset.default_depth_rect)
        .or(instance.collision_rect)
        .or(asset.default_collision_rect)
        .map(|rect| instance_rect_to_world(origin, tile_size, instance.x, instance.y, rect))
        .map(|rect| rect.bottom())
        .unwrap_or_else(|| fallback_object_depth_y(sprite_rect, tile_size));

    sprites.push(MapSprite {
        texture_id: asset.id.clone(),
        source: None,
        rect: sprite_rect,
        z_index: instance.z_index,
        depth_y,
        layer,
        flip_x: instance.flip_x,
        rotation: instance.rotation,
    });

    if layer == MapSpriteLayer::Object {
        if let Some(rect) = instance.collision_rect.or(asset.default_collision_rect) {
            collision_rects.push(instance_rect_to_world(
                origin, tile_size, instance.x, instance.y, rect,
            ));
        }
    }

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

fn collision_rects_from_zones(
    zones: &[EditorZoneInstance],
    origin: Vec2,
    tile_size: f32,
    map_width: u32,
    map_height: u32,
) -> (Vec<Rect>, Vec<MapSurfaceCollisionRect>) {
    let mut ground_collision_rects = Vec::new();
    let mut surface_collision_rects = Vec::new();

    for zone in zones {
        let rects = match zone.zone_type.as_str() {
            "CollisionArea" => collision_area_rects(zone, origin, tile_size, map_width, map_height),
            "CollisionLine" => collision_line_rects(zone, origin, tile_size, map_width, map_height),
            _ => Vec::new(),
        };
        if rects.is_empty() {
            continue;
        }

        if let Some(surface_id) = zone
            .collision
            .as_ref()
            .and_then(|collision| clean_optional_string(collision.surface_id.clone()))
        {
            surface_collision_rects.extend(rects.into_iter().map(|rect| MapSurfaceCollisionRect {
                surface_id: surface_id.clone(),
                rect,
            }));
        } else {
            ground_collision_rects.extend(rects);
        }
    }

    (ground_collision_rects, surface_collision_rects)
}

fn collision_area_rects(
    zone: &EditorZoneInstance,
    origin: Vec2,
    tile_size: f32,
    map_width: u32,
    map_height: u32,
) -> Vec<Rect> {
    if zone.points.len() < 3 {
        return Vec::new();
    }

    let points = zone_tile_points(zone);
    let bounds = bounds_for_points(&points);
    let min_x = bounds.origin.x.floor().max(0.0) as i32;
    let max_x = bounds.right().ceil().min(map_width as f32) as i32;
    let min_y = bounds.origin.y.floor().max(0.0) as i32;
    let max_y = bounds.bottom().ceil().min(map_height as f32) as i32;
    let mut rects = Vec::new();

    for y in min_y..max_y {
        let mut run_start = None::<i32>;
        for x in min_x..max_x {
            let center = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let solid = polygon_contains_point(&points, center);
            if solid && run_start.is_none() {
                run_start = Some(x);
            } else if !solid && run_start.is_some() {
                push_tile_collision_rect(
                    &mut rects,
                    origin,
                    tile_size,
                    run_start.take().unwrap(),
                    y,
                    x,
                    y + 1,
                );
            }
        }
        if let Some(start) = run_start {
            push_tile_collision_rect(&mut rects, origin, tile_size, start, y, max_x, y + 1);
        }
    }

    rects
}

fn collision_line_rects(
    zone: &EditorZoneInstance,
    origin: Vec2,
    tile_size: f32,
    map_width: u32,
    map_height: u32,
) -> Vec<Rect> {
    if zone.points.len() < 2 {
        return Vec::new();
    }

    let points = zone_tile_points(zone);
    let half_width = COLLISION_LINE_WIDTH_TILES * 0.5;
    let mut rects = Vec::new();
    for pair in points.windows(2) {
        let start = pair[0];
        let end = pair[1];
        let segment = Vec2::new(end.x - start.x, end.y - start.y);
        let length = (segment.x * segment.x + segment.y * segment.y).sqrt();
        if length <= f32::EPSILON {
            continue;
        }

        let steps = (length / COLLISION_LINE_SAMPLE_STEP_TILES).ceil().max(1.0) as i32;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let point = Vec2::new(start.x + segment.x * t, start.y + segment.y * t);
            let left = (point.x - half_width).clamp(0.0, map_width as f32);
            let top = (point.y - half_width).clamp(0.0, map_height as f32);
            let right = (point.x + half_width).clamp(0.0, map_width as f32);
            let bottom = (point.y + half_width).clamp(0.0, map_height as f32);
            if right > left && bottom > top {
                rects.push(tile_rect_to_world(
                    origin,
                    tile_size,
                    left,
                    top,
                    right - left,
                    bottom - top,
                ));
            }
        }
    }

    rects
}

fn zone_tile_points(zone: &EditorZoneInstance) -> Vec<Vec2> {
    zone.points
        .iter()
        .map(|point| Vec2::new(point[0], point[1]))
        .collect()
}

fn push_tile_collision_rect(
    rects: &mut Vec<Rect>,
    origin: Vec2,
    tile_size: f32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    if right <= left || bottom <= top {
        return;
    }
    rects.push(tile_rect_to_world(
        origin,
        tile_size,
        left as f32,
        top as f32,
        (right - left) as f32,
        (bottom - top) as f32,
    ));
}

fn tile_rect_to_world(origin: Vec2, tile_size: f32, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(
        Vec2::new(origin.x + x * tile_size, origin.y + y * tile_size),
        Vec2::new(w * tile_size, h * tile_size),
    )
}

fn fallback_object_depth_y(sprite_rect: Rect, tile_size: f32) -> f32 {
    let max_inset = (sprite_rect.size.y * 0.35).max(0.0);
    let inset = (tile_size * DEFAULT_OBJECT_DEPTH_INSET_TILES).min(max_inset);
    sprite_rect.bottom() - inset
}

fn bounds_for_points(points: &[Vec2]) -> Rect {
    if points.is_empty() {
        return Rect::new(Vec2::ZERO, Vec2::ZERO);
    }

    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);

    Rect::new(
        Vec2::new(min_x, min_y),
        Vec2::new(max_x - min_x, max_y - min_y),
    )
}

fn rect_contains_point_expanded(rect: Rect, point: Vec2, tolerance: f32) -> bool {
    point.x >= rect.origin.x - tolerance
        && point.x <= rect.right() + tolerance
        && point.y >= rect.origin.y - tolerance
        && point.y <= rect.bottom() + tolerance
}

fn rect_center(rect: Rect) -> Vec2 {
    Vec2::new(
        rect.origin.x + rect.size.x * 0.5,
        rect.origin.y + rect.size.y * 0.5,
    )
}

fn walk_surface_zone_contains_point(zone: &MapZone, point: Vec2) -> bool {
    rect_contains_point_expanded(zone.bounds, point, WALK_SURFACE_EDGE_TOLERANCE)
        && polygon_contains_point_with_tolerance(&zone.points, point, WALK_SURFACE_EDGE_TOLERANCE)
}

fn polygon_contains_point(points: &[Vec2], point: Vec2) -> bool {
    if points.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for current in points {
        let intersects_y = (current.y > point.y) != (previous.y > point.y);
        if intersects_y {
            let t = (point.y - current.y) / (previous.y - current.y);
            let crossing_x = current.x + t * (previous.x - current.x);
            if point.x < crossing_x {
                inside = !inside;
            }
        }
        previous = *current;
    }
    inside
}

fn polygon_contains_point_with_tolerance(points: &[Vec2], point: Vec2, tolerance: f32) -> bool {
    polygon_contains_point(points, point) || point_is_near_polygon_edge(points, point, tolerance)
}

fn point_is_near_polygon_edge(points: &[Vec2], point: Vec2, tolerance: f32) -> bool {
    if points.len() < 2 {
        return false;
    }

    let tolerance_squared = tolerance * tolerance;
    let mut previous = points[points.len() - 1];
    for current in points {
        if distance_squared_to_segment(point, previous, *current) <= tolerance_squared {
            return true;
        }
        previous = *current;
    }
    false
}

fn distance_squared_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let segment_length_squared = vec2_length_squared(segment);
    if segment_length_squared <= f32::EPSILON {
        return vec2_length_squared(point - start);
    }

    let t = (vec2_dot(point - start, segment) / segment_length_squared).clamp(0.0, 1.0);
    let closest = start + segment * t;
    vec2_length_squared(point - closest)
}

fn segments_intersect(
    first_start: Vec2,
    first_end: Vec2,
    second_start: Vec2,
    second_end: Vec2,
) -> bool {
    let first = first_end - first_start;
    let second = second_end - second_start;
    let denominator = vec2_cross(first, second);
    let offset = second_start - first_start;

    if denominator.abs() <= f32::EPSILON {
        let tolerance_squared = WALK_SURFACE_GATE_INTERSECTION_TOLERANCE_PIXELS
            * WALK_SURFACE_GATE_INTERSECTION_TOLERANCE_PIXELS;
        return distance_squared_to_segment(first_start, second_start, second_end)
            <= tolerance_squared
            || distance_squared_to_segment(first_end, second_start, second_end)
                <= tolerance_squared
            || distance_squared_to_segment(second_start, first_start, first_end)
                <= tolerance_squared
            || distance_squared_to_segment(second_end, first_start, first_end)
                <= tolerance_squared;
    }

    let t = vec2_cross(offset, second) / denominator;
    let u = vec2_cross(offset, first) / denominator;
    let epsilon = 0.001;
    (-epsilon..=1.0 + epsilon).contains(&t) && (-epsilon..=1.0 + epsilon).contains(&u)
}

fn vec2_cross(left: Vec2, right: Vec2) -> f32 {
    left.x * right.y - left.y * right.x
}

fn vec2_dot(left: Vec2, right: Vec2) -> f32 {
    left.x * right.x + left.y * right.y
}

fn vec2_length(vector: Vec2) -> f32 {
    vec2_length_squared(vector).sqrt()
}

fn vec2_length_squared(vector: Vec2) -> f32 {
    vector.x * vector.x + vector.y * vector.y
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
                    default_collision_rect: asset.default_collision_rect,
                    default_depth_rect: asset.default_depth_rect,
                    default_interaction_rect: asset.default_interaction_rect,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_is_depth_sorted_between_world_objects() {
        let map = Map {
            tiles: Vec::new(),
            sprites: vec![
                sprite("front", 50.0, 0, MapSpriteLayer::Object),
                sprite("decal", 100.0, 999, MapSpriteLayer::Decal),
                sprite("rear", 0.0, 0, MapSpriteLayer::Object),
            ],
            entities: Vec::new(),
            zones: Vec::new(),
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };
        let mut renderer = RecordingRenderer::default();

        map.draw_with_actor(&mut renderer, 30.0, |renderer| {
            renderer.draw_rect(Rect::new(Vec2::ZERO, Vec2::ZERO), Color::rgb(1.0, 0.0, 1.0));
        });

        assert_eq!(
            renderer.commands,
            ["decal", "rear", "actor", "front"],
            "decals stay below, while actor joins the object Y-depth pass"
        );
    }

    #[test]
    fn object_depth_line_delays_cover_until_actor_feet_enter_body() {
        let mut body = sprite("body", 0.0, 0, MapSpriteLayer::Object);
        body.rect = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(64.0, 100.0));
        body.depth_y = 60.0;

        let map = Map {
            tiles: Vec::new(),
            sprites: vec![body],
            entities: Vec::new(),
            zones: Vec::new(),
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };
        let mut renderer = RecordingRenderer::default();

        map.draw_with_actor(&mut renderer, 80.0, |renderer| {
            renderer.draw_rect(Rect::new(Vec2::ZERO, Vec2::ZERO), Color::rgb(1.0, 0.0, 1.0));
        });
        assert_eq!(
            renderer.commands,
            ["body", "actor"],
            "actor stays in front while its feet are below the object's body depth line"
        );

        renderer.commands.clear();
        map.draw_with_actor(&mut renderer, 50.0, |renderer| {
            renderer.draw_rect(Rect::new(Vec2::ZERO, Vec2::ZERO), Color::rgb(1.0, 0.0, 1.0));
        });
        assert_eq!(
            renderer.commands,
            ["actor", "body"],
            "object covers the actor only after the actor feet cross into its body depth line"
        );
    }

    #[test]
    fn fallback_object_depth_y_is_inset_from_visual_bottom() {
        let rect = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(64.0, 100.0));

        assert_eq!(fallback_object_depth_y(rect, 32.0), 84.0);
    }

    #[test]
    fn collision_area_zone_rasterizes_to_solid_rect_rows() {
        let zone = EditorZoneInstance {
            id: "mesa_wall".to_owned(),
            zone_type: "CollisionArea".to_owned(),
            points: vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        };

        let rects = collision_area_rects(&zone, Vec2::new(-64.0, -64.0), 32.0, 4, 4);

        assert_eq!(
            rects,
            vec![
                Rect::new(Vec2::new(-32.0, -32.0), Vec2::new(64.0, 32.0)),
                Rect::new(Vec2::new(-32.0, 0.0), Vec2::new(64.0, 32.0)),
            ]
        );
    }

    #[test]
    fn collision_line_zone_creates_thin_barrier_rects() {
        let zone = EditorZoneInstance {
            id: "cliff_edge".to_owned(),
            zone_type: "CollisionLine".to_owned(),
            points: vec![[1.0, 1.0], [3.0, 1.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        };

        let rects = collision_line_rects(&zone, Vec2::new(-64.0, -64.0), 32.0, 4, 4);

        assert!(rects.len() > 2);
        assert!(
            rects
                .iter()
                .all(|rect| rect.size.x <= 8.0 && rect.size.y <= 8.0),
            "collision line should create a thin sampled barrier"
        );
    }

    #[test]
    fn walk_surface_z_index_can_lift_actor_above_object_depth() {
        let map = Map {
            tiles: Vec::new(),
            sprites: vec![
                sprite("front", 50.0, 0, MapSpriteLayer::Object),
                sprite("rear", 0.0, 0, MapSpriteLayer::Object),
            ],
            entities: Vec::new(),
            zones: Vec::new(),
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };
        let mut renderer = RecordingRenderer::default();

        map.draw_with_actor_at_depth(&mut renderer, 30.0, 64, |renderer| {
            renderer.draw_rect(Rect::new(Vec2::ZERO, Vec2::ZERO), Color::rgb(1.0, 0.0, 1.0));
        });

        assert_eq!(
            renderer.commands,
            ["rear", "front", "actor"],
            "a walk surface can temporarily lift the actor above normal object Y-depth"
        );
    }

    #[test]
    fn walk_surface_z_index_beats_large_composite_object_depth() {
        let map = Map {
            tiles: Vec::new(),
            sprites: vec![
                sprite("large_platform_sprite", 260.0, 0, MapSpriteLayer::Object),
                sprite("platform_crystal", 120.0, 999, MapSpriteLayer::Object),
            ],
            entities: Vec::new(),
            zones: Vec::new(),
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };
        let mut renderer = RecordingRenderer::default();

        map.draw_with_actor_at_depth(&mut renderer, 170.0, 64, |renderer| {
            renderer.draw_rect(Rect::new(Vec2::ZERO, Vec2::ZERO), Color::rgb(1.0, 0.0, 1.0));
        });

        assert_eq!(
            renderer.commands,
            ["large_platform_sprite", "actor", "platform_crystal"],
            "surface actor layer should draw above the composite ramp/platform sprite but below high z props"
        );
    }

    #[test]
    fn walk_surface_at_uses_zone_polygon() {
        let map = Map {
            tiles: Vec::new(),
            sprites: Vec::new(),
            entities: Vec::new(),
            zones: vec![MapZone {
                id: "ramp_surface".to_owned(),
                zone_type: "WalkSurface".to_owned(),
                points: vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(10.0, 0.0),
                    Vec2::new(10.0, 10.0),
                    Vec2::new(0.0, 10.0),
                ],
                bounds: Rect::new(Vec2::ZERO, Vec2::new(10.0, 10.0)),
                hazard: None,
                prompt: None,
                objective: None,
                surface: Some(MapWalkSurface {
                    surface_id: "platform_01".to_owned(),
                    kind: MapWalkSurfaceKind::Platform,
                    constrain_movement: true,
                    z_index: 48,
                    depth_offset: -8.0,
                }),
                unlock: None,
                transition: None,
            }],
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };

        assert_eq!(
            map.walk_surface_at(Vec2::new(5.0, 5.0)),
            Some(MapWalkSurface {
                surface_id: "platform_01".to_owned(),
                kind: MapWalkSurfaceKind::Platform,
                constrain_movement: true,
                z_index: 48,
                depth_offset: -8.0
            })
        );
        assert_eq!(map.walk_surface_at(Vec2::new(16.0, 5.0)), None);
    }

    #[test]
    fn debug_geometry_draws_world_collision_interaction_and_player_rects() {
        let map = Map {
            tiles: Vec::new(),
            sprites: Vec::new(),
            entities: vec![MapEntity {
                id: "scan_target".to_owned(),
                kind: MapEntityKind::Decoration,
                rect: Rect::new(Vec2::new(20.0, 20.0), Vec2::new(24.0, 24.0)),
                collision_rect: Some(Rect::new(Vec2::new(22.0, 36.0), Vec2::new(20.0, 8.0))),
                sprite_rect: Rect::new(Vec2::new(12.0, 4.0), Vec2::new(40.0, 40.0)),
                depth_y: 44.0,
                color: Color::rgb(0.65, 0.35, 1.0),
                solid: false,
                z_index: 0,
                asset_id: None,
                codex_id: Some("codex.test.scan".to_owned()),
                unlock: None,
                transition: None,
                texture_id: None,
                source: None,
                flip_x: false,
                rotation: 0,
            }],
            zones: vec![MapZone {
                id: "exit_zone".to_owned(),
                zone_type: "MapTransition".to_owned(),
                points: vec![
                    Vec2::new(60.0, 60.0),
                    Vec2::new(90.0, 60.0),
                    Vec2::new(90.0, 90.0),
                    Vec2::new(60.0, 90.0),
                ],
                bounds: Rect::new(Vec2::new(60.0, 60.0), Vec2::new(30.0, 30.0)),
                hazard: None,
                prompt: None,
                objective: None,
                surface: None,
                unlock: None,
                transition: None,
            }],
            surface_gates: Vec::new(),
            collision_rects: vec![Rect::new(Vec2::new(0.0, 0.0), Vec2::new(16.0, 16.0))],
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };
        let mut renderer = RecordingRenderer::default();

        map.draw_debug_geometry(
            &mut renderer,
            Some("scan_target"),
            Some(Rect::new(Vec2::new(4.0, 4.0), Vec2::new(8.0, 8.0))),
        );

        assert!(
            renderer.commands.len() >= 20,
            "debug layer should draw visible collision, interaction, zone, and player rects"
        );
    }

    #[test]
    fn walk_surface_entry_requires_ramp_and_groups_by_surface_id() {
        let map = Map {
            tiles: Vec::new(),
            sprites: Vec::new(),
            entities: Vec::new(),
            zones: vec![
                surface_zone(
                    "platform_top",
                    "platform_01",
                    MapWalkSurfaceKind::Platform,
                    Vec2::new(0.0, 0.0),
                    Vec2::new(10.0, 10.0),
                ),
                surface_zone(
                    "platform_ramp",
                    "platform_01",
                    MapWalkSurfaceKind::Ramp,
                    Vec2::new(10.0, 2.0),
                    Vec2::new(6.0, 4.0),
                ),
            ],
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };

        assert_eq!(map.walk_surface_entry_at(Vec2::new(5.0, 5.0)), None);
        assert_eq!(
            map.walk_surface_entry_at(Vec2::new(15.0, 4.0))
                .map(|surface| surface.kind),
            Some(MapWalkSurfaceKind::Ramp)
        );
        assert!(map.walk_surface_contains("platform_01", Vec2::new(5.0, 5.0)));
        assert!(map.walk_surface_contains("platform_01", Vec2::new(15.0, 4.0)));
        assert_eq!(
            map.walk_surface_for_id_at("platform_01", Vec2::new(15.0, 4.0))
                .map(|surface| surface.kind),
            Some(MapWalkSurfaceKind::Ramp)
        );
        assert_eq!(
            map.walk_surface_for_id_at("platform_01", Vec2::new(5.0, 5.0))
                .map(|surface| surface.kind),
            Some(MapWalkSurfaceKind::Platform)
        );
        assert!(!map.walk_surface_contains("platform_02", Vec2::new(15.0, 4.0)));
    }

    #[test]
    fn overlapping_walk_surface_prefers_platform_over_ramp() {
        let map = Map {
            tiles: Vec::new(),
            sprites: Vec::new(),
            entities: Vec::new(),
            zones: vec![
                surface_zone(
                    "platform_ramp",
                    "platform_01",
                    MapWalkSurfaceKind::Ramp,
                    Vec2::new(0.0, 0.0),
                    Vec2::new(10.0, 10.0),
                ),
                surface_zone(
                    "platform_top",
                    "platform_01",
                    MapWalkSurfaceKind::Platform,
                    Vec2::new(0.0, 0.0),
                    Vec2::new(10.0, 10.0),
                ),
            ],
            surface_gates: Vec::new(),
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };

        assert_eq!(
            map.walk_surface_for_id_at("platform_01", Vec2::new(5.0, 5.0))
                .map(|surface| surface.kind),
            Some(MapWalkSurfaceKind::Platform)
        );
    }

    #[test]
    fn active_walk_surface_blocks_platform_edge_falloff() {
        let map = platform_with_front_ramp();

        assert!(
            !map.walk_surface_allows_movement(
                "platform_01",
                Vec2::new(50.0, 50.0),
                Vec2::new(116.0, 50.0)
            ),
            "platform movement should not drop to the ground through the platform side"
        );
    }

    #[test]
    fn ground_movement_can_pass_under_platform_without_entering_surface() {
        let map = platform_with_front_ramp();

        assert!(
            map.walk_surface_allows_ground_movement(Vec2::new(50.0, -20.0), Vec2::new(50.0, 50.0)),
            "ground movement behind the platform should stay on ground so the art can cover the actor"
        );
        assert!(
            !map.walk_surface_allows_ground_entry(Vec2::new(50.0, -20.0), Vec2::new(50.0, 50.0)),
            "ground movement into a platform zone must not activate the raised surface"
        );
    }

    #[test]
    fn ground_movement_enters_ramp_only_from_foot_direction() {
        let map = platform_with_front_ramp();

        assert!(
            map.walk_surface_allows_ground_movement(Vec2::new(50.0, 220.0), Vec2::new(50.0, 150.0)),
            "walking from the ramp foot toward the platform should enter the ramp"
        );
        assert!(
            map.walk_surface_allows_ground_entry(Vec2::new(50.0, 220.0), Vec2::new(50.0, 150.0)),
            "ground-to-ramp entry is the only case where ground collision zones can yield"
        );
        assert!(
            !map.walk_surface_allows_ground_entry(Vec2::new(-20.0, 150.0), Vec2::new(50.0, 150.0)),
            "walking into the ramp side should not switch to the raised surface"
        );
        assert!(
            !map.walk_surface_allows_ground_entry(Vec2::new(50.0, 220.0), Vec2::new(50.0, 220.0)),
            "plain ground movement should still keep zone collision active"
        );
        assert!(
            !map.walk_surface_allows_ground_entry(Vec2::new(-20.0, 220.0), Vec2::new(20.0, 150.0)),
            "diagonal movement from below the side should not count as a ramp-foot entry"
        );
        assert!(
            !map.walk_surface_allows_ground_entry(Vec2::new(106.0, 220.0), Vec2::new(106.0, 190.0)),
            "crossing an expanded gate endpoint must not activate the surface unless the feet land on the ramp"
        );
    }

    #[test]
    fn zone_collision_can_be_separated_from_object_collision() {
        let mut map = platform_with_front_ramp();
        let object_collision = Rect::new(Vec2::new(8.0, 8.0), Vec2::new(8.0, 8.0));
        let zone_collision = Rect::new(Vec2::new(40.0, 160.0), Vec2::new(20.0, 8.0));
        let surface_collision = Rect::new(Vec2::new(48.0, 48.0), Vec2::new(12.0, 12.0));
        map.collision_rects = vec![object_collision];
        map.zone_collision_rects = vec![zone_collision];
        map.surface_collision_rects = vec![MapSurfaceCollisionRect {
            surface_id: "platform_01".to_owned(),
            rect: surface_collision,
        }];

        assert_eq!(
            map.solid_rects_without_zone_collision().collect::<Vec<_>>(),
            vec![object_collision]
        );
        assert_eq!(
            map.zone_collision_rects().collect::<Vec<_>>(),
            vec![zone_collision]
        );
        assert_eq!(
            map.solid_rects().collect::<Vec<_>>(),
            vec![object_collision, zone_collision, surface_collision]
        );
        assert_eq!(
            map.surface_collision_rects("platform_01")
                .collect::<Vec<_>>(),
            vec![surface_collision]
        );
        assert_eq!(
            map.surface_collision_rects("platform_02")
                .collect::<Vec<_>>(),
            Vec::<Rect>::new()
        );
    }

    #[test]
    fn collision_zone_scope_routes_rects_to_surface_layer() {
        let zones = vec![
            EditorZoneInstance {
                id: "ground_wall".to_owned(),
                zone_type: "CollisionArea".to_owned(),
                points: vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]],
                hazard: None,
                prompt: None,
                objective: None,
                surface: None,
                gate: None,
                collision: None,
                unlock: None,
                transition: None,
            },
            EditorZoneInstance {
                id: "crystal_base".to_owned(),
                zone_type: "CollisionArea".to_owned(),
                points: vec![[4.0, 1.0], [6.0, 1.0], [6.0, 3.0], [4.0, 3.0]],
                hazard: None,
                prompt: None,
                objective: None,
                surface: None,
                gate: None,
                collision: Some(content::CollisionZoneRule {
                    surface_id: Some("platform_01".to_owned()),
                }),
                unlock: None,
                transition: None,
            },
        ];

        let (ground_rects, surface_rects) =
            collision_rects_from_zones(&zones, Vec2::ZERO, 32.0, 8, 8);

        assert_eq!(ground_rects.len(), 2);
        assert_eq!(surface_rects.len(), 2);
        assert!(
            surface_rects
                .iter()
                .all(|collision| collision.surface_id == "platform_01")
        );
    }

    #[test]
    fn active_walk_surface_allows_only_outward_ramp_exit() {
        let map = platform_with_front_ramp();

        assert!(
            map.walk_surface_contains("platform_01", Vec2::new(50.0, 202.0)),
            "surface edge tolerance keeps the actor on the ramp for a few pixels past the gate"
        );
        assert!(
            map.walk_surface_exits(
                "platform_01",
                Vec2::new(50.0, 198.0),
                Vec2::new(50.0, 203.0)
            ),
            "crossing the gate outward must switch back to ground before edge tolerance traps the actor"
        );
        assert!(
            !map.walk_surface_exits(
                "platform_01",
                Vec2::new(50.0, 198.0),
                Vec2::new(50.0, 199.5)
            ),
            "surface exit should require crossing the authored gate, not merely moving near it"
        );
        assert!(
            map.walk_surface_allows_movement(
                "platform_01",
                Vec2::new(50.0, 150.0),
                Vec2::new(50.0, 216.0)
            ),
            "walking down the ramp should return to ground level"
        );
        assert!(
            !map.walk_surface_allows_movement(
                "platform_01",
                Vec2::new(50.0, 112.0),
                Vec2::new(116.0, 84.0)
            ),
            "leaving from the ramp side or a platform seam should stay constrained to the surface"
        );
    }

    #[test]
    fn right_ramp_gate_requires_authored_segment_crossing() {
        let map = Map {
            tiles: Vec::new(),
            sprites: Vec::new(),
            entities: Vec::new(),
            zones: vec![
                surface_zone(
                    "platform_top",
                    "platform_01",
                    MapWalkSurfaceKind::Platform,
                    Vec2::new(192.0, 96.0),
                    Vec2::new(144.0, 80.0),
                ),
                surface_zone(
                    "platform_ramp_right",
                    "platform_01",
                    MapWalkSurfaceKind::Ramp,
                    Vec2::new(352.0, 160.0),
                    Vec2::new(64.0, 128.0),
                ),
            ],
            surface_gates: vec![surface_gate(
                "right_gate",
                "platform_01",
                Vec2::new(358.4, 284.8),
                Vec2::new(403.2, 259.2),
            )],
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        };

        assert!(
            !map.walk_surface_exits(
                "platform_01",
                Vec2::new(352.0, 260.0),
                Vec2::new(352.0, 300.0)
            ),
            "runtime should not expand SurfaceGate endpoints; the authored gate must cover the actual feet path"
        );
        assert!(
            map.walk_surface_exits(
                "platform_01",
                Vec2::new(360.0, 260.0),
                Vec2::new(360.0, 300.0)
            ),
            "a correctly covered feet path should still exit through the authored right gate segment"
        );
    }

    #[test]
    fn ground_cache_splits_tiles_across_visible_chunks() {
        let source = GroundTextureSource {
            path: PathBuf::from("dummy.png"),
            x: 31,
            y: 0,
            w: 2,
            h: 1,
            flip_x: false,
            rotation: 0,
        };
        let cache = build_ground_cache("map one", &[source], Vec2::ZERO, 65, 33, 32)
            .expect("ground cache should build")
            .expect("source should create chunks");

        assert_eq!(cache.chunks.len(), 2);
        assert!(
            cache
                .chunks
                .iter()
                .any(|chunk| chunk.texture_id == "__map_ground_cache_map_one_0_0")
        );
        let visible_chunks = cache
            .visible_chunks(Rect::new(Vec2::new(1024.0, 0.0), Vec2::new(1024.0, 1024.0)))
            .collect::<Vec<_>>();
        assert_eq!(visible_chunks.len(), 1);
        assert_eq!(
            visible_chunks[0].texture_id,
            "__map_ground_cache_map_one_1_0"
        );
    }

    #[test]
    fn texture_atlas_rewrites_sprite_sources() {
        let dir =
            std::env::temp_dir().join(format!("alien_archive_atlas_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("atlas temp dir should be created");
        let left_path = dir.join("left.png");
        let right_path = dir.join("right.png");
        RgbaImage::new(4, 4)
            .save(&left_path)
            .expect("left test image should save");
        RgbaImage::new(6, 5)
            .save(&right_path)
            .expect("right test image should save");

        let mut textures = HashMap::new();
        textures.insert("left".to_owned(), left_path);
        textures.insert("right".to_owned(), right_path);
        let mut sprites = vec![
            sprite("left", 0.0, 0, MapSpriteLayer::Object),
            sprite("right", 10.0, 0, MapSpriteLayer::Decal),
        ];
        let mut entities = Vec::new();

        let atlas = build_texture_atlas("test map", &textures, &mut sprites, &mut entities)
            .expect("atlas should build")
            .expect("multiple textures should create an atlas");

        assert_eq!(atlas.texture_id, "__map_texture_atlas_test_map");
        assert!(atlas.width >= 12);
        assert!(
            sprites
                .iter()
                .all(|sprite| sprite.texture_id == atlas.texture_id)
        );
        assert!(sprites.iter().all(|sprite| sprite.source.is_some()));

        let _ = std::fs::remove_dir_all(dir);
    }

    fn sprite(texture_id: &str, y: f32, z_index: i32, layer: MapSpriteLayer) -> MapSprite {
        MapSprite {
            texture_id: texture_id.to_owned(),
            source: None,
            rect: Rect::new(Vec2::new(0.0, y), Vec2::new(10.0, 10.0)),
            z_index,
            depth_y: y + 10.0,
            layer,
            flip_x: false,
            rotation: 0,
        }
    }

    fn surface_zone(
        id: &str,
        surface_id: &str,
        kind: MapWalkSurfaceKind,
        origin: Vec2,
        size: Vec2,
    ) -> MapZone {
        MapZone {
            id: id.to_owned(),
            zone_type: "WalkSurface".to_owned(),
            points: vec![
                origin,
                Vec2::new(origin.x + size.x, origin.y),
                Vec2::new(origin.x + size.x, origin.y + size.y),
                Vec2::new(origin.x, origin.y + size.y),
            ],
            bounds: Rect::new(origin, size),
            hazard: None,
            prompt: None,
            objective: None,
            surface: Some(MapWalkSurface {
                surface_id: surface_id.to_owned(),
                kind,
                constrain_movement: true,
                z_index: 64,
                depth_offset: 0.0,
            }),
            unlock: None,
            transition: None,
        }
    }

    fn surface_gate(id: &str, surface_id: &str, start: Vec2, end: Vec2) -> MapSurfaceGate {
        MapSurfaceGate {
            id: id.to_owned(),
            surface_id: surface_id.to_owned(),
            start,
            end,
        }
    }

    fn platform_with_front_ramp() -> Map {
        Map {
            tiles: Vec::new(),
            sprites: Vec::new(),
            entities: Vec::new(),
            zones: vec![
                surface_zone(
                    "platform_top",
                    "platform_01",
                    MapWalkSurfaceKind::Platform,
                    Vec2::new(0.0, 0.0),
                    Vec2::new(100.0, 100.0),
                ),
                surface_zone(
                    "platform_ramp",
                    "platform_01",
                    MapWalkSurfaceKind::Ramp,
                    Vec2::new(0.0, 100.0),
                    Vec2::new(100.0, 100.0),
                ),
            ],
            surface_gates: vec![surface_gate(
                "platform_ramp_gate",
                "platform_01",
                Vec2::new(0.0, 200.0),
                Vec2::new(100.0, 200.0),
            )],
            collision_rects: Vec::new(),
            zone_collision_rects: Vec::new(),
            surface_collision_rects: Vec::new(),
            ground_cache: None,
            textures: HashMap::new(),
            texture_atlas: None,
        }
    }

    #[derive(Default)]
    struct RecordingRenderer {
        commands: Vec<String>,
    }

    impl Renderer for RecordingRenderer {
        fn load_texture(&mut self, _id: &str, _path: &Path) -> Result<()> {
            Ok(())
        }

        fn load_texture_rgba(
            &mut self,
            _id: &str,
            _width: u32,
            _height: u32,
            _rgba: &[u8],
        ) -> Result<()> {
            Ok(())
        }

        fn texture_size(&self, _id: &str) -> Option<Vec2> {
            None
        }

        fn screen_size(&self) -> Vec2 {
            Vec2::ZERO
        }

        fn visible_world_rect(&self) -> Rect {
            Rect::new(Vec2::new(-1000.0, -1000.0), Vec2::new(2000.0, 2000.0))
        }

        fn set_camera(&mut self, _camera: runtime::Camera2d) {}

        fn draw_rect(&mut self, _rect: Rect, _color: Color) {
            self.commands.push("actor".to_owned());
        }

        fn draw_image(&mut self, texture_id: &str, _rect: Rect, _tint: Color) {
            self.commands.push(texture_id.to_owned());
        }

        fn draw_image_region(
            &mut self,
            texture_id: &str,
            _rect: Rect,
            _source: Rect,
            _tint: Color,
        ) {
            self.commands.push(texture_id.to_owned());
        }
    }
}
