use std::path::PathBuf;

use content::{
    DEFAULT_EVENT_DB_PATH, EventAction, EventCondition, EventDatabase, EventScope, EventTrigger,
    EventValidationSeverity, WorldEventDefinition,
};
use eframe::egui;

use crate::{
    app::{maps::display_project_path, state::EditorApp},
    ui::{
        command_bar::{
            CommandBadgeStatus, command_bar, command_button, command_status_badge,
            enabled_command_button,
        },
        panel_surface::{detail_surface, empty_state, panel_header, panel_surface},
        property_grid::{helper_text, multiline_field, picker_field, property_row, text_field},
        resource_list::{resource_list_header, resource_row, resource_search},
        rule_card::{add_rule_menu, card_gap, card_section_header, compact_card_button, rule_card},
        theme::*,
        validation_panel::{ValidationLevel, ValidationMessage, info_panel, validation_panel},
    },
};

const EVENT_LIST_WIDTH: f32 = 320.0;
const EVENT_REFERENCE_WIDTH: f32 = 360.0;

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
        let height = ui.available_height();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(EVENT_LIST_WIDTH, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| panel_surface(ui, |ui| self.draw_event_list_panel(ui)),
            );
            ui.separator();
            let main_width = (ui.available_width() - EVENT_REFERENCE_WIDTH - 7.0).max(420.0);
            ui.allocate_ui_with_layout(
                egui::vec2(main_width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    panel_surface(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("event_detail_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.draw_event_detail_panel(ui));
                    });
                },
            );
            ui.separator();
            ui.allocate_ui_with_layout(
                egui::vec2(EVENT_REFERENCE_WIDTH.min(ui.available_width()), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| panel_surface(ui, |ui| self.draw_event_reference_panel(ui)),
            );
        });
    }

    fn draw_event_list_panel(&mut self, ui: &mut egui::Ui) {
        let search = self.event_search.trim().to_ascii_lowercase();
        let visible_count = self
            .event_database
            .events()
            .iter()
            .filter(|event| search.is_empty() || event.id.to_ascii_lowercase().contains(&search))
            .count();
        resource_list_header(
            ui,
            "Events",
            &display_project_path(&self.project_root, &self.event_db_path()),
            visible_count,
            self.event_database.events().len(),
            self.event_db_dirty,
        );
        command_bar(ui, |ui| {
            if command_button(ui, "新增").clicked() {
                self.add_event();
            }
            if enabled_command_button(ui, self.normalized_selected_event_index().is_some(), "复制")
                .clicked()
            {
                self.duplicate_selected_event();
            }
            if enabled_command_button(ui, self.normalized_selected_event_index().is_some(), "删除")
                .clicked()
            {
                self.delete_selected_event();
            }
        });
        command_bar(ui, |ui| {
            if enabled_command_button(ui, self.event_db_dirty, "保存").clicked() {
                self.save_event_database();
            }
            if command_button(ui, "重载").clicked() {
                self.reload_event_database();
            }
            if command_button(ui, "校验").clicked() {
                self.validate_event_database_command();
            }
            command_status_badge(
                ui,
                if self.event_db_dirty {
                    "dirty"
                } else {
                    "clean"
                },
                if self.event_db_dirty {
                    CommandBadgeStatus::Dirty
                } else {
                    CommandBadgeStatus::Ok
                },
            );
        });
        ui.separator();
        resource_search(ui, &mut self.event_search, "搜索事件 id");

        egui::ScrollArea::vertical()
            .id_salt("event_list_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, event) in self.event_database.events().iter().enumerate() {
                    if !search.is_empty() && !event.id.to_ascii_lowercase().contains(&search) {
                        continue;
                    }
                    let selected = self.selected_event_index == Some(index);
                    let detail = format!(
                        "{} conditions / {} actions / {:?}",
                        event.conditions.len(),
                        event.actions.len(),
                        event.scope
                    );
                    let badge = if event.actions.is_empty() {
                        vec![crate::ui::tree::TreeBadge {
                            label: "warn",
                            color: THEME_WARNING,
                        }]
                    } else {
                        Vec::new()
                    };
                    if resource_row(ui, selected, &event.id, &detail, badge).clicked() {
                        self.selected_event_index = Some(index);
                    }
                }
            });
    }

    fn draw_event_detail_panel(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.normalized_selected_event_index() else {
            empty_state(
                ui,
                "No Event Selected",
                "在左侧新建或选择一个事件，然后配置触发条件和动作。",
            );
            return;
        };

        let original = self.event_database.events()[index].clone();
        let mut draft = original.clone();
        panel_header(
            ui,
            if draft.id.trim().is_empty() {
                "Untitled Event"
            } else {
                &draft.id
            },
            Some("EnterZone driven world event"),
        );
        draw_event_definition_editor(
            ui,
            index,
            &mut draft,
            &self.cutscene_database,
            &self.objective_database,
        );
        if draft != original {
            self.event_database.events_mut()[index] = draft;
            self.mark_event_database_dirty();
        }
    }

    fn draw_event_reference_panel(&self, ui: &mut egui::Ui) {
        let messages = self.event_validation_messages();
        validation_panel(ui, "Validation", &messages);
        ui.add_space(8.0);

        let selected_event = self
            .selected_event_index
            .and_then(|index| self.event_database.events().get(index));
        let Some(event) = selected_event else {
            info_panel(
                ui,
                "References",
                ["选择事件后，这里会显示当前地图中的 Zone 引用。".to_owned()],
            );
            return;
        };

        let mut zones = self
            .document
            .layers
            .zones
            .iter()
            .filter(|zone| zone.event_id.as_deref() == Some(event.id.as_str()))
            .map(|zone| zone.id.clone())
            .collect::<Vec<_>>();
        zones.sort();
        let mut lines = vec![
            format!("Event: {}", event.id),
            format!("Scope: {:?}", event.scope),
            format!("Current map zones: {}", zones.len()),
        ];
        if zones.is_empty() {
            lines.push(
                "没有 Zone 引用当前事件。请在地图 Zone Inspector 的 Event 字段选择它。".to_owned(),
            );
        } else {
            lines.extend(
                zones
                    .into_iter()
                    .take(8)
                    .map(|zone| format!("Zone: {zone}")),
            );
        }
        info_panel(ui, "References", lines);
    }

    fn event_validation_messages(&self) -> Vec<ValidationMessage> {
        self.event_database
            .validate(
                Some(&self.cutscene_database),
                Some(&self.objective_database),
            )
            .into_iter()
            .map(|issue| ValidationMessage {
                level: match issue.severity {
                    EventValidationSeverity::Error => ValidationLevel::Error,
                    EventValidationSeverity::Warning => ValidationLevel::Warning,
                },
                message: issue.message,
            })
            .collect()
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
    detail_surface(ui, |ui| {
        ui.heading("Definition");
        ui.separator();
        text_field(ui, "ID", ("event_id", event_index), &mut event.id);
        property_row(ui, "Trigger", |ui| {
            egui::ComboBox::from_id_salt(("event_trigger", event_index))
                .selected_text("EnterZone")
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut event.trigger, EventTrigger::EnterZone, "EnterZone");
                });
        });
        property_row(ui, "Scope", |ui| {
            egui::ComboBox::from_id_salt(("event_scope", event_index))
                .selected_text(format!("{:?}", event.scope))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut event.scope, EventScope::OncePerZone, "OncePerZone");
                    ui.selectable_value(&mut event.scope, EventScope::WorldOnce, "WorldOnce");
                    ui.selectable_value(&mut event.scope, EventScope::Repeatable, "Repeatable");
                });
        });
        helper_text(
            ui,
            "Zone 只引用 event_id；条件和动作由运行时解释。无条件表示 always allowed。",
        );
    });

    ui.add_space(10.0);
    card_section_header(ui, "Conditions", event.conditions.len());
    add_rule_menu(
        ui,
        "+ Add Condition",
        &EventConditionKind::ALL,
        EventConditionKind::label,
        |kind| event.conditions.push(kind.default_condition()),
    );
    if event.conditions.is_empty() {
        helper_text(ui, "无条件：玩家进入引用该 event_id 的 Zone 时即可触发。");
    }
    for index in (0..event.conditions.len()).rev() {
        let mut remove = false;
        let kind_label = EventConditionKind::from_condition(&event.conditions[index]).label();
        rule_card(
            ui,
            index + 1,
            kind_label,
            "Condition",
            |ui| {
                if compact_card_button(ui, "删除").clicked() {
                    remove = true;
                }
            },
            |ui| {
                draw_condition_editor(
                    ui,
                    event_index,
                    index,
                    &mut event.conditions[index],
                    objectives,
                );
            },
        );
        if remove {
            event.conditions.remove(index);
        }
        card_gap(ui);
    }

    ui.add_space(10.0);
    card_section_header(ui, "Actions", event.actions.len());
    add_rule_menu(
        ui,
        "+ Add Action",
        &EventActionKind::ALL,
        EventActionKind::label,
        |kind| event.actions.push(kind.default_action()),
    );
    if event.actions.is_empty() {
        ui.colored_label(
            THEME_WARNING,
            "没有 action：事件会通过校验警告，运行时不会产生效果。",
        );
    }
    for index in (0..event.actions.len()).rev() {
        let mut remove = false;
        let kind_label = EventActionKind::from_action(&event.actions[index]).label();
        rule_card(
            ui,
            index + 1,
            kind_label,
            "Action",
            |ui| {
                if compact_card_button(ui, "删除").clicked() {
                    remove = true;
                }
            },
            |ui| {
                draw_action_editor(
                    ui,
                    event_index,
                    index,
                    &mut event.actions[index],
                    cutscenes,
                    objectives,
                );
            },
        );
        if remove {
            event.actions.remove(index);
        }
        card_gap(ui);
    }
}

