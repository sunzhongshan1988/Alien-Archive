use std::path::PathBuf;

use content::{
    DEFAULT_EVENT_DB_PATH, EventAction, EventCondition, EventDatabase, EventScope, EventTrigger,
    EventValidationSeverity, WorldEventDefinition,
};
use eframe::egui;

use crate::{
    app::{maps::display_project_path, state::EditorApp},
    ui::{search::search_field, theme::*},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventConditionKind {
    FlagSet,
    FlagMissing,
    CutsceneSeen,
    CutsceneMissing,
    CodexScanned,
    CodexMissing,
    ObjectiveCheckpointDone,
    ObjectiveCheckpointMissing,
}

impl EventConditionKind {
    const ALL: [Self; 8] = [
        Self::FlagSet,
        Self::FlagMissing,
        Self::CutsceneSeen,
        Self::CutsceneMissing,
        Self::CodexScanned,
        Self::CodexMissing,
        Self::ObjectiveCheckpointDone,
        Self::ObjectiveCheckpointMissing,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::FlagSet => "FlagSet",
            Self::FlagMissing => "FlagMissing",
            Self::CutsceneSeen => "CutsceneSeen",
            Self::CutsceneMissing => "CutsceneMissing",
            Self::CodexScanned => "CodexScanned",
            Self::CodexMissing => "CodexMissing",
            Self::ObjectiveCheckpointDone => "ObjectiveCheckpointDone",
            Self::ObjectiveCheckpointMissing => "ObjectiveCheckpointMissing",
        }
    }

    fn from_condition(condition: &EventCondition) -> Self {
        match condition {
            EventCondition::FlagSet(_) => Self::FlagSet,
            EventCondition::FlagMissing(_) => Self::FlagMissing,
            EventCondition::CutsceneSeen(_) => Self::CutsceneSeen,
            EventCondition::CutsceneMissing(_) => Self::CutsceneMissing,
            EventCondition::CodexScanned(_) => Self::CodexScanned,
            EventCondition::CodexMissing(_) => Self::CodexMissing,
            EventCondition::ObjectiveCheckpointDone { .. } => Self::ObjectiveCheckpointDone,
            EventCondition::ObjectiveCheckpointMissing { .. } => Self::ObjectiveCheckpointMissing,
        }
    }

    fn default_condition(self) -> EventCondition {
        match self {
            Self::FlagSet => EventCondition::FlagSet("flag.id".to_owned()),
            Self::FlagMissing => EventCondition::FlagMissing("flag.id".to_owned()),
            Self::CutsceneSeen => EventCondition::CutsceneSeen("intro.new_game".to_owned()),
            Self::CutsceneMissing => EventCondition::CutsceneMissing("intro.new_game".to_owned()),
            Self::CodexScanned => EventCondition::CodexScanned("codex.id".to_owned()),
            Self::CodexMissing => EventCondition::CodexMissing("codex.id".to_owned()),
            Self::ObjectiveCheckpointDone => EventCondition::ObjectiveCheckpointDone {
                objective_id: "secure_landing_site".to_owned(),
                checkpoint_id: "checkpoint_id".to_owned(),
            },
            Self::ObjectiveCheckpointMissing => EventCondition::ObjectiveCheckpointMissing {
                objective_id: "secure_landing_site".to_owned(),
                checkpoint_id: "checkpoint_id".to_owned(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventActionKind {
    PlayCutscene,
    SetFlag,
    AdvanceObjective,
    ShowNotice,
}

impl EventActionKind {
    const ALL: [Self; 4] = [
        Self::PlayCutscene,
        Self::SetFlag,
        Self::AdvanceObjective,
        Self::ShowNotice,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::PlayCutscene => "PlayCutscene",
            Self::SetFlag => "SetFlag",
            Self::AdvanceObjective => "AdvanceObjective",
            Self::ShowNotice => "ShowNotice",
        }
    }

    fn from_action(action: &EventAction) -> Self {
        match action {
            EventAction::PlayCutscene(_) => Self::PlayCutscene,
            EventAction::SetFlag(_) => Self::SetFlag,
            EventAction::AdvanceObjective { .. } => Self::AdvanceObjective,
            EventAction::ShowNotice(_) => Self::ShowNotice,
        }
    }

    fn default_action(self) -> EventAction {
        match self {
            Self::PlayCutscene => EventAction::PlayCutscene("intro.new_game".to_owned()),
            Self::SetFlag => EventAction::SetFlag("flag.id".to_owned()),
            Self::AdvanceObjective => EventAction::AdvanceObjective {
                objective_id: "secure_landing_site".to_owned(),
                checkpoint_id: None,
                complete_objective: false,
            },
            Self::ShowNotice => EventAction::ShowNotice("Signal recorded.".to_owned()),
        }
    }
}

impl EditorApp {
    pub(crate) fn event_db_path(&self) -> PathBuf {
        self.project_root.join(DEFAULT_EVENT_DB_PATH)
    }

    pub(crate) fn save_event_database(&mut self) -> bool {
        let issues = self.event_database.validate(
            Some(&self.cutscene_database),
            Some(&self.objective_database),
        );
        if issues
            .iter()
            .any(|issue| issue.severity == EventValidationSeverity::Error)
        {
            self.status = "保存失败：Events 有错误".to_owned();
            return false;
        }

        self.event_database.reindex();
        let path = self.event_db_path();
        match self.event_database.save(&path) {
            Ok(()) => {
                self.event_db_dirty = false;
                self.status = format!(
                    "Events saved {}",
                    display_project_path(&self.project_root, &path)
                );
                true
            }
            Err(error) => {
                self.status = format!("Event save failed: {error:#}");
                false
            }
        }
    }

    pub(crate) fn reload_event_database(&mut self) {
        let path = self.event_db_path();
        match EventDatabase::load(&path) {
            Ok(database) => {
                self.event_database = database;
                self.event_db_dirty = false;
                self.selected_event_index = if self.event_database.events().is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.status = format!(
                    "Events reloaded {}",
                    display_project_path(&self.project_root, &path)
                );
            }
            Err(error) => {
                self.status = format!("Event reload failed: {error:#}");
            }
        }
    }

    pub(crate) fn add_event(&mut self) {
        let id = self.unique_event_id("event.new");
        let event = WorldEventDefinition {
            id,
            actions: vec![EventAction::ShowNotice("Signal recorded.".to_owned())],
            ..WorldEventDefinition::default()
        };
        self.event_database.events_mut().push(event);
        self.selected_event_index = Some(self.event_database.events().len() - 1);
        self.mark_event_database_dirty();
    }

    pub(crate) fn duplicate_selected_event(&mut self) {
        let Some(index) = self.normalized_selected_event_index() else {
            self.status = "请先选择事件".to_owned();
            return;
        };
        let mut event = self.event_database.events()[index].clone();
        event.id = self.unique_event_id(&event.id);
        self.event_database.events_mut().insert(index + 1, event);
        self.selected_event_index = Some(index + 1);
        self.mark_event_database_dirty();
    }

    pub(crate) fn delete_selected_event(&mut self) {
        let Some(index) = self.normalized_selected_event_index() else {
            self.status = "请先选择事件".to_owned();
            return;
        };
        let removed = self.event_database.events_mut().remove(index);
        let next_len = self.event_database.events().len();
        self.selected_event_index = if next_len == 0 {
            None
        } else {
            Some(index.min(next_len - 1))
        };
        self.mark_event_database_dirty();
        self.status = format!("已删除事件 {}", removed.id);
    }

    pub(crate) fn validate_event_database_command(&mut self) {
        let issues = self.event_database.validate(
            Some(&self.cutscene_database),
            Some(&self.objective_database),
        );
        let errors = issues
            .iter()
            .filter(|issue| issue.severity == EventValidationSeverity::Error)
            .count();
        let warnings = issues
            .iter()
            .filter(|issue| issue.severity == EventValidationSeverity::Warning)
            .count();
        self.status = if errors == 0 && warnings == 0 {
            "Events 校验通过".to_owned()
        } else {
            format!("Events 校验：{errors} 个错误，{warnings} 个警告")
        };
    }

    pub(crate) fn draw_event_workspace(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 8.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| self.draw_event_list_panel(ui));
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("event_detail_scroll")
                .show(ui, |ui| self.draw_event_detail_panel(ui));
        });
    }

    fn draw_event_list_panel(&mut self, ui: &mut egui::Ui) {
        ui.set_width(310.0);
        ui.heading("Events");
        ui.label(display_project_path(
            &self.project_root,
            &self.event_db_path(),
        ));
        ui.horizontal(|ui| {
            if ui.button("新增").clicked() {
                self.add_event();
            }
            if ui
                .add_enabled(
                    self.normalized_selected_event_index().is_some(),
                    egui::Button::new("复制"),
                )
                .clicked()
            {
                self.duplicate_selected_event();
            }
            if ui
                .add_enabled(
                    self.normalized_selected_event_index().is_some(),
                    egui::Button::new("删除"),
                )
                .clicked()
            {
                self.delete_selected_event();
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.event_db_dirty, egui::Button::new("保存"))
                .clicked()
            {
                self.save_event_database();
            }
            if ui.button("重载").clicked() {
                self.reload_event_database();
            }
            if ui.button("校验").clicked() {
                self.validate_event_database_command();
            }
        });
        search_field(ui, &mut self.event_search, "搜索事件");
        ui.label(format!(
            "{} events{}",
            self.event_database.events().len(),
            if self.event_db_dirty { " *" } else { "" }
        ));

        let search = self.event_search.trim().to_ascii_lowercase();
        egui::ScrollArea::vertical()
            .id_salt("event_list_scroll")
            .show(ui, |ui| {
                for (index, event) in self.event_database.events().iter().enumerate() {
                    if !search.is_empty() && !event.id.to_ascii_lowercase().contains(&search) {
                        continue;
                    }
                    let selected = self.selected_event_index == Some(index);
                    if ui.selectable_label(selected, &event.id).clicked() {
                        self.selected_event_index = Some(index);
                    }
                }
            });
    }

    fn draw_event_detail_panel(&mut self, ui: &mut egui::Ui) {
        ui.set_min_width(520.0);
        let Some(index) = self.normalized_selected_event_index() else {
            ui.heading("No Event Selected");
            return;
        };

        let original = self.event_database.events()[index].clone();
        let mut draft = original.clone();
        ui.heading(if draft.id.trim().is_empty() {
            "Untitled Event"
        } else {
            &draft.id
        });
        draw_event_definition_editor(
            ui,
            index,
            &mut draft,
            &self.cutscene_database,
            &self.objective_database,
        );
        self.draw_event_validation(ui);
        if draft != original {
            self.event_database.events_mut()[index] = draft;
            self.mark_event_database_dirty();
        }
    }

    fn draw_event_validation(&self, ui: &mut egui::Ui) {
        let issues = self.event_database.validate(
            Some(&self.cutscene_database),
            Some(&self.objective_database),
        );
        if issues.is_empty() {
            ui.colored_label(THEME_MUTED_TEXT, "校验：没有发现结构问题");
            return;
        }
        for issue in issues {
            let color = match issue.severity {
                EventValidationSeverity::Error => THEME_ERROR,
                EventValidationSeverity::Warning => THEME_WARNING,
            };
            ui.colored_label(color, issue.message);
        }
    }

    fn normalized_selected_event_index(&mut self) -> Option<usize> {
        let len = self.event_database.events().len();
        if len == 0 {
            self.selected_event_index = None;
            return None;
        }
        let index = self.selected_event_index.unwrap_or(0).min(len - 1);
        self.selected_event_index = Some(index);
        Some(index)
    }

    fn mark_event_database_dirty(&mut self) {
        self.event_db_dirty = true;
        self.event_database.reindex();
    }

    fn unique_event_id(&self, base_id: &str) -> String {
        let base = if base_id.trim().is_empty() {
            "event.new"
        } else {
            base_id.trim()
        };
        let ids = self
            .event_database
            .events()
            .iter()
            .map(|event| event.id.trim())
            .collect::<std::collections::HashSet<_>>();
        if !ids.contains(base) {
            return base.to_owned();
        }
        for index in 2..1000 {
            let candidate = format!("{base}.{index}");
            if !ids.contains(candidate.as_str()) {
                return candidate;
            }
        }
        format!("{base}.copy")
    }

    pub(crate) fn event_id_options(&self) -> Vec<String> {
        self.event_database
            .events()
            .iter()
            .map(|event| event.id.clone())
            .collect()
    }
}

fn draw_event_definition_editor(
    ui: &mut egui::Ui,
    event_index: usize,
    event: &mut WorldEventDefinition,
    cutscenes: &content::CutsceneDatabase,
    objectives: &content::ObjectiveDatabase,
) {
    property_text_edit_with_id(ui, "ID", ("event_id", event_index), &mut event.id);
    ui.horizontal(|ui| {
        ui.label("Trigger");
        egui::ComboBox::from_id_salt(("event_trigger", event_index))
            .selected_text("EnterZone")
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut event.trigger, EventTrigger::EnterZone, "EnterZone");
            });
    });
    ui.horizontal(|ui| {
        ui.label("Scope");
        egui::ComboBox::from_id_salt(("event_scope", event_index))
            .selected_text(format!("{:?}", event.scope))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut event.scope, EventScope::OncePerZone, "OncePerZone");
                ui.selectable_value(&mut event.scope, EventScope::WorldOnce, "WorldOnce");
                ui.selectable_value(&mut event.scope, EventScope::Repeatable, "Repeatable");
            });
    });

    ui.separator();
    ui.heading("Conditions");
    draw_condition_add_buttons(ui, &mut event.conditions);
    for index in (0..event.conditions.len()).rev() {
        let mut remove = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            draw_condition_editor(
                ui,
                event_index,
                index,
                &mut event.conditions[index],
                objectives,
            );
            if ui.button("删除条件").clicked() {
                remove = true;
            }
        });
        if remove {
            event.conditions.remove(index);
        }
    }

    ui.separator();
    ui.heading("Actions");
    draw_action_add_buttons(ui, &mut event.actions);
    for index in (0..event.actions.len()).rev() {
        let mut remove = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            draw_action_editor(
                ui,
                event_index,
                index,
                &mut event.actions[index],
                cutscenes,
                objectives,
            );
            if ui.button("删除动作").clicked() {
                remove = true;
            }
        });
        if remove {
            event.actions.remove(index);
        }
    }
}

