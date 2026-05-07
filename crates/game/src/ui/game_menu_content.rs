use crate::scenes::{GameMenuTab, Language};

#[derive(Clone, Copy)]
pub struct LocalizedText {
    english: &'static str,
    chinese: &'static str,
}

impl LocalizedText {
    pub const fn new(english: &'static str, chinese: &'static str) -> Self {
        Self { english, chinese }
    }

    pub fn get(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.chinese,
            Language::English => self.english,
        }
    }
}

#[derive(Clone, Copy)]
pub struct QuestPreview {
    pub label: LocalizedText,
    pub status: LocalizedText,
    pub progress: u32,
}

#[derive(Clone, Copy)]
pub struct BottomAction {
    pub label: LocalizedText,
    pub sublabel: LocalizedText,
}

pub const QUEST_PREVIEWS: &[QuestPreview] = &[
    QuestPreview {
        label: LocalizedText::new("Secure Landing Site", "稳固着陆点"),
        status: LocalizedText::new("Active", "进行中"),
        progress: 75,
    },
    QuestPreview {
        label: LocalizedText::new("Survey Crystal Field", "调查晶体田"),
        status: LocalizedText::new("Tracked", "已追踪"),
        progress: 40,
    },
    QuestPreview {
        label: LocalizedText::new("Decode Ruin Signal", "解析遗迹信号"),
        status: LocalizedText::new("Pending", "待处理"),
        progress: 18,
    },
];

pub const BOTTOM_ACTIONS: &[BottomAction] = &[
    BottomAction {
        label: LocalizedText::new("Equip", "装备"),
        sublabel: LocalizedText::new("Loadout", "装备管理"),
    },
    BottomAction {
        label: LocalizedText::new("Skills", "技能"),
        sublabel: LocalizedText::new("Abilities", "能力树"),
    },
    BottomAction {
        label: LocalizedText::new("Logs", "日志"),
        sublabel: LocalizedText::new("Records", "外勤记录"),
    },
    BottomAction {
        label: LocalizedText::new("Craft", "制作"),
        sublabel: LocalizedText::new("Workshop", "工作台"),
    },
    BottomAction {
        label: LocalizedText::new("Comms", "通讯"),
        sublabel: LocalizedText::new("Relay", "信号中继"),
    },
    BottomAction {
        label: LocalizedText::new("Save", "存档"),
        sublabel: LocalizedText::new("Progress", "进度保存"),
    },
];

pub fn tab_index(tab: GameMenuTab) -> usize {
    GameMenuTab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or_default()
}

pub fn tab_label(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "属性",
            GameMenuTab::Inventory => "背包",
            GameMenuTab::Codex => "图鉴",
            GameMenuTab::Map => "地图",
            GameMenuTab::Quests => "日志",
            GameMenuTab::Settings => "设置",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Profile",
            GameMenuTab::Inventory => "Inventory",
            GameMenuTab::Codex => "Codex",
            GameMenuTab::Map => "Map",
            GameMenuTab::Quests => "Log",
            GameMenuTab::Settings => "Settings",
        },
    }
}

pub fn tab_sublabel(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "角色状态",
            GameMenuTab::Inventory => "物资管理",
            GameMenuTab::Codex => "发现记录",
            GameMenuTab::Map => "区域路线",
            GameMenuTab::Quests => "外勤记录",
            GameMenuTab::Settings => "系统偏好",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Status",
            GameMenuTab::Inventory => "Storage",
            GameMenuTab::Codex => "Discoveries",
            GameMenuTab::Map => "Routes",
            GameMenuTab::Quests => "Field Log",
            GameMenuTab::Settings => "Preferences",
        },
    }
}

pub fn tab_title(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "外勤档案",
            GameMenuTab::Inventory => "背包",
            GameMenuTab::Codex => "异星图鉴",
            GameMenuTab::Map => "区域地图",
            GameMenuTab::Quests => "外勤日志",
            GameMenuTab::Settings => "设置",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Field Dossier",
            GameMenuTab::Inventory => "Inventory",
            GameMenuTab::Codex => "Alien Codex",
            GameMenuTab::Map => "Region Map",
            GameMenuTab::Quests => "Field Log",
            GameMenuTab::Settings => "Settings",
        },
    }
}

pub fn tab_subtitle(tab: GameMenuTab, language: Language) -> &'static str {
    match language {
        Language::Chinese => match tab {
            GameMenuTab::Profile => "查看探索者状态、能力与装备模块",
            GameMenuTab::Inventory => "管理样本、消耗品、工具与关键物品",
            GameMenuTab::Codex => "追踪已发现的生物、矿物和遗迹资料",
            GameMenuTab::Map => "确认探索路线、入口和未调查区域",
            GameMenuTab::Quests => "查看最近扫描、拾取、解锁和状态变化",
            GameMenuTab::Settings => "调整语言与游戏内菜单偏好",
        },
        Language::English => match tab {
            GameMenuTab::Profile => "Review explorer status, aptitudes, and suit modules",
            GameMenuTab::Inventory => "Manage samples, consumables, tools, and key items",
            GameMenuTab::Codex => "Track discovered organisms, minerals, and ruin records",
            GameMenuTab::Map => "Check routes, entrances, and unsurveyed sectors",
            GameMenuTab::Quests => {
                "Review recent scans, pickups, access checks, and status changes"
            }
            GameMenuTab::Settings => "Adjust language and in-game menu preferences",
        },
    }
}

