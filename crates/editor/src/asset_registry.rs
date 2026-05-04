use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::map_document::LayerKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetKind {
    Tile,
    Decal,
    Object,
    Entity,
    Zone,
}

#[derive(Clone, Debug)]
pub struct AssetEntry {
    pub id: String,
    pub category: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub kind: AssetKind,
    pub default_layer: LayerKind,
    pub default_size: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct AssetRegistry {
    assets: Vec<AssetEntry>,
}

impl AssetRegistry {
    pub fn scan(project_root: &Path) -> Result<Self> {
        let mut assets = Vec::new();
        let sprites_root = project_root.join("assets").join("sprites");

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

                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };

                let kind = infer_asset_kind(category);
                let source_size = image::image_dimensions(&path)
                    .with_context(|| format!("failed to read image size {}", path.display()))?;
                let relative_path = path
                    .strip_prefix(project_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                assets.push(AssetEntry {
                    id: stem.to_owned(),
                    category: category.to_string(),
                    path,
                    relative_path,
                    kind,
                    default_layer: default_layer(category, kind),
                    default_size: default_size(category, kind, source_size),
                });
            }
        }

        assets.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(Self { assets })
    }

    pub fn assets(&self) -> &[AssetEntry] {
        &self.assets
    }

    pub fn get(&self, id: &str) -> Option<&AssetEntry> {
        self.assets.iter().find(|asset| asset.id == id)
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

fn infer_asset_kind(category: &str) -> AssetKind {
    match category {
        "tiles" => AssetKind::Tile,
        "decals" => AssetKind::Decal,
        "structures" | "ruins" | "interactables" => AssetKind::Entity,
        "zones" => AssetKind::Zone,
        _ => AssetKind::Object,
    }
}

fn default_layer(category: &str, kind: AssetKind) -> LayerKind {
    match kind {
        AssetKind::Tile => LayerKind::Ground,
        AssetKind::Decal => LayerKind::Decals,
        AssetKind::Zone => LayerKind::Zones,
        AssetKind::Entity => LayerKind::Entities,
        AssetKind::Object => match category {
            "pickups" | "fauna" => LayerKind::Entities,
            _ => LayerKind::Objects,
        },
    }
}

fn default_size(category: &str, kind: AssetKind, source_size: (u32, u32)) -> [f32; 2] {
    let target_height = match kind {
        AssetKind::Tile => 32.0,
        AssetKind::Decal | AssetKind::Zone => 48.0,
        AssetKind::Entity => match category {
            "ruins" | "structures" => 128.0,
            _ => 72.0,
        },
        AssetKind::Object => 72.0,
    };

    scale_to_height(source_size, target_height)
}

fn scale_to_height(source_size: (u32, u32), target_height: f32) -> [f32; 2] {
    let width = source_size.0.max(1) as f32;
    let height = source_size.1.max(1) as f32;
    let scale = target_height / height;

    [width * scale, target_height]
}
