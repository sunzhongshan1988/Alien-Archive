use std::path::Path;

use content::{CodexDatabase, DEFAULT_CODEX_DB_PATH, MapValidationIssue, MapValidationSeverity};

use super::maps::display_project_path;

pub(crate) const DEFAULT_ENTITY_TYPES: &[&str] = &[
    "Decoration",
    "ScanTarget",
    "FacilityEntrance",
    "FacilityExit",
    "Door",
];

pub(crate) const DEFAULT_UNLOCK_ITEM_IDS: &[&str] = &[
    "ruin_key",
    "scanner_tool",
    "artifact_core",
    "data_shard",
    "energy_cell",
    "alien_crystal_sample",
    "bio_sample_vial",
    "scrap_part",
    "metal_fragment",
    "glow_fungus_sample",
];

pub(crate) fn load_codex_database(project_root: &Path) -> (Option<CodexDatabase>, String) {
    let path = project_root.join(DEFAULT_CODEX_DB_PATH);
    match CodexDatabase::load(&path) {
        Ok(database) => {
            let count = database.entries().len();
            (Some(database), format!("Codex 数据已加载：{count} 个条目"))
        }
        Err(error) => {
            eprintln!("codex database load failed: {error:?}");
            (
                None,
                format!(
                    "Codex 数据读取失败 {}：{error:#}",
                    display_project_path(project_root, &path)
                ),
            )
        }
    }
}

pub(crate) fn launch_scene_for_mode(mode: &str) -> &'static str {
    if mode.eq_ignore_ascii_case("facility") {
        "facility"
    } else {
        "overworld"
    }
}

pub(crate) fn validation_summary(issues: &[MapValidationIssue]) -> String {
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == MapValidationSeverity::Error)
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == MapValidationSeverity::Warning)
        .count();
    format!("校验结果：{errors} 个错误，{warnings} 个警告")
}
