use std::collections::BTreeSet;

use content::semantics;
use runtime::Rect;

use crate::world::{MapHazardRule, MapObjectiveRule, MapPromptRule, MapZone, World};

use super::{
    GameContext, Language, notice_system::NoticeState, profile_meter_label, zone_progress_key,
};

#[derive(Default)]
pub(super) struct ZoneRuntimeState {
    hazards: ZoneActivationTracker,
    prompts: ZoneActivationTracker,
    objectives: ZoneActivationTracker,
}

impl ZoneRuntimeState {
    pub(super) fn update(
        &mut self,
        ctx: &mut GameContext,
        notice: &mut NoticeState,
        world: &World,
        map_path: &str,
        player_rect: Rect,
        dt: f32,
    ) {
        self.begin_frame();
        {
            let mut executor = ZoneRuleExecutor {
                ctx,
                notice,
                map_path,
                dt,
            };

            for zone in world.overlapping_zones(player_rect) {
                match zone.zone_type.as_str() {
                    semantics::ZONE_HAZARD => {
                        if let Some(hazard) = effectful_hazard(zone) {
                            let presence = self.hazards.track(map_path, &zone.id);
                            executor.apply_hazard(zone, hazard, presence);
                        }
                    }
                    semantics::ZONE_PROMPT => {
                        if let Some(prompt) = zone.prompt.as_ref() {
                            let presence = self.prompts.track(map_path, &zone.id);
                            executor.apply_prompt(zone, prompt, presence);
                        }
                    }
                    semantics::ZONE_OBJECTIVE | semantics::ZONE_CHECKPOINT => {
                        if let Some(objective) = zone.objective.as_ref() {
                            let presence = self.objectives.track(map_path, &zone.id);
                            executor.apply_objective(zone, objective, presence);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.finish_frame();
    }

    fn begin_frame(&mut self) {
        self.hazards.begin_frame();
        self.prompts.begin_frame();
        self.objectives.begin_frame();
    }

    fn finish_frame(&mut self) {
        self.hazards.finish_frame();
        self.prompts.finish_frame();
        self.objectives.finish_frame();
    }
}

#[derive(Default)]
struct ZoneActivationTracker {
    active: BTreeSet<String>,
    current: BTreeSet<String>,
}

impl ZoneActivationTracker {
    fn begin_frame(&mut self) {
        self.current.clear();
    }

    fn track(&mut self, map_path: &str, zone_id: &str) -> ZonePresence {
        let key = zone_progress_key(map_path, zone_id);
        self.current.insert(key.clone());
        if self.active.insert(key) {
            ZonePresence::Entered
        } else {
            ZonePresence::Stayed
        }
    }

    fn finish_frame(&mut self) {
        self.active.retain(|key| self.current.contains(key));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ZonePresence {
    Entered,
    Stayed,
}

impl ZonePresence {
    fn is_entered(self) -> bool {
        self == Self::Entered
    }
}

struct ZoneRuleExecutor<'a> {
    ctx: &'a mut GameContext,
    notice: &'a mut NoticeState,
    map_path: &'a str,
    dt: f32,
}

impl ZoneRuleExecutor<'_> {
    fn apply_hazard(&mut self, zone: &MapZone, hazard: &MapHazardRule, presence: ZonePresence) {
        if presence.is_entered() {
            let message = hazard_message(self.ctx.language, &zone.id, hazard);
            self.notice.push_warning_message(message.clone());
            self.ctx.log_zone_hazard(&zone.id, message);
        }

        let mut changed = false;
        for effect in &hazard.effects {
            changed |=
                self.ctx
                    .apply_zone_meter_effect(&effect.meter_id, effect.rate_per_second, self.dt);
        }
        if changed {
            self.ctx.update_zone_status_alerts();
        }
    }

    fn apply_prompt(&mut self, zone: &MapZone, prompt: &MapPromptRule, presence: ZonePresence) {
        if !presence.is_entered() || self.once_zone_already_triggered(zone, prompt.once) {
            return;
        }

        let message = prompt_message(self.ctx.language, &zone.id, prompt);
        self.notice.push_info_message(message.clone());
        self.ctx.log_zone_prompt(prompt, &message);
        self.mark_once_zone_triggered(zone, prompt.once);
    }

    fn apply_objective(
        &mut self,
        zone: &MapZone,
        objective: &MapObjectiveRule,
        presence: ZonePresence,
    ) {
        if !presence.is_entered() || self.once_zone_already_triggered(zone, objective.once) {
            return;
        }

        if let Some(event) = self.ctx.apply_objective_zone(objective) {
            self.notice
                .push_info_message(objective_message(objective, event.notice));
            self.mark_once_zone_triggered(zone, objective.once);
        }
    }

    fn once_zone_already_triggered(&self, zone: &MapZone, once: bool) -> bool {
        once && self.ctx.is_zone_triggered(self.map_path, &zone.id)
    }

    fn mark_once_zone_triggered(&mut self, zone: &MapZone, once: bool) {
        if once {
            self.ctx.mark_zone_triggered(self.map_path, &zone.id);
        }
    }
}

fn effectful_hazard(zone: &MapZone) -> Option<&MapHazardRule> {
    zone.hazard
        .as_ref()
        .filter(|hazard| !hazard.effects.is_empty())
}

fn hazard_message(language: Language, _zone_id: &str, hazard: &MapHazardRule) -> String {
    if let Some(message) = hazard.message.as_deref() {
        return message.to_owned();
    }

    let effect_labels = hazard
        .effects
        .iter()
        .map(|effect| profile_meter_label(&effect.meter_id, language))
        .collect::<Vec<_>>()
        .join(" / ");
    match language {
        Language::Chinese => format!("危险区域，影响 {effect_labels}"),
        Language::English => format!("Hazard zone. Affects {effect_labels}"),
    }
}

fn prompt_message(language: Language, zone_id: &str, prompt: &MapPromptRule) -> String {
    if let Some(message) = prompt.message.as_deref() {
        return message.to_owned();
    }
    if let Some(detail) = prompt.log_detail.as_deref() {
        return detail.to_owned();
    }
    match language {
        Language::Chinese => format!("发现区域：{zone_id}"),
        Language::English => format!("Area discovered: {zone_id}"),
    }
}

fn objective_message(objective: &MapObjectiveRule, fallback: String) -> String {
    objective.message.clone().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::MapZone;
    use runtime::Vec2;

    #[test]
    fn hazard_message_prefers_custom_message() {
        let hazard = MapHazardRule {
            effects: Vec::new(),
            message: Some("空气异常".to_owned()),
        };

        assert_eq!(
            hazard_message(Language::Chinese, "toxic", &hazard),
            "空气异常"
        );
    }

    #[test]
    fn prompt_message_prefers_custom_message() {
        let prompt = MapPromptRule {
            message: Some("前方有遗迹".to_owned()),
            log_title: None,
            log_detail: None,
            once: true,
        };

        assert_eq!(
            prompt_message(Language::Chinese, "ruin", &prompt),
            "前方有遗迹"
        );
    }

    #[test]
    fn zone_progress_keys_are_scoped_to_map_path() {
        assert_eq!(
            zone_progress_key("assets/data/maps/a.ron", "zone_001"),
            "assets/data/maps/a.ron::zone_001"
        );
    }

    #[test]
    fn activation_tracker_reports_entry_until_zone_exits() {
        let mut tracker = ZoneActivationTracker::default();

        tracker.begin_frame();
        assert_eq!(
            tracker.track("assets/data/maps/a.ron", "hazard_001"),
            ZonePresence::Entered
        );
        tracker.finish_frame();

        tracker.begin_frame();
        assert_eq!(
            tracker.track("assets/data/maps/a.ron", "hazard_001"),
            ZonePresence::Stayed
        );
        tracker.finish_frame();

        tracker.begin_frame();
        tracker.finish_frame();

        tracker.begin_frame();
        assert_eq!(
            tracker.track("assets/data/maps/a.ron", "hazard_001"),
            ZonePresence::Entered
        );
    }

    #[allow(dead_code)]
    fn zone_with_prompt() -> MapZone {
        MapZone {
            id: "prompt".to_owned(),
            zone_type: "PromptZone".to_owned(),
            points: Vec::new(),
            bounds: Rect::new(Vec2::ZERO, Vec2::new(1.0, 1.0)),
            hazard: None,
            prompt: Some(MapPromptRule::default()),
            objective: None,
            surface: None,
            unlock: None,
            transition: None,
        }
    }
}
