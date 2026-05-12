use std::borrow::Cow;

use crate::scenes::{GameMenuTab, Language};
use crate::ui::localization;

#[derive(Clone, Copy)]
pub struct LocalizedText {
    key: &'static str,
    english: &'static str,
    chinese: &'static str,
}

impl LocalizedText {
    pub const fn new(key: &'static str, english: &'static str, chinese: &'static str) -> Self {
        Self {
            key,
            english,
            chinese,
        }
    }

    pub fn get(self, language: Language) -> Cow<'static, str> {
        localization::text(language, self.key, self.english, self.chinese)
    }
}

#[derive(Clone, Copy)]
pub struct BottomAction {
    pub label: LocalizedText,
    pub sublabel: LocalizedText,
}

pub const BOTTOM_ACTIONS: &[BottomAction] = &[
    BottomAction {
        label: LocalizedText::new("game.bottom.equip.label", "Equip", "装备"),
        sublabel: LocalizedText::new("game.bottom.equip.sublabel", "Loadout", "装备管理"),
    },
    BottomAction {
        label: LocalizedText::new("game.bottom.skills.label", "Skills", "技能"),
        sublabel: LocalizedText::new("game.bottom.skills.sublabel", "Abilities", "能力树"),
    },
    BottomAction {
        label: LocalizedText::new("game.bottom.logs.label", "Logs", "日志"),
        sublabel: LocalizedText::new("game.bottom.logs.sublabel", "Records", "外勤记录"),
    },
    BottomAction {
        label: LocalizedText::new("game.bottom.craft.label", "Craft", "制作"),
        sublabel: LocalizedText::new("game.bottom.craft.sublabel", "Workshop", "工作台"),
    },
    BottomAction {
        label: LocalizedText::new("game.bottom.comms.label", "Comms", "通讯"),
        sublabel: LocalizedText::new("game.bottom.comms.sublabel", "Relay", "信号中继"),
    },
    BottomAction {
        label: LocalizedText::new("game.bottom.save.label", "Save", "存档"),
        sublabel: LocalizedText::new("game.bottom.save.sublabel", "Progress", "进度保存"),
    },
];

pub fn tab_index(tab: GameMenuTab) -> usize {
    GameMenuTab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or_default()
}

pub fn tab_label(tab: GameMenuTab, language: Language) -> Cow<'static, str> {
    match tab {
        GameMenuTab::Profile => {
            localization::text(language, "game.tab.profile.label", "Profile", "属性")
        }
        GameMenuTab::Inventory => {
            localization::text(language, "game.tab.inventory.label", "Inventory", "背包")
        }
        GameMenuTab::Codex => localization::text(language, "game.tab.codex.label", "Codex", "图鉴"),
        GameMenuTab::Map => localization::text(language, "game.tab.map.label", "Map", "地图"),
        GameMenuTab::Quests => localization::text(language, "game.tab.quests.label", "Log", "日志"),
        GameMenuTab::Settings => {
            localization::text(language, "game.tab.settings.label", "Settings", "设置")
        }
    }
}

pub fn tab_sublabel(tab: GameMenuTab, language: Language) -> Cow<'static, str> {
    match tab {
        GameMenuTab::Profile => {
            localization::text(language, "game.tab.profile.sublabel", "Status", "角色状态")
        }
        GameMenuTab::Inventory => localization::text(
            language,
            "game.tab.inventory.sublabel",
            "Storage",
            "物资管理",
        ),
        GameMenuTab::Codex => localization::text(
            language,
            "game.tab.codex.sublabel",
            "Discoveries",
            "发现记录",
        ),
        GameMenuTab::Map => {
            localization::text(language, "game.tab.map.sublabel", "Routes", "区域路线")
        }
        GameMenuTab::Quests => localization::text(
            language,
            "game.tab.quests.sublabel",
            "Field Log",
            "外勤记录",
        ),
        GameMenuTab::Settings => localization::text(
            language,
            "game.tab.settings.sublabel",
            "Preferences",
            "系统偏好",
        ),
    }
}

