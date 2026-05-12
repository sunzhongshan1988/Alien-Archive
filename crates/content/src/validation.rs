use std::collections::HashSet;

use crate::{
    AssetDatabase, AssetKind, CodexDatabase, LayerKind, MapDocument, UnlockRule, semantics,
};

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
    validate_map_with_codex(document, assets, None)
}

pub fn validate_map_with_codex(
    document: &MapDocument,
    assets: &AssetDatabase,
    codex: Option<&CodexDatabase>,
) -> Vec<MapValidationIssue> {
    let mut issues = Vec::new();
    let solid_collision_bounds = document
        .layers
        .collision
        .iter()
        .filter(|cell| cell.solid)
        .map(|cell| cell.bounds())
        .collect::<Vec<_>>();

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
        if let Some(bounds) = solid_collision_bounds
            .iter()
            .find(|bounds| bounds.contains_point(spawn.x, spawn.y))
        {
            issues.push(MapValidationIssue::error(format!(
                "spawn {} overlaps solid collision {:.2},{:.2} {:.2}x{:.2}",
                spawn.id, bounds.x, bounds.y, bounds.w, bounds.h
            )));
        }
    }

    for tile in &document.layers.ground {
        let asset = assets.get(&tile.asset);
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
        if let Some(asset) = asset {
            if let Some([width, height]) = asset
                .footprint
                .or_else(|| infer_tile_footprint(asset.default_size, document.tile_size))
            {
                if tile.w != width || tile.h != height {
                    issues.push(MapValidationIssue::warning(format!(
                        "ground {} at {},{} is {}x{}, asset footprint is {}x{}",
                        tile.asset, tile.x, tile.y, tile.w, tile.h, width, height
                    )));
                }
            }
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
        let asset = assets.get(&object.asset);
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
        if asset.and_then(|asset| asset.codex_id.as_deref()).is_some() {
            issues.push(MapValidationIssue::warning(format!(
                "object {} uses codex asset {}, but the current runtime only scans entities",
                object.id, object.asset
            )));
        }
        if let Some(codex_id) = asset.and_then(|asset| asset.codex_id.as_deref()) {
            validate_codex_reference(codex_id, codex, &mut issues);
        }
    }

    let mut seen_codex_ids = HashSet::new();
    for entity in &document.layers.entities {
        let asset = assets.get(&entity.asset);
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
        if let Some(codex_id) = asset.and_then(|asset| asset.codex_id.as_deref()) {
            validate_codex_reference(codex_id, codex, &mut issues);
            if !seen_codex_ids.insert(codex_id.to_owned()) {
                issues.push(MapValidationIssue::warning(format!(
                    "codex_id {codex_id} appears on multiple entities in this map"
                )));
            }
            if entity.interaction_rect.is_none()
                && asset
                    .and_then(|asset| asset.default_interaction_rect)
                    .is_none()
            {
                issues.push(MapValidationIssue::warning(format!(
                    "entity {} uses codex_id {codex_id} but has no interaction_rect; runtime will use a 1x1 default scan area",
                    entity.id
                )));
            }
        }
        if let Some(unlock) = &entity.unlock {
            validate_unlock_rule("entity", &entity.id, unlock, codex, &mut issues);
        } else if entity_uses_implicit_legacy_unlock(&entity.entity_type)
            && asset.and_then(|asset| asset.codex_id.as_deref()).is_some()
        {
            issues.push(MapValidationIssue::warning(format!(
                "entity {} relies on asset codex_id for legacy door unlock; set unlock.requires_codex_id explicitly",
                entity.id
            )));
        }
        if let Some(transition) = &entity.transition {
            validate_transition_target("entity", &entity.id, transition, &mut issues);
        }
    }

    for cell in &document.layers.collision {
        let bounds = cell.bounds();
        if bounds.x < 0.0
            || bounds.y < 0.0
            || bounds.right() > document.width as f32
            || bounds.bottom() > document.height as f32
        {
            issues.push(MapValidationIssue::error(format!(
                "collision {:.2},{:.2} {:.2}x{:.2} is outside map bounds",
                bounds.x, bounds.y, bounds.w, bounds.h
            )));
        }
    }

    for zone in &document.layers.zones {
        validate_id("zone", &zone.id, &mut ids, &mut issues);
        if zone.zone_type.trim().is_empty() {
            issues.push(MapValidationIssue::error(format!(
                "zone {} has empty zone_type",
                zone.id
            )));
        } else if !semantics::is_known_zone_type(&zone.zone_type) {
            issues.push(MapValidationIssue::warning(format!(
                "zone {} uses unknown zone_type {}",
                zone.id, zone.zone_type
            )));
        }
        let min_zone_points = if semantics::zone_type_is_line_like(&zone.zone_type) {
            2
        } else {
            3
        };
        if zone.points.len() < min_zone_points {
            issues.push(MapValidationIssue::warning(format!(
                "zone {} has fewer than {} points",
                zone.id, min_zone_points
            )));
        }
        if zone.zone_type == semantics::ZONE_WALK_SURFACE && zone.surface.is_none() {
            issues.push(MapValidationIssue::warning(format!(
                "zone {} is WalkSurface but has no surface settings",
                zone.id
            )));
        } else if let Some(surface) = &zone.surface {
            let surface_id = surface
                .surface_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if !semantics::zone_type_allows_surface(&zone.zone_type) {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} has walk surface data but zone_type is {}",
                    zone.id, zone.zone_type
                )));
            }
            if surface.kind == crate::WalkSurfaceKind::Ramp && surface_id.is_none() {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} is a WalkSurface ramp but has no surface_id; it will not connect to a platform",
                    zone.id
                )));
            }
        }
        if zone.zone_type == semantics::ZONE_SURFACE_GATE {
            if let Some(gate) = &zone.gate {
                if gate
                    .surface_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    issues.push(MapValidationIssue::warning(format!(
                        "zone {} is SurfaceGate but has no surface_id",
                        zone.id
                    )));
                }
            } else {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} is SurfaceGate but has no gate settings",
                    zone.id
                )));
            }
        } else if zone.gate.is_some() {
            issues.push(MapValidationIssue::warning(format!(
                "zone {} has surface gate data but zone_type is {}",
                zone.id, zone.zone_type
            )));
        }
        if let Some(collision) = &zone.collision {
            if !semantics::zone_type_is_collision_scope(&zone.zone_type) {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} has collision scope data but zone_type is {}",
                    zone.id, zone.zone_type
                )));
            }
            if collision
                .surface_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} has collision scope data but no surface_id",
                    zone.id
                )));
            }
        }
        if zone.zone_type == semantics::ZONE_HAZARD {
            if let Some(hazard) = &zone.hazard {
                validate_hazard_rule("zone", &zone.id, hazard, &mut issues);
            } else {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} is HazardZone but has no hazard effects",
                    zone.id
                )));
            }
        } else if let Some(hazard) = &zone.hazard {
            validate_hazard_rule("zone", &zone.id, hazard, &mut issues);
            issues.push(MapValidationIssue::warning(format!(
                "zone {} has hazard data but zone_type is {}",
                zone.id, zone.zone_type
            )));
        }
        if zone.zone_type == semantics::ZONE_PROMPT {
            if let Some(prompt) = &zone.prompt {
                validate_prompt_rule("zone", &zone.id, prompt, &mut issues);
            } else {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} is PromptZone but has no prompt message or log detail",
                    zone.id
                )));
            }
        } else if let Some(prompt) = &zone.prompt {
            validate_prompt_rule("zone", &zone.id, prompt, &mut issues);
            issues.push(MapValidationIssue::warning(format!(
                "zone {} has prompt data but zone_type is {}",
                zone.id, zone.zone_type
            )));
        }
        if semantics::zone_type_is_objective_like(&zone.zone_type) {
            if let Some(objective) = &zone.objective {
                validate_objective_rule("zone", &zone.id, &zone.zone_type, objective, &mut issues);
            } else {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} is {} but has no objective settings",
                    zone.id, zone.zone_type
                )));
            }
        } else if let Some(objective) = &zone.objective {
            validate_objective_rule("zone", &zone.id, &zone.zone_type, objective, &mut issues);
            issues.push(MapValidationIssue::warning(format!(
                "zone {} has objective data but zone_type is {}",
                zone.id, zone.zone_type
            )));
        }
        if let Some(unlock) = &zone.unlock {
            validate_unlock_rule("zone", &zone.id, unlock, codex, &mut issues);
        }
        if zone.zone_type == semantics::ZONE_MAP_TRANSITION {
            if let Some(transition) = &zone.transition {
                validate_transition_target("zone", &zone.id, transition, &mut issues);
            } else {
                issues.push(MapValidationIssue::warning(format!(
                    "zone {} is MapTransition but has no transition target",
                    zone.id
                )));
            }
        } else if let Some(transition) = &zone.transition {
            validate_transition_target("zone", &zone.id, transition, &mut issues);
        }
    }

    issues
}

