#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZoneTypeDef {
    pub key: &'static str,
    pub zh_label: &'static str,
    pub kind: ZoneTypeKind,
    pub editor_preset: bool,
}

impl ZoneTypeDef {
    pub const fn new(
        key: &'static str,
        zh_label: &'static str,
        kind: ZoneTypeKind,
        editor_preset: bool,
    ) -> Self {
        Self {
            key,
            zh_label,
            kind,
            editor_preset,
        }
    }

    pub const fn allows_surface(self) -> bool {
        matches!(
            self.kind,
            ZoneTypeKind::WalkSurface | ZoneTypeKind::Objective | ZoneTypeKind::Checkpoint
        )
    }

    pub const fn is_line_like(self) -> bool {
        matches!(
            self.kind,
            ZoneTypeKind::CollisionLine | ZoneTypeKind::SurfaceGate
        )
    }

    pub const fn is_collision_scope(self) -> bool {
        matches!(
            self.kind,
            ZoneTypeKind::CollisionArea | ZoneTypeKind::CollisionLine
        )
    }

    pub const fn is_objective_like(self) -> bool {
        matches!(
            self.kind,
            ZoneTypeKind::Objective | ZoneTypeKind::Checkpoint
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZoneTypeKind {
    ScanArea,
    MapTransition,
    NoSpawn,
    CameraBounds,
    WalkSurface,
    SurfaceGate,
    CollisionArea,
    CollisionLine,
    Hazard,
    Prompt,
    Objective,
    Checkpoint,
    EventTrigger,
    Trigger,
}

pub const ZONE_SCAN_AREA: &str = "ScanArea";
pub const ZONE_MAP_TRANSITION: &str = "MapTransition";
pub const ZONE_NO_SPAWN: &str = "NoSpawn";
pub const ZONE_CAMERA_BOUNDS: &str = "CameraBounds";
pub const ZONE_WALK_SURFACE: &str = "WalkSurface";
pub const ZONE_SURFACE_GATE: &str = "SurfaceGate";
pub const ZONE_COLLISION_AREA: &str = "CollisionArea";
pub const ZONE_COLLISION_LINE: &str = "CollisionLine";
pub const ZONE_HAZARD: &str = "HazardZone";
pub const ZONE_PROMPT: &str = "PromptZone";
pub const ZONE_OBJECTIVE: &str = "ObjectiveZone";
pub const ZONE_CHECKPOINT: &str = "Checkpoint";
pub const ZONE_EVENT_TRIGGER: &str = "EventTrigger";
pub const ZONE_TRIGGER: &str = "Trigger";

pub const ZONE_DEF_SCAN_AREA: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_SCAN_AREA, "扫描区", ZoneTypeKind::ScanArea, false);
pub const ZONE_DEF_MAP_TRANSITION: ZoneTypeDef = ZoneTypeDef::new(
    ZONE_MAP_TRANSITION,
    "转场",
    ZoneTypeKind::MapTransition,
    true,
);
pub const ZONE_DEF_NO_SPAWN: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_NO_SPAWN, "禁止出生", ZoneTypeKind::NoSpawn, false);
pub const ZONE_DEF_CAMERA_BOUNDS: ZoneTypeDef = ZoneTypeDef::new(
    ZONE_CAMERA_BOUNDS,
    "镜头边界",
    ZoneTypeKind::CameraBounds,
    false,
);
pub const ZONE_DEF_WALK_SURFACE: ZoneTypeDef = ZoneTypeDef::new(
    ZONE_WALK_SURFACE,
    "可走表面",
    ZoneTypeKind::WalkSurface,
    true,
);
pub const ZONE_DEF_SURFACE_GATE: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_SURFACE_GATE, "表面门", ZoneTypeKind::SurfaceGate, true);
pub const ZONE_DEF_COLLISION_AREA: ZoneTypeDef = ZoneTypeDef::new(
    ZONE_COLLISION_AREA,
    "碰撞区域",
    ZoneTypeKind::CollisionArea,
    true,
);
pub const ZONE_DEF_COLLISION_LINE: ZoneTypeDef = ZoneTypeDef::new(
    ZONE_COLLISION_LINE,
    "碰撞线",
    ZoneTypeKind::CollisionLine,
    true,
);
pub const ZONE_DEF_HAZARD: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_HAZARD, "危险区", ZoneTypeKind::Hazard, true);
pub const ZONE_DEF_PROMPT: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_PROMPT, "提示区", ZoneTypeKind::Prompt, true);
pub const ZONE_DEF_OBJECTIVE: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_OBJECTIVE, "目标区", ZoneTypeKind::Objective, true);
pub const ZONE_DEF_CHECKPOINT: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_CHECKPOINT, "检查点", ZoneTypeKind::Checkpoint, true);
pub const ZONE_DEF_EVENT_TRIGGER: ZoneTypeDef = ZoneTypeDef::new(
    ZONE_EVENT_TRIGGER,
    "事件触发",
    ZoneTypeKind::EventTrigger,
    true,
);
pub const ZONE_DEF_TRIGGER: ZoneTypeDef =
    ZoneTypeDef::new(ZONE_TRIGGER, "触发区", ZoneTypeKind::Trigger, false);

