use crate::asset_registry::AssetEntry;

pub(crate) fn category_label(category: &str) -> &str {
    match category {
        "tiles" => "地块",
        "decals" => "贴花",
        "props" => "道具",
        "flora" => "植物",
        "fauna" => "生物",
        "structures" => "结构",
        "ruins" => "遗迹",
        "interactables" => "交互物",
        "pickups" => "拾取物",
        "zones" => "区域",
        _ => category,
    }
}

pub(crate) fn asset_matches_search(asset: &AssetEntry, search: &str) -> bool {
    asset.id.to_ascii_lowercase().contains(search)
        || asset.relative_path.to_ascii_lowercase().contains(search)
        || asset
            .tags
            .iter()
            .any(|tag| tag.to_ascii_lowercase().contains(search))
}

pub(crate) fn compact_asset_label(id: &str) -> String {
    let label = id
        .trim_start_matches("ow_tile_")
        .trim_start_matches("ow_")
        .replace('_', " ");
    let mut chars = label.chars();
    let compact = chars.by_ref().take(12).collect::<String>();
    if chars.next().is_some() {
        format!("{compact}...")
    } else {
        compact
    }
}
