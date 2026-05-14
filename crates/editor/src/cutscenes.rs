use std::collections::HashSet;

use content::{CutsceneCompletion, CutsceneDefinition, CutsceneStep, CutsceneText};

use super::*;
use crate::ui::{
    command_bar::{
        CommandBadgeStatus, command_bar, command_button, command_status_badge,
        enabled_command_button,
    },
    fields::property_row,
    panel_surface::{
        detail_surface, empty_state, panel_header as surface_panel_header, panel_surface,
    },
    property_grid,
    resource_list::{resource_list_header, resource_row, resource_search},
    rule_card::{add_rule_menu, card_gap, card_section_header, compact_card_button, rule_card},
    validation_panel::{ValidationLevel, ValidationMessage, info_panel, validation_panel},
};

const DEFAULT_SCENE_KEYS: [&str; 3] = ["Overworld", "Facility", "MainMenu"];
const CUTSCENE_LIST_WIDTH: f32 = 330.0;
const CUTSCENE_REFERENCE_WIDTH: f32 = 360.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CutsceneStepKind {
    FadeIn,
    FadeOut,
    Wait,
    TextPanel,
    SetFlag,
}

impl CutsceneStepKind {
    const ALL: [Self; 5] = [
        Self::FadeIn,
        Self::FadeOut,
        Self::Wait,
        Self::TextPanel,
        Self::SetFlag,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::FadeIn => "Fade In",
            Self::FadeOut => "Fade Out",
            Self::Wait => "Wait",
            Self::TextPanel => "Text Panel",
            Self::SetFlag => "Set Flag",
        }
    }

    fn default_step(self) -> CutsceneStep {
        match self {
            Self::FadeIn => CutsceneStep::FadeIn { duration: 0.35 },
            Self::FadeOut => CutsceneStep::FadeOut { duration: 0.35 },
            Self::Wait => CutsceneStep::Wait { duration: 0.5 },
            Self::TextPanel => CutsceneStep::TextPanel {
                speaker: Some(CutsceneText::new("AI", "AI")),
                body: CutsceneText::new("新的传输。", "新的传输。"),
                min_duration: 0.25,
                require_confirm: true,
            },
            Self::SetFlag => CutsceneStep::SetFlag {
                flag: "story.flag".to_owned(),
            },
        }
    }

    fn from_step(step: &CutsceneStep) -> Self {
        match step {
            CutsceneStep::FadeIn { .. } => Self::FadeIn,
            CutsceneStep::FadeOut { .. } => Self::FadeOut,
            CutsceneStep::Wait { .. } => Self::Wait,
            CutsceneStep::TextPanel { .. } => Self::TextPanel,
            CutsceneStep::SetFlag { .. } => Self::SetFlag,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CutsceneValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
struct CutsceneValidationIssue {
    severity: CutsceneValidationSeverity,
    message: String,
}

enum StepAction {
    MoveUp,
    MoveDown,
    Delete,
}

impl EditorApp {
    pub(crate) fn cutscene_db_path(&self) -> PathBuf {
        self.project_root.join(DEFAULT_CUTSCENE_DB_PATH)
    }

    pub(crate) fn save_cutscene_database(&mut self) -> bool {
        let issues = self.cutscene_validation_issues();
        if issues
            .iter()
            .any(|issue| issue.severity == CutsceneValidationSeverity::Error)
        {
            self.status = "保存失败：过场动画有错误".to_owned();
            return false;
        }

        self.cutscene_database.reindex();
        let path = self.cutscene_db_path();
        match self.cutscene_database.save(&path) {
            Ok(()) => {
                self.cutscene_db_dirty = false;
                self.status = format!(
                    "Cutscenes saved {}",
                    display_project_path(&self.project_root, &path)
                );
                true
            }
            Err(error) => {
                self.status = format!("Cutscene save failed: {error:#}");
                false
            }
        }
    }

    pub(crate) fn reload_cutscene_database(&mut self) {
        let path = self.cutscene_db_path();
        match CutsceneDatabase::load(&path) {
            Ok(database) => {
                self.cutscene_database = database;
                self.cutscene_db_dirty = false;
                self.selected_cutscene_index = if self.cutscene_database.cutscenes().is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.status = format!(
                    "Cutscenes reloaded {}",
                    display_project_path(&self.project_root, &path)
                );
            }
            Err(error) => {
                self.status = format!("Cutscene reload failed: {error:#}");
            }
        }
    }

    pub(crate) fn add_cutscene(&mut self) {
        let id = self.unique_cutscene_id("cutscene.new");
        let mut cutscene = CutsceneDefinition {
            id,
            ..CutsceneDefinition::default()
        };
        cutscene
            .steps
            .push(CutsceneStepKind::FadeOut.default_step());
        cutscene
            .steps
            .push(CutsceneStepKind::TextPanel.default_step());
        cutscene.steps.push(CutsceneStepKind::FadeIn.default_step());

        self.cutscene_database.cutscenes_mut().push(cutscene);
        self.selected_cutscene_index = Some(self.cutscene_database.cutscenes().len() - 1);
        self.mark_cutscene_database_dirty();
    }

    pub(crate) fn duplicate_selected_cutscene(&mut self) {
        let Some(index) = self.normalized_selected_cutscene_index() else {
            self.status = "请先选择过场动画".to_owned();
            return;
        };

        let mut cutscene = self.cutscene_database.cutscenes()[index].clone();
        cutscene.id = self.unique_cutscene_id(&cutscene.id);
        self.cutscene_database
            .cutscenes_mut()
            .insert(index + 1, cutscene);
        self.selected_cutscene_index = Some(index + 1);
        self.mark_cutscene_database_dirty();
    }

    pub(crate) fn delete_selected_cutscene(&mut self) {
        let Some(index) = self.normalized_selected_cutscene_index() else {
            self.status = "请先选择过场动画".to_owned();
            return;
        };

        let removed = self.cutscene_database.cutscenes_mut().remove(index);
        let next_len = self.cutscene_database.cutscenes().len();
        self.selected_cutscene_index = if next_len == 0 {
            None
        } else {
            Some(index.min(next_len - 1))
        };
        self.mark_cutscene_database_dirty();
        self.status = format!("已删除过场 {}", removed.id);
    }

    pub(crate) fn validate_cutscene_database_command(&mut self) {
        let issues = self.cutscene_validation_issues();
        let errors = issues
            .iter()
            .filter(|issue| issue.severity == CutsceneValidationSeverity::Error)
            .count();
        let warnings = issues
            .iter()
            .filter(|issue| issue.severity == CutsceneValidationSeverity::Warning)
            .count();

        self.status = if errors == 0 && warnings == 0 {
            "Cutscenes 校验通过".to_owned()
        } else {
            format!("Cutscenes 校验：{errors} 个错误，{warnings} 个警告")
        };
    }

    pub(crate) fn draw_cutscene_workspace(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.allocate_ui_with_layout(
                vec2(CUTSCENE_LIST_WIDTH, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| panel_surface(ui, |ui| self.draw_cutscene_list_panel(ui)),
            );
            ui.separator();
            let main_width = (ui.available_width() - CUTSCENE_REFERENCE_WIDTH - 7.0).max(420.0);
            ui.allocate_ui_with_layout(
                vec2(main_width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    panel_surface(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("cutscene_detail_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.draw_cutscene_detail_panel(ui));
                    });
                },
            );
            ui.separator();
            ui.allocate_ui_with_layout(
                vec2(CUTSCENE_REFERENCE_WIDTH.min(ui.available_width()), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| panel_surface(ui, |ui| self.draw_cutscene_reference_panel(ui)),
            );
        });
    }

    fn draw_cutscene_list_scroll(&mut self, ui: &mut egui::Ui, rows: Vec<CutsceneListRow>) {
        egui::ScrollArea::vertical()
            .id_salt("cutscene_list_scroll")
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                for row in rows {
                    let selected = self.selected_cutscene_index == Some(row.index);
                    let label = if row.id.trim().is_empty() {
                        "<empty id>".to_owned()
                    } else {
                        row.id.clone()
                    };
                    let detail = format!("{} steps / {}", row.step_count, row.completion);
                    let response =
                        resource_row(ui, selected, &label, &detail, Vec::new()).on_hover_text(
                            format!("{} steps / completion {}", row.step_count, row.completion),
                        );
                    if response.clicked() {
                        self.selected_cutscene_index = Some(row.index);
                    }
                }
            });
    }

    fn draw_cutscene_list_panel(&mut self, ui: &mut egui::Ui) {
        let search = self.cutscene_search.trim().to_ascii_lowercase();
        let visible_count = self
            .cutscene_database
            .cutscenes()
            .iter()
            .filter(|cutscene| cutscene_matches_search(cutscene, &search))
            .count();
        resource_list_header(
            ui,
            "Cutscenes",
            &display_project_path(&self.project_root, &self.cutscene_db_path()),
            visible_count,
            self.cutscene_database.cutscenes().len(),
            self.cutscene_db_dirty,
        );
        command_bar(ui, |ui| {
            if command_button(ui, "新增").clicked() {
                self.add_cutscene();
            }
            if enabled_command_button(
                ui,
                self.normalized_selected_cutscene_index().is_some(),
                "复制",
            )
            .clicked()
            {
                self.duplicate_selected_cutscene();
            }
            if enabled_command_button(
                ui,
                self.normalized_selected_cutscene_index().is_some(),
                "删除",
            )
            .clicked()
            {
                self.delete_selected_cutscene();
            }
        });
        command_bar(ui, |ui| {
            if enabled_command_button(ui, self.cutscene_db_dirty, "保存").clicked() {
                self.save_cutscene_database();
            }
            if command_button(ui, "重载").clicked() {
                self.reload_cutscene_database();
            }
            if command_button(ui, "校验").clicked() {
                self.validate_cutscene_database_command();
            }
            command_status_badge(
                ui,
                if self.cutscene_db_dirty {
                    "dirty"
                } else {
                    "clean"
                },
                if self.cutscene_db_dirty {
                    CommandBadgeStatus::Dirty
                } else {
                    CommandBadgeStatus::Ok
                },
            );
        });
        ui.separator();
        resource_search(
            ui,
            &mut self.cutscene_search,
            "搜索 id / 文本 / flag / scene",
        );
        ui.separator();

        let rows = self
            .cutscene_database
            .cutscenes()
            .iter()
            .enumerate()
            .filter(|(_, cutscene)| cutscene_matches_search(cutscene, &search))
            .map(|(index, cutscene)| CutsceneListRow {
                index,
                id: cutscene.id.clone(),
                step_count: cutscene.steps.len(),
                completion: cutscene_completion_label(&cutscene.completion).to_owned(),
            })
            .collect::<Vec<_>>();

        self.draw_cutscene_list_scroll(ui, rows);
    }

    fn draw_cutscene_detail_panel(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.normalized_selected_cutscene_index() else {
            empty_state(
                ui,
                "No Cutscene Selected",
                "新建或选择一个 cutscene 后，可以在这里编辑步骤。",
            );
            return;
        };

        let original = self.cutscene_database.cutscenes()[index].clone();
        let mut draft = original.clone();

        surface_panel_header(
            ui,
            if draft.id.trim().is_empty() {
                "Untitled Cutscene"
            } else {
                draft.id.as_str()
            },
            Some("blocking sequence"),
        );

        detail_surface(ui, |ui| {
            draw_cutscene_definition_editor(ui, index, &mut draft)
        });

        ui.add_space(8.0);
        card_section_header(ui, "Steps", draft.steps.len());
        draw_step_add_buttons(ui, &mut draft.steps);
        egui::ScrollArea::vertical()
            .id_salt(("cutscene_steps_scroll", index))
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                draw_steps_editor(ui, index, &mut draft.steps);
            });

        if draft != original
            && self
                .cutscene_database
                .cutscenes_mut()
                .get_mut(index)
                .is_some()
        {
            self.cutscene_database.cutscenes_mut()[index] = draft;
            self.mark_cutscene_database_dirty();
        }
    }

    fn draw_cutscene_reference_panel(&self, ui: &mut egui::Ui) {
        let messages = self.cutscene_validation_messages();
        validation_panel(ui, "Validation", &messages);
        ui.add_space(8.0);
        let selected = self
            .selected_cutscene_index
            .and_then(|index| self.cutscene_database.cutscenes().get(index));
        let lines = selected
            .map(|cutscene| {
                vec![
                    format!("Cutscene: {}", cutscene.id),
                    format!("Steps: {}", cutscene.steps.len()),
                    format!(
                        "Completion: {}",
                        cutscene_completion_label(&cutscene.completion)
                    ),
                    format!("Blocking: {}", cutscene.blocking),
                    format!("Play once: {}", cutscene.play_once),
                ]
            })
            .unwrap_or_else(|| vec!["选择 cutscene 后，这里会显示播放和完成行为。".to_owned()]);
        info_panel(ui, "Playback", lines);
    }

    fn cutscene_validation_messages(&self) -> Vec<ValidationMessage> {
        self.cutscene_validation_issues()
            .into_iter()
            .map(|issue| ValidationMessage {
                level: match issue.severity {
                    CutsceneValidationSeverity::Error => ValidationLevel::Error,
                    CutsceneValidationSeverity::Warning => ValidationLevel::Warning,
                },
                message: issue.message,
            })
            .collect()
    }

    fn normalized_selected_cutscene_index(&mut self) -> Option<usize> {
        let len = self.cutscene_database.cutscenes().len();
        if len == 0 {
            self.selected_cutscene_index = None;
            return None;
        }

        let index = self.selected_cutscene_index.unwrap_or(0).min(len - 1);
        self.selected_cutscene_index = Some(index);
        Some(index)
    }

    fn mark_cutscene_database_dirty(&mut self) {
        self.cutscene_db_dirty = true;
        self.cutscene_database.reindex();
    }

    fn unique_cutscene_id(&self, base_id: &str) -> String {
        let base = if base_id.trim().is_empty() {
            "cutscene.new"
        } else {
            base_id.trim()
        };
        let ids = self
            .cutscene_database
            .cutscenes()
            .iter()
            .map(|cutscene| cutscene.id.trim())
            .collect::<HashSet<_>>();
        if !ids.contains(base) {
            return base.to_owned();
        }

        for suffix in 2..1000 {
            let candidate = format!("{base}.{suffix}");
            if !ids.contains(candidate.as_str()) {
                return candidate;
            }
        }
        format!("{base}.copy")
    }

    fn cutscene_validation_issues(&self) -> Vec<CutsceneValidationIssue> {
        let mut issues = Vec::new();
        let mut ids = HashSet::new();
        for cutscene in self.cutscene_database.cutscenes() {
            let id = cutscene.id.trim();
            if id.is_empty() {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Error,
                    message: "Cutscene id 不能为空".to_owned(),
                });
                continue;
            }
            if !ids.insert(id.to_owned()) {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Error,
                    message: format!("Cutscene id 重复：{id}"),
                });
            }
            if cutscene.steps.is_empty() {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Warning,
                    message: format!("{id} 没有任何步骤"),
                });
            }
            for (index, step) in cutscene.steps.iter().enumerate() {
                validate_cutscene_step(id, index, step, &mut issues);
            }
        }
        issues
    }
}