fn draw_condition_add_buttons(ui: &mut egui::Ui, conditions: &mut Vec<EventCondition>) {
    ui.horizontal_wrapped(|ui| {
        for kind in EventConditionKind::ALL {
            if ui.button(format!("+ {}", kind.label())).clicked() {
                conditions.push(kind.default_condition());
            }
        }
    });
}

fn draw_action_add_buttons(ui: &mut egui::Ui, actions: &mut Vec<EventAction>) {
    ui.horizontal_wrapped(|ui| {
        for kind in EventActionKind::ALL {
            if ui.button(format!("+ {}", kind.label())).clicked() {
                actions.push(kind.default_action());
            }
        }
    });
}

fn draw_condition_editor(
    ui: &mut egui::Ui,
    event_index: usize,
    condition_index: usize,
    condition: &mut EventCondition,
    objectives: &content::ObjectiveDatabase,
) {
    let mut kind = EventConditionKind::from_condition(condition);
    ui.horizontal(|ui| {
        ui.label("Kind");
        egui::ComboBox::from_id_salt(("event_condition_kind", event_index, condition_index))
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                for option in EventConditionKind::ALL {
                    ui.selectable_value(&mut kind, option, option.label());
                }
            });
    });
    if kind != EventConditionKind::from_condition(condition) {
        *condition = kind.default_condition();
    }

    match condition {
        EventCondition::FlagSet(value)
        | EventCondition::FlagMissing(value)
        | EventCondition::CutsceneSeen(value)
        | EventCondition::CutsceneMissing(value)
        | EventCondition::CodexScanned(value)
        | EventCondition::CodexMissing(value) => {
            property_text_edit_with_id(
                ui,
                "ID",
                ("event_condition_value", event_index, condition_index),
                value,
            );
        }
        EventCondition::ObjectiveCheckpointDone {
            objective_id,
            checkpoint_id,
        }
        | EventCondition::ObjectiveCheckpointMissing {
            objective_id,
            checkpoint_id,
        } => draw_objective_checkpoint_picker(
            ui,
            ("event_condition_objective", event_index, condition_index),
            objective_id,
            checkpoint_id,
            objectives,
        ),
    }
}

