use std::collections::HashMap;

use content::AssetKind;

use crate::asset_registry::AssetEntry;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TerrainMask {
    pub(crate) north: bool,
    pub(crate) east: bool,
    pub(crate) south: bool,
    pub(crate) west: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TerrainNeighborFamilies {
    pub(crate) north: Option<String>,
    pub(crate) east: Option<String>,
    pub(crate) south: Option<String>,
    pub(crate) west: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TerrainChoice {
    pub(crate) asset_id: String,
    pub(crate) rotation: i32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TerrainRules {
    asset_families: HashMap<String, String>,
    asset_roles: HashMap<String, TerrainRole>,
    families: HashMap<String, TerrainFamily>,
    transitions: HashMap<(String, String), TerrainTransition>,
}

#[derive(Clone, Debug, Default)]
struct TerrainFamily {
    centers: Vec<String>,
    edges: HashMap<Direction, String>,
    corners: HashMap<Corner, String>,
}

#[derive(Clone, Debug, Default)]
struct TerrainTransition {
    edges: HashMap<Direction, String>,
    corners: HashMap<Corner, String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Direction {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Corner {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerrainRole {
    Center,
    Edge(Direction),
    Corner(Corner),
}

impl TerrainRules {
    pub(crate) fn from_assets(assets: &[AssetEntry]) -> Self {
        let mut rules = Self::default();

        for asset in assets {
            let Some(classification) = classify_asset(asset) else {
                continue;
            };
            rules
                .asset_families
                .insert(asset.id.clone(), classification.family.clone());
            rules
                .asset_roles
                .insert(asset.id.clone(), classification.role);

            if let Some(target) = classification.transition_to {
                let transition = rules
                    .transitions
                    .entry((classification.family, target))
                    .or_default();
                match classification.role {
                    TerrainRole::Center => {}
                    TerrainRole::Edge(direction) => {
                        transition
                            .edges
                            .entry(direction)
                            .or_insert_with(|| asset.id.clone());
                    }
                    TerrainRole::Corner(corner) => {
                        transition
                            .corners
                            .entry(corner)
                            .or_insert_with(|| asset.id.clone());
                    }
                }
                continue;
            }

            let family = rules.families.entry(classification.family).or_default();
            match classification.role {
                TerrainRole::Center => {
                    family.centers.push(asset.id.clone());
                }
                TerrainRole::Edge(direction) => {
                    family
                        .edges
                        .entry(direction)
                        .or_insert_with(|| asset.id.clone());
                }
                TerrainRole::Corner(corner) => {
                    family
                        .corners
                        .entry(corner)
                        .or_insert_with(|| asset.id.clone());
                }
            }
        }

        rules
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.asset_families.is_empty()
    }

    pub(crate) fn family_for_asset(&self, asset_id: &str) -> Option<&str> {
        self.asset_families.get(asset_id).map(String::as_str)
    }

    pub(crate) fn choice_for_neighbors(
        &self,
        asset_id: &str,
        neighbors: &TerrainNeighborFamilies,
    ) -> Option<TerrainChoice> {
        let family_id = self.family_for_asset(asset_id)?;
        let current_role = self.asset_roles.get(asset_id).copied();
        if let Some(choice) = self.transition_choice_for(family_id, neighbors) {
            return Some(choice);
        }

        let family = self.families.get(family_id)?;
        let mask = TerrainMask {
            north: neighbors.north.as_deref() == Some(family_id),
            east: neighbors.east.as_deref() == Some(family_id),
            south: neighbors.south.as_deref() == Some(family_id),
            west: neighbors.west.as_deref() == Some(family_id),
        };
        family.choice_for(asset_id, current_role, role_for_mask(mask))
    }

    fn transition_choice_for(
        &self,
        source: &str,
        neighbors: &TerrainNeighborFamilies,
    ) -> Option<TerrainChoice> {
        let target = dominant_transition_target(source, neighbors)?;
        let transition = self.transitions.get(&(source.to_owned(), target.clone()))?;
        let role = transition_role_for_target(source, &target, neighbors)?;
        transition.choice_for(role)
    }
}

impl TerrainFamily {
    fn choice_for(
        &self,
        asset_id: &str,
        current_role: Option<TerrainRole>,
        role: TerrainRole,
    ) -> Option<TerrainChoice> {
        match role {
            TerrainRole::Center => {
                if current_role == Some(TerrainRole::Center)
                    && self.centers.iter().any(|center| center == asset_id)
                {
                    return Some(TerrainChoice {
                        asset_id: asset_id.to_owned(),
                        rotation: 0,
                    });
                }
                self.center_choice()
            }
            TerrainRole::Edge(direction) => self
                .edge_choice(direction)
                .or_else(|| self.current_center_choice(asset_id, current_role))
                .or_else(|| self.center_choice()),
            TerrainRole::Corner(corner) => self
                .corner_choice(corner)
                .or_else(|| {
                    corner
                        .edge_fallbacks()
                        .into_iter()
                        .find_map(|direction| self.edge_choice(direction))
                })
                .or_else(|| self.current_center_choice(asset_id, current_role))
                .or_else(|| self.center_choice()),
        }
    }

    fn current_center_choice(
        &self,
        asset_id: &str,
        current_role: Option<TerrainRole>,
    ) -> Option<TerrainChoice> {
        (current_role == Some(TerrainRole::Center)
            && self.centers.iter().any(|center| center == asset_id))
        .then(|| TerrainChoice {
            asset_id: asset_id.to_owned(),
            rotation: 0,
        })
    }

    fn center_choice(&self) -> Option<TerrainChoice> {
        self.centers.first().map(|asset_id| TerrainChoice {
            asset_id: asset_id.to_owned(),
            rotation: 0,
        })
    }

    fn edge_choice(&self, direction: Direction) -> Option<TerrainChoice> {
        self.edges
            .get(&direction)
            .map(|asset_id| TerrainChoice {
                asset_id: asset_id.clone(),
                rotation: 0,
            })
            .or_else(|| {
                self.edges
                    .iter()
                    .next()
                    .map(|(base, asset_id)| TerrainChoice {
                        asset_id: asset_id.clone(),
                        rotation: rotation_between(base.degrees(), direction.degrees()),
                    })
            })
    }

    fn corner_choice(&self, corner: Corner) -> Option<TerrainChoice> {
        self.corners
            .get(&corner)
            .map(|asset_id| TerrainChoice {
                asset_id: asset_id.clone(),
                rotation: 0,
            })
            .or_else(|| {
                self.corners
                    .iter()
                    .next()
                    .map(|(base, asset_id)| TerrainChoice {
                        asset_id: asset_id.clone(),
                        rotation: rotation_between(base.degrees(), corner.degrees()),
                    })
            })
    }
}

impl TerrainTransition {
    fn choice_for(&self, role: TerrainRole) -> Option<TerrainChoice> {
        match role {
            TerrainRole::Center => None,
            TerrainRole::Edge(direction) => self.edge_choice(direction),
            TerrainRole::Corner(corner) => self.corner_choice(corner).or_else(|| {
                corner
                    .edge_fallbacks()
                    .into_iter()
                    .find_map(|direction| self.edge_choice(direction))
            }),
        }
    }

    fn edge_choice(&self, direction: Direction) -> Option<TerrainChoice> {
        self.edges
            .get(&direction)
            .map(|asset_id| TerrainChoice {
                asset_id: asset_id.clone(),
                rotation: 0,
            })
            .or_else(|| {
                self.edges
                    .iter()
                    .next()
                    .map(|(base, asset_id)| TerrainChoice {
                        asset_id: asset_id.clone(),
                        rotation: rotation_between(base.degrees(), direction.degrees()),
                    })
            })
    }

    fn corner_choice(&self, corner: Corner) -> Option<TerrainChoice> {
        self.corners
            .get(&corner)
            .map(|asset_id| TerrainChoice {
                asset_id: asset_id.clone(),
                rotation: 0,
            })
            .or_else(|| {
                self.corners
                    .iter()
                    .next()
                    .map(|(base, asset_id)| TerrainChoice {
                        asset_id: asset_id.clone(),
                        rotation: rotation_between(base.degrees(), corner.degrees()),
                    })
            })
    }
}

impl Direction {
    fn degrees(self) -> i32 {
        match self {
            Self::North => 0,
            Self::East => 90,
            Self::South => 180,
            Self::West => 270,
        }
    }
}

impl Corner {
    fn degrees(self) -> i32 {
        match self {
            Self::NorthEast => 0,
            Self::SouthEast => 90,
            Self::SouthWest => 180,
            Self::NorthWest => 270,
        }
    }

    fn edge_fallbacks(self) -> [Direction; 2] {
        match self {
            Self::NorthEast => [Direction::North, Direction::East],
            Self::SouthEast => [Direction::South, Direction::East],
            Self::SouthWest => [Direction::South, Direction::West],
            Self::NorthWest => [Direction::North, Direction::West],
        }
    }
}

struct TerrainClassification {
    family: String,
    role: TerrainRole,
    transition_to: Option<String>,
}

fn classify_asset(asset: &AssetEntry) -> Option<TerrainClassification> {
    if asset.kind != AssetKind::Tile {
        return None;
    }

    let material_family = asset
        .tags
        .iter()
        .find_map(|tag| terrain_tag_value(tag, "material"));
    let explicit_family = asset
        .tags
        .iter()
        .find_map(|tag| terrain_tag_value(tag, "terrain"));
    let transition_from = asset
        .tags
        .iter()
        .find_map(|tag| terrain_tag_value(tag, "transition_from"));
    let transition_to = asset
        .tags
        .iter()
        .find_map(|tag| terrain_tag_value(tag, "transition_to"));
    let explicit_role = asset.tags.iter().find_map(|tag| terrain_role_tag(tag));
    let (inferred_family, inferred_role) = infer_family_and_role(&inferred_terrain_name(&asset.id));
    let family = transition_from
        .or(material_family)
        .or(explicit_family)
        .unwrap_or(&inferred_family)
        .trim_matches('_')
        .to_owned();
    let family = normalize_family_id(&family);
    if family.is_empty() {
        return None;
    }

    Some(TerrainClassification {
        family,
        role: explicit_role.unwrap_or(inferred_role),
        transition_to: transition_to.map(normalize_family_id),
    })
}

fn normalize_family_id(value: &str) -> String {
    let value = value
        .trim()
        .trim_matches('_')
        .to_ascii_lowercase()
        .replace('-', "_");
    value
        .strip_prefix("t32_")
        .or_else(|| value.strip_prefix("terrain32_"))
        .unwrap_or(&value)
        .to_owned()
}

fn terrain_tag_value<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    tag.strip_prefix(&format!("{key}:"))
        .or_else(|| tag.strip_prefix(&format!("{key}=")))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn terrain_role_tag(tag: &str) -> Option<TerrainRole> {
    let value =
        terrain_tag_value(tag, "terrain_role").or_else(|| terrain_tag_value(tag, "role"))?;
    role_from_token(value)
}

fn inferred_terrain_name(asset_id: &str) -> String {
    asset_id
        .trim_start_matches("ow_tile_")
        .trim_start_matches("tile_")
        .to_ascii_lowercase()
}

fn infer_family_and_role(name: &str) -> (String, TerrainRole) {
    let name = name.to_ascii_lowercase();
    for (suffix, role) in role_suffixes() {
        if let Some(family) = name.strip_suffix(suffix) {
            return (family.to_owned(), role);
        }
    }

    let tokens = name.split('_').collect::<Vec<_>>();

    for (index, token) in tokens.iter().enumerate() {
        if let Some(role) = role_from_token(token) {
            return (tokens[..index].join("_"), role);
        }
    }

    for suffix in ["_ground", "_floor", "_center", "_middle", "_base", "_tile"] {
        if let Some(family) = name.strip_suffix(suffix) {
            return (family.to_owned(), TerrainRole::Center);
        }
    }

    (name, TerrainRole::Center)
}

fn role_suffixes() -> Vec<(&'static str, TerrainRole)> {
    vec![
        ("_edge_n", TerrainRole::Edge(Direction::North)),
        ("_edge_north", TerrainRole::Edge(Direction::North)),
        ("_edge_top", TerrainRole::Edge(Direction::North)),
        ("_side_n", TerrainRole::Edge(Direction::North)),
        ("_side_north", TerrainRole::Edge(Direction::North)),
        ("_top_edge", TerrainRole::Edge(Direction::North)),
        ("_edge_e", TerrainRole::Edge(Direction::East)),
        ("_edge_east", TerrainRole::Edge(Direction::East)),
        ("_edge_right", TerrainRole::Edge(Direction::East)),
        ("_side_e", TerrainRole::Edge(Direction::East)),
        ("_side_east", TerrainRole::Edge(Direction::East)),
        ("_right_edge", TerrainRole::Edge(Direction::East)),
        ("_edge_s", TerrainRole::Edge(Direction::South)),
        ("_edge_south", TerrainRole::Edge(Direction::South)),
        ("_edge_bottom", TerrainRole::Edge(Direction::South)),
        ("_side_s", TerrainRole::Edge(Direction::South)),
        ("_side_south", TerrainRole::Edge(Direction::South)),
        ("_bottom_edge", TerrainRole::Edge(Direction::South)),
        ("_edge_w", TerrainRole::Edge(Direction::West)),
        ("_edge_west", TerrainRole::Edge(Direction::West)),
        ("_edge_left", TerrainRole::Edge(Direction::West)),
        ("_side_w", TerrainRole::Edge(Direction::West)),
        ("_side_west", TerrainRole::Edge(Direction::West)),
        ("_left_edge", TerrainRole::Edge(Direction::West)),
        ("_corner_ne", TerrainRole::Corner(Corner::NorthEast)),
        ("_corner_north_east", TerrainRole::Corner(Corner::NorthEast)),
        ("_corner_top_right", TerrainRole::Corner(Corner::NorthEast)),
        ("_outer_ne", TerrainRole::Corner(Corner::NorthEast)),
        ("_inner_ne", TerrainRole::Corner(Corner::NorthEast)),
        ("_corner_se", TerrainRole::Corner(Corner::SouthEast)),
        ("_corner_south_east", TerrainRole::Corner(Corner::SouthEast)),
        (
            "_corner_bottom_right",
            TerrainRole::Corner(Corner::SouthEast),
        ),
        ("_outer_se", TerrainRole::Corner(Corner::SouthEast)),
        ("_inner_se", TerrainRole::Corner(Corner::SouthEast)),
        ("_corner_sw", TerrainRole::Corner(Corner::SouthWest)),
        ("_corner_south_west", TerrainRole::Corner(Corner::SouthWest)),
        (
            "_corner_bottom_left",
            TerrainRole::Corner(Corner::SouthWest),
        ),
        ("_outer_sw", TerrainRole::Corner(Corner::SouthWest)),
        ("_inner_sw", TerrainRole::Corner(Corner::SouthWest)),
        ("_corner_nw", TerrainRole::Corner(Corner::NorthWest)),
        ("_corner_north_west", TerrainRole::Corner(Corner::NorthWest)),
        ("_corner_top_left", TerrainRole::Corner(Corner::NorthWest)),
        ("_outer_nw", TerrainRole::Corner(Corner::NorthWest)),
        ("_inner_nw", TerrainRole::Corner(Corner::NorthWest)),
    ]
}

fn role_from_token(token: &str) -> Option<TerrainRole> {
    let token = token.to_ascii_lowercase();
    let token = token
        .trim_start_matches("outer_")
        .trim_start_matches("inner_")
        .trim_start_matches("edge_")
        .trim_start_matches("corner_")
        .trim_start_matches("side_")
        .trim_start_matches("cap_");

    match token {
        "center" | "middle" | "ground" | "floor" | "base" => Some(TerrainRole::Center),
        "n" | "north" | "top" => Some(TerrainRole::Edge(Direction::North)),
        "e" | "east" | "right" => Some(TerrainRole::Edge(Direction::East)),
        "s" | "south" | "bottom" => Some(TerrainRole::Edge(Direction::South)),
        "w" | "west" | "left" => Some(TerrainRole::Edge(Direction::West)),
        "ne" | "north_east" | "top_right" => Some(TerrainRole::Corner(Corner::NorthEast)),
        "se" | "south_east" | "bottom_right" => Some(TerrainRole::Corner(Corner::SouthEast)),
        "sw" | "south_west" | "bottom_left" => Some(TerrainRole::Corner(Corner::SouthWest)),
        "nw" | "north_west" | "top_left" => Some(TerrainRole::Corner(Corner::NorthWest)),
        _ => None,
    }
}

fn role_for_mask(mask: TerrainMask) -> TerrainRole {
    let missing = [
        (!mask.north, Direction::North),
        (!mask.east, Direction::East),
        (!mask.south, Direction::South),
        (!mask.west, Direction::West),
    ];
    let missing_count = missing.iter().filter(|(is_missing, _)| *is_missing).count();

    if missing_count == 1 {
        return missing
            .into_iter()
            .find_map(|(is_missing, direction)| is_missing.then_some(TerrainRole::Edge(direction)))
            .unwrap_or(TerrainRole::Center);
    }

    if missing_count == 2 {
        if !mask.north && !mask.east {
            return TerrainRole::Corner(Corner::NorthEast);
        }
        if !mask.east && !mask.south {
            return TerrainRole::Corner(Corner::SouthEast);
        }
        if !mask.south && !mask.west {
            return TerrainRole::Corner(Corner::SouthWest);
        }
        if !mask.west && !mask.north {
            return TerrainRole::Corner(Corner::NorthWest);
        }
    }

    TerrainRole::Center
}

fn dominant_transition_target(source: &str, neighbors: &TerrainNeighborFamilies) -> Option<String> {
    let mut counts = HashMap::<String, usize>::new();
    for family in neighbor_families(neighbors).into_iter().flatten() {
        if family != source {
            *counts.entry(family.to_owned()).or_default() += 1;
        }
    }

    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(family, _)| family)
}

fn transition_role_for_target(
    source: &str,
    target: &str,
    neighbors: &TerrainNeighborFamilies,
) -> Option<TerrainRole> {
    let north = neighbors.north.as_deref() == Some(target);
    let east = neighbors.east.as_deref() == Some(target);
    let south = neighbors.south.as_deref() == Some(target);
    let west = neighbors.west.as_deref() == Some(target);

    if north && east {
        return Some(TerrainRole::Corner(Corner::NorthEast));
    }
    if east && south {
        return Some(TerrainRole::Corner(Corner::SouthEast));
    }
    if south && west {
        return Some(TerrainRole::Corner(Corner::SouthWest));
    }
    if west && north {
        return Some(TerrainRole::Corner(Corner::NorthWest));
    }

    [
        (north, Direction::North),
        (east, Direction::East),
        (south, Direction::South),
        (west, Direction::West),
    ]
    .into_iter()
    .find_map(|(matches, direction)| matches.then_some(TerrainRole::Edge(direction)))
    .or_else(|| {
        neighbor_families(neighbors)
            .into_iter()
            .flatten()
            .any(|family| family != source)
            .then_some(TerrainRole::Center)
    })
}

fn neighbor_families(neighbors: &TerrainNeighborFamilies) -> [Option<&str>; 4] {
    [
        neighbors.north.as_deref(),
        neighbors.east.as_deref(),
        neighbors.south.as_deref(),
        neighbors.west.as_deref(),
    ]
}

fn rotation_between(from: i32, to: i32) -> i32 {
    (to - from).rem_euclid(360)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use content::{AnchorKind, LayerKind, SnapMode};

    use super::*;

    #[test]
    fn chooses_rotated_edge_variant_from_single_base_edge() {
        let rules = TerrainRules::from_assets(&[
            asset("ow_tile_sand_ground"),
            asset("ow_tile_sand_edge_n"),
        ]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_sand_ground",
                &neighbors_for_mask(
                    "sand",
                    TerrainMask {
                        north: true,
                        east: false,
                        south: true,
                        west: true,
                    },
                ),
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_sand_edge_n");
        assert_eq!(choice.rotation, 90);
    }

    #[test]
    fn falls_back_to_center_when_no_variant_exists() {
        let rules = TerrainRules::from_assets(&[asset("ow_tile_sand_ground")]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_sand_ground",
                &neighbors_for_mask(
                    "sand",
                    TerrainMask {
                        north: false,
                        east: true,
                        south: true,
                        west: true,
                    },
                ),
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_sand_ground");
        assert_eq!(choice.rotation, 0);
    }

    #[test]
    fn terrain_tag_can_group_assets_without_hiding_id_role() {
        let mut center = asset("ow_tile_custom_ground");
        center.tags.push("terrain:sand".to_owned());
        let mut edge = asset("ow_tile_custom_edge_n");
        edge.tags.push("terrain:sand".to_owned());

        let rules = TerrainRules::from_assets(&[center, edge]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_custom_ground",
                &neighbors_for_mask(
                    "sand",
                    TerrainMask {
                        north: false,
                        east: true,
                        south: true,
                        west: true,
                    },
                ),
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_custom_edge_n");
    }

    #[test]
    fn keeps_current_center_variant_inside_full_terrain_patch() {
        let mut base = asset("ow_tile_gen_sand_01");
        base.tags.push("terrain:sand".to_owned());
        base.tags.push("terrain_role:center".to_owned());
        let mut variant = asset("ow_tile_gen_sand_02");
        variant.tags.push("terrain:sand".to_owned());
        variant.tags.push("terrain_role:center".to_owned());

        let rules = TerrainRules::from_assets(&[base, variant]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_gen_sand_02",
                &neighbors_for_mask(
                    "sand",
                    TerrainMask {
                        north: true,
                        east: true,
                        south: true,
                        west: true,
                    },
                ),
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_gen_sand_02");
        assert_eq!(choice.rotation, 0);
    }

    #[test]
    fn keeps_current_center_variant_when_family_has_no_edge_variant() {
        let mut base = asset("ow_tile_gen_sand_01");
        base.tags.push("terrain:sand".to_owned());
        base.tags.push("terrain_role:center".to_owned());
        let mut variant = asset("ow_tile_gen_sand_02");
        variant.tags.push("terrain:sand".to_owned());
        variant.tags.push("terrain_role:center".to_owned());

        let rules = TerrainRules::from_assets(&[base, variant]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_gen_sand_02",
                &neighbors_for_mask(
                    "sand",
                    TerrainMask {
                        north: false,
                        east: true,
                        south: true,
                        west: true,
                    },
                ),
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_gen_sand_02");
        assert_eq!(choice.rotation, 0);
    }

    #[test]
    fn directed_transition_edge_wins_against_plain_family_edge() {
        let mut sand = asset("ow_tile_32_sand_center_01");
        sand.tags.push("terrain:t32_sand".to_owned());
        sand.tags.push("material:sand".to_owned());
        sand.tags.push("terrain_role:center".to_owned());
        let mut sand_edge = asset("ow_tile_32_sand_edge_n");
        sand_edge.tags.push("terrain:t32_sand".to_owned());
        sand_edge.tags.push("material:sand".to_owned());
        sand_edge.tags.push("terrain_role:edge_n".to_owned());
        let mut transition = asset("ow_tile_32_sand_to_rock_edge_n");
        transition.tags.push("transition".to_owned());
        transition.tags.push("terrain:t32_sand_to_rock".to_owned());
        transition.tags.push("material:sand".to_owned());
        transition.tags.push("transition_to:rock".to_owned());
        transition.tags.push("terrain_role:edge_n".to_owned());

        let rules = TerrainRules::from_assets(&[sand, sand_edge, transition]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_32_sand_center_01",
                &TerrainNeighborFamilies {
                    north: Some("rock".to_owned()),
                    east: Some("sand".to_owned()),
                    south: Some("sand".to_owned()),
                    west: Some("sand".to_owned()),
                },
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_32_sand_to_rock_edge_n");
        assert_eq!(choice.rotation, 0);
    }

    #[test]
    fn directed_transition_corner_uses_matching_target_sides() {
        let mut sand = asset("ow_tile_32_sand_center_01");
        sand.tags.push("terrain:t32_sand".to_owned());
        sand.tags.push("material:sand".to_owned());
        sand.tags.push("terrain_role:center".to_owned());
        let mut transition = asset("ow_tile_32_sand_to_rock_corner_ne");
        transition.tags.push("transition".to_owned());
        transition.tags.push("terrain:t32_sand_to_rock".to_owned());
        transition.tags.push("material:sand".to_owned());
        transition.tags.push("transition_to:rock".to_owned());
        transition.tags.push("terrain_role:corner_ne".to_owned());

        let rules = TerrainRules::from_assets(&[sand, transition]);
        let choice = rules
            .choice_for_neighbors(
                "ow_tile_32_sand_center_01",
                &TerrainNeighborFamilies {
                    north: Some("rock".to_owned()),
                    east: Some("rock".to_owned()),
                    south: Some("sand".to_owned()),
                    west: Some("sand".to_owned()),
                },
            )
            .unwrap();

        assert_eq!(choice.asset_id, "ow_tile_32_sand_to_rock_corner_ne");
    }

    fn asset(id: &str) -> AssetEntry {
        AssetEntry {
            id: id.to_owned(),
            category: "tiles".to_owned(),
            path: PathBuf::from(format!("assets/sprites/tiles/{id}.png")),
            relative_path: format!("assets/sprites/tiles/{id}.png"),
            kind: AssetKind::Tile,
            default_layer: LayerKind::Ground,
            default_size: [128.0, 128.0],
            footprint: Some([4, 4]),
            default_collision_rect: None,
            default_depth_rect: None,
            default_interaction_rect: None,
            anchor: AnchorKind::TopLeft,
            snap: SnapMode::Grid,
            entity_type: None,
            codex_id: None,
            tags: vec!["tiles".to_owned()],
        }
    }

    fn neighbors_for_mask(family: &str, mask: TerrainMask) -> TerrainNeighborFamilies {
        TerrainNeighborFamilies {
            north: mask.north.then(|| family.to_owned()),
            east: mask.east.then(|| family.to_owned()),
            south: mask.south.then(|| family.to_owned()),
            west: mask.west.then(|| family.to_owned()),
        }
    }
}