fn validate_hazard_rule(
    owner: &str,
    id: &str,
    hazard: &crate::HazardRule,
    issues: &mut Vec<MapValidationIssue>,
) {
    if hazard.effects.is_empty() {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} has hazard data but no effects"
        )));
    }

    for effect in &hazard.effects {
        let meter = effect.meter.trim();
        if meter.is_empty() {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} has hazard effect with empty meter"
            )));
        } else if !semantics::is_known_hazard_meter(meter) {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} hazard effect meter {meter} is not a known runtime meter"
            )));
        }
        if effect.rate_per_second.abs() <= f32::EPSILON {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} hazard effect {meter} has zero rate_per_second"
            )));
        }
    }

    if hazard
        .message
        .as_deref()
        .is_some_and(|message| message.trim().is_empty())
    {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} hazard message is empty"
        )));
    }
}

fn validate_prompt_rule(
    owner: &str,
    id: &str,
    prompt: &crate::PromptRule,
    issues: &mut Vec<MapValidationIssue>,
) {
    if prompt.is_empty() {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} has prompt data but no message, log_title, or log_detail"
        )));
    }
}

fn validate_objective_rule(
    owner: &str,
    id: &str,
    zone_type: &str,
    objective: &crate::ObjectiveRule,
    issues: &mut Vec<MapValidationIssue>,
) {
    if objective.objective_id.trim().is_empty() {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} objective rule has no objective_id"
        )));
    }
    if zone_type == semantics::ZONE_CHECKPOINT
        && objective
            .checkpoint_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        && !objective.complete_objective
    {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} is Checkpoint but has no checkpoint_id or complete_objective flag"
        )));
    }
}

