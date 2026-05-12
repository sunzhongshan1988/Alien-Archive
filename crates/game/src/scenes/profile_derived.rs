use std::collections::HashSet;

use content::semantics;

use crate::save::{InventorySave, PlayerProfileSave};

use super::{inventory_scene, rewards};

const SCAN_XP_REWARD: u32 = 120;
const RESEARCH_PROGRESS_PER_SCAN: u32 = 6;

#[derive(Clone, Copy, Debug)]
pub(super) struct ProfileDerivationInput<'a> {
    pub profile: &'a PlayerProfileSave,
    pub inventory: &'a InventorySave,
    pub scanned_codex_ids: &'a HashSet<String>,
    pub codex_entry_count: usize,
    pub collected_entity_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileDerivedState {
    pub level: u32,
    pub xp: u32,
    pub xp_next: u32,
    pub research: Vec<DerivedMeterValue>,
    pub attributes: Vec<DerivedScoreValue>,
    pub movement_speed_multiplier: f32,
    pub scanned_codex_count: usize,
    pub codex_entry_count: usize,
    pub collected_entity_count: usize,
    pub inventory_discovery_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedMeterValue {
    pub id: String,
    pub value: u32,
    pub max: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedScoreValue {
    pub id: String,
    pub value: u32,
    pub max: u32,
}

pub(super) fn derive_profile_state(input: ProfileDerivationInput<'_>) -> ProfileDerivedState {
    let scanned_codex_count = input.scanned_codex_ids.len();
    let (level, xp, xp_next) = scan_level_and_xp(scanned_codex_count as u32);
    let research = derived_research_progress(input.scanned_codex_ids, input.profile);
    let inventory_load = inventory_load_units(input.inventory);
    let load_max = profile_meter_max(input.profile, semantics::METER_LOAD);
    let movement_speed_multiplier = movement_speed_multiplier_from_values(
        profile_meter_value(input.profile, semantics::METER_STAMINA),
        profile_meter_max(input.profile, semantics::METER_STAMINA),
        inventory_load,
        load_max,
    );
    let inventory_discovery_count = inventory_discovery_count(input.inventory);
    let attributes =
        derived_profile_attributes(input, &research, inventory_load, inventory_discovery_count);

    ProfileDerivedState {
        level,
        xp,
        xp_next,
        research,
        attributes,
        movement_speed_multiplier,
        scanned_codex_count,
        codex_entry_count: input.codex_entry_count,
        collected_entity_count: input.collected_entity_count,
        inventory_discovery_count,
    }
}

pub(super) fn inventory_load_units(inventory: &InventorySave) -> u32 {
    inventory
        .slots
        .iter()
        .flatten()
        .map(|stack| inventory_scene::inventory_item_weight(&stack.item_id) * stack.quantity)
        .sum()
}

fn derived_research_progress(
    scanned_codex_ids: &HashSet<String>,
    profile: &PlayerProfileSave,
) -> Vec<DerivedMeterValue> {
    let mut progress = std::collections::BTreeMap::<&'static str, u32>::new();
    for codex_id in scanned_codex_ids {
        let meter_id = rewards::research_meter_for_codex(codex_id);
        *progress.entry(meter_id).or_default() += RESEARCH_PROGRESS_PER_SCAN;
    }

    profile
        .research
        .iter()
        .map(|meter| DerivedMeterValue {
            id: meter.id.clone(),
            value: progress
                .get(meter.id.as_str())
                .copied()
                .unwrap_or(0)
                .min(meter.max),
            max: meter.max,
        })
        .collect()
}

fn derived_profile_attributes(
    input: ProfileDerivationInput<'_>,
    research: &[DerivedMeterValue],
    inventory_load: u32,
    inventory_discovery_count: usize,
) -> Vec<DerivedScoreValue> {
    let health = profile_meter_ratio(input.profile, semantics::METER_HEALTH);
    let stamina = profile_meter_ratio(input.profile, semantics::METER_STAMINA);
    let suit = profile_meter_ratio(input.profile, semantics::METER_SUIT);
    let load = meter_ratio(
        inventory_load,
        profile_meter_max(input.profile, semantics::METER_LOAD),
    );
    let oxygen = profile_meter_ratio(input.profile, semantics::METER_OXYGEN);
    let radiation = profile_meter_ratio(input.profile, semantics::METER_RADIATION);
    let spores = profile_meter_ratio(input.profile, semantics::METER_SPORES);
    let research = average_derived_meter_ratio(research);
    let scanned = progress_ratio(input.scanned_codex_ids.len(), input.codex_entry_count);
    let harvesting =
        capped_count_ratio(input.collected_entity_count + inventory_discovery_count, 20);
    let derived = [
        (
            "survival",
            score_from_ratio(average_slice(&[health, suit, oxygen, radiation, spores])),
        ),
        (
            "mobility",
            score_from_ratio(
                (stamina * 0.70 + (1.0 - load).clamp(0.0, 1.0) * 0.30).clamp(0.0, 1.0),
            ),
        ),
        ("scanning", score_from_ratio(scanned)),
        ("harvesting", score_from_ratio(harvesting)),
        ("analysis", score_from_ratio(research)),
    ];

    input
        .profile
        .attributes
        .iter()
        .filter_map(|score| {
            derived
                .iter()
                .find(|(id, _)| *id == score.id)
                .map(|(_, value)| DerivedScoreValue {
                    id: score.id.clone(),
                    value: (*value).min(score.max),
                    max: score.max,
                })
        })
        .collect()
}

fn average_derived_meter_ratio(meters: &[DerivedMeterValue]) -> f32 {
    if meters.is_empty() {
        return 0.0;
    }

    let total = meters
        .iter()
        .map(|meter| meter_ratio(meter.value, meter.max))
        .sum::<f32>();
    (total / meters.len() as f32).clamp(0.0, 1.0)
}

fn average_slice(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }

    (values.iter().copied().sum::<f32>() / values.len() as f32).clamp(0.0, 1.0)
}

fn scan_level_and_xp(scanned_count: u32) -> (u32, u32, u32) {
    let mut total_xp = scanned_count * SCAN_XP_REWARD;
    let mut level = 1;
    let mut xp_next = 1_000;
    while xp_next > 0 && total_xp >= xp_next {
        total_xp -= xp_next;
        level += 1;
        xp_next += 2_500;
    }

    (level, total_xp, xp_next)
}

fn movement_speed_multiplier_from_values(
    stamina_value: u32,
    stamina_max: u32,
    load_value: u32,
    load_max: u32,
) -> f32 {
    let stamina = meter_ratio(stamina_value, stamina_max);
    let load = meter_ratio(load_value, load_max);
    let stamina_factor = if stamina_value == 0 {
        0.55
    } else if stamina < 0.20 {
        0.78
    } else {
        1.0
    };
    let load_factor = if load >= 0.95 {
        0.70
    } else if load >= 0.80 {
        0.86
    } else {
        1.0
    };

    stamina_factor * load_factor
}

fn profile_meter_value(profile: &PlayerProfileSave, id: &str) -> u32 {
    profile.meter(id).map_or(0, |meter| meter.value)
}

fn profile_meter_max(profile: &PlayerProfileSave, id: &str) -> u32 {
    profile.meter(id).map_or(0, |meter| meter.max)
}

fn profile_meter_ratio(profile: &PlayerProfileSave, id: &str) -> f32 {
    profile
        .meter(id)
        .map_or(0.0, |meter| meter_ratio(meter.value, meter.max))
}

fn meter_ratio(value: u32, max: u32) -> f32 {
    if max == 0 {
        0.0
    } else {
        (value as f32 / max as f32).clamp(0.0, 1.0)
    }
}

fn progress_ratio(current: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }

    (current as f32 / total as f32).clamp(0.0, 1.0)
}

