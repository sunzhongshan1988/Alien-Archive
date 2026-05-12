use crate::ui::localization;
use crate::world::MapPromptRule;

use super::{Language, SceneId};

pub(super) const LOG_CATEGORY_PICKUP: &str = "pickup";
pub(super) const LOG_CATEGORY_SCAN: &str = "scan";
pub(super) const LOG_CATEGORY_UNLOCK: &str = "unlock";
pub(super) const LOG_CATEGORY_STATUS: &str = "status";
pub(super) const LOG_CATEGORY_ITEM: &str = "item";
pub(super) const LOG_CATEGORY_ZONE: &str = "zone";
pub(super) const LOG_CATEGORY_OBJECTIVE: &str = "objective";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActivityLogEvent {
    pub category: &'static str,
    pub title: String,
    pub detail: String,
}

impl ActivityLogEvent {
    fn new(category: &'static str, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            category,
            title: title.into(),
            detail: detail.into(),
        }
    }
}

pub(super) fn inventory_full(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.inventory_full.title",
            "Inventory full",
            "背包已满",
        ),
        localization::text(
            language,
            "activity.event.inventory_full.detail",
            "No empty slot was available for the pickup",
            "没有空槽位，物品未收入背包",
        ),
    )
}

pub(super) fn stamina_low(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.stamina_blocked.title",
            "Stamina too low",
            "体力不足",
        ),
        localization::text(
            language,
            "activity.event.stamina_blocked.detail",
            "The action was cancelled; pause to recover stamina",
            "本次动作被取消，先停止移动恢复体力",
        ),
    )
}

pub(super) fn access_blocked(language: Language, detail: String) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_UNLOCK,
        localization::text(
            language,
            "activity.event.access_blocked.title",
            "Access blocked",
            "入口受限",
        ),
        detail,
    )
}

pub(super) fn zone_prompt(
    language: Language,
    prompt: &MapPromptRule,
    fallback: &str,
) -> ActivityLogEvent {
    let title = prompt.log_title.clone().unwrap_or_else(|| {
        localization::text(
            language,
            "activity.event.zone_prompt.title",
            "Area note",
            "区域提示",
        )
        .into_owned()
    });
    let detail = prompt
        .log_detail
        .clone()
        .unwrap_or_else(|| fallback.to_owned());
    ActivityLogEvent::new(LOG_CATEGORY_ZONE, title, detail)
}

pub(super) fn zone_hazard(language: Language, zone_id: &str, detail: String) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_ZONE,
        localization::text(
            language,
            "activity.event.zone_hazard.title",
            "Hazard zone",
            "危险区域",
        ),
        localization::format_text(
            language,
            "activity.event.zone_hazard.detail",
            "{zone}: {detail}",
            "{zone}：{detail}",
            &[("zone", zone_id.to_owned()), ("detail", detail)],
        ),
    )
}

pub(super) fn objective(log_title: String, log_detail: String) -> ActivityLogEvent {
    ActivityLogEvent::new(LOG_CATEGORY_OBJECTIVE, log_title, log_detail)
}

pub(super) fn scene_transition(
    language: Language,
    scene_id: SceneId,
    map_path: &str,
) -> ActivityLogEvent {
    let title = match scene_id {
        SceneId::Facility => localization::text(
            language,
            "activity.event.transition.facility.title",
            "Entered facility",
            "进入设施",
        ),
        _ => localization::text(
            language,
            "activity.event.transition.overworld.title",
            "Returned to overworld",
            "返回外部区域",
        ),
    };
    ActivityLogEvent::new(
        LOG_CATEGORY_UNLOCK,
        title,
        localization::format_text(
            language,
            "activity.event.transition.detail",
            "Destination: {map}",
            "目的地：{map}",
            &[("map", map_path.to_owned())],
        ),
    )
}

pub(super) fn item_used(
    language: Language,
    item_name: &str,
    meter_name: &str,
    amount: u32,
) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_ITEM,
        localization::format_text(
            language,
            "activity.event.item_used.title",
            "Used {item}",
            "使用 {item}",
            &[("item", item_name.to_owned())],
        ),
        localization::format_text(
            language,
            "activity.event.item_used.detail",
            "Restored {meter} by {amount}",
            "{meter} 恢复 {amount}",
            &[
                ("meter", meter_name.to_owned()),
                ("amount", amount.to_string()),
            ],
        ),
    )
}

