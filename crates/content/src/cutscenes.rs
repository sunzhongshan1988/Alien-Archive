use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CUTSCENE_DB_PATH: &str = "crates/content/data/cutscenes.ron";

const BUNDLED_CUTSCENE_DB: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/cutscenes.ron"));

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CutsceneDatabase {
    cutscenes: Vec<CutsceneDefinition>,
    #[serde(skip)]
    by_id: HashMap<String, usize>,
}

impl Default for CutsceneDatabase {
    fn default() -> Self {
        bundled_cutscene_database()
    }
}

impl CutsceneDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read cutscenes {}", path.display()))?;
        Self::from_ron(&source)
            .with_context(|| format!("failed to parse cutscenes {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let mut source = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .context("failed to serialize cutscene database")?;
        source.push('\n');
        fs::write(path, source)
            .with_context(|| format!("failed to write cutscene database {}", path.display()))
    }

    pub fn load_default() -> Self {
        Self::load(Path::new(DEFAULT_CUTSCENE_DB_PATH)).unwrap_or_else(|error| {
            eprintln!("cutscene database load failed: {error:?}");
            bundled_cutscene_database()
        })
    }

    pub fn from_ron(source: &str) -> Result<Self> {
        let document: CutsceneDocument =
            ron::from_str(source).context("failed to parse cutscene RON")?;
        Ok(Self::from_definitions(document.cutscenes))
    }

    pub fn from_definitions(cutscenes: Vec<CutsceneDefinition>) -> Self {
        let cutscenes = cutscenes
            .into_iter()
            .filter(|cutscene| !cutscene.id.trim().is_empty())
            .map(|mut cutscene| {
                cutscene.id = cutscene.id.trim().to_owned();
                cutscene
            })
            .collect::<Vec<_>>();
        let by_id = cutscenes
            .iter()
            .enumerate()
            .map(|(index, cutscene)| (cutscene.id.clone(), index))
            .collect();
        Self { cutscenes, by_id }
    }

    pub fn reindex(&mut self) {
        self.by_id = self
            .cutscenes
            .iter()
            .enumerate()
            .filter(|(_, cutscene)| !cutscene.id.trim().is_empty())
            .map(|(index, cutscene)| (cutscene.id.trim().to_owned(), index))
            .collect();
    }

    pub fn get(&self, id: &str) -> Option<&CutsceneDefinition> {
        self.by_id
            .get(id.trim())
            .and_then(|index| self.cutscenes.get(*index))
    }

    pub fn cutscenes(&self) -> &[CutsceneDefinition] {
        &self.cutscenes
    }

    pub fn cutscenes_mut(&mut self) -> &mut Vec<CutsceneDefinition> {
        &mut self.cutscenes
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct CutsceneDocument {
    cutscenes: Vec<CutsceneDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct CutsceneDefinition {
    pub id: String,
    pub blocking: bool,
    pub play_once: bool,
    pub steps: Vec<CutsceneStep>,
    pub completion: CutsceneCompletion,
}

impl Default for CutsceneDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            blocking: true,
            play_once: true,
            steps: Vec::new(),
            completion: CutsceneCompletion::Pop,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CutsceneStep {
    FadeIn {
        duration: f32,
    },
    FadeOut {
        duration: f32,
    },
    Wait {
        duration: f32,
    },
    TextPanel {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        speaker: Option<CutsceneText>,
        body: CutsceneText,
        #[serde(default = "default_text_min_duration")]
        min_duration: f32,
        #[serde(default = "default_require_confirm")]
        require_confirm: bool,
    },
    SetFlag {
        flag: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CutsceneCompletion {
    Pop,
    SwitchScene { scene: String },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct CutsceneText {
    pub english: String,
    pub chinese: String,
}

impl CutsceneText {
    pub fn new(english: impl Into<String>, chinese: impl Into<String>) -> Self {
        Self {
            english: english.into(),
            chinese: chinese.into(),
        }
    }
}

fn default_text_min_duration() -> f32 {
    0.25
}

fn default_require_confirm() -> bool {
    true
}

fn bundled_cutscene_database() -> CutsceneDatabase {
    CutsceneDatabase::from_ron(BUNDLED_CUTSCENE_DB).expect("bundled cutscene database should parse")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cutscene_database() {
        let source = r#"
        (
            cutscenes: [
                (
                    id: "intro.test",
                    steps: [
                        FadeOut(duration: 0.2),
                        TextPanel(
                            speaker: Some((english: "AI", chinese: "AI")),
                            body: (
                                english: "Wake signal confirmed.",
                                chinese: "唤醒信号确认。",
                            ),
                        ),
                        SetFlag(flag: "intro_seen"),
                    ],
                    completion: SwitchScene(scene: "Overworld"),
                ),
            ],
        )
        "#;

        let database = CutsceneDatabase::from_ron(source).unwrap();
        let cutscene = database.get("intro.test").unwrap();

        assert_eq!(cutscene.steps.len(), 3);
        assert_eq!(
            cutscene.completion,
            CutsceneCompletion::SwitchScene {
                scene: "Overworld".to_owned()
            }
        );
    }

    #[test]
    fn empty_cutscene_ids_are_filtered_and_trimmed() {
        let database = CutsceneDatabase::from_definitions(vec![
            CutsceneDefinition {
                id: "  intro.trimmed  ".to_owned(),
                ..CutsceneDefinition::default()
            },
            CutsceneDefinition::default(),
        ]);

        assert!(database.get("intro.trimmed").is_some());
        assert_eq!(database.cutscenes().len(), 1);
    }

    #[test]
    fn reindex_tracks_editor_mutations() {
        let mut database = CutsceneDatabase::from_definitions(vec![CutsceneDefinition {
            id: "intro.old".to_owned(),
            ..CutsceneDefinition::default()
        }]);

        database.cutscenes_mut()[0].id = "intro.new".to_owned();
        database.reindex();

        assert!(database.get("intro.old").is_none());
        assert!(database.get("intro.new").is_some());
    }

    #[test]
    fn bundled_cutscene_database_contains_new_game_intro() {
        let database = CutsceneDatabase::default();

        assert!(database.get("intro.new_game").is_some());
    }
}