fn capped_count_ratio(current: usize, cap: usize) -> f32 {
    if cap == 0 {
        return 0.0;
    }

    (current as f32 / cap as f32).clamp(0.0, 1.0)
}

fn score_from_ratio(ratio: f32) -> u32 {
    let ratio = ratio.clamp(0.0, 1.0);
    if ratio <= f32::EPSILON {
        0
    } else {
        ((ratio * 10.0).round() as u32).max(1)
    }
}

fn inventory_discovery_count(inventory: &InventorySave) -> usize {
    inventory
        .slots
        .iter()
        .flatten()
        .filter(|stack| !stack.locked && stack.quantity > 0)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{ItemStackSave, PlayerProfileSave};

    fn input<'a>(
        profile: &'a PlayerProfileSave,
        inventory: &'a InventorySave,
        scanned_codex_ids: &'a HashSet<String>,
    ) -> ProfileDerivationInput<'a> {
        ProfileDerivationInput {
            profile,
            inventory,
            scanned_codex_ids,
            codex_entry_count: 2,
            collected_entity_count: 1,
        }
    }

    #[test]
    fn derive_profile_state_counts_scan_and_inventory_sources() {
        let profile = PlayerProfileSave::default();
        let mut inventory = InventorySave::default();
        inventory.slots = vec![Some(ItemStackSave::new("bio_sample_vial", 2, false))];
        let scanned_codex_ids = HashSet::from(["codex.flora.glowfungus".to_owned()]);

        let derived = derive_profile_state(input(&profile, &inventory, &scanned_codex_ids));

        assert_eq!(derived.level, 1);
        assert_eq!(derived.xp, SCAN_XP_REWARD);
        assert_eq!(derived.scanned_codex_count, 1);
        assert_eq!(derived.codex_entry_count, 2);
        assert_eq!(derived.collected_entity_count, 1);
        assert_eq!(derived.inventory_discovery_count, 1);
        assert_eq!(
            derived
                .research
                .iter()
                .find(|meter| meter.id == semantics::METER_BIO)
                .map(|meter| meter.value),
            Some(RESEARCH_PROGRESS_PER_SCAN)
        );
    }

    #[test]
    fn movement_speed_slows_for_empty_stamina_and_heavy_load() {
        assert!(movement_speed_multiplier_from_values(0, 100, 0, 60) < 1.0);
        assert!(movement_speed_multiplier_from_values(100, 100, 60, 60) < 1.0);
        assert_eq!(movement_speed_multiplier_from_values(100, 100, 0, 60), 1.0);
    }
}
