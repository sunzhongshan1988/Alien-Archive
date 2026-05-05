use std::collections::HashSet;

use crate::{AssetDatabase, AssetKind, LayerKind, MapDocument};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct MapValidationIssue {
    pub severity: MapValidationSeverity,
    pub message: String,
}

impl MapValidationIssue {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: MapValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: MapValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

pub fn validate_map(document: &MapDocument, assets: &AssetDatabase) -> Vec<MapValidationIssue> {
    let mut issues = Vec::new();

    if document.id.trim().is_empty() {
        issues.push(MapValidationIssue::error("map id is empty"));
    }
    if document.tile_size == 0 {
        issues.push(MapValidationIssue::error(
            "tile_size must be greater than zero",
        ));
    }
    if document.width == 0 || document.height == 0 {
        issues.push(MapValidationIssue::error(
            "map width and height must be greater than zero",
        ));
    }
    if document.spawns.is_empty() {
        issues.push(MapValidationIssue::error("map has no spawn points"));
    }

    let mut ids = HashSet::new();
    for spawn in &document.spawns {
        validate_id("spawn", &spawn.id, &mut ids, &mut issues);
        validate_point("spawn", &spawn.id, spawn.x, spawn.y, document, &mut issues);
    }

    for tile in &document.layers.ground {
        validate_asset(
            "ground",
            &tile.asset,
            AssetKind::Tile,
            LayerKind::Ground,
            assets,
            &mut issues,
        );
        if tile.w <= 0 || tile.h <= 0 {
            issues.push(MapValidationIssue::error(format!(
                "ground {} at {},{} has invalid size {}x{}",
                tile.asset, tile.x, tile.y, tile.w, tile.h
            )));
        }
        if tile.x < 0
            || tile.y < 0
            || tile.x + tile.w.max(1) > document.width as i32
            || tile.y + tile.h.max(1) > document.height as i32
        {
            issues.push(MapValidationIssue::error(format!(
                "ground {} at {},{} is outside map bounds",
                tile.asset, tile.x, tile.y
            )));
        }
    }

    for decal in &document.layers.decals {
        validate_id("decal", &decal.id, &mut ids, &mut issues);
        validate_asset(
            "decal",
            &decal.asset,
            AssetKind::Decal,
            LayerKind::Decals,
            assets,
            &mut issues,
        );
        validate_scale(
            "decal",
            &decal.id,
            decal.scale_x,
            decal.scale_y,
            &mut issues,
        );
        validate_point("decal", &decal.id, decal.x, decal.y, document, &mut issues);
    }

    for object in &document.layers.objects {
        validate_id("object", &object.id, &mut ids, &mut issues);
        validate_asset(
            "object",
            &object.asset,
            AssetKind::Object,
            LayerKind::Objects,
            assets,
            &mut issues,
        );
        validate_scale(
            "object",
            &object.id,
            object.scale_x,
            object.scale_y,
            &mut issues,
        );
        validate_point(
            "object",
            &object.id,
            object.x,
            object.y,
            document,
            &mut issues,
        );
    }

    for entity in &document.layers.entities {
        validate_id("entity", &entity.id, &mut ids, &mut issues);
        validate_asset(
            "entity",
            &entity.asset,
            AssetKind::Entity,
            LayerKind::Entities,
            assets,
            &mut issues,
        );
        if entity.entity_type.trim().is_empty() {
            issues.push(MapValidationIssue::error(format!(
                "entity {} has empty entity_type",
                entity.id
            )));
        }
        validate_scale(
            "entity",
            &entity.id,
            entity.scale_x,
            entity.scale_y,
            &mut issues,
        );
        validate_point(
            "entity",
            &entity.id,
            entity.x,
            entity.y,
            document,
            &mut issues,
        );
    }

    for cell in &document.layers.collision {
        if cell.x < 0
            || cell.y < 0
            || cell.x >= document.width as i32
            || cell.y >= document.height as i32
        {
            issues.push(MapValidationIssue::error(format!(
                "collision cell {},{} is outside map bounds",
                cell.x, cell.y
            )));
        }
    }

    for zone in &document.layers.zones {
        validate_id("zone", &zone.id, &mut ids, &mut issues);
        if zone.points.len() < 3 {
            issues.push(MapValidationIssue::warning(format!(
                "zone {} has fewer than three points",
                zone.id
            )));
        }
    }

    issues
}

fn validate_id(
    kind: &str,
    id: &str,
    ids: &mut HashSet<String>,
    issues: &mut Vec<MapValidationIssue>,
) {
    if id.trim().is_empty() {
        issues.push(MapValidationIssue::error(format!("{kind} id is empty")));
    } else if !ids.insert(id.to_owned()) {
        issues.push(MapValidationIssue::error(format!("duplicate id {id}")));
    }
}

fn validate_asset(
    owner: &str,
    asset_id: &str,
    expected_kind: AssetKind,
    expected_layer: LayerKind,
    assets: &AssetDatabase,
    issues: &mut Vec<MapValidationIssue>,
) {
    let Some(asset) = assets.get(asset_id) else {
        issues.push(MapValidationIssue::error(format!(
            "{owner} references unknown asset {asset_id}"
        )));
        return;
    };

    if asset.kind != expected_kind {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} asset {asset_id} is {:?}, expected {:?}",
            asset.kind, expected_kind
        )));
    }
    if asset.default_layer != expected_layer {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} asset {asset_id} defaults to {:?}, placed in {:?}",
            asset.default_layer, expected_layer
        )));
    }
}