struct CutsceneListRow {
    index: usize,
    id: String,
    step_count: usize,
    completion: String,
}

fn draw_cutscene_definition_editor(
    ui: &mut egui::Ui,
    cutscene_index: usize,
    cutscene: &mut CutsceneDefinition,
) {
    property_text_edit_with_id(ui, "ID", ("cutscene_id", cutscene_index), &mut cutscene.id);
    property_row(ui, "Playback", |ui| {
        ui.checkbox(&mut cutscene.blocking, "blocking");
        ui.checkbox(&mut cutscene.play_once, "play once");
    });
    draw_completion_editor(ui, cutscene_index, &mut cutscene.completion);
}

fn draw_completion_editor(
    ui: &mut egui::Ui,
    cutscene_index: usize,
    completion: &mut CutsceneCompletion,
) {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum CompletionKind {
        Pop,
        SwitchScene,
    }

    let mut kind = match completion {
        CutsceneCompletion::Pop => CompletionKind::Pop,
        CutsceneCompletion::SwitchScene { .. } => CompletionKind::SwitchScene,
    };
    property_row(ui, "完成后", |ui| {
        egui::ComboBox::from_id_salt(("cutscene_completion_kind", cutscene_index))
            .selected_text(match kind {
                CompletionKind::Pop => "Pop",
                CompletionKind::SwitchScene => "Switch Scene",
            })
            .width(160.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut kind, CompletionKind::Pop, "Pop");
                ui.selectable_value(&mut kind, CompletionKind::SwitchScene, "Switch Scene");
            });
    });

    match (kind, &*completion) {
        (CompletionKind::Pop, CutsceneCompletion::Pop) => {}
        (CompletionKind::Pop, _) => *completion = CutsceneCompletion::Pop,
        (CompletionKind::SwitchScene, CutsceneCompletion::SwitchScene { .. }) => {}
        (CompletionKind::SwitchScene, _) => {
            *completion = CutsceneCompletion::SwitchScene {
                scene: "Overworld".to_owned(),
            };
        }
    }

    if let CutsceneCompletion::SwitchScene { scene } = completion {
        property_text_edit_with_id(
            ui,
            "Scene",
            ("cutscene_completion_scene_text", cutscene_index),
            scene,
        );
        property_row(ui, "常用", |ui| {
            egui::ComboBox::from_id_salt(("cutscene_completion_scene", cutscene_index))
                .selected_text("选择场景")
                .width(160.0)
                .show_ui(ui, |ui| {
                    for option in DEFAULT_SCENE_KEYS {
                        if ui.selectable_label(scene == option, option).clicked() {
                            *scene = option.to_owned();
                            ui.close();
                        }
                    }
                });
        });
    }
}

