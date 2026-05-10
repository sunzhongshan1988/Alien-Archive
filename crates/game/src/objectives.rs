use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::save::{ObjectiveStateSave, ObjectivesSave};
use crate::scenes::Language;
use crate::ui::localization;

pub const DEFAULT_OBJECTIVE_DB_PATH: &str = "assets/data/objectives/field_objectives.ron";

#[derive(Clone, Debug)]
pub struct ObjectiveDatabase {
    objectives: Vec<ObjectiveDefinition>,
    by_id: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct ObjectiveDocument {
    objectives: Vec<ObjectiveDefinition>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ObjectiveDefinition {
    pub id: String,
    pub title: ObjectiveText,
    pub summary: ObjectiveText,
    pub initial_status: String,
    pub checkpoints: Vec<ObjectiveCheckpoint>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ObjectiveCheckpoint {
    pub id: String,
    pub title: ObjectiveText,
    pub detail: ObjectiveText,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ObjectiveText {
    pub english: String,
    pub chinese: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveMenuRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub progress: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveAdvanceEvent {
    pub objective_id: String,
    pub checkpoint_id: Option<String>,
    pub notice: String,
    pub log_title: String,
    pub log_detail: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectiveStatus {
    Inactive,
    Active,
    Completed,
}

impl Default for ObjectiveDatabase {
    fn default() -> Self {
        Self::from_document(fallback_document())
    }
}

impl ObjectiveDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read objectives {}", path.display()))?;
        let document: ObjectiveDocument = ron::from_str(&source)
            .with_context(|| format!("failed to parse objectives {}", path.display()))?;
        Ok(Self::from_document(document))
    }

    pub fn load_default() -> Self {
        match Self::load(Path::new(DEFAULT_OBJECTIVE_DB_PATH)) {
            Ok(database) => database,
            Err(error) => {
                eprintln!("objective database load failed: {error:?}");
                Self::default()
            }
        }
    }

    pub fn menu_rows(&self, save: &ObjectivesSave, language: Language) -> Vec<ObjectiveMenuRow> {
        self.objectives
            .iter()
            .map(|objective| {
                let state = save.states.get(&objective.id);
                let status = objective_status(objective, state);
                ObjectiveMenuRow {
                    id: objective.id.clone(),
                    title: objective.title.get(language, &objective.id),
                    status: status_label(language, status),
                    progress: objective_progress(objective, state, status),
                }
            })
            .collect()
    }

    pub fn apply_rule(
        &self,
        save: &mut ObjectivesSave,
        language: Language,
        objective_id: &str,
        checkpoint_id: Option<&str>,
        complete_objective: bool,
    ) -> Option<ObjectiveAdvanceEvent> {
        let objective = self.objective(objective_id)?;
        let previous_state = save.states.get(objective_id).cloned();
        let had_state = previous_state.is_some();
        let previous_status = objective_status(objective, previous_state.as_ref());

        let state = save
            .states
            .entry(objective_id.to_owned())
            .or_insert_with(|| ObjectiveStateSave::new(status_key(previous_status)));

        if previous_status == ObjectiveStatus::Completed {
            return None;
        }

        let mut changed = !had_state && checkpoint_id.is_none() && !complete_objective;
        if current_status(&state.status) == ObjectiveStatus::Inactive {
            state.status = "active".to_owned();
            changed = true;
        }

        let mut checkpoint_title = None;
        if let Some(checkpoint_id) = checkpoint_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let inserted = state.completed_checkpoints.insert(checkpoint_id.to_owned());
            changed |= inserted;
            checkpoint_title = objective
                .checkpoints
                .iter()
                .find(|checkpoint| checkpoint.id == checkpoint_id)
                .map(|checkpoint| checkpoint.title.get(language, checkpoint_id))
                .or_else(|| Some(checkpoint_id.to_owned()));
        }

        let completed = complete_objective || all_checkpoints_complete(objective, state);
        if completed && current_status(&state.status) != ObjectiveStatus::Completed {
            state.status = "completed".to_owned();
            changed = true;
        }

        if !changed {
            return None;
        }

        let status = current_status(&state.status);
        let objective_title = objective.title.get(language, objective_id);
        let summary = objective.summary.get(language, &objective_title);
        let progress = objective_progress(objective, Some(state), status);
        let (notice_key, title_key, english_notice, chinese_notice, english_title, chinese_title) =
            if status == ObjectiveStatus::Completed {
                (
                    "objective.notice.completed",
                    "activity.event.objective_completed.title",
                    "Objective complete: {objective}",
                    "目标完成：{objective}",
                    "Objective complete",
                    "目标完成",
                )
            } else if checkpoint_title.is_some() {
                (
                    "objective.notice.checkpoint",
                    "activity.event.objective_checkpoint.title",
                    "Objective updated: {objective}",
                    "目标更新：{objective}",
                    "Objective updated",
                    "目标更新",
                )
            } else {
                (
                    "objective.notice.started",
                    "activity.event.objective_started.title",
                    "Objective started: {objective}",
                    "目标开始：{objective}",
                    "Objective started",
                    "目标开始",
                )
            };

        let log_detail = if let Some(checkpoint_title) = checkpoint_title {
            localization::format_text(
                language,
                "activity.event.objective_checkpoint.detail",
                "{checkpoint} · {progress}% · {summary}",
                "{checkpoint} · {progress}% · {summary}",
                &[
                    ("checkpoint", checkpoint_title),
                    ("progress", progress.to_string()),
                    ("summary", summary),
                ],
            )
        } else {
            localization::format_text(
                language,
                "activity.event.objective_started.detail",
                "{objective} · {progress}% · {summary}",
                "{objective} · {progress}% · {summary}",
                &[
                    ("objective", objective_title.clone()),
                    ("progress", progress.to_string()),
                    ("summary", summary),
                ],
            )
        };

        Some(ObjectiveAdvanceEvent {
            objective_id: objective_id.to_owned(),
            checkpoint_id: checkpoint_id.map(str::to_owned),
            notice: localization::format_text(
                language,
                notice_key,
                english_notice,
                chinese_notice,
                &[("objective", objective_title)],
            ),
            log_title: localization::text(language, title_key, english_title, chinese_title)
                .into_owned(),
            log_detail,
        })
    }

    fn from_document(document: ObjectiveDocument) -> Self {
        let objectives = document
            .objectives
            .into_iter()
            .filter(|objective| !objective.id.trim().is_empty())
            .map(|mut objective| {
                objective.id = objective.id.trim().to_owned();
                objective.initial_status =
                    status_key(status_from_key(&objective.initial_status)).to_owned();
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

    fn objective(&self, id: &str) -> Option<&ObjectiveDefinition> {
        self.by_id
            .get(id.trim())
            .and_then(|index| self.objectives.get(*index))
    }
}

impl ObjectiveText {
    fn get(&self, language: Language, fallback: &str) -> String {
        let value = match language {
            Language::Chinese => self.chinese.trim(),
            Language::English => self.english.trim(),
        };
        if value.is_empty() {
            fallback.to_owned()
        } else {
            value.to_owned()
        }
    }
}

fn objective_status(
    objective: &ObjectiveDefinition,
    state: Option<&ObjectiveStateSave>,
) -> ObjectiveStatus {
    state
        .map(|state| current_status(&state.status))
        .unwrap_or_else(|| status_from_key(&objective.initial_status))
}

fn objective_progress(
    objective: &ObjectiveDefinition,
    state: Option<&ObjectiveStateSave>,
    status: ObjectiveStatus,
) -> u32 {
    if status == ObjectiveStatus::Completed {
        return 100;
    }
    if objective.checkpoints.is_empty() {
        return match status {
            ObjectiveStatus::Active => 50,
            ObjectiveStatus::Completed => 100,
            ObjectiveStatus::Inactive => 0,
        };
    }
    let completed = state
        .map(|state| {
            objective
                .checkpoints
                .iter()
                .filter(|checkpoint| state.completed_checkpoints.contains(&checkpoint.id))
                .count()
        })
        .unwrap_or(0);
    ((completed as f32 / objective.checkpoints.len() as f32) * 100.0).round() as u32
}

fn all_checkpoints_complete(objective: &ObjectiveDefinition, state: &ObjectiveStateSave) -> bool {
    !objective.checkpoints.is_empty()
        && objective
            .checkpoints
            .iter()
            .all(|checkpoint| state.completed_checkpoints.contains(&checkpoint.id))
}

fn status_label(language: Language, status: ObjectiveStatus) -> String {
    match status {
        ObjectiveStatus::Inactive => {
            localization::text(language, "objective.status.pending", "Pending", "待处理")
        }
        ObjectiveStatus::Active => {
            localization::text(language, "objective.status.active", "Active", "进行中")
        }
        ObjectiveStatus::Completed => localization::text(
            language,
            "objective.status.completed",
            "Completed",
            "已完成",
        ),
    }
    .into_owned()
}

fn current_status(status: &str) -> ObjectiveStatus {
    status_from_key(status)
}

fn status_from_key(status: &str) -> ObjectiveStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "active" | "started" | "tracked" => ObjectiveStatus::Active,
        "completed" | "complete" | "done" => ObjectiveStatus::Completed,
        _ => ObjectiveStatus::Inactive,
    }
}

fn status_key(status: ObjectiveStatus) -> &'static str {
    match status {
        ObjectiveStatus::Inactive => "inactive",
        ObjectiveStatus::Active => "active",
        ObjectiveStatus::Completed => "completed",
    }
}

fn fallback_document() -> ObjectiveDocument {
    ObjectiveDocument {
        objectives: vec![
            ObjectiveDefinition {
                id: "secure_landing_site".to_owned(),
                title: ObjectiveText {
                    english: "Secure Landing Site".to_owned(),
                    chinese: "稳固着陆点".to_owned(),
                },
                summary: ObjectiveText {
                    english: "Confirm the first safe route out of the landing perimeter."
                        .to_owned(),
                    chinese: "确认着陆点外围的第一条安全路线。".to_owned(),
                },
                initial_status: "active".to_owned(),
                checkpoints: vec![
                    ObjectiveCheckpoint {
                        id: "landing_perimeter".to_owned(),
                        title: ObjectiveText {
                            english: "Reach the landing perimeter".to_owned(),
                            chinese: "抵达着陆点外围".to_owned(),
                        },
                        detail: ObjectiveText::default(),
                    },
                    ObjectiveCheckpoint {
                        id: "scan_first_signal".to_owned(),
                        title: ObjectiveText {
                            english: "Record the first anomaly signal".to_owned(),
                            chinese: "记录第一处异常信号".to_owned(),
                        },
                        detail: ObjectiveText::default(),
                    },
                ],
            },
            ObjectiveDefinition {
                id: "survey_crystal_field".to_owned(),
                title: ObjectiveText {
                    english: "Survey Crystal Field".to_owned(),
                    chinese: "调查晶体田".to_owned(),
                },
                summary: ObjectiveText {
                    english: "Build an initial survey record for the crystal field.".to_owned(),
                    chinese: "为晶体田建立第一份调查记录。".to_owned(),
                },
                initial_status: "inactive".to_owned(),
                checkpoints: vec![ObjectiveCheckpoint {
                    id: "enter_crystal_field".to_owned(),
                    title: ObjectiveText {
                        english: "Enter the crystal field".to_owned(),
                        chinese: "进入晶体田".to_owned(),
                    },
                    detail: ObjectiveText::default(),
                }],
            },
            ObjectiveDefinition {
                id: "decode_ruin_signal".to_owned(),
                title: ObjectiveText {
                    english: "Decode Ruin Signal".to_owned(),
                    chinese: "解析遗迹信号".to_owned(),
                },
                summary: ObjectiveText {
                    english: "Find enough ruin data to identify the signal source.".to_owned(),
                    chinese: "收集足够遗迹资料以定位信号来源。".to_owned(),
                },
                initial_status: "inactive".to_owned(),
                checkpoints: vec![ObjectiveCheckpoint {
                    id: "find_ruin_terminal".to_owned(),
                    title: ObjectiveText {
                        english: "Find a ruin terminal".to_owned(),
                        chinese: "找到遗迹终端".to_owned(),
                    },
                    detail: ObjectiveText::default(),
                }],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_objective_document() {
        let document: ObjectiveDocument = ron::from_str(
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

        let database = ObjectiveDatabase::from_document(document);
        let rows = database.menu_rows(&ObjectivesSave::default(), Language::Chinese);
        assert_eq!(rows[0].title, "测试");
        assert_eq!(rows[0].progress, 0);
    }

    #[test]
    fn checkpoint_progress_completes_objective() {
        let database = ObjectiveDatabase::default();
        let mut save = ObjectivesSave::default();

        let event = database
            .apply_rule(
                &mut save,
                Language::English,
                "survey_crystal_field",
                Some("enter_crystal_field"),
                false,
            )
            .expect("checkpoint should advance objective");

        assert_eq!(event.objective_id, "survey_crystal_field");
        assert_eq!(
            save.states
                .get("survey_crystal_field")
                .map(|state| state.status.as_str()),
            Some("completed")
        );
        assert_eq!(
            database
                .menu_rows(&save, Language::English)
                .iter()
                .find(|row| row.id == "survey_crystal_field")
                .map(|row| row.progress),
            Some(100)
        );
    }

    #[test]
    fn bundled_objective_file_loads_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(DEFAULT_OBJECTIVE_DB_PATH);
        if path.exists() {
            let database = ObjectiveDatabase::load(&path).expect("objective file should load");
            assert!(
                !database
                    .menu_rows(&ObjectivesSave::default(), Language::English)
                    .is_empty()
            );
        }
    }
}