pub const ZONE_TYPE_DEFS: &[ZoneTypeDef] = &[
    ZONE_DEF_SCAN_AREA,
    ZONE_DEF_MAP_TRANSITION,
    ZONE_DEF_NO_SPAWN,
    ZONE_DEF_CAMERA_BOUNDS,
    ZONE_DEF_WALK_SURFACE,
    ZONE_DEF_SURFACE_GATE,
    ZONE_DEF_COLLISION_AREA,
    ZONE_DEF_COLLISION_LINE,
    ZONE_DEF_HAZARD,
    ZONE_DEF_PROMPT,
    ZONE_DEF_OBJECTIVE,
    ZONE_DEF_CHECKPOINT,
    ZONE_DEF_EVENT_TRIGGER,
    ZONE_DEF_TRIGGER,
];

pub const EDITOR_ZONE_TYPE_PRESETS: &[ZoneTypeDef] = &[
    ZONE_DEF_WALK_SURFACE,
    ZONE_DEF_SURFACE_GATE,
    ZONE_DEF_COLLISION_AREA,
    ZONE_DEF_COLLISION_LINE,
    ZONE_DEF_MAP_TRANSITION,
    ZONE_DEF_HAZARD,
    ZONE_DEF_PROMPT,
    ZONE_DEF_OBJECTIVE,
    ZONE_DEF_CHECKPOINT,
];

pub const KNOWN_ZONE_TYPE_KEYS: &[&str] = &[
    ZONE_SCAN_AREA,
    ZONE_MAP_TRANSITION,
    ZONE_NO_SPAWN,
    ZONE_CAMERA_BOUNDS,
    ZONE_WALK_SURFACE,
    ZONE_SURFACE_GATE,
    ZONE_COLLISION_AREA,
    ZONE_COLLISION_LINE,
    ZONE_HAZARD,
    ZONE_PROMPT,
    ZONE_OBJECTIVE,
    ZONE_CHECKPOINT,
    ZONE_TRIGGER,
];

pub fn zone_type_def(zone_type: &str) -> Option<&'static ZoneTypeDef> {
    ZONE_TYPE_DEFS
        .iter()
        .find(|definition| definition.key == zone_type)
}

pub fn is_known_zone_type(zone_type: &str) -> bool {
    zone_type_def(zone_type).is_some()
}

pub fn zone_type_allows_surface(zone_type: &str) -> bool {
    zone_type_def(zone_type).is_some_and(|definition| definition.allows_surface())
}

pub fn zone_type_is_line_like(zone_type: &str) -> bool {
    zone_type_def(zone_type).is_some_and(|definition| definition.is_line_like())
}

pub fn zone_type_is_collision_scope(zone_type: &str) -> bool {
    zone_type_def(zone_type).is_some_and(|definition| definition.is_collision_scope())
}

pub fn zone_type_is_objective_like(zone_type: &str) -> bool {
    zone_type_def(zone_type).is_some_and(|definition| definition.is_objective_like())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityTypeDef {
    pub key: &'static str,
    pub zh_label: &'static str,
    pub aliases: &'static [&'static str],
    pub legacy_unlock_from_codex: bool,
}

impl EntityTypeDef {
    pub const fn new(
        key: &'static str,
        zh_label: &'static str,
        aliases: &'static [&'static str],
        legacy_unlock_from_codex: bool,
    ) -> Self {
        Self {
            key,
            zh_label,
            aliases,
            legacy_unlock_from_codex,
        }
    }

    fn matches(self, value: &str) -> bool {
        self.key == value || self.aliases.contains(&value)
    }
}

pub const ENTITY_DECORATION: &str = "Decoration";
pub const ENTITY_SCAN_TARGET: &str = "ScanTarget";
pub const ENTITY_FACILITY_ENTRANCE: &str = "FacilityEntrance";
pub const ENTITY_FACILITY_EXIT: &str = "FacilityExit";
pub const ENTITY_DOOR: &str = "Door";

pub const ENTITY_DEF_DECORATION: EntityTypeDef =
    EntityTypeDef::new(ENTITY_DECORATION, "装饰", &[], false);
pub const ENTITY_DEF_SCAN_TARGET: EntityTypeDef =
    EntityTypeDef::new(ENTITY_SCAN_TARGET, "扫描目标", &[], false);