fn draw_action_editor(
    ui: &mut egui::Ui,
    event_index: usize,
    action_index: usize,
    action: &mut EventAction,
    cutscenes: &content::CutsceneDatabase,
    objectives: &content::ObjectiveDatabase,
) {
    let mut kind = EventActionKind::from_action(action);
    ui.horizontal(|ui| {
        ui.label("Kind");
        egui::ComboBox::from_id_salt(("event_action_kind", event_index, action_index))
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                for option in EventActionKind::ALL {
                    ui.selectable_value(&mut kind, option, option.label());
                }
            });
    });
    if kind != EventActionKind::from_action(action) {
        *action = kind.default_action();
    }

    match action {
        EventAction::PlayCutscene(cutscene_id) => {
            draw_cutscene_picker(
                ui,
                ("event_action_cutscene", event_index, action_index),
                cutscene_id,
                cutscenes,
            );
        }
        EventAction::SetFlag(flag) => {
            property_text_edit_with_id(
                ui,
                "Flag",
                ("event_action_flag", event_index, action_index),
                flag,
            );
        }
        EventAction::AdvanceObjective {
            objective_id,
            checkpoint_id,
            complete_objective,
        } => {
            let mut checkpoint = checkpoint_id.clone().unwrap_or_default();
            draw_objective_checkpoint_picker(
                ui,
                ("event_action_objective", event_index, action_index),
                objective_id,
                &mut checkpoint,
                objectives,
            );
            set_optional_string(checkpoint_id, checkpoint);
            ui.checkbox(complete_objective, "完成整个目标");
        }
        EventAction::ShowNotice(message) => {
            property_text_edit_with_id(
                ui,
                "Notice",
                ("event_action_notice", event_index, action_index),
                message,
            );
        }
    }
}

