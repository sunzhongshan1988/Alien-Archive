mod assets;
mod codex;
mod map;
pub mod semantics;
mod validation;

pub use assets::{
    AnchorKind, AssetDatabase, AssetDefinition, AssetKind, DEFAULT_ASSET_DB_PATH, SnapMode,
};
pub use codex::{CodexDatabase, CodexEntry, DEFAULT_CODEX_DB_PATH};
pub use map::{
    CollisionCell, CollisionZoneRule, DEFAULT_MAP_ID, DEFAULT_MAP_PATH, EntityInstance,
    HazardEffect, HazardRule, InstanceRect, LayerKind, MapDocument, MapLayers, ObjectInstance,
    ObjectiveRule, PromptRule, SpawnPoint, SurfaceGateRule, TileInstance, TransitionTarget,
    UnlockRule, WalkSurfaceKind, WalkSurfaceRule, ZoneInstance,
};
pub use validation::{
    MapValidationIssue, MapValidationSeverity, validate_map, validate_map_with_codex,
};