fn validate_point(
    kind: &str,
    id: &str,
    x: f32,
    y: f32,
    document: &MapDocument,
    issues: &mut Vec<MapValidationIssue>,
) {
    if x < 0.0 || y < 0.0 || x >= document.width as f32 || y >= document.height as f32 {
        issues.push(MapValidationIssue::error(format!(
            "{kind} {id} at {x:.2},{y:.2} is outside map bounds"
        )));
    }
}

fn validate_scale(
    kind: &str,
    id: &str,
    scale_x: f32,
    scale_y: f32,
    issues: &mut Vec<MapValidationIssue>,
) {
    if scale_x <= 0.0 || scale_y <= 0.0 {
        issues.push(MapValidationIssue::error(format!(
            "{kind} {id} has invalid scale {scale_x:.2} x {scale_y:.2}"
        )));
    }
}

#[cfg(test)]
mod tests {
    use crate::{AnchorKind, AssetDatabase, AssetDefinition, LayerKind, MapDocument, SnapMode};

    use super::*;

    #[test]
    fn reports_unknown_asset() {
        let document = MapDocument::new_landing_site();
        let database = AssetDatabase {
            mode: "Overworld".to_owned(),
            assets: Vec::new(),
            by_id: Default::default(),
        };

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| issue.severity != MapValidationSeverity::Error)
        );
    }

    #[test]
    fn validates_known_ground_asset() {
        let mut document = MapDocument::new_landing_site();
        document.place_tile("ow_tile_sand_ground", 0, 0);
        let mut database = AssetDatabase {
            mode: "Overworld".to_owned(),
            assets: vec![AssetDefinition {
                id: "ow_tile_sand_ground".to_owned(),
                category: "tiles".to_owned(),
                path: "assets/sprites/tiles/overworld/ow_tile_sand_ground.png".into(),
                kind: AssetKind::Tile,
                default_layer: LayerKind::Ground,
                default_size: [32.0, 32.0],
                anchor: AnchorKind::TopLeft,
                snap: SnapMode::Grid,
                tags: Vec::new(),
                entity_type: None,
                codex_id: None,
            }],
            by_id: Default::default(),
        };
        database.reindex();

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| issue.severity != MapValidationSeverity::Error)
        );
    }
}
