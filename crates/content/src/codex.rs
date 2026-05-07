use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CODEX_DB_PATH: &str = "assets/data/codex/overworld_codex.ron";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CodexDatabase {
    pub mode: String,
    pub entries: Vec<CodexEntry>,
    #[serde(skip)]
    pub(crate) by_id: HashMap<String, usize>,
}

impl CodexDatabase {
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            entries: Vec::new(),
            by_id: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read codex database {}", path.display()))?;
        let mut database: Self = ron::from_str(&source)
            .with_context(|| format!("failed to parse codex database {}", path.display()))?;
        database.reindex();
        Ok(database)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let source = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .context("failed to serialize codex database")?;
        fs::write(path, source)
            .with_context(|| format!("failed to write codex database {}", path.display()))
    }

    pub fn reindex(&mut self) {
        self.by_id = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.id.clone(), index))
            .collect();
    }

    pub fn get(&self, id: &str) -> Option<&CodexEntry> {
        self.by_id
            .get(id)
            .and_then(|index| self.entries.get(*index))
            .or_else(|| self.entries.iter().find(|entry| entry.id == id))
    }

    pub fn entries(&self) -> &[CodexEntry] {
        &self.entries
    }

    pub fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(|entry| entry.id.as_str()).collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CodexEntry {
    pub id: String,
    pub category: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_time: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unlock_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn parses_codex_database() {
        let source = r#"
        (
            mode: "Overworld",
            entries: [
                (
                    id: "codex.flora.glowfungus",
                    category: "Flora",
                    title: "Glowfungus",
                    description: "A test entry.",
                    scan_time: Some(1.25),
                    unlock_tags: ["flora"],
                ),
            ],
        )
        "#;

        let mut database: CodexDatabase = ron::from_str(source).unwrap();
        database.reindex();

        assert_eq!(
            database
                .get("codex.flora.glowfungus")
                .map(|entry| entry.title.as_str()),
            Some("Glowfungus")
        );
    }

    #[test]
    fn loads_optional_workspace_codex_file_when_present() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(DEFAULT_CODEX_DB_PATH);
        if !path.exists() {
            return;
        }

        let database = CodexDatabase::load(&path).unwrap();

        assert!(!database.entries().is_empty());
    }
}