fn validate_unlock_rule(
    owner: &str,
    id: &str,
    unlock: &UnlockRule,
    codex: Option<&CodexDatabase>,
    issues: &mut Vec<MapValidationIssue>,
) {
    let codex_id = unlock
        .requires_codex_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let item_id = unlock
        .requires_item_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if codex_id.is_none() && item_id.is_none() {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} has unlock data but no requires_codex_id or requires_item_id"
        )));
    }

    if let Some(codex_id) = codex_id {
        validate_codex_reference(codex_id, codex, issues);
    }

    if let Some(item_id) = item_id {
        if item_id.chars().any(char::is_whitespace) {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} unlock requires_item_id {item_id} contains whitespace"
            )));
        }
    }

    if unlock
        .locked_message
        .as_deref()
        .is_some_and(|message| message.trim().is_empty())
    {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} unlock locked_message is empty"
        )));
    }
}

fn validate_transition_target(
    owner: &str,
    id: &str,
    transition: &crate::TransitionTarget,
    issues: &mut Vec<MapValidationIssue>,
) {
    let scene = transition
        .scene
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let map_path = transition
        .map_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let spawn_id = transition
        .spawn_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if scene.is_none() && map_path.is_none() && spawn_id.is_none() {
        issues.push(MapValidationIssue::warning(format!(
            "{owner} {id} has transition data but no scene, map_path, or spawn_id"
        )));
    }

    if let Some(scene) = scene {
        if !semantics::is_known_runtime_scene(scene) {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} transition scene {scene} is not a known runtime scene"
            )));
        }
    }

    if let Some(map_path) = map_path {
        if !map_path.ends_with(".ron") {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} transition map_path {map_path} does not point to a RON map"
            )));
        }
    }

    if let Some(spawn_id) = spawn_id {
        if spawn_id.chars().any(char::is_whitespace) {
            issues.push(MapValidationIssue::warning(format!(
                "{owner} {id} transition spawn_id {spawn_id} contains whitespace"
            )));
        }
    }
}

fn entity_uses_implicit_legacy_unlock(entity_type: &str) -> bool {
    semantics::entity_type_uses_implicit_legacy_unlock(entity_type)
}