fn draw_step_add_buttons(ui: &mut egui::Ui, steps: &mut Vec<CutsceneStep>) {
    add_rule_menu(
        ui,
        "+ Add Step",
        &CutsceneStepKind::ALL,
        CutsceneStepKind::label,
        |kind| steps.push(kind.default_step()),
    );
}

fn draw_steps_editor(ui: &mut egui::Ui, cutscene_index: usize, steps: &mut Vec<CutsceneStep>) {
    if steps.is_empty() {
        ui.colored_label(THEME_MUTED_TEXT, "还没有步骤。");
        return;
    }

    let mut index = 0usize;
    while index < steps.len() {
        let mut action = None;
        let title = CutsceneStepKind::from_step(&steps[index]).label();
        let can_move_up = index > 0;
        let can_move_down = index + 1 < steps.len();
        rule_card(
            ui,
            index + 1,
            title,
            "Step",
            |ui| {
                if compact_card_button(ui, "删除").clicked() {
                    action = Some(StepAction::Delete);
                }
                if ui
                    .add_enabled(can_move_down, egui::Button::new("下移"))
                    .clicked()
                {
                    action = Some(StepAction::MoveDown);
                }
                if ui
                    .add_enabled(can_move_up, egui::Button::new("上移"))
                    .clicked()
                {
                    action = Some(StepAction::MoveUp);
                }
            },
            |ui| {
                draw_step_editor(ui, cutscene_index, index, &mut steps[index]);
            },
        );
        match action {
            Some(StepAction::MoveUp) if index > 0 => {
                steps.swap(index, index - 1);
                index = index.saturating_sub(1);
            }
            Some(StepAction::MoveDown) if index + 1 < steps.len() => {
                steps.swap(index, index + 1);
                index += 1;
            }
            Some(StepAction::Delete) => {
                steps.remove(index);
                continue;
            }
            _ => {}
        }
        card_gap(ui);
        index += 1;
    }
}

