use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(crate) struct MapListEntry {
    pub(crate) label: String,
    pub(crate) path: PathBuf,
}

pub(crate) fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn project_relative_path(project_root: &Path, path: &Path) -> Option<String> {
    if let Ok(relative) = path.strip_prefix(project_root) {
        return Some(relative.to_string_lossy().replace('\\', "/"));
    }

    let canonical_root = project_root.canonicalize().ok()?;
    let canonical_path = path.canonicalize().ok()?;
    canonical_path
        .strip_prefix(canonical_root)
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn maps_dir(project_root: &Path) -> PathBuf {
    project_root.join("assets").join("data").join("maps")
}

pub(crate) fn scan_map_entries(project_root: &Path) -> Vec<MapListEntry> {
    let Ok(entries) = fs::read_dir(maps_dir(project_root)) else {
        return Vec::new();
    };

    let mut maps = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ron"))
        })
        .filter_map(|path| {
            let label = path.file_name()?.to_str()?.to_owned();
            Some(MapListEntry { label, path })
        })
        .collect::<Vec<_>>();

    maps.sort_by(|left, right| left.label.cmp(&right.label));
    maps
}
