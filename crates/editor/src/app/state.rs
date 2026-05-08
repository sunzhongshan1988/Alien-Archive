use std::{collections::HashMap, path::PathBuf, time::Instant};

use content::{self, AssetDatabase, AssetKind, CodexDatabase, LayerKind, MapDocument};
use eframe::egui::{Color32, Pos2, TextureHandle, Vec2};

use super::{config::EditorConfig, maps::MapListEntry};
use crate::{asset_registry::AssetRegistry, assets::AssetDraft, native_menu, tools::ToolKind};

#[derive(Clone, Copy, Debug)]
pub(crate) struct LayerUiState {
    pub(crate) visible: bool,
    pub(crate) locked: bool,
}

impl Default for LayerUiState {
    fn default() -> Self {
        Self {
            visible: true,
            locked: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ClipboardItem {
    Ground(content::TileInstance),
    Decal(content::ObjectInstance),
    Object(content::ObjectInstance),
    Entity(content::EntityInstance),
    Zone(content::ZoneInstance),
}

#[derive(Clone, Debug)]
pub(crate) struct NewMapDraft {
    pub(crate) id: String,
    pub(crate) mode: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) tile_size: u32,
    pub(crate) spawn_id: String,
    pub(crate) spawn_x: f32,
    pub(crate) spawn_y: f32,
}

impl Default for NewMapDraft {
    fn default() -> Self {
        let document = MapDocument::new_landing_site();
        let spawn = document
            .spawns
            .first()
            .cloned()
            .unwrap_or(content::SpawnPoint {
                id: "player_start".to_owned(),
                x: 4.0,
                y: 4.0,
            });
        Self {
            id: document.id,
            mode: document.mode,
            width: document.width,
            height: document.height,
            tile_size: document.tile_size,
            spawn_id: spawn.id,
            spawn_x: spawn.x,
            spawn_y: spawn.y,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResizeDrag {
    pub(crate) selection: SelectedItem,
}

#[derive(Clone, Debug)]
pub(crate) struct ZoneVertexDrag {
    pub(crate) zone_id: String,
    pub(crate) vertex_index: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SelectionMarquee {
    pub(crate) start: Pos2,
    pub(crate) current: Pos2,
    pub(crate) additive: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct MultiMoveDrag {
    pub(crate) start: [f32; 2],
    pub(crate) origins: Vec<MoveOrigin>,
}

#[derive(Clone, Debug)]
pub(crate) enum MoveOrigin {
    Ground {
        selection: SelectedItem,
        x: i32,
        y: i32,
    },
    ObjectLike {
        selection: SelectedItem,
        x: f32,
        y: f32,
    },
    Zone {
        selection: SelectedItem,
        points: Vec<[f32; 2]>,
    },
}

impl MoveOrigin {
    pub(crate) fn layer(&self) -> LayerKind {
        match self {
            Self::Ground { selection, .. }
            | Self::ObjectLike { selection, .. }
            | Self::Zone { selection, .. } => selection.layer,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedItem {
    pub(crate) layer: LayerKind,
    pub(crate) id: String,
}

impl SelectedItem {
    pub(crate) fn label(&self) -> String {
        format!("{}:{}", self.layer.zh_label(), self.id)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OutlinerBadge {
    pub(crate) label: &'static str,
    pub(crate) color: Color32,
}

#[derive(Clone, Debug)]
pub(crate) struct OutlinerEntry {
    pub(crate) group: &'static str,
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) search_text: String,
    pub(crate) selection: Option<SelectedItem>,
    pub(crate) focus_world: Vec2,
    pub(crate) badges: Vec<OutlinerBadge>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LeftSidebarTab {
    Assets,
    Layers,
    Outliner,
}

impl LeftSidebarTab {
    pub(crate) const ALL: [Self; 3] = [Self::Assets, Self::Layers, Self::Outliner];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Assets => "资源库",
            Self::Layers => "图层",
            Self::Outliner => "对象",
        }
    }
}

pub(crate) struct EditorApp {
    pub(crate) native_menu: native_menu::NativeMenu,
    pub(crate) project_root: PathBuf,
    pub(crate) map_path: PathBuf,
    pub(crate) map_entries: Vec<MapListEntry>,
    pub(crate) selected_map_path: PathBuf,
    pub(crate) pending_open_path: Option<PathBuf>,
    pub(crate) open_confirm_path: Option<PathBuf>,
    pub(crate) delete_confirm_path: Option<PathBuf>,
    pub(crate) config: EditorConfig,
    pub(crate) save_as_id: String,
    pub(crate) dirty: bool,
    pub(crate) registry: AssetRegistry,
    pub(crate) asset_database: AssetDatabase,
    pub(crate) asset_db_dirty: bool,
    pub(crate) codex_database: Option<CodexDatabase>,
    pub(crate) codex_db_status: String,
    pub(crate) show_asset_dialog: bool,
    pub(crate) show_unregistered_assets: bool,
    pub(crate) asset_scan_root: PathBuf,
    pub(crate) asset_editing_id: Option<String>,
    pub(crate) asset_draft: AssetDraft,
    pub(crate) document: MapDocument,
    pub(crate) undo_stack: Vec<MapDocument>,
    pub(crate) redo_stack: Vec<MapDocument>,
    pub(crate) clipboard: Vec<ClipboardItem>,
    pub(crate) selected_asset: Option<String>,
    pub(crate) selected_item: Option<SelectedItem>,
    pub(crate) selected_items: Vec<SelectedItem>,
    pub(crate) active_layer: LayerKind,
    pub(crate) layer_states: HashMap<LayerKind, LayerUiState>,
    pub(crate) tool: ToolKind,
    pub(crate) ground_footprint_w: i32,
    pub(crate) ground_footprint_h: i32,
    pub(crate) terrain_autotile: bool,
    pub(crate) terrain_variation: bool,
    pub(crate) collision_brush_w: i32,
    pub(crate) collision_brush_h: i32,
    pub(crate) rectangle_erase_mode: bool,
    pub(crate) asset_search: String,
    pub(crate) outliner_search: String,
    pub(crate) asset_kind_filter: Option<AssetKind>,
    pub(crate) show_grid: bool,
    pub(crate) show_collision: bool,
    pub(crate) show_entity_bounds: bool,
    pub(crate) show_zones: bool,
    pub(crate) show_left_sidebar: bool,
    pub(crate) active_left_tab: LeftSidebarTab,
    pub(crate) show_right_sidebar: bool,
    pub(crate) show_new_map_dialog: bool,
    pub(crate) show_validation_panel: bool,
    pub(crate) new_map_draft: NewMapDraft,
    pub(crate) validation_issues: Vec<content::MapValidationIssue>,
    pub(crate) last_autosave: Instant,
    pub(crate) rectangle_drag_start: Option<[i32; 2]>,
    pub(crate) rectangle_drag_current: Option<[i32; 2]>,
    pub(crate) lock_aspect_ratio: bool,
    pub(crate) resize_drag: Option<ResizeDrag>,
    pub(crate) selection_marquee: Option<SelectionMarquee>,
    pub(crate) multi_move_drag: Option<MultiMoveDrag>,
    pub(crate) zone_draft_points: Vec<[f32; 2]>,
    pub(crate) zone_vertex_drag: Option<ZoneVertexDrag>,
    pub(crate) show_zone_labels: bool,
    pub(crate) pan: Vec2,
    pub(crate) zoom: f32,
    pub(crate) pending_focus_world: Option<Vec2>,
    pub(crate) mouse_tile: Option<[i32; 2]>,
    pub(crate) thumbnails: HashMap<String, TextureHandle>,
    pub(crate) status: String,
}

pub(crate) fn default_layer_states() -> HashMap<LayerKind, LayerUiState> {
    LayerKind::ALL
        .into_iter()
        .map(|layer| (layer, LayerUiState::default()))
        .collect()
}
