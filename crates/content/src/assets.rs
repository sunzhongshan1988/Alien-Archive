use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::LayerKind;

pub const DEFAULT_ASSET_DB_PATH: &str = "assets/data/assets/overworld_assets.ron";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AssetKind {
    Tile,
    Decal,
    Object,
    Entity,
    Zone,
}

impl AssetKind {
    pub const ALL: [Self; 5] = [
        Self::Tile,
        Self::Decal,
        Self::Object,
        Self::Entity,
        Self::Zone,
    ];

    pub fn zh_label(self) -> &'static str {
        match self {
            Self::Tile => "地块",
            Self::Decal => "贴花",
            Self::Object => "物件",
            Self::Entity => "实体",
            Self::Zone => "区域",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AnchorKind {
    TopLeft,
    Center,
    BottomCenter,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SnapMode {
    Grid,
    HalfGrid,
    Free,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssetDatabase {
    pub mode: String,
    pub assets: Vec<AssetDefinition>,
    #[serde(skip)]
    pub(crate) by_id: HashMap<String, usize>,
}

impl AssetDatabase {
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            assets: Vec::new(),
            by_id: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read asset database {}", path.display()))?;
        let mut database: Self = ron::from_str(&source)
            .with_context(|| format!("failed to parse asset database {}", path.display()))?;
        database.reindex();

        Ok(database)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let source = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .context("failed to serialize asset database")?;
        fs::write(path, source)
            .with_context(|| format!("failed to write asset database {}", path.display()))
    }

    pub fn reindex(&mut self) {
        self.by_id = self
            .assets
            .iter()
            .enumerate()
            .map(|(index, asset)| (asset.id.clone(), index))
            .collect();
    }

    pub fn get(&self, id: &str) -> Option<&AssetDefinition> {
        self.by_id
            .get(id)
            .and_then(|index| self.assets.get(*index))
            .or_else(|| self.assets.iter().find(|asset| asset.id == id))
    }

    pub fn assets(&self) -> &[AssetDefinition] {
        &self.assets
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

    pub fn in_category<'a>(
        &'a self,
        category: &'a str,
    ) -> impl Iterator<Item = &'a AssetDefinition> {
        self.assets
            .iter()
            .filter(move |asset| asset.category == category)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssetDefinition {
    pub id: String,
    pub category: String,
    pub path: PathBuf,
    pub kind: AssetKind,
    pub default_layer: LayerKind,
    pub default_size: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<[i32; 2]>,
    pub anchor: AnchorKind,
    pub snap: SnapMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_id: Option<String>,
}
