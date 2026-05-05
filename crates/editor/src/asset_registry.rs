use std::path::{Path, PathBuf};

use content::{AnchorKind, AssetDatabase, AssetDefinition, AssetKind, LayerKind, SnapMode};

#[derive(Clone, Debug)]
pub struct AssetEntry {
    pub id: String,
    pub category: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub kind: AssetKind,
    pub default_layer: LayerKind,
    pub default_size: [f32; 2],
    pub footprint: Option<[i32; 2]>,
    pub anchor: AnchorKind,
    pub snap: SnapMode,
    pub entity_type: Option<String>,
    pub codex_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AssetRegistry {
    assets: Vec<AssetEntry>,
}

impl AssetRegistry {
    pub fn from_database(project_root: &Path, database: AssetDatabase) -> Self {
        let mut assets = database
            .assets
            .into_iter()
            .map(|asset| asset_entry(project_root, asset))
            .collect::<Vec<_>>();

        assets.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| left.id.cmp(&right.id))
        });

        Self { assets }
    }

    pub fn assets(&self) -> &[AssetEntry] {
        &self.assets
    }

    pub fn get(&self, id: &str) -> Option<&AssetEntry> {
        self.assets.iter().find(|asset| asset.id == id)
    }

    pub fn contains_path(&self, relative_path: &str) -> bool {
        self.assets
            .iter()
            .any(|asset| asset.relative_path == relative_path)
    }

    pub fn categories(&self) -> Vec<&str> {
        let mut categories = Vec::new();

        for asset in &self.assets {
            if !categories.contains(&asset.category.as_str()) {
                categories.push(asset.category.as_str());
            }
        }

        categories
    }

    pub fn in_category<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a AssetEntry> {
        self.assets
            .iter()
            .filter(move |asset| asset.category == category)
    }
}

fn asset_entry(project_root: &Path, asset: AssetDefinition) -> AssetEntry {
    let relative_path = asset.path.to_string_lossy().replace('\\', "/");
    let path = project_root.join(&asset.path);

    AssetEntry {
        id: asset.id,
        category: asset.category,
        path,
        relative_path,
        kind: asset.kind,
        default_layer: asset.default_layer,
        default_size: asset.default_size,
        footprint: asset.footprint,
        anchor: asset.anchor,
        snap: asset.snap,
        entity_type: asset.entity_type,
        codex_id: asset.codex_id,
        tags: asset.tags,
    }
}