pub fn tab_title(tab: GameMenuTab, language: Language) -> Cow<'static, str> {
    match tab {
        GameMenuTab::Profile => localization::text(
            language,
            "game.tab.profile.title",
            "Field Dossier",
            "外勤档案",
        ),
        GameMenuTab::Inventory => {
            localization::text(language, "game.tab.inventory.title", "Inventory", "背包")
        }
        GameMenuTab::Codex => {
            localization::text(language, "game.tab.codex.title", "Alien Codex", "异星图鉴")
        }
        GameMenuTab::Map => {
            localization::text(language, "game.tab.map.title", "Region Map", "区域地图")
        }
        GameMenuTab::Quests => {
            localization::text(language, "game.tab.quests.title", "Field Log", "外勤日志")
        }
        GameMenuTab::Settings => {
            localization::text(language, "game.tab.settings.title", "Settings", "设置")
        }
    }
}

pub fn tab_subtitle(tab: GameMenuTab, language: Language) -> Cow<'static, str> {
    match tab {
        GameMenuTab::Profile => localization::text(
            language,
            "game.tab.profile.subtitle",
            "Review explorer status, aptitudes, and suit modules",
            "查看探索者状态、能力与装备模块",
        ),
        GameMenuTab::Inventory => localization::text(
            language,
            "game.tab.inventory.subtitle",
            "Manage samples, consumables, tools, and key items",
            "管理样本、消耗品、工具与关键物品",
        ),
        GameMenuTab::Codex => localization::text(
            language,
            "game.tab.codex.subtitle",
            "Track discovered organisms, minerals, and ruin records",
            "追踪已发现的生物、矿物和遗迹资料",
        ),
        GameMenuTab::Map => localization::text(
            language,
            "game.tab.map.subtitle",
            "Check routes, entrances, and unsurveyed sectors",
            "确认探索路线、入口和未调查区域",
        ),
        GameMenuTab::Quests => localization::text(
            language,
            "game.tab.quests.subtitle",
            "Review recent scans, pickups, access checks, and status changes",
            "查看最近扫描、拾取、解锁和状态变化",
        ),
        GameMenuTab::Settings => localization::text(
            language,
            "game.tab.settings.subtitle",
            "Adjust language and in-game menu preferences",
            "调整语言与游戏内菜单偏好",
        ),
    }
}

pub fn menu_status(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.menu.status",
        "Field Menu · Click the left rail to switch",
        "外勤菜单 · 可点击左侧切换",
    )
}

pub fn close_hint(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.menu.close_hint", "Esc Close", "Esc 关闭")
}

pub fn activity_log_header(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.activity.header",
        "Recent Field Activity",
        "最近外勤记录",
    )
}

pub fn activity_log_empty(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.activity.empty",
        "No field activity yet. Scans, pickups, access checks, and status changes appear here.",
        "还没有外勤记录。扫描、拾取、解锁和状态变化会显示在这里。",
    )
}

pub fn activity_category_label(category: &str, language: Language) -> Cow<'static, str> {
    match category {
        "pickup" => localization::text(language, "game.activity.category.pickup", "Pickup", "拾取"),
        "scan" => localization::text(language, "game.activity.category.scan", "Scan", "扫描"),
        "unlock" => localization::text(language, "game.activity.category.unlock", "Access", "解锁"),
        "status" => localization::text(language, "game.activity.category.status", "Status", "状态"),
        "objective" => localization::text(
            language,
            "game.activity.category.objective",
            "Objective",
            "目标",
        ),
        _ => localization::text(language, "game.activity.category.system", "System", "系统"),
    }
}

pub fn top_location_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.top.location_label", "Location", "当前位置")
}

pub fn top_location_value(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.top.location_value",
        "Deep Frontier Outpost",
        "深空边境站",
    )
}

pub fn top_status_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.top.status_label", "Status", "状态")
}

pub fn top_status_value(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.top.status_value", "Exploring", "探索中")
}

pub fn profile_level_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.profile.level_label", "Level", "等级")
}

pub fn stat_header(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.profile.stat_header",
        "Vital Status",
        "生命状态",
    )
}

pub fn compact_vital_label(index: usize, language: Language) -> Cow<'static, str> {
    match index {
        0 => localization::text(language, "game.profile.vital.health", "Health", "生命"),
        1 => localization::text(language, "game.profile.vital.stamina", "Stamina", "体力"),
        2 => localization::text(language, "game.profile.vital.armor", "Armor", "护甲"),
        _ => localization::text(language, "game.profile.vital.carry", "Carry", "负重"),
    }
}