fn draw_cutscene_picker(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    cutscene_id: &mut String,
    cutscenes: &content::CutsceneDatabase,
) {
    property_text_edit_with_id(ui, "Cutscene ID", (&id, "text"), cutscene_id);
    egui::ComboBox::from_id_salt((&id, "combo"))
        .selected_text(if cutscene_id.trim().is_empty() {
            "选择 Cutscene"
        } else {
            cutscene_id.as_str()
        })
        .show_ui(ui, |ui| {
            for cutscene in cutscenes.cutscenes() {
                ui.selectable_value(cutscene_id, cutscene.id.clone(), &cutscene.id);
            }
        });
}

fn draw_objective_checkpoint_picker(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    objective_id: &mut String,
    checkpoint_id: &mut String,
    objectives: &content::ObjectiveDatabase,
) {
    property_text_edit_with_id(ui, "Objective ID", (&id, "objective_text"), objective_id);
    egui::ComboBox::from_id_salt((&id, "objective_combo"))
        .selected_text(if objective_id.trim().is_empty() {
            "选择 Objective"
        } else {
            objective_id.as_str()
        })
        .show_ui(ui, |ui| {
            for objective in objectives.objectives() {
                ui.selectable_value(objective_id, objective.id.clone(), &objective.id);
            }
        });
    property_text_edit_with_id(ui, "Checkpoint ID", (&id, "checkpoint_text"), checkpoint_id);
    if let Some(objective) = objectives.get(objective_id) {
        egui::ComboBox::from_id_salt((&id, "checkpoint_combo"))
            .selected_text(if checkpoint_id.trim().is_empty() {
                "选择 Checkpoint"
            } else {
                checkpoint_id.as_str()
            })
            .show_ui(ui, |ui| {
                for checkpoint in &objective.checkpoints {
                    ui.selectable_value(checkpoint_id, checkpoint.id.clone(), &checkpoint.id);
                }
            });
    }
}

fn property_text_edit_with_id(
    ui: &mut egui::Ui,
    label: &str,
    id: impl std::hash::Hash,
    value: &mut String,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui
            .push_id(id, |ui| ui.text_edit_singleline(value).changed())
            .inner;
    });
    changed
}

fn set_optional_string(target: &mut Option<String>, value: String) {
    let value = value.trim().to_owned();
    *target = (!value.is_empty()).then_some(value);
}
