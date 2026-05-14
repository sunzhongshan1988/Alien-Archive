use content::{EventAction, EventCondition, EventScope, WorldEventDefinition};
use runtime::SceneCommand;

use crate::world::MapObjectiveRule;

use super::{GameContext, SceneId, notice_system::NoticeState};

pub(super) struct EventExecutionContext<'a> {
    pub(super) map_path: &'a str,
    pub(super) zone_id: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EventExecutionResult {
    NotFound,
    Skipped,
    Executed(SceneCommand<SceneId>),
}

impl EventExecutionResult {
    pub(super) fn scene_command(self) -> Option<SceneCommand<SceneId>> {
        match self {
            Self::Executed(SceneCommand::None) | Self::NotFound | Self::Skipped => None,
            Self::Executed(command) => Some(command),
        }
    }
}

pub(super) fn execute_event(
    ctx: &mut GameContext,
    event: &WorldEventDefinition,
    execution: EventExecutionContext<'_>,
    notice: &mut NoticeState,
) -> EventExecutionResult {
    if !scope_allows_execution(ctx, event, &execution) {
        return EventExecutionResult::Skipped;
    }
    if !event
        .conditions
        .iter()
        .all(|condition| condition_is_met(ctx, condition))
    {
        return EventExecutionResult::Skipped;
    }

    let mut command = SceneCommand::None;
    for action in &event.actions {
        if command != SceneCommand::None && matches!(action, EventAction::PlayCutscene(_)) {
            continue;
        }
        if let Some(next_command) = execute_action(ctx, action, notice) {
            command = next_command;
        }
    }

    mark_scope_executed(ctx, event, &execution);
    EventExecutionResult::Executed(command)
}

fn scope_allows_execution(
    ctx: &GameContext,
    event: &WorldEventDefinition,
    execution: &EventExecutionContext<'_>,
) -> bool {
    match event.scope {
        EventScope::Repeatable => true,
        EventScope::OncePerZone => !ctx.is_zone_triggered(execution.map_path, execution.zone_id),
        EventScope::WorldOnce => !ctx.is_cutscene_flag_set(&event_flag(&event.id)),
    }
}

fn mark_scope_executed(
    ctx: &mut GameContext,
    event: &WorldEventDefinition,
    execution: &EventExecutionContext<'_>,
) {
    match event.scope {
        EventScope::Repeatable => {}
        EventScope::OncePerZone => {
            ctx.mark_zone_triggered(execution.map_path, execution.zone_id);
        }
        EventScope::WorldOnce => {
            ctx.mark_cutscene_flag(&event_flag(&event.id));
        }
    }
}

fn condition_is_met(ctx: &GameContext, condition: &EventCondition) -> bool {
    match condition {
        EventCondition::FlagSet(flag) => ctx.is_cutscene_flag_set(flag),
        EventCondition::FlagMissing(flag) => !ctx.is_cutscene_flag_set(flag),
        EventCondition::CutsceneSeen(cutscene_id) => ctx.has_seen_cutscene(cutscene_id),
        EventCondition::CutsceneMissing(cutscene_id) => !ctx.has_seen_cutscene(cutscene_id),
        EventCondition::CodexScanned(codex_id) => ctx.scanned_codex_ids.contains(codex_id),
        EventCondition::CodexMissing(codex_id) => !ctx.scanned_codex_ids.contains(codex_id),
        EventCondition::ObjectiveCheckpointDone {
            objective_id,
            checkpoint_id,
        } => ctx.is_objective_checkpoint_done(objective_id, checkpoint_id),
        EventCondition::ObjectiveCheckpointMissing {
            objective_id,
            checkpoint_id,
        } => !ctx.is_objective_checkpoint_done(objective_id, checkpoint_id),
    }
}

fn execute_action(
    ctx: &mut GameContext,
    action: &EventAction,
    notice: &mut NoticeState,
) -> Option<SceneCommand<SceneId>> {
    match action {
        EventAction::PlayCutscene(cutscene_id) => ctx
            .request_cutscene_once(cutscene_id)
            .then_some(SceneCommand::Push(SceneId::Cutscene)),
        EventAction::SetFlag(flag) => {
            ctx.mark_cutscene_flag(flag);
            None
        }
        EventAction::AdvanceObjective {
            objective_id,
            checkpoint_id,
            complete_objective,
        } => {
            let rule = MapObjectiveRule {
                objective_id: objective_id.clone(),
                checkpoint_id: checkpoint_id.clone(),
                complete_objective: *complete_objective,
                message: None,
                log_title: None,
                log_detail: None,
                once: false,
            };
            if let Some(event) = ctx.apply_objective_zone(&rule) {
                notice.push_info_message(event.notice);
            }
            None
        }
        EventAction::ShowNotice(message) => {
            if !message.trim().is_empty() {
                notice.push_info_message(message.clone());
            }
            None
        }
    }
}

fn event_flag(event_id: &str) -> String {
    format!("event:{}", event_id.trim())
}

#[cfg(test)]
mod tests {
    use content::{EventAction, EventScope, WorldEventDefinition};
    use runtime::SceneCommand;

    use super::*;
    use crate::save::SaveData;

    #[test]
    fn world_once_event_only_runs_once() {
        let mut ctx = GameContext::from_save(
            "saves/test_event.ron".into(),
            SaveData::default(),
            content::CodexDatabase::default(),
        );
        ctx.cutscene_database = content::CutsceneDatabase::default();
        let event = WorldEventDefinition {
            id: "landing.once".to_owned(),
            scope: EventScope::WorldOnce,
            actions: vec![EventAction::SetFlag("landing.once.done".to_owned())],
            ..WorldEventDefinition::default()
        };
        let mut notice = NoticeState::default();

        assert_eq!(
            execute_event(
                &mut ctx,
                &event,
                EventExecutionContext {
                    map_path: "maps/a.ron",
                    zone_id: "zone_intro"
                },
                &mut notice,
            ),
            EventExecutionResult::Executed(SceneCommand::None)
        );
        assert_eq!(
            execute_event(
                &mut ctx,
                &event,
                EventExecutionContext {
                    map_path: "maps/a.ron",
                    zone_id: "zone_intro"
                },
                &mut notice,
            ),
            EventExecutionResult::Skipped
        );
    }

    #[test]
    fn cutscene_event_still_runs_later_actions() {
        let mut ctx = GameContext::from_save(
            "saves/test_event.ron".into(),
            SaveData::default(),
            content::CodexDatabase::default(),
        );
        ctx.cutscene_database =
            content::CutsceneDatabase::from_definitions(vec![content::CutsceneDefinition {
                id: "intro.event".to_owned(),
                ..content::CutsceneDefinition::default()
            }]);
        let event = WorldEventDefinition {
            id: "landing.cutscene".to_owned(),
            scope: EventScope::Repeatable,
            actions: vec![
                EventAction::PlayCutscene("intro.event".to_owned()),
                EventAction::SetFlag("landing.cutscene.followup".to_owned()),
            ],
            ..WorldEventDefinition::default()
        };
        let mut notice = NoticeState::default();

        assert_eq!(
            execute_event(
                &mut ctx,
                &event,
                EventExecutionContext {
                    map_path: "maps/a.ron",
                    zone_id: "zone_intro"
                },
                &mut notice,
            ),
            EventExecutionResult::Executed(SceneCommand::Push(SceneId::Cutscene))
        );
        assert!(ctx.is_cutscene_flag_set("landing.cutscene.followup"));
    }
}
