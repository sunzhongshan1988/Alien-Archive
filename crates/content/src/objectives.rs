use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_OBJECTIVE_DB_PATH: &str = "crates/content/data/objectives.ron";

const BUNDLED_OBJECTIVE_DB: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/objectives.ron"));

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectiveDatabase {
    objectives: Vec<ObjectiveDefinition>,
    #[serde(skip)]
    by_id: HashMap<String, usize>,
}

impl Default for ObjectiveDatabase {
    fn default() -> Self {
        bundled_objective_database()
    }
}

impl ObjectiveDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read objectives {}", path.display()))?;
        Self::from_ron(&source)
            .with_context(|| format!("failed to parse objectives {}", path.display()))
    }

    pub fn load_default() -> Self {
        Self::load(Path::new(DEFAULT_OBJECTIVE_DB_PATH)).unwrap_or_else(|error| {
            eprintln!("objective database load failed: {error:?}");
            bundled_objective_database()
        })
    }

    pub fn from_ron(source: &str) -> Result<Self> {
        let document: ObjectiveDocument =
            ron::from_str(source).context("failed to parse objective RON")?;
        Ok(Self::from_document(document))
    }

    pub fn from_definitions(objectives: Vec<ObjectiveDefinition>) -> Self {
        let objectives = objectives
            .into_iter()
            .filter(|objective| !objective.id.trim().is_empty())
            .map(|mut objective| {
                objective.id = objective.id.trim().to_owned();
                objective
            })
            .collect::<Vec<_>>();
        let by_id = objectives
            .iter()
            .enumerate()
            .map(|(index, objective)| (objective.id.clone(), index))
            .collect();
        Self { objectives, by_id }
    }

    pub fn get(&self, id: &str) -> Option<&ObjectiveDefinition> {
        self.by_id
            .get(id.trim())
            .and_then(|index| self.objectives.get(*index))
    }

    pub fn objectives(&self) -> &[ObjectiveDefinition] {
        &self.objectives
    }

    fn from_document(document: ObjectiveDocument) -> Self {
        Self::from_definitions(document.objectives)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct ObjectiveDocument {
    objectives: Vec<ObjectiveDefinition>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ObjectiveDefinition {
    pub id: String,
    pub title: ObjectiveText,
    pub summary: ObjectiveText,
    pub initial_status: String,
    pub checkpoints: Vec<ObjectiveCheckpoint>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ObjectiveCheckpoint {
    pub id: String,
    pub title: ObjectiveText,
    pub detail: ObjectiveText,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ObjectiveText {
    pub english: String,
    pub chinese: String,
}

fn bundled_objective_database() -> ObjectiveDatabase {
    ObjectiveDatabase::from_ron(BUNDLED_OBJECTIVE_DB)
        .expect("bundled objective database should parse")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn parses_objective_database() {
        let database = ObjectiveDatabase::from_ron(
            r#"(
                objectives: [
                    (
                        id: "test_objective",
                        title: (english: "Test", chinese: "测试"),
                        summary: (english: "Summary", chinese: "摘要"),
                        initial_status: "active",
                        checkpoints: [
                            (id: "step_1", title: (english: "Step", chinese: "步骤")),
                        ],
                    ),
                ],
            )"#,
        )
        .expect("objective document should parse");

        assert_eq!(
            database
                .get("test_objective")
                .map(|objective| objective.title.chinese.as_str()),
            Some("测试")
        );
    }

    #[test]
    fn parses_bundled_objective_database() {
        let database = bundled_objective_database();

        assert!(database.get("secure_landing_site").is_some());
        assert!(!database.objectives().is_empty());
    }

    #[test]
    fn loads_default_objective_file_when_present() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(DEFAULT_OBJECTIVE_DB_PATH);
        if !path.exists() {
            return;
        }

        let database = ObjectiveDatabase::load(&path).expect("objective file should load");

        assert!(!database.objectives().is_empty());
    }
}