fn validate_codex_reference(
    codex_id: &str,
    codex: Option<&CodexDatabase>,
    issues: &mut Vec<MapValidationIssue>,
) {
    let Some(codex) = codex else {
        return;
    };
    let Some(entry) = codex.get(codex_id) else {
        issues.push(MapValidationIssue::warning(format!(
            "codex_id {codex_id} is not defined in codex database"
        )));
        return;
    };

    if entry.title.trim().is_empty() {
        issues.push(MapValidationIssue::warning(format!(
            "codex entry {codex_id} has empty title"
        )));
    }
    if entry.category.trim().is_empty() {
        issues.push(MapValidationIssue::warning(format!(
            "codex entry {codex_id} has empty category"
        )));
    }
    if entry.description.trim().is_empty() {
        issues.push(MapValidationIssue::warning(format!(
            "codex entry {codex_id} has empty description"
        )));
    }
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

fn infer_tile_footprint(default_size: [f32; 2], tile_size: u32) -> Option<[i32; 2]> {
    let tile_size = tile_size.max(1) as f32;
    let width_units = default_size[0] / tile_size;
    let height_units = default_size[1] / tile_size;
    let width = width_units.round();
    let height = height_units.round();
    ((width_units - width).abs() < 0.01 && (height_units - height).abs() < 0.01)
        .then_some([width.max(1.0) as i32, height.max(1.0) as i32])
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
    use crate::{
        AnchorKind, AssetDatabase, AssetDefinition, AssetKind, CodexDatabase, CodexEntry,
        CollisionCell, CollisionZoneRule, HazardEffect, HazardRule, LayerKind, MapDocument,
        ObjectiveRule, PromptRule, SnapMode, SurfaceGateRule, UnlockRule, WalkSurfaceKind,
        WalkSurfaceRule, ZoneInstance,
    };

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
        let database = test_database(vec![test_asset(
            "ow_tile_sand_ground",
            AssetKind::Tile,
            LayerKind::Ground,
            None,
            None,
        )]);

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| issue.severity != MapValidationSeverity::Error)
        );
    }

    #[test]
    fn warns_when_scannable_entity_has_no_interaction_rect() {
        let mut document = MapDocument::new_landing_site();
        document.place_entity("ow_scan_terminal", "ScanTarget", 2.0, 2.0);
        let database = test_database(vec![test_asset(
            "ow_scan_terminal",
            AssetKind::Entity,
            LayerKind::Entities,
            Some("ScanTarget"),
            Some("codex.ruin.terminal"),
        )]);

        let issues = validate_map(&document, &database);

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("has no interaction_rect")
        }));
    }

    #[test]
    fn warns_when_codex_asset_is_placed_as_object() {
        let mut document = MapDocument::new_landing_site();
        document.place_object("ow_flora_glowfungus", 3.0, 4.0);
        let database = test_database(vec![test_asset(
            "ow_flora_glowfungus",
            AssetKind::Object,
            LayerKind::Objects,
            None,
            Some("codex.flora.glowfungus"),
        )]);

        let issues = validate_map(&document, &database);

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue
                    .message
                    .contains("current runtime only scans entities")
        }));
    }

    #[test]
    fn errors_when_spawn_overlaps_solid_collision() {
        let mut document = MapDocument::new_landing_site();
        document.layers.collision.push(CollisionCell {
            x: 8,
            y: 12,
            offset: [0.0, 0.0],
            size: [1.0, 1.0],
            solid: true,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Error
                && issue.message.contains("overlaps solid collision")
        }));
    }

    #[test]
    fn warns_when_codex_reference_is_missing_from_database() {
        let mut document = MapDocument::new_landing_site();
        document.place_entity("ow_scan_terminal", "ScanTarget", 2.0, 2.0);
        let database = test_database(vec![test_asset(
            "ow_scan_terminal",
            AssetKind::Entity,
            LayerKind::Entities,
            Some("ScanTarget"),
            Some("codex.ruin.terminal"),
        )]);
        let codex = test_codex(Vec::new());

        let issues = validate_map_with_codex(&document, &database, Some(&codex));

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("not defined in codex database")
        }));
    }

    #[test]
    fn warns_when_codex_entry_has_empty_content_fields() {
        let mut document = MapDocument::new_landing_site();
        document.place_entity("ow_scan_terminal", "ScanTarget", 2.0, 2.0);
        let database = test_database(vec![test_asset(
            "ow_scan_terminal",
            AssetKind::Entity,
            LayerKind::Entities,
            Some("ScanTarget"),
            Some("codex.ruin.terminal"),
        )]);
        let codex = test_codex(vec![CodexEntry {
            id: "codex.ruin.terminal".to_owned(),
            category: String::new(),
            title: String::new(),
            description: String::new(),
            scan_time: None,
            unlock_tags: Vec::new(),
            image: None,
        }]);

        let issues = validate_map_with_codex(&document, &database, Some(&codex));

        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("empty title"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("empty category"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("empty description"))
        );
    }

    #[test]
    fn validates_explicit_unlock_codex_reference() {
        let mut document = MapDocument::new_landing_site();
        document.place_entity("ow_ruin_door", "Door", 2.0, 2.0);
        document.layers.entities[0].unlock = Some(UnlockRule {
            requires_codex_id: Some("codex.ruin.door".to_owned()),
            ..UnlockRule::default()
        });
        let database = test_database(vec![test_asset(
            "ow_ruin_door",
            AssetKind::Entity,
            LayerKind::Entities,
            Some("Door"),
            None,
        )]);
        let codex = test_codex(Vec::new());

        let issues = validate_map_with_codex(&document, &database, Some(&codex));

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("not defined in codex database")
        }));
    }

    #[test]
    fn warns_when_unlock_has_no_requirement() {
        let mut document = MapDocument::new_landing_site();
        document.place_entity("ow_ruin_door", "Door", 2.0, 2.0);
        document.layers.entities[0].unlock = Some(UnlockRule {
            locked_message: Some("Locked".to_owned()),
            ..UnlockRule::default()
        });
        let database = test_database(vec![test_asset(
            "ow_ruin_door",
            AssetKind::Entity,
            LayerKind::Entities,
            Some("Door"),
            None,
        )]);

        let issues = validate_map(&document, &database);

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("unlock data but no requires")
        }));
    }

    #[test]
    fn warns_when_transition_target_has_bad_fields() {
        let mut document = MapDocument::new_landing_site();
        document.place_entity("ow_ruin_door", "Door", 2.0, 2.0);
        document.layers.entities[0].transition = Some(crate::TransitionTarget {
            scene: Some("UnknownScene".to_owned()),
            map_path: Some("assets/data/maps/facility.txt".to_owned()),
            spawn_id: Some("bad spawn".to_owned()),
        });
        let database = test_database(vec![test_asset(
            "ow_ruin_door",
            AssetKind::Entity,
            LayerKind::Entities,
            Some("Door"),
            None,
        )]);

        let issues = validate_map(&document, &database);

        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("not a known runtime scene")
        }));
        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("does not point to a RON map")
        }));
        assert!(issues.iter().any(|issue| {
            issue.severity == MapValidationSeverity::Warning
                && issue.message.contains("spawn_id")
                && issue.message.contains("contains whitespace")
        }));
    }

    #[test]
    fn validates_walk_surface_zone_settings() {
        let mut document = MapDocument::new_landing_site();
        document.layers.zones.push(ZoneInstance {
            id: "ramp_surface".to_owned(),
            zone_type: "WalkSurface".to_owned(),
            points: vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: Some(WalkSurfaceRule {
                surface_id: Some("platform_01".to_owned()),
                kind: WalkSurfaceKind::Platform,
                constrain_movement: true,
                z_index: 48,
                depth_offset: -8.0,
            }),
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("unknown zone_type"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("WalkSurface but has no surface"))
        );
    }

    #[test]
    fn validates_collision_line_zone_with_two_points() {
        let mut document = MapDocument::new_landing_site();
        document.layers.zones.push(ZoneInstance {
            id: "cliff_edge".to_owned(),
            zone_type: "CollisionLine".to_owned(),
            points: vec![[1.0, 1.0], [4.0, 1.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("unknown zone_type"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("fewer than"))
        );
    }

    #[test]
    fn validates_surface_scoped_collision_zone() {
        let mut document = MapDocument::new_landing_site();
        document.layers.zones.push(ZoneInstance {
            id: "crystal_base_collision".to_owned(),
            zone_type: "CollisionArea".to_owned(),
            points: vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            gate: None,
            collision: Some(CollisionZoneRule {
                surface_id: Some("platform_01".to_owned()),
            }),
            unlock: None,
            transition: None,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("collision scope data"))
        );
    }

    #[test]
    fn validates_surface_gate_zone_with_two_points() {
        let mut document = MapDocument::new_landing_site();
        document.layers.zones.push(ZoneInstance {
            id: "platform_gate".to_owned(),
            zone_type: "SurfaceGate".to_owned(),
            points: vec![[1.0, 1.0], [4.0, 1.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            gate: Some(SurfaceGateRule {
                surface_id: Some("platform_01".to_owned()),
            }),
            collision: None,
            unlock: None,
            transition: None,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("unknown zone_type"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("fewer than"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("SurfaceGate but has no"))
        );
    }

    #[test]
    fn validates_hazard_and_prompt_zone_rules() {
        let mut document = MapDocument::new_landing_site();
        document.layers.zones.push(ZoneInstance {
            id: "toxic_pocket".to_owned(),
            zone_type: "HazardZone".to_owned(),
            points: vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]],
            hazard: Some(HazardRule {
                effects: vec![HazardEffect::new("oxygen", -4.0)],
                message: Some("空气质量异常".to_owned()),
            }),
            prompt: None,
            objective: None,
            surface: None,
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        });
        document.layers.zones.push(ZoneInstance {
            id: "first_view".to_owned(),
            zone_type: "PromptZone".to_owned(),
            points: vec![[4.0, 1.0], [6.0, 1.0], [6.0, 3.0], [4.0, 3.0]],
            hazard: None,
            prompt: Some(PromptRule {
                message: Some("前方发现遗迹轮廓".to_owned()),
                ..PromptRule::default()
            }),
            objective: None,
            surface: None,
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("unknown zone_type"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("no hazard effects"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("no prompt message"))
        );
    }

    #[test]
    fn validates_objective_and_checkpoint_zone_rules() {
        let mut document = MapDocument::new_landing_site();
        document.layers.zones.push(ZoneInstance {
            id: "objective_start".to_owned(),
            zone_type: "ObjectiveZone".to_owned(),
            points: vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]],
            hazard: None,
            prompt: None,
            objective: Some(ObjectiveRule {
                objective_id: "secure_landing_site".to_owned(),
                ..ObjectiveRule::default()
            }),
            surface: None,
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        });
        document.layers.zones.push(ZoneInstance {
            id: "objective_checkpoint".to_owned(),
            zone_type: "Checkpoint".to_owned(),
            points: vec![[4.0, 1.0], [6.0, 1.0], [6.0, 3.0], [4.0, 3.0]],
            hazard: None,
            prompt: None,
            objective: Some(ObjectiveRule {
                objective_id: "secure_landing_site".to_owned(),
                checkpoint_id: Some("landing_perimeter".to_owned()),
                ..ObjectiveRule::default()
            }),
            surface: Some(WalkSurfaceRule {
                surface_id: Some("platform_01".to_owned()),
                kind: WalkSurfaceKind::Platform,
                constrain_movement: true,
                z_index: 64,
                depth_offset: 0.0,
            }),
            gate: None,
            collision: None,
            unlock: None,
            transition: None,
        });
        let database = test_database(Vec::new());

        let issues = validate_map(&document, &database);

        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("unknown zone_type"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("objective settings"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("no checkpoint_id"))
        );
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("walk surface data"))
        );
    }

    fn test_database(assets: Vec<AssetDefinition>) -> AssetDatabase {
        let mut database = AssetDatabase {
            mode: "Overworld".to_owned(),
            assets,
            by_id: Default::default(),
        };
        database.reindex();
        database
    }

    fn test_codex(entries: Vec<CodexEntry>) -> CodexDatabase {
        let mut database = CodexDatabase {
            mode: "Overworld".to_owned(),
            entries,
            ..CodexDatabase::new("Overworld")
        };
        database.reindex();
        database
    }

    fn test_asset(
        id: &str,
        kind: AssetKind,
        default_layer: LayerKind,
        entity_type: Option<&str>,
        codex_id: Option<&str>,
    ) -> AssetDefinition {
        AssetDefinition {
            id: id.to_owned(),
            category: "test".to_owned(),
            path: format!("assets/sprites/test/{id}.png").into(),
            kind,
            default_layer,
            default_size: [32.0, 32.0],
            footprint: (kind == AssetKind::Tile).then_some([1, 1]),
            default_collision_rect: None,
            default_depth_rect: None,
            default_interaction_rect: None,
            anchor: AnchorKind::TopLeft,
            snap: SnapMode::Grid,
            tags: Vec::new(),
            entity_type: entity_type.map(str::to_owned),
            codex_id: codex_id.map(str::to_owned),
        }
    }
}