fn draw_step_editor(
    ui: &mut egui::Ui,
    cutscene_index: usize,
    step_index: usize,
    step: &mut CutsceneStep,
) {
    let mut kind = CutsceneStepKind::from_step(step);
    property_grid::property_row(ui, "Kind", |ui| {
        egui::ComboBox::from_id_salt(("cutscene_step_kind", cutscene_index, step_index))
            .selected_text(kind.label())
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for option in CutsceneStepKind::ALL {
                    ui.selectable_value(&mut kind, option, option.label());
                }
            });
    });
    if kind != CutsceneStepKind::from_step(step) {
        *step = kind.default_step();
    }

    match step {
        CutsceneStep::FadeIn { duration }
        | CutsceneStep::FadeOut { duration }
        | CutsceneStep::Wait { duration } => {
            property_duration(ui, "Duration", duration);
        }
        CutsceneStep::TextPanel {
            speaker,
            body,
            min_duration,
            require_confirm,
        } => {
            property_row(ui, "行为", |ui| {
                ui.checkbox(require_confirm, "需要确认推进");
                ui.add(
                    egui::DragValue::new(min_duration)
                        .range(0.0..=30.0)
                        .speed(0.05)
                        .prefix("min "),
                );
            });
            let mut has_speaker = speaker.is_some();
            property_row(ui, "Speaker", |ui| {
                ui.checkbox(&mut has_speaker, "显示说话人");
            });
            if has_speaker && speaker.is_none() {
                *speaker = Some(CutsceneText::new("AI", "AI"));
            } else if !has_speaker {
                *speaker = None;
            }
            if let Some(speaker) = speaker {
                draw_source_text_editor(
                    ui,
                    "Speaker",
                    ("cutscene_step_speaker", cutscene_index, step_index),
                    speaker,
                    1,
                );
            }
            draw_source_text_editor(
                ui,
                "源文本",
                ("cutscene_step_body", cutscene_index, step_index),
                body,
                4,
            );
        }
        CutsceneStep::SetFlag { flag } => {
            property_text_edit_with_id(
                ui,
                "Flag",
                ("cutscene_step_flag", cutscene_index, step_index),
                flag,
            );
        }
    }
}

