use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct EditorConfig {
    pub(crate) recent_maps: Vec<String>,
    pub(crate) language: String,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            recent_maps: Vec::new(),
            language: "zh-Hans".to_owned(),
        }
    }
}

pub(crate) fn editor_config_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join(".editor").join("editor_config.ron")
}

pub(crate) fn load_editor_config(project_root: &Path) -> EditorConfig {
    let path = editor_config_path(project_root);
    fs::read_to_string(&path)
        .ok()
        .and_then(|source| ron::from_str(&source).ok())
        .unwrap_or_default()
}

pub(crate) fn save_editor_config(project_root: &Path, config: &EditorConfig) -> Result<()> {
    let path = editor_config_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let source = ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::new())
        .context("failed to serialize editor config")?;
    fs::write(&path, source).with_context(|| format!("failed to write {}", path.display()))
}
