use content::{self, UnlockRule};
use eframe::egui::{Vec2, vec2};

use super::state::{OutlinerBadge, OutlinerEntry, SelectedItem};

pub(crate) const OUTLINER_GROUPS: &[&str] =
    &["Spawns", "Entities", "Objects", "Decals", "Zones", "Ground"];
pub(crate) const EDITOR_KNOWN_ZONE_TYPES: &[&str] =
    &["ScanArea", "MapTransition", "NoSpawn", "CameraBounds"];

pub(crate) fn outliner_entry(
    group: &'static str,
    label: String,
    detail: String,
    selection: Option<SelectedItem>,
    focus_world: Vec2,
    badges: Vec<OutlinerBadge>,
    search_text: String,
) -> OutlinerEntry {
    let badge_text = badges
        .iter()
        .map(|badge| badge.label)
        .collect::<Vec<_>>()
        .join(" ");
    OutlinerEntry {
        group,
        label: label.clone(),
        detail: detail.clone(),
        search_text: [label, detail, search_text, badge_text]
            .join(" ")
            .to_ascii_lowercase(),
        selection,
        focus_world,
        badges,
    }
}

pub(crate) fn outliner_matches(entry: &OutlinerEntry, search: &str) -> bool {
    search
        .split_whitespace()
        .all(|term| entry.search_text.contains(term))
}

pub(crate) fn unlock_search_text(unlock: Option<&UnlockRule>) -> String {
    let Some(unlock) = unlock else {
        return String::new();
    };

    [
        unlock.requires_codex_id.as_deref().unwrap_or_default(),
        unlock.requires_item_id.as_deref().unwrap_or_default(),
        unlock.locked_message.as_deref().unwrap_or_default(),
    ]
    .join(" ")
}

pub(crate) fn zone_focus_world(zone: &content::ZoneInstance, tile_size: f32) -> Vec2 {
    if zone.points.is_empty() {
        return vec2(0.0, 0.0);
    }
    let mut min = vec2(f32::INFINITY, f32::INFINITY);
    let mut max = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for point in &zone.points {
        min.x = min.x.min(point[0]);
        min.y = min.y.min(point[1]);
        max.x = max.x.max(point[0]);
        max.y = max.y.max(point[1]);
    }
    vec2(
        (min.x + max.x) * 0.5 * tile_size,
        (min.y + max.y) * 0.5 * tile_size,
    )
}