fn draw_condition_editor(
    ui: &mut egui::Ui,
    event_index: usize,
    condition_index: usize,
    condition: &mut EventCondition,
    objectives: &content::ObjectiveDatabase,
) {
    let mut kind = EventConditionKind::from_condition(condition);
    property_row(ui, "Kind", |ui| {
        egui::ComboBox::from_id_salt(("event_condition_kind", event_index, condition_index))
            .selected_text(kind.label())
            .width(ui.available_width())
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
        | EventCondition::CodexScanned(value)
        | EventCondition::CodexMissing(value) => {
            text_field(
                ui,
                "ID",
                ("event_condition_value", event_index, condition_index),
                value,
            );
        }
        EventCondition::CutsceneSeen(value) | EventCondition::CutsceneMissing(value) => {
            text_field(
                ui,
                "Cutscene ID",
                ("event_condition_cutscene", event_index, condition_index),
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
    property_row(ui, "Kind", |ui| {
        egui::ComboBox::from_id_salt(("event_action_kind", event_index, action_index))
            .selected_text(kind.label())
            .width(ui.available_width())
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
            text_field(
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
            property_row(ui, "Complete", |ui| {
                ui.checkbox(complete_objective, "完成整个目标");
            });
        }
        EventAction::ShowNotice(message) => {
            multiline_field(
                ui,
                "Notice",
                ("event_action_notice", event_index, action_index),
                message,
                3,
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
    let options = cutscenes
        .cutscenes()
        .iter()
        .map(|cutscene| cutscene.id.clone())
        .collect::<Vec<_>>();
    picker_field(
        ui,
        "Cutscene ID",
        id,
        cutscene_id,
        "选择 Cutscene",
        &options,
    );
}

fn draw_objective_checkpoint_picker(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    objective_id: &mut String,
    checkpoint_id: &mut String,
    objectives: &content::ObjectiveDatabase,
) {
    let objective_options = objectives
        .objectives()
        .iter()
        .map(|objective| objective.id.clone())
        .collect::<Vec<_>>();
    picker_field(
        ui,
        "Objective ID",
        (&id, "objective"),
        objective_id,
        "选择 Objective",
        &objective_options,
    );
    let checkpoint_options = objectives
        .get(objective_id)
        .map(|objective| {
            objective
                .checkpoints
                .iter()
                .map(|checkpoint| checkpoint.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    picker_field(
        ui,
        "Checkpoint ID",
        (&id, "checkpoint"),
        checkpoint_id,
        "选择 Checkpoint",
        &checkpoint_options,
    );
}

fn set_optional_string(target: &mut Option<String>, value: String) {
    let value = value.trim().to_owned();
    *target = (!value.is_empty()).then_some(value);
}
