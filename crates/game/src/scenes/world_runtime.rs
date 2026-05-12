use std::collections::BTreeSet;

use runtime::Vec2;

use crate::{
    save::{SaveVec2, WorldSave},
    world::MapTransitionTarget,
};

use super::SceneId;

const POSITION_SAVE_EPSILON_SQUARED: f32 = 16.0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MapTransitionDestination {
    pub scene_id: SceneId,
    pub map_path: String,
    pub spawn_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WorldLocationUpdate {
    pub world: WorldSave,
    pub changed: bool,
}

pub(super) fn resolve_map_transition(
    transition: Option<&MapTransitionTarget>,
    default_scene: SceneId,
    default_map: &str,
    default_spawn: &str,
) -> MapTransitionDestination {
    MapTransitionDestination {
        scene_id: transition
            .and_then(|transition| transition.scene.as_deref())
            .map(scene_id_from_transition_key)
            .unwrap_or(default_scene),
        map_path: transition
            .and_then(|transition| transition.map_path.as_deref())
            .unwrap_or(default_map)
            .to_owned(),
        spawn_id: transition
            .and_then(|transition| transition.spawn_id.as_deref())
            .unwrap_or(default_spawn)
            .to_owned(),
    }
}

pub(super) fn scene_save_key(scene_id: SceneId) -> &'static str {
    match scene_id {
        SceneId::Facility => "Facility",
        _ => "Overworld",
    }
}

pub(super) fn scene_id_from_save_key(value: &str) -> SceneId {
    if value.trim().eq_ignore_ascii_case("facility") {
        SceneId::Facility
    } else {
        SceneId::Overworld
    }
}

fn scene_id_from_transition_key(value: &str) -> SceneId {
    match value.trim().to_ascii_lowercase().as_str() {
        "facility" => SceneId::Facility,
        _ => SceneId::Overworld,
    }
}

pub(super) fn entity_progress_key(map_path: &str, entity_id: &str) -> String {
    format!("{map_path}::{entity_id}")
}

pub(super) fn zone_progress_key(map_path: &str, zone_id: &str) -> String {
    format!("{map_path}::{zone_id}")
}

pub(super) fn collected_entity_ids_for_map(
    collected_entities: &BTreeSet<String>,
    map_path: &str,
) -> BTreeSet<String> {
    let prefix = format!("{map_path}::");
    collected_entities
        .iter()
        .filter_map(|key| key.strip_prefix(&prefix).map(str::to_owned))
        .collect()
}

pub(super) fn make_world_location_update(
    previous: &WorldSave,
    scene_id: SceneId,
    map_path: &str,
    spawn_id: Option<String>,
    position: Vec2,
) -> WorldLocationUpdate {
    let scene_key = scene_save_key(scene_id);
    let changed = previous.current_scene != scene_key
        || previous.map_path != map_path
        || previous.spawn_id.as_deref() != spawn_id.as_deref()
        || position_changed(previous.player_position, position);

    WorldLocationUpdate {
        world: WorldSave {
            current_scene: scene_key.to_owned(),
            map_path: map_path.to_owned(),
            spawn_id,
            player_position: Some(position.into()),
            collected_entities: previous.collected_entities.clone(),
            triggered_zones: previous.triggered_zones.clone(),
            field_time_minutes: previous.field_time_minutes,
            weather: previous.weather.clone(),
        },
        changed,
    }
}

fn position_changed(previous: Option<SaveVec2>, current: Vec2) -> bool {
    let Some(previous) = previous else {
        return true;
    };

    let dx = previous.x - current.x;
    let dy = previous.y - current.y;
    dx * dx + dy * dy > POSITION_SAVE_EPSILON_SQUARED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_destination_uses_defaults_without_override() {
        let destination =
            resolve_map_transition(None, SceneId::Overworld, "maps/landing.ron", "start");

        assert_eq!(
            destination,
            MapTransitionDestination {
                scene_id: SceneId::Overworld,
                map_path: "maps/landing.ron".to_owned(),
                spawn_id: "start".to_owned(),
            }
        );
    }

    #[test]
    fn transition_destination_accepts_content_overrides() {
        let transition = MapTransitionTarget {
            scene: Some("facility".to_owned()),
            map_path: Some("maps/facility.ron".to_owned()),
            spawn_id: Some("airlock".to_owned()),
        };

        let destination = resolve_map_transition(
            Some(&transition),
            SceneId::Overworld,
            "maps/landing.ron",
            "start",
        );

        assert_eq!(
            destination,
            MapTransitionDestination {
                scene_id: SceneId::Facility,
                map_path: "maps/facility.ron".to_owned(),
                spawn_id: "airlock".to_owned(),
            }
        );
    }

    #[test]
    fn collected_entity_ids_are_scoped_to_map_path() {
        let mut collected_entities = BTreeSet::new();
        collected_entities.insert(entity_progress_key("maps/a.ron", "crate"));
        collected_entities.insert(entity_progress_key("maps/a.ron", "terminal"));
        collected_entities.insert(entity_progress_key("maps/b.ron", "crate"));

        assert_eq!(
            collected_entity_ids_for_map(&collected_entities, "maps/a.ron"),
            BTreeSet::from(["crate".to_owned(), "terminal".to_owned()])
        );
    }

    #[test]
    fn world_location_update_preserves_progress_and_uses_position_threshold() {
        let mut previous = WorldSave {
            current_scene: "Overworld".to_owned(),
            map_path: "maps/a.ron".to_owned(),
            spawn_id: Some("camp".to_owned()),
            player_position: Some(SaveVec2 { x: 10.0, y: 10.0 }),
            ..WorldSave::default()
        };
        previous
            .collected_entities
            .insert(entity_progress_key("maps/a.ron", "crate"));
        previous
            .triggered_zones
            .insert(zone_progress_key("maps/a.ron", "intro"));

        let small_move = make_world_location_update(
            &previous,
            SceneId::Overworld,
            "maps/a.ron",
            Some("camp".to_owned()),
            Vec2::new(12.0, 10.0),
        );

        assert!(!small_move.changed);
        assert_eq!(
            small_move.world.collected_entities,
            previous.collected_entities
        );
        assert_eq!(small_move.world.triggered_zones, previous.triggered_zones);
        assert_eq!(small_move.world.spawn_id.as_deref(), Some("camp"));

        let spawn_change = make_world_location_update(
            &previous,
            SceneId::Overworld,
            "maps/a.ron",
            Some("north_gate".to_owned()),
            Vec2::new(12.0, 10.0),
        );

        assert!(spawn_change.changed);

        let large_move = make_world_location_update(
            &previous,
            SceneId::Overworld,
            "maps/a.ron",
            Some("camp".to_owned()),
            Vec2::new(15.0, 10.0),
        );

        assert!(large_move.changed);
    }
}