pub fn menu_status(language: Language) -> &'static str {
    match language {
        Language::Chinese => "外勤菜单 · 可点击左侧切换",
        Language::English => "Field Menu · Click the left rail to switch",
    }
}

pub fn close_hint(language: Language) -> &'static str {
    match language {
        Language::Chinese => "Esc 关闭",
        Language::English => "Esc Close",
    }
}

pub fn activity_log_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "最近外勤记录",
        Language::English => "Recent Field Activity",
    }
}

pub fn activity_log_empty(language: Language) -> &'static str {
    match language {
        Language::Chinese => "还没有外勤记录。扫描、拾取、解锁和状态变化会显示在这里。",
        Language::English => {
            "No field activity yet. Scans, pickups, access checks, and status changes appear here."
        }
    }
}

pub fn activity_category_label(category: &str, language: Language) -> &'static str {
    match (language, category) {
        (Language::Chinese, "pickup") => "拾取",
        (Language::Chinese, "scan") => "扫描",
        (Language::Chinese, "unlock") => "解锁",
        (Language::Chinese, "status") => "状态",
        (Language::English, "pickup") => "Pickup",
        (Language::English, "scan") => "Scan",
        (Language::English, "unlock") => "Access",
        (Language::English, "status") => "Status",
        (Language::Chinese, _) => "系统",
        (Language::English, _) => "System",
    }
}

pub fn top_location_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "当前位置",
        Language::English => "Location",
    }
}

pub fn top_location_value(language: Language) -> &'static str {
    match language {
        Language::Chinese => "深空边境站",
        Language::English => "Deep Frontier Outpost",
    }
}

pub fn top_status_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "状态",
        Language::English => "Status",
    }
}

pub fn top_status_value(language: Language) -> &'static str {
    match language {
        Language::Chinese => "探索中",
        Language::English => "Exploring",
    }
}

pub fn profile_level_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "等级",
        Language::English => "Level",
    }
}

pub fn stat_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "生命状态",
        Language::English => "Vital Status",
    }
}

pub fn compact_vital_label(index: usize, language: Language) -> &'static str {
    match language {
        Language::Chinese => match index {
            0 => "生命",
            1 => "体力",
            2 => "护甲",
            _ => "负重",
        },
        Language::English => match index {
            0 => "Health",
            1 => "Stamina",
            2 => "Armor",
            _ => "Carry",
        },
    }
}

pub fn profile_core_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "属性",
        Language::English => "Attributes",
    }
}

pub fn profile_research_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "研究专精",
        Language::English => "Research Mastery",
    }
}

pub fn codex_discoveries_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "图鉴发现",
        Language::English => "Codex Discoveries",
    }
}

pub fn return_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "返回游戏",
        Language::English => "Return to Game",
    }
}

pub fn return_sublabel(language: Language) -> &'static str {
    match language {
        Language::Chinese => "继续探索",
        Language::English => "Continue Exploring",
    }
}

pub fn quantity_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "数量",
        Language::English => "Qty",
    }
}

pub fn category_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "类别",
        Language::English => "Category",
    }
}

pub fn rarity_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "稀有度",
        Language::English => "Rarity",
    }
}

pub fn stack_limit_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "最大堆叠",
        Language::English => "Max Stack",
    }
}

pub fn research_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "研究进度",
        Language::English => "Research",
    }
}

pub fn locked_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已锁定",
        Language::English => "Locked",
    }
}

pub fn empty_slot_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "空槽位",
        Language::English => "Empty Slot",
    }
}

pub fn empty_slot_body(language: Language) -> &'static str {
    match language {
        Language::Chinese => "此槽位当前没有物品。",
        Language::English => "There is no item in this slot.",
    }
}

pub fn inventory_hint(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已接入当前背包物品、图标、数量与稀有度。",
        Language::English => "Connected to current item icons, quantities, and rarities.",
    }
}

pub fn map_labels(language: Language) -> [&'static str; 3] {
    match language {
        Language::Chinese => ["当前位置: 着陆点", "目标: 晶体田", "未调查区域: 3"],
        Language::English => [
            "Current: Landing Site",
            "Target: Crystal Field",
            "Unsurveyed Sectors: 3",
        ],
    }
}

pub fn language_setting_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "语言",
        Language::English => "Language",
    }
}

pub fn language_option_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "English",
    }
}

pub fn settings_hint(language: Language) -> &'static str {
    match language {
        Language::Chinese => "菜单不会同时堆中英文；这里切换后全局界面会跟随语言刷新。",
        Language::English => {
            "The UI does not stack both languages; switch here to refresh the global language."
        }
    }
}

pub fn placeholder_text(language: Language) -> &'static str {
    match language {
        Language::Chinese => "音量、窗口、控制等设置后续接入。",
        Language::English => "Audio, display, and control settings will be connected later.",
    }
}
