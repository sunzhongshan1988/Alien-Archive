use std::path::Path;

use content::AssetDatabase;

pub(crate) fn sanitize_map_id(id: &str) -> Option<String> {
    let without_extension = id.strip_suffix(".ron").unwrap_or(id);
    let mut output = String::new();
    let mut previous_was_separator = false;

    for character in without_extension.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if matches!(character, '_' | '-' | ' ') && !previous_was_separator {
            output.push('_');
            previous_was_separator = true;
        }
    }

    let sanitized = output.trim_matches('_').to_owned();
    (!sanitized.is_empty()).then_some(sanitized)
}

pub(crate) fn sanitize_asset_id(id: &str) -> Option<String> {
    let mut output = String::new();
    let mut previous_was_separator = false;

    for character in id.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if matches!(character, '_' | '-' | ' ' | '.') && !previous_was_separator {
            output.push('_');
            previous_was_separator = true;
        }
    }

    let sanitized = output.trim_matches('_').to_owned();
    (!sanitized.is_empty()).then_some(sanitized)
}

pub(crate) fn sanitize_category(category: &str) -> Option<String> {
    sanitize_asset_id(category)
}

pub(crate) fn sanitize_relative_path(path: &str) -> Option<String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains("../")
        || normalized.contains("/..")
    {
        None
    } else {
        Some(normalized)
    }
}

pub(crate) fn parse_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(crate) fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

pub(crate) fn unique_map_id(project_root: &Path, base_id: &str) -> String {
    let base = sanitize_map_id(base_id).unwrap_or_else(|| "untitled_overworld".to_owned());
    let map_dir = project_root.join("assets").join("data").join("maps");
    if !map_dir.join(format!("{base}.ron")).exists() {
        return base;
    }

    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if !map_dir.join(format!("{candidate}.ron")).exists() {
            return candidate;
        }
    }

    unreachable!("unbounded id scan should always find a candidate")
}

pub(crate) fn unique_asset_id(database: &AssetDatabase, base_id: &str) -> String {
    let base = sanitize_asset_id(base_id).unwrap_or_else(|| "asset".to_owned());
    if database.assets.iter().all(|asset| asset.id != base) {
        return base;
    }

    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if database.assets.iter().all(|asset| asset.id != candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded id scan should always find a candidate")
}
