use std::path::PathBuf;

use content::{AnchorKind, AssetDefinition, AssetKind, InstanceRect, LayerKind, SnapMode};

use crate::asset_registry::AssetEntry;
use crate::assets::import::infer_tile_footprint;
use crate::util::sanitize::{
    non_empty_string, parse_tags, sanitize_asset_id, sanitize_category, sanitize_relative_path,
};

#[derive(Clone, Debug)]
pub(crate) struct AssetDraft {
    pub(crate) id: String,
    pub(crate) category: String,
    pub(crate) path: String,
    pub(crate) kind: AssetKind,
    pub(crate) default_layer: LayerKind,
    pub(crate) default_size: [f32; 2],
    pub(crate) footprint: [i32; 2],
    pub(crate) default_collision_rect: Option<InstanceRect>,
    pub(crate) default_depth_rect: Option<InstanceRect>,
    pub(crate) default_interaction_rect: Option<InstanceRect>,
    pub(crate) anchor: AnchorKind,
    pub(crate) snap: SnapMode,
    pub(crate) tags: String,
    pub(crate) entity_type: String,
    pub(crate) codex_id: String,
}

impl Default for AssetDraft {
    fn default() -> Self {
        Self {
            id: String::new(),
            category: "props".to_owned(),
            path: String::new(),
            kind: AssetKind::Object,
            default_layer: LayerKind::Objects,
            default_size: [72.0, 72.0],
            footprint: [1, 1],
            default_collision_rect: None,
            default_depth_rect: None,
            default_interaction_rect: None,
            anchor: AnchorKind::BottomCenter,
            snap: SnapMode::Grid,
            tags: "props".to_owned(),
            entity_type: String::new(),
            codex_id: String::new(),
        }
    }
}

impl AssetDraft {
    pub(crate) fn from_entry(entry: &AssetEntry) -> Self {
        Self {
            id: entry.id.clone(),
            category: entry.category.clone(),
            path: entry.relative_path.clone(),
            kind: entry.kind,
            default_layer: entry.default_layer,
            default_size: entry.default_size,
            footprint: entry
                .footprint
                .unwrap_or_else(|| infer_tile_footprint(entry.default_size, 32).unwrap_or([1, 1])),
            default_collision_rect: entry.default_collision_rect,
            default_depth_rect: entry.default_depth_rect,
            default_interaction_rect: entry.default_interaction_rect,
            anchor: entry.anchor,
            snap: entry.snap,
            tags: entry.tags.join(", "),
            entity_type: entry.entity_type.clone().unwrap_or_default(),
            codex_id: entry.codex_id.clone().unwrap_or_default(),
        }
    }

    pub(crate) fn to_definition(&self) -> Option<AssetDefinition> {
        let id = sanitize_asset_id(&self.id)?;
        let path = sanitize_relative_path(&self.path)?;
        let category = sanitize_category(&self.category).unwrap_or_else(|| "props".to_owned());
        Some(AssetDefinition {
            id,
            category,
            path: PathBuf::from(path),
            kind: self.kind,
            default_layer: self.default_layer,
            default_size: [self.default_size[0].max(1.0), self.default_size[1].max(1.0)],
            footprint: (self.kind == AssetKind::Tile)
                .then_some([self.footprint[0].max(1), self.footprint[1].max(1)]),
            default_collision_rect: matches!(
                self.kind,
                AssetKind::Tile | AssetKind::Object | AssetKind::Entity
            )
            .then_some(self.default_collision_rect)
            .flatten(),
            default_depth_rect: matches!(self.kind, AssetKind::Object | AssetKind::Entity)
                .then_some(self.default_depth_rect)
                .flatten(),
            default_interaction_rect: (self.kind == AssetKind::Entity)
                .then_some(self.default_interaction_rect)
                .flatten(),
            anchor: self.anchor,
            snap: self.snap,
            tags: parse_tags(&self.tags),
            entity_type: non_empty_string(&self.entity_type),
            codex_id: non_empty_string(&self.codex_id),
        })
    }
}
