#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use anyhow::Result;
use content::{ObjectiveDatabase as ObjectiveDefinitions, ObjectiveDefinition, ObjectiveText};

use crate::save::{ObjectiveStateSave, ObjectivesSave};
use crate::scenes::Language;
use crate::ui::localization;

#[derive(Clone, Debug)]
pub struct ObjectiveDatabase {
    definitions: ObjectiveDefinitions,
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
        Self::from_definitions(ObjectiveDefinitions::default())
    }
}

impl ObjectiveDatabase {
    pub fn load_default() -> Self {
        Self::from_definitions(ObjectiveDefinitions::load_default())
    }

    pub fn menu_rows(&self, save: &ObjectivesSave, language: Language) -> Vec<ObjectiveMenuRow> {
        self.definitions
            .objectives()
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

    fn from_definitions(definitions: ObjectiveDefinitions) -> Self {
        Self { definitions }
    }

    fn objective(&self, id: &str) -> Option<&ObjectiveDefinition> {
        self.definitions.get(id)
    }
}

#[cfg(test)]
impl ObjectiveDatabase {
    fn load(path: &Path) -> Result<Self> {
        ObjectiveDefinitions::load(path).map(Self::from_definitions)
    }

    fn from_ron(source: &str) -> Result<Self> {
        ObjectiveDefinitions::from_ron(source).map(Self::from_definitions)
    }
}

trait ObjectiveTextExt {
    fn get(&self, language: Language, fallback: &str) -> String;
}

impl ObjectiveTextExt for ObjectiveText {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_objective_document() {
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
            .join(content::DEFAULT_OBJECTIVE_DB_PATH);
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
