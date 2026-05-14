use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{CutsceneDatabase, ObjectiveDatabase};

pub const DEFAULT_EVENT_DB_PATH: &str = "crates/content/data/events.ron";

const BUNDLED_EVENT_DB: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/events.ron"));

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventDatabase {
    events: Vec<WorldEventDefinition>,
    #[serde(skip)]
    by_id: HashMap<String, usize>,
}

impl Default for EventDatabase {
    fn default() -> Self {
        bundled_event_database()
    }
}

impl EventDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read events {}", path.display()))?;
        Self::from_ron(&source)
            .with_context(|| format!("failed to parse events {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let mut source = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .context("failed to serialize event database")?;
        source.push('\n');
        fs::write(path, source)
            .with_context(|| format!("failed to write event database {}", path.display()))
    }

    pub fn load_default() -> Self {
        Self::load(Path::new(DEFAULT_EVENT_DB_PATH)).unwrap_or_else(|error| {
            eprintln!("event database load failed: {error:?}");
            bundled_event_database()
        })
    }

    pub fn from_ron(source: &str) -> Result<Self> {
        let document: EventDocument = ron::from_str(source).context("failed to parse event RON")?;
        Ok(Self::from_definitions(document.events))
    }

    pub fn from_definitions(events: Vec<WorldEventDefinition>) -> Self {
        let events = events
            .into_iter()
            .filter(|event| !event.id.trim().is_empty())
            .map(|mut event| {
                event.id = event.id.trim().to_owned();
                event
            })
            .collect::<Vec<_>>();
        let by_id = events
            .iter()
            .enumerate()
            .map(|(index, event)| (event.id.clone(), index))
            .collect();
        Self { events, by_id }
    }

    pub fn reindex(&mut self) {
        self.by_id = self
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| !event.id.trim().is_empty())
            .map(|(index, event)| (event.id.trim().to_owned(), index))
            .collect();
    }

    pub fn get(&self, id: &str) -> Option<&WorldEventDefinition> {
        self.by_id
            .get(id.trim())
            .and_then(|index| self.events.get(*index))
    }

    pub fn events(&self) -> &[WorldEventDefinition] {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<WorldEventDefinition> {
        &mut self.events
    }

    pub fn validate(
        &self,
        cutscenes: Option<&CutsceneDatabase>,
        objectives: Option<&ObjectiveDatabase>,
    ) -> Vec<EventValidationIssue> {
        validate_events(self, cutscenes, objectives)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct EventDocument {
    events: Vec<WorldEventDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct WorldEventDefinition {
    pub id: String,
    pub trigger: EventTrigger,
    pub scope: EventScope,
    pub conditions: Vec<EventCondition>,
    pub actions: Vec<EventAction>,
}

impl Default for WorldEventDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            trigger: EventTrigger::EnterZone,
            scope: EventScope::WorldOnce,
            conditions: Vec::new(),
            actions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventTrigger {
    EnterZone,
}

impl Default for EventTrigger {
    fn default() -> Self {
        Self::EnterZone
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventScope {
    OncePerZone,
    WorldOnce,
    Repeatable,
}

impl Default for EventScope {
    fn default() -> Self {
        Self::WorldOnce
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventCondition {
    FlagSet(String),
    FlagMissing(String),
    CutsceneSeen(String),
    CutsceneMissing(String),
    CodexScanned(String),
    CodexMissing(String),
    ObjectiveCheckpointDone {
        objective_id: String,
        checkpoint_id: String,
    },
    ObjectiveCheckpointMissing {
        objective_id: String,
        checkpoint_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventAction {
    PlayCutscene(String),
    SetFlag(String),
    AdvanceObjective {
        objective_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        checkpoint_id: Option<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        complete_objective: bool,
    },
    ShowNotice(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventValidationIssue {
    pub severity: EventValidationSeverity,
    pub message: String,
}

impl EventValidationIssue {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: EventValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: EventValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

fn validate_events(
    database: &EventDatabase,
    cutscenes: Option<&CutsceneDatabase>,
    objectives: Option<&ObjectiveDatabase>,
) -> Vec<EventValidationIssue> {
    let mut issues = Vec::new();
    let mut ids = std::collections::HashSet::new();

    for event in database.events() {
        let id = event.id.trim();
        if id.is_empty() {
            issues.push(EventValidationIssue::error("event id is empty"));
            continue;
        }
        if !ids.insert(id.to_owned()) {
            issues.push(EventValidationIssue::error(format!(
                "event id duplicated: {id}"
            )));
        }
        if event.actions.is_empty() {
            issues.push(EventValidationIssue::warning(format!(
                "event {id} has no actions"
            )));
        }
        for condition in &event.conditions {
            validate_condition(id, condition, objectives, &mut issues);
        }
        for action in &event.actions {
            validate_action(id, action, cutscenes, objectives, &mut issues);
        }
    }

    issues
}

fn validate_condition(
    event_id: &str,
    condition: &EventCondition,
    objectives: Option<&ObjectiveDatabase>,
    issues: &mut Vec<EventValidationIssue>,
) {
    match condition {
        EventCondition::FlagSet(value)
        | EventCondition::FlagMissing(value)
        | EventCondition::CutsceneSeen(value)
        | EventCondition::CutsceneMissing(value)
        | EventCondition::CodexScanned(value)
        | EventCondition::CodexMissing(value) => {
            if value.trim().is_empty() {
                issues.push(EventValidationIssue::error(format!(
                    "event {event_id} has a condition with an empty id"
                )));
            }
        }
        EventCondition::ObjectiveCheckpointDone {
            objective_id,
            checkpoint_id,
        }
        | EventCondition::ObjectiveCheckpointMissing {
            objective_id,
            checkpoint_id,
        } => {
            validate_objective_checkpoint(event_id, objective_id, checkpoint_id, objectives, issues)
        }
    }
}

fn validate_action(
    event_id: &str,
    action: &EventAction,
    cutscenes: Option<&CutsceneDatabase>,
    objectives: Option<&ObjectiveDatabase>,
    issues: &mut Vec<EventValidationIssue>,
) {
    match action {
        EventAction::PlayCutscene(cutscene_id) => {
            if cutscene_id.trim().is_empty() {
                issues.push(EventValidationIssue::error(format!(
                    "event {event_id} PlayCutscene has an empty cutscene id"
                )));
            } else if cutscenes.is_some_and(|database| database.get(cutscene_id).is_none()) {
                issues.push(EventValidationIssue::error(format!(
                    "event {event_id} references unknown cutscene {cutscene_id}"
                )));
            }
        }
        EventAction::SetFlag(flag) => {
            if flag.trim().is_empty() {
                issues.push(EventValidationIssue::error(format!(
                    "event {event_id} SetFlag has an empty flag"
                )));
            }
        }
        EventAction::AdvanceObjective {
            objective_id,
            checkpoint_id,
            ..
        } => {
            if objective_id.trim().is_empty() {
                issues.push(EventValidationIssue::error(format!(
                    "event {event_id} AdvanceObjective has an empty objective id"
                )));
            } else if let Some(checkpoint_id) = checkpoint_id {
                validate_objective_checkpoint(
                    event_id,
                    objective_id,
                    checkpoint_id,
                    objectives,
                    issues,
                );
            } else if objectives.is_some_and(|database| database.get(objective_id).is_none()) {
                issues.push(EventValidationIssue::error(format!(
                    "event {event_id} references unknown objective {objective_id}"
                )));
            }
        }
        EventAction::ShowNotice(message) => {
            if message.trim().is_empty() {
                issues.push(EventValidationIssue::warning(format!(
                    "event {event_id} ShowNotice has an empty message"
                )));
            }
        }
    }
}

fn validate_objective_checkpoint(
    event_id: &str,
    objective_id: &str,
    checkpoint_id: &str,
    objectives: Option<&ObjectiveDatabase>,
    issues: &mut Vec<EventValidationIssue>,
) {
    if objective_id.trim().is_empty() || checkpoint_id.trim().is_empty() {
        issues.push(EventValidationIssue::error(format!(
            "event {event_id} objective checkpoint reference is incomplete"
        )));
        return;
    }

    let Some(objectives) = objectives else {
        return;
    };
    let Some(objective) = objectives.get(objective_id) else {
        issues.push(EventValidationIssue::error(format!(
            "event {event_id} references unknown objective {objective_id}"
        )));
        return;
    };
    if !objective
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.id == checkpoint_id)
    {
        issues.push(EventValidationIssue::error(format!(
            "event {event_id} references unknown checkpoint {objective_id}/{checkpoint_id}"
        )));
    }
}

fn bundled_event_database() -> EventDatabase {
    EventDatabase::from_ron(BUNDLED_EVENT_DB).expect("bundled event database should parse")
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_event_database() {
        let database = EventDatabase::from_ron(
            r#"(
                events: [
                    (
                        id: "landing.test",
                        trigger: EnterZone,
                        scope: WorldOnce,
                        conditions: [FlagMissing("landing.test.done")],
                        actions: [
                            PlayCutscene("intro.new_game"),
                            SetFlag("landing.test.done"),
                            ShowNotice("Signal logged"),
                        ],
                    ),
                ],
            )"#,
        )
        .unwrap();

        assert_eq!(database.get("landing.test").unwrap().actions.len(), 3);
    }

    #[test]
    fn validates_unknown_cutscene() {
        let database = EventDatabase::from_definitions(vec![WorldEventDefinition {
            id: "bad".to_owned(),
            actions: vec![EventAction::PlayCutscene("missing".to_owned())],
            ..WorldEventDefinition::default()
        }]);

        let issues = database.validate(Some(&CutsceneDatabase::default()), None);
        assert!(
            issues
                .iter()
                .any(|issue| issue.severity == EventValidationSeverity::Error)
        );
    }
}