pub const ENTITY_DEF_FACILITY_ENTRANCE: EntityTypeDef =
    EntityTypeDef::new(ENTITY_FACILITY_ENTRANCE, "设施入口", &["Entrance"], true);
pub const ENTITY_DEF_FACILITY_EXIT: EntityTypeDef =
    EntityTypeDef::new(ENTITY_FACILITY_EXIT, "设施出口", &["Exit"], false);
pub const ENTITY_DEF_DOOR: EntityTypeDef = EntityTypeDef::new(ENTITY_DOOR, "门", &[], true);

pub const ENTITY_TYPE_DEFS: &[EntityTypeDef] = &[
    ENTITY_DEF_DECORATION,
    ENTITY_DEF_SCAN_TARGET,
    ENTITY_DEF_FACILITY_ENTRANCE,
    ENTITY_DEF_FACILITY_EXIT,
    ENTITY_DEF_DOOR,
];

pub const DEFAULT_ENTITY_TYPE_KEYS: &[&str] = &[
    ENTITY_DECORATION,
    ENTITY_SCAN_TARGET,
    ENTITY_FACILITY_ENTRANCE,
    ENTITY_FACILITY_EXIT,
    ENTITY_DOOR,
];

pub fn entity_type_def(entity_type: &str) -> Option<&'static EntityTypeDef> {
    ENTITY_TYPE_DEFS
        .iter()
        .find(|definition| definition.matches(entity_type))
}

pub fn canonical_entity_type(entity_type: &str) -> Option<&'static str> {
    entity_type_def(entity_type).map(|definition| definition.key)
}

pub fn entity_type_uses_implicit_legacy_unlock(entity_type: &str) -> bool {
    entity_type_def(entity_type).is_some_and(|definition| definition.legacy_unlock_from_codex)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeterDef {
    pub key: &'static str,
    pub english_label: &'static str,
    pub zh_label: &'static str,
    pub hazard_allowed: bool,
}

impl MeterDef {
    pub const fn new(
        key: &'static str,
        english_label: &'static str,
        zh_label: &'static str,
        hazard_allowed: bool,
    ) -> Self {
        Self {
            key,
            english_label,
            zh_label,
            hazard_allowed,
        }
    }
}

pub const METER_HEALTH: &str = "health";
pub const METER_STAMINA: &str = "stamina";
pub const METER_SUIT: &str = "suit";
pub const METER_LOAD: &str = "load";
pub const METER_OXYGEN: &str = "oxygen";
pub const METER_RADIATION: &str = "radiation";
pub const METER_SPORES: &str = "spores";
pub const METER_HEAT: &str = "heat";
pub const METER_BIO: &str = "bio";
pub const METER_MINERAL: &str = "mineral";
pub const METER_RUIN: &str = "ruin";
pub const METER_DATA: &str = "data";

pub const METER_DEFS: &[MeterDef] = &[
    MeterDef::new(METER_HEALTH, "Health", "生命", true),
    MeterDef::new(METER_STAMINA, "Stamina", "体力", true),
    MeterDef::new(METER_SUIT, "Suit", "外骨骼", true),
    MeterDef::new(METER_LOAD, "Load", "负重", false),
    MeterDef::new(METER_OXYGEN, "Oxygen", "氧气", true),
    MeterDef::new(METER_RADIATION, "Radiation", "辐射抗性", true),
    MeterDef::new(METER_SPORES, "Spore resistance", "孢子抗性", true),
    MeterDef::new(METER_HEAT, "Heat resistance", "热抗性", false),
    MeterDef::new(METER_BIO, "Bio", "生物", false),
    MeterDef::new(METER_MINERAL, "Mineral", "矿物", false),
    MeterDef::new(METER_RUIN, "Ruin", "遗迹", false),
    MeterDef::new(METER_DATA, "Data", "数据", false),
];

pub const HAZARD_METER_KEYS: &[&str] = &[
    METER_HEALTH,
    METER_STAMINA,
    METER_SUIT,
    METER_OXYGEN,
    METER_RADIATION,
    METER_SPORES,
];

pub fn meter_def(meter: &str) -> Option<&'static MeterDef> {
    METER_DEFS.iter().find(|definition| definition.key == meter)
}

pub fn is_known_hazard_meter(meter: &str) -> bool {
    meter_def(meter).is_some_and(|definition| definition.hazard_allowed)
}

