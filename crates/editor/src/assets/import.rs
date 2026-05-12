use std::{
    fs,
    path::{Path, PathBuf},
};

use content::{AnchorKind, AssetKind, LayerKind, SnapMode};

use crate::assets::draft::AssetDraft;
use crate::util::sanitize::{sanitize_asset_id, sanitize_category};

pub(crate) fn infer_asset_draft_from_path(project_root: &Path, relative_path: &str) -> AssetDraft {
    let normalized = relative_path.trim().replace('\\', "/");
    let file_stem = Path::new(&normalized)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("asset");
    let id = sanitize_asset_id(file_stem).unwrap_or_else(|| "asset".to_owned());
    let category = normalized
        .split('/')
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|pair| (pair[1] == "overworld").then(|| pair[0].to_owned()))
        .unwrap_or_else(|| "props".to_owned());
    let kind = infer_asset_kind(&category, &id);
    let mut draft = AssetDraft {
        id,
        category: sanitize_category(&category).unwrap_or_else(|| "props".to_owned()),
        path: normalized.clone(),
        kind,
        default_layer: LayerKind::Objects,
        default_size: image_dimensions(&project_root.join(&normalized)).unwrap_or([72.0, 72.0]),
        footprint: [1, 1],
        default_collision_rect: None,
        default_depth_rect: None,
        default_interaction_rect: None,
        anchor: AnchorKind::BottomCenter,
        snap: SnapMode::Grid,
        tags: category.replace('_', ", "),
        entity_type: String::new(),
        codex_id: String::new(),
    };
    apply_kind_defaults(&mut draft);
    draft
}

fn infer_asset_kind(category: &str, id: &str) -> AssetKind {
    if category == "tiles" || id.contains("_tile_") {
        AssetKind::Tile
    } else if category == "decals" || id.contains("_decal_") {
        AssetKind::Decal
    } else if category == "fauna" || category == "pickups" || id.contains("_fauna_") {
        AssetKind::Entity
    } else {
        AssetKind::Object
    }
}

pub(crate) fn apply_kind_defaults(draft: &mut AssetDraft) {
    match draft.kind {
        AssetKind::Tile => {
            draft.default_layer = LayerKind::Ground;
            if draft.default_size[0] <= 1.0 || draft.default_size[1] <= 1.0 {
                draft.default_size = [32.0, 32.0];
            }
            draft.footprint = infer_tile_footprint(draft.default_size, 32).unwrap_or([1, 1]);
            draft.anchor = AnchorKind::TopLeft;
            draft.snap = SnapMode::Grid;
        }
        AssetKind::Decal => {
            draft.default_layer = LayerKind::Decals;
            draft.anchor = AnchorKind::Center;
            draft.snap = SnapMode::HalfGrid;
        }
        AssetKind::Object => {
            draft.default_layer = LayerKind::Objects;
            draft.anchor = AnchorKind::BottomCenter;
            draft.snap = SnapMode::Grid;
        }
        AssetKind::Entity => {
            draft.default_layer = LayerKind::Entities;
            draft.anchor = AnchorKind::BottomCenter;
            draft.snap = SnapMode::Grid;
            if draft.entity_type.trim().is_empty() {
                draft.entity_type = content::semantics::ENTITY_DECORATION.to_owned();
            }
        }
        AssetKind::Zone => {
            draft.default_layer = LayerKind::Zones;
            draft.anchor = AnchorKind::TopLeft;
            draft.snap = SnapMode::Grid;
        }
    }
}

pub(crate) fn image_dimensions(path: &Path) -> Option<[f32; 2]> {
    image::image_dimensions(path)
        .ok()
        .map(|(width, height)| [width as f32, height as f32])
}

pub(crate) fn infer_tile_footprint(default_size: [f32; 2], tile_size: u32) -> Option<[i32; 2]> {
    let tile_size = tile_size.max(1) as f32;
    let width_units = default_size[0] / tile_size;
    let height_units = default_size[1] / tile_size;
    let width = width_units.round();
    let height = height_units.round();
    ((width_units - width).abs() < 0.01 && (height_units - height).abs() < 0.01)
        .then_some([width.max(1.0) as i32, height.max(1.0) as i32])
}

pub(crate) fn collect_png_paths(dir: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_png_paths(&path, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            output.push(path);
        }
    }
}