fn property_duration(ui: &mut egui::Ui, label: &str, value: &mut f32) {
    property_row(ui, label, |ui| {
        ui.add(
            egui::DragValue::new(value)
                .range(0.0..=60.0)
                .speed(0.05)
                .suffix("s"),
        );
    });
}

fn draw_source_text_editor(
    ui: &mut egui::Ui,
    label: &str,
    id_salt: impl std::hash::Hash + Copy,
    text: &mut CutsceneText,
    rows: usize,
) {
    ui.label(egui::RichText::new(label).color(THEME_MUTED_TEXT));
    let mut source = cutscene_source_text(text).to_owned();
    let changed = property_row(ui, "Text", |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut source)
                .id_salt((id_salt, "source"))
                .desired_width(ui.available_width())
                .desired_rows(rows),
        )
        .changed()
    })
    .inner;
    if changed {
        set_cutscene_source_text(text, source);
    }
    ui.colored_label(
        THEME_MUTED_TEXT,
        "翻译和多语言校对留给 Language 工作区统一处理。",
    );
}

fn cutscene_source_text(text: &CutsceneText) -> &str {
    if !text.chinese.trim().is_empty() {
        &text.chinese
    } else {
        &text.english
    }
}

fn set_cutscene_source_text(text: &mut CutsceneText, source: String) {
    // Cutscenes author source text only. The future Language workspace owns divergent translations.
    text.chinese = source.clone();
    text.english = source;
}