pub const ITEM_RUIN_KEY: &str = "ruin_key";
pub const ITEM_MED_INJECTOR: &str = "med_injector";
pub const ITEM_SCANNER_TOOL: &str = "scanner_tool";
pub const ITEM_ARTIFACT_CORE: &str = "artifact_core";
pub const ITEM_DATA_SHARD: &str = "data_shard";
pub const ITEM_ENERGY_CELL: &str = "energy_cell";
pub const ITEM_COOLANT_CANISTER: &str = "coolant_canister";
pub const ITEM_ALIEN_CRYSTAL_SAMPLE: &str = "alien_crystal_sample";
pub const ITEM_BIO_SAMPLE_VIAL: &str = "bio_sample_vial";
pub const ITEM_SCRAP_PART: &str = "scrap_part";
pub const ITEM_METAL_FRAGMENT: &str = "metal_fragment";
pub const ITEM_GLOW_FUNGUS_SAMPLE: &str = "glow_fungus_sample";

pub const COMMON_UNLOCK_ITEM_IDS: &[&str] = &[
    ITEM_RUIN_KEY,
    ITEM_MED_INJECTOR,
    ITEM_SCANNER_TOOL,
    ITEM_ARTIFACT_CORE,
    ITEM_DATA_SHARD,
    ITEM_ENERGY_CELL,
    ITEM_COOLANT_CANISTER,
    ITEM_ALIEN_CRYSTAL_SAMPLE,
    ITEM_BIO_SAMPLE_VIAL,
    ITEM_SCRAP_PART,
    ITEM_METAL_FRAGMENT,
    ITEM_GLOW_FUNGUS_SAMPLE,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeSceneDef {
    pub key: &'static str,
    pub launch_arg: &'static str,
    pub aliases: &'static [&'static str],
}

impl RuntimeSceneDef {
    pub const fn new(
        key: &'static str,
        launch_arg: &'static str,
        aliases: &'static [&'static str],
    ) -> Self {
        Self {
            key,
            launch_arg,
            aliases,
        }
    }

    fn matches(self, value: &str) -> bool {
        let value = value.trim();
        self.key.eq_ignore_ascii_case(value)
            || self.launch_arg.eq_ignore_ascii_case(value)
            || self
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(value))
    }
}

pub const SCENE_OVERWORLD: &str = "Overworld";
pub const SCENE_FACILITY: &str = "Facility";
pub const SCENE_MAIN_MENU: &str = "MainMenu";

pub const RUNTIME_SCENE_DEFS: &[RuntimeSceneDef] = &[
    RuntimeSceneDef::new(SCENE_OVERWORLD, "overworld", &["world"]),
    RuntimeSceneDef::new(SCENE_FACILITY, "facility", &[]),
    RuntimeSceneDef::new(
        SCENE_MAIN_MENU,
        "main-menu",
        &["main_menu", "mainmenu", "menu"],
    ),
];

pub const FIELD_SCENE_DEFS: &[RuntimeSceneDef] = &[
    RuntimeSceneDef::new(SCENE_OVERWORLD, "overworld", &["world"]),
    RuntimeSceneDef::new(SCENE_FACILITY, "facility", &[]),
];

pub const FIELD_SCENE_KEYS: &[&str] = &[SCENE_OVERWORLD, SCENE_FACILITY];

pub fn runtime_scene_def(scene: &str) -> Option<&'static RuntimeSceneDef> {
    RUNTIME_SCENE_DEFS
        .iter()
        .find(|definition| definition.matches(scene))
}

pub fn field_scene_def(scene: &str) -> Option<&'static RuntimeSceneDef> {
    FIELD_SCENE_DEFS
        .iter()
        .find(|definition| definition.matches(scene))
}

pub fn is_known_runtime_scene(scene: &str) -> bool {
    runtime_scene_def(scene).is_some()
}

pub fn launch_scene_for_mode(mode: &str) -> &'static str {
    field_scene_def(mode)
        .map(|definition| definition.launch_arg)
        .unwrap_or("overworld")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_metadata_covers_line_and_surface_rules() {
        assert!(zone_type_is_line_like(ZONE_COLLISION_LINE));
        assert!(zone_type_is_line_like(ZONE_SURFACE_GATE));
        assert!(zone_type_allows_surface(ZONE_WALK_SURFACE));
        assert!(zone_type_allows_surface(ZONE_CHECKPOINT));
        assert!(!zone_type_allows_surface(ZONE_HAZARD));
    }

    #[test]
    fn entity_aliases_preserve_legacy_names() {
        assert_eq!(
            canonical_entity_type("Entrance"),
            Some(ENTITY_FACILITY_ENTRANCE)
        );
        assert_eq!(canonical_entity_type("Exit"), Some(ENTITY_FACILITY_EXIT));
        assert!(entity_type_uses_implicit_legacy_unlock("Entrance"));
    }

    #[test]
    fn scene_aliases_match_existing_launch_keys() {
        assert!(is_known_runtime_scene("main_menu"));
        assert_eq!(launch_scene_for_mode("Facility"), "facility");
        assert_eq!(launch_scene_for_mode("unknown"), "overworld");
    }
}