pub fn profile_core_header(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.profile.core_header", "Attributes", "属性")
}

pub fn profile_research_header(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.profile.research_header",
        "Research Mastery",
        "研究专精",
    )
}

pub fn codex_discoveries_title(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.codex.discoveries_title",
        "Codex Discoveries",
        "图鉴发现",
    )
}

pub fn return_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.return.label", "Return to Game", "返回游戏")
}

pub fn return_sublabel(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.return.sublabel",
        "Continue Exploring",
        "继续探索",
    )
}

pub fn quantity_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.inventory.quantity", "Qty", "数量")
}

pub fn category_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.inventory.category", "Category", "类别")
}

pub fn rarity_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.inventory.rarity", "Rarity", "稀有度")
}

pub fn stack_limit_label(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.inventory.max_stack",
        "Max Stack",
        "最大堆叠",
    )
}

pub fn research_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.inventory.research", "Research", "研究进度")
}

pub fn locked_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.inventory.locked", "Locked", "已锁定")
}

pub fn empty_slot_title(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.inventory.empty_title",
        "Empty Slot",
        "空槽位",
    )
}

pub fn empty_slot_body(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.inventory.empty_body",
        "There is no item in this slot.",
        "此槽位当前没有物品。",
    )
}

pub fn inventory_hint(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.inventory.hint",
        "Connected to current item icons, quantities, and rarities.",
        "已接入当前背包物品、图标、数量与稀有度。",
    )
}

pub fn equipment_title(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.equipment.title", "Loadout", "装备管理")
}

pub fn equipment_subtitle(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.subtitle",
        "Field gear, quickbar, and active suit status",
        "外勤装备、快捷栏与当前服役状态",
    )
}

pub fn equipment_quickbar_header(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.quickbar_header",
        "Quickbar",
        "快捷栏",
    )
}

pub fn equipment_modules_header(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.modules_header",
        "Field Modules",
        "外勤模块",
    )
}

pub fn equipment_status_header(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.status_header",
        "Suit Status",
        "服役状态",
    )
}

pub fn equipment_empty_slot(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.empty_slot",
        "Empty slot",
        "空槽位",
    )
}

pub fn equipment_no_modules(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.no_modules",
        "No tool or artifact module is installed.",
        "当前没有可显示的工具或遗物模块。",
    )
}

pub fn equipment_hint(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.equipment.hint",
        "Number keys switch quickbar slots in the field. Q uses the selected consumable.",
        "外勤中数字键切换快捷栏，Q 使用当前选中的消耗品。",
    )
}

pub fn map_labels(language: Language) -> [Cow<'static, str>; 3] {
    [
        localization::text(
            language,
            "game.map.label.current",
            "Current: Landing Site",
            "当前位置: 着陆点",
        ),
        localization::text(
            language,
            "game.map.label.target",
            "Target: Crystal Field",
            "目标: 晶体田",
        ),
        localization::text(
            language,
            "game.map.label.unsurveyed",
            "Unsurveyed Sectors: 3",
            "未调查区域: 3",
        ),
    ]
}

pub fn language_setting_label(language: Language) -> Cow<'static, str> {
    localization::text(language, "game.settings.language_label", "Language", "语言")
}

pub fn language_option_label(language: Language) -> Cow<'static, str> {
    match language {
        Language::Chinese => localization::text(
            language,
            "game.settings.language.chinese",
            "Chinese",
            "中文",
        ),
        Language::English => localization::text(
            language,
            "game.settings.language.english",
            "English",
            "English",
        ),
    }
}

pub fn settings_hint(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.settings.hint",
        "The UI does not stack both languages; switch here to refresh the global language.",
        "菜单不会同时堆中英文；这里切换后全局界面会跟随语言刷新。",
    )
}

pub fn placeholder_text(language: Language) -> Cow<'static, str> {
    localization::text(
        language,
        "game.settings.placeholder",
        "Audio, display, and control settings will be connected later.",
        "音量、窗口、控制等设置后续接入。",
    )
}