fn property_text_edit_with_id(
    ui: &mut egui::Ui,
    label: &str,
    id_salt: impl std::hash::Hash,
    value: &mut String,
) -> bool {
    property_row(ui, label, |ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .id_salt(id_salt)
                .desired_width(ui.available_width()),
        )
        .changed()
    })
    .inner
}

fn validate_cutscene_step(
    cutscene_id: &str,
    step_index: usize,
    step: &CutsceneStep,
    issues: &mut Vec<CutsceneValidationIssue>,
) {
    let step_label = format!("{cutscene_id} step {}", step_index + 1);
    match step {
        CutsceneStep::FadeIn { duration }
        | CutsceneStep::FadeOut { duration }
        | CutsceneStep::Wait { duration } => {
            if !duration.is_finite() || *duration < 0.0 {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Error,
                    message: format!("{step_label} duration 必须 >= 0"),
                });
            }
        }
        CutsceneStep::TextPanel {
            speaker,
            body,
            min_duration,
            ..
        } => {
            if !min_duration.is_finite() || *min_duration < 0.0 {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Error,
                    message: format!("{step_label} min_duration 必须 >= 0"),
                });
            }
            if body.english.trim().is_empty() && body.chinese.trim().is_empty() {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Warning,
                    message: format!("{step_label} 文本正文为空"),
                });
            }
            if speaker.as_ref().is_some_and(|speaker| {
                speaker.english.trim().is_empty() && speaker.chinese.trim().is_empty()
            }) {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Warning,
                    message: format!("{step_label} speaker 为空"),
                });
            }
        }
        CutsceneStep::SetFlag { flag } => {
            if flag.trim().is_empty() {
                issues.push(CutsceneValidationIssue {
                    severity: CutsceneValidationSeverity::Error,
                    message: format!("{step_label} flag 不能为空"),
                });
            }
        }
    }
}