pub(super) fn item_added(language: Language, item_name: &str, quantity: u32) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_PICKUP,
        localization::text(
            language,
            "activity.event.item_added.title",
            "Item acquired",
            "获得物品",
        ),
        localization::format_text(
            language,
            "activity.event.item_added.detail",
            "{item} x{quantity} added to inventory",
            "{item} x{quantity} 已写入背包",
            &[
                ("item", item_name.to_owned()),
                ("quantity", quantity.to_string()),
            ],
        ),
    )
}

pub(super) fn codex_scan(
    language: Language,
    entry_title: &str,
    research_meter: &str,
) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_SCAN,
        localization::text(
            language,
            "activity.event.scan_recorded.title",
            "Scan recorded",
            "扫描完成",
        ),
        localization::format_text(
            language,
            "activity.event.scan_recorded.detail",
            "{title} · {meter} research +6 · XP +120",
            "{title} · {meter} 研究 +6 · XP +120",
            &[
                ("title", entry_title.to_owned()),
                ("meter", research_meter.to_owned()),
            ],
        ),
    )
}

pub(super) fn access_blocked_detail(
    language: Language,
    locked_message: Option<&str>,
    codex_title: Option<&str>,
    item_name: Option<&str>,
) -> String {
    if let Some(message) = locked_message {
        return message.to_owned();
    }

    match (codex_title, item_name) {
        (Some(codex), Some(item)) => localization::format_text(
            language,
            "activity.event.access_blocked.scan_and_item",
            "Requires scan: {codex}; item: {item}",
            "需要先扫描 {codex}，并携带 {item}",
            &[("codex", codex.to_owned()), ("item", item.to_owned())],
        ),
        (Some(codex), None) => localization::format_text(
            language,
            "activity.event.access_blocked.scan",
            "Requires scan: {codex}",
            "需要先扫描 {codex}",
            &[("codex", codex.to_owned())],
        ),
        (None, Some(item)) => localization::format_text(
            language,
            "activity.event.access_blocked.item",
            "Requires item: {item}",
            "需要携带 {item}",
            &[("item", item.to_owned())],
        ),
        (None, None) => localization::text(
            language,
            "activity.event.access_blocked.missing_requirement",
            "Missing access requirement",
            "缺少进入条件",
        )
        .into_owned(),
    }
}

pub(super) fn access_unavailable(language: Language) -> String {
    localization::text(
        language,
        "activity.event.access_blocked.unavailable",
        "The entrance is currently unavailable",
        "入口当前不可用",
    )
    .into_owned()
}

pub(super) fn status_stamina_low(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.status.stamina_low.title",
            "Stamina low",
            "体力偏低",
        ),
        localization::text(
            language,
            "activity.event.status.stamina_low.detail",
            "Movement, scans, and jumps are limited",
            "移动、扫描和跳跃会受到限制",
        ),
    )
}

pub(super) fn status_load_high(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.status.load_high.title",
            "Load high",
            "负重过高",
        ),
        localization::text(
            language,
            "activity.event.status.load_high.detail",
            "Inventory weight is slowing movement",
            "背包重量已经开始拖慢移动速度",
        ),
    )
}

pub(super) fn status_suit_low(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.status.suit_low.title",
            "Suit integrity low",
            "外骨骼受损",
        ),
        localization::text(
            language,
            "activity.event.status.suit_low.detail",
            "Continued exposure increases health risk",
            "继续暴露会提高生命风险",
        ),
    )
}

pub(super) fn status_oxygen_low(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.status.oxygen_low.title",
            "Oxygen low",
            "氧气偏低",
        ),
        localization::text(
            language,
            "activity.event.status.oxygen_low.detail",
            "Health will fall once oxygen is depleted",
            "氧气耗尽后生命值会下降",
        ),
    )
}

pub(super) fn status_health_critical(language: Language) -> ActivityLogEvent {
    ActivityLogEvent::new(
        LOG_CATEGORY_STATUS,
        localization::text(
            language,
            "activity.event.status.health_critical.title",
            "Health critical",
            "生命值危险",
        ),
        localization::text(
            language,
            "activity.event.status.health_critical.detail",
            "Extract or use recovery supplies soon",
            "建议尽快撤离或使用恢复道具",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_blocked_detail_prefers_custom_message() {
        assert_eq!(
            access_blocked_detail(
                Language::Chinese,
                Some("门禁锁定"),
                Some("终端"),
                Some("钥匙")
            ),
            "门禁锁定"
        );
    }

    #[test]
    fn item_added_uses_pickup_category() {
        let event = item_added(Language::English, "Crystal Sample", 2);

        assert_eq!(event.category, LOG_CATEGORY_PICKUP);
        assert!(event.detail.contains("Crystal Sample"));
        assert!(event.detail.contains("2"));
    }
}
