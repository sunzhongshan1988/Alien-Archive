use std::collections::BTreeSet;

use runtime::Rect;

use crate::world::{MapHazardRule, MapObjectiveRule, MapPromptRule, MapZone, World};

use super::{GameContext, Language, notice_system::NoticeState, profile_meter_label};

#[derive(Default)]
pub(super) struct ZoneRuntimeState {
    active_hazards: BTreeSet<String>,
    active_prompts: BTreeSet<String>,
    active_objectives: BTreeSet<String>,
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
        let mut current_hazards = BTreeSet::new();
        let mut current_prompts = BTreeSet::new();
        let mut current_objectives = BTreeSet::new();

        for zone in world.overlapping_zones(player_rect) {
            match zone.zone_type.as_str() {
                "HazardZone" => {
                    current_hazards.insert(zone_progress_key(map_path, &zone.id));
                    self.update_hazard_zone(ctx, notice, map_path, zone, dt);
                }
                "PromptZone" => {
                    current_prompts.insert(zone_progress_key(map_path, &zone.id));
                    self.update_prompt_zone(ctx, notice, map_path, zone);
                }
                "ObjectiveZone" | "Checkpoint" => {
                    current_objectives.insert(zone_progress_key(map_path, &zone.id));
                    self.update_objective_zone(ctx, notice, map_path, zone);
                }
                _ => {}
            }
        }

        self.active_hazards
            .retain(|key| current_hazards.contains(key));
        self.active_prompts
            .retain(|key| current_prompts.contains(key));
        self.active_objectives
            .retain(|key| current_objectives.contains(key));
    }

    fn update_hazard_zone(
        &mut self,
        ctx: &mut GameContext,
        notice: &mut NoticeState,
        map_path: &str,
        zone: &MapZone,
        dt: f32,
    ) {
        let Some(hazard) = zone.hazard.as_ref() else {
            return;
        };

        if hazard.effects.is_empty() {
            return;
        }

        let key = zone_progress_key(map_path, &zone.id);
        let entered = self.active_hazards.insert(key);
        if entered {
            let message = hazard_message(ctx.language, &zone.id, hazard);
            notice.push_warning_message(message.clone());
            ctx.log_zone_hazard(&zone.id, message);
        }

        let mut changed = false;
        for effect in &hazard.effects {
            changed |= ctx.apply_zone_meter_effect(&effect.meter_id, effect.rate_per_second, dt);
        }
        if changed {
            ctx.update_zone_status_alerts();
        }
    }

    fn update_prompt_zone(
        &mut self,
        ctx: &mut GameContext,
        notice: &mut NoticeState,
        map_path: &str,
        zone: &MapZone,
    ) {
        let Some(prompt) = zone.prompt.as_ref() else {
            return;
        };

        let key = zone_progress_key(map_path, &zone.id);
        if !self.active_prompts.insert(key) {
            return;
        }
        if prompt.once && ctx.is_zone_triggered(map_path, &zone.id) {
            return;
        }

        let message = prompt_message(ctx.language, &zone.id, prompt);
        notice.push_info_message(message.clone());
        ctx.log_zone_prompt(prompt, &message);
        if prompt.once {
            ctx.mark_zone_triggered(map_path, &zone.id);
        }
    }

    fn update_objective_zone(
        &mut self,
        ctx: &mut GameContext,
        notice: &mut NoticeState,
        map_path: &str,
        zone: &MapZone,
    ) {
        let Some(objective) = zone.objective.as_ref() else {
            return;
        };

        let key = zone_progress_key(map_path, &zone.id);
        if !self.active_objectives.insert(key) {
            return;
        }
        if objective.once && ctx.is_zone_triggered(map_path, &zone.id) {
            return;
        }

        if let Some(event) = ctx.apply_objective_zone(objective) {
            notice.push_info_message(objective_message(objective, event.notice));
            if objective.once {
                ctx.mark_zone_triggered(map_path, &zone.id);
            }
        }
    }
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

fn zone_progress_key(map_path: &str, zone_id: &str) -> String {
    format!("{map_path}::{zone_id}")
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