fn cutscene_matches_search(cutscene: &CutsceneDefinition, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }

    let mut haystack = cutscene.id.to_ascii_lowercase();
    haystack.push(' ');
    haystack.push_str(&cutscene_completion_label(&cutscene.completion).to_ascii_lowercase());
    for step in &cutscene.steps {
        append_step_search_text(step, &mut haystack);
    }
    haystack.contains(search)
}

fn append_step_search_text(step: &CutsceneStep, haystack: &mut String) {
    match step {
        CutsceneStep::TextPanel { speaker, body, .. } => {
            if let Some(speaker) = speaker {
                haystack.push(' ');
                haystack.push_str(&speaker.english.to_ascii_lowercase());
                haystack.push(' ');
                haystack.push_str(&speaker.chinese.to_ascii_lowercase());
            }
            haystack.push(' ');
            haystack.push_str(&body.english.to_ascii_lowercase());
            haystack.push(' ');
            haystack.push_str(&body.chinese.to_ascii_lowercase());
        }
        CutsceneStep::SetFlag { flag } => {
            haystack.push(' ');
            haystack.push_str(&flag.to_ascii_lowercase());
        }
        CutsceneStep::FadeIn { .. } | CutsceneStep::FadeOut { .. } | CutsceneStep::Wait { .. } => {}
    }
}

fn cutscene_completion_label(completion: &CutsceneCompletion) -> &str {
    match completion {
        CutsceneCompletion::Pop => "Pop",
        CutsceneCompletion::SwitchScene { scene } => scene,
    }
}
