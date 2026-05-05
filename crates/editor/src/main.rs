mod asset_registry;

use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use asset_registry::{AssetEntry, AssetRegistry};
use content::{
    AnchorKind, AssetDatabase, AssetDefinition, AssetKind, DEFAULT_ASSET_DB_PATH, DEFAULT_MAP_PATH,
    InstanceRect, LayerKind, MapDocument, MapValidationIssue, MapValidationSeverity, SnapMode,
    validate_map,
};
use eframe::egui::{
    self, Color32, Context as EguiContext, FontData, FontDefinitions, FontFamily, Key, Modifiers,
    Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2,
    epaint::{Mesh, Vertex},
    vec2,
};
use serde::{Deserialize, Serialize};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Alien Archive Overworld Map Editor",
        options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolKind {
    Select,
    Brush,
    Bucket,
    Rectangle,
    Erase,
    Eyedropper,
    Collision,
    Zone,
    Pan,
    Zoom,
}

impl ToolKind {
    const ALL: [Self; 10] = [
        Self::Select,
        Self::Brush,
        Self::Bucket,
        Self::Rectangle,
        Self::Erase,
        Self::Eyedropper,
        Self::Collision,
        Self::Zone,
        Self::Pan,
        Self::Zoom,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Select => "选择",
            Self::Brush => "画笔",
            Self::Bucket => "油漆桶",
            Self::Rectangle => "矩形",
            Self::Erase => "橡皮",
            Self::Eyedropper => "吸管",
            Self::Collision => "碰撞",
            Self::Zone => "区域",
            Self::Pan => "平移",
            Self::Zoom => "缩放",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LayerUiState {
    visible: bool,
    locked: bool,
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
enum ClipboardItem {
    Ground(content::TileInstance),
    Decal(content::ObjectInstance),
    Object(content::ObjectInstance),
    Entity(content::EntityInstance),
    Zone(content::ZoneInstance),
}

#[derive(Clone, Debug)]
struct NewMapDraft {
    id: String,
    mode: String,
    width: u32,
    height: u32,
    tile_size: u32,
    spawn_id: String,
    spawn_x: f32,
    spawn_y: f32,
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
struct AssetDraft {
    id: String,
    category: String,
    path: String,
    kind: AssetKind,
    default_layer: LayerKind,
    default_size: [f32; 2],
    footprint: [i32; 2],
    anchor: AnchorKind,
    snap: SnapMode,
    tags: String,
    entity_type: String,
    codex_id: String,
}

impl Default for AssetDraft {
    fn default() -> Self {
        Self {
            id: String::new(),
            category: "props".to_owned(),
            path: String::new(),
            kind: AssetKind::Object,
            default_layer: LayerKind::Objects,
            default_size: [72.0, 72.0],
            footprint: [1, 1],
            anchor: AnchorKind::BottomCenter,
            snap: SnapMode::Grid,
            tags: "props".to_owned(),
            entity_type: String::new(),
            codex_id: String::new(),
        }
    }
}

impl AssetDraft {
    fn from_entry(entry: &AssetEntry) -> Self {
        Self {
            id: entry.id.clone(),
            category: entry.category.clone(),
            path: entry.relative_path.clone(),
            kind: entry.kind,
            default_layer: entry.default_layer,
            default_size: entry.default_size,
            footprint: entry
                .footprint
                .unwrap_or_else(|| infer_tile_footprint(entry.default_size, 32).unwrap_or([1, 1])),
            anchor: entry.anchor,
            snap: entry.snap,
            tags: entry.tags.join(", "),
            entity_type: entry.entity_type.clone().unwrap_or_default(),
            codex_id: entry.codex_id.clone().unwrap_or_default(),
        }
    }

    fn to_definition(&self) -> Option<AssetDefinition> {
        let id = sanitize_asset_id(&self.id)?;
        let path = sanitize_relative_path(&self.path)?;
        let category = sanitize_category(&self.category).unwrap_or_else(|| "props".to_owned());
        Some(AssetDefinition {
            id,
            category,
            path: PathBuf::from(path),
            kind: self.kind,
            default_layer: self.default_layer,
            default_size: [self.default_size[0].max(1.0), self.default_size[1].max(1.0)],
            footprint: (self.kind == AssetKind::Tile)
                .then_some([self.footprint[0].max(1), self.footprint[1].max(1)]),
            anchor: self.anchor,
            snap: self.snap,
            tags: parse_tags(&self.tags),
            entity_type: non_empty_string(&self.entity_type),
            codex_id: non_empty_string(&self.codex_id),
        })
    }
}

#[derive(Clone, Debug)]
struct ResizeDrag {
    selection: SelectedItem,
}

#[derive(Clone, Debug)]
struct ZoneVertexDrag {
    zone_id: String,
    vertex_index: usize,
}

#[derive(Clone, Debug)]
struct SelectionMarquee {
    start: Pos2,
    current: Pos2,
    additive: bool,
}

#[derive(Clone, Debug)]
struct MultiMoveDrag {
    start: [f32; 2],
    origins: Vec<MoveOrigin>,
}

#[derive(Clone, Debug)]
enum MoveOrigin {
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
    fn layer(&self) -> LayerKind {
        match self {
            Self::Ground { selection, .. }
            | Self::ObjectLike { selection, .. }
            | Self::Zone { selection, .. } => selection.layer,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EditorConfig {
    recent_maps: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectedItem {
    layer: LayerKind,
    id: String,
}

impl SelectedItem {
    fn label(&self) -> String {
        format!("{}:{}", self.layer.zh_label(), self.id)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MapListEntry {
    label: String,
    path: PathBuf,
}

struct EditorApp {
    project_root: PathBuf,
    map_path: PathBuf,
    map_entries: Vec<MapListEntry>,
    selected_map_path: PathBuf,
    pending_open_path: Option<PathBuf>,
    open_confirm_path: Option<PathBuf>,
    delete_confirm_path: Option<PathBuf>,
    config: EditorConfig,
    save_as_id: String,
    dirty: bool,
    registry: AssetRegistry,
    asset_database: AssetDatabase,
    asset_db_dirty: bool,
    show_asset_dialog: bool,
    show_unregistered_assets: bool,
    asset_scan_root: PathBuf,
    asset_editing_id: Option<String>,
    asset_draft: AssetDraft,
    document: MapDocument,
    undo_stack: Vec<MapDocument>,
    redo_stack: Vec<MapDocument>,
    clipboard: Vec<ClipboardItem>,
    selected_asset: Option<String>,
    selected_item: Option<SelectedItem>,
    selected_items: Vec<SelectedItem>,
    active_layer: LayerKind,
    layer_states: HashMap<LayerKind, LayerUiState>,
    tool: ToolKind,
    ground_footprint_w: i32,
    ground_footprint_h: i32,
    rectangle_erase_mode: bool,
    asset_search: String,
    asset_kind_filter: Option<AssetKind>,
    show_grid: bool,
    show_collision: bool,
    show_entity_bounds: bool,
    show_zones: bool,
    show_new_map_dialog: bool,
    show_validation_panel: bool,
    new_map_draft: NewMapDraft,
    validation_issues: Vec<MapValidationIssue>,
    last_autosave: Instant,
    rectangle_drag_start: Option<[i32; 2]>,
    rectangle_drag_current: Option<[i32; 2]>,
    lock_aspect_ratio: bool,
    resize_drag: Option<ResizeDrag>,
    selection_marquee: Option<SelectionMarquee>,
    multi_move_drag: Option<MultiMoveDrag>,
    zone_draft_points: Vec<[f32; 2]>,
    zone_vertex_drag: Option<ZoneVertexDrag>,
    show_zone_labels: bool,
    pan: Vec2,
    zoom: f32,
    mouse_tile: Option<[i32; 2]>,
    thumbnails: HashMap<String, TextureHandle>,
    status: String,
}

impl EditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let project_root = project_root();
        configure_editor_fonts(&cc.egui_ctx);
        let map_path = project_root.join(DEFAULT_MAP_PATH);
        let map_entries = scan_map_entries(&project_root);
        let config = load_editor_config(&project_root);
        let asset_database = AssetDatabase::load(&project_root.join(DEFAULT_ASSET_DB_PATH))
            .unwrap_or_else(|error| {
                eprintln!("asset database load failed: {error:?}");
                AssetDatabase::new("Overworld")
            });
        let registry = AssetRegistry::from_database(&project_root, asset_database.clone());
        let document =
            MapDocument::load(&map_path).unwrap_or_else(|_| MapDocument::new_landing_site());
        let save_as_id = document.id.clone();
        let mut app = Self {
            project_root: project_root.clone(),
            selected_map_path: map_path.clone(),
            map_path,
            map_entries,
            pending_open_path: None,
            open_confirm_path: None,
            delete_confirm_path: None,
            config,
            save_as_id,
            dirty: false,
            registry,
            asset_database,
            asset_db_dirty: false,
            show_asset_dialog: false,
            show_unregistered_assets: false,
            asset_scan_root: project_root.join("assets").join("sprites"),
            asset_editing_id: None,
            asset_draft: AssetDraft::default(),
            document,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: Vec::new(),
            selected_asset: None,
            selected_item: None,
            selected_items: Vec::new(),
            active_layer: LayerKind::Ground,
            layer_states: default_layer_states(),
            tool: ToolKind::Brush,
            ground_footprint_w: 4,
            ground_footprint_h: 4,
            rectangle_erase_mode: false,
            asset_search: String::new(),
            asset_kind_filter: None,
            show_grid: true,
            show_collision: true,
            show_entity_bounds: false,
            show_zones: true,
            show_new_map_dialog: false,
            show_validation_panel: false,
            new_map_draft: NewMapDraft::default(),
            validation_issues: Vec::new(),
            last_autosave: Instant::now(),
            rectangle_drag_start: None,
            rectangle_drag_current: None,
            lock_aspect_ratio: true,
            resize_drag: None,
            selection_marquee: None,
            multi_move_drag: None,
            zone_draft_points: Vec::new(),
            zone_vertex_drag: None,
            show_zone_labels: true,
            pan: vec2(48.0, 48.0),
            zoom: 1.0,
            mouse_tile: None,
            thumbnails: HashMap::new(),
            status: "Ready".to_owned(),
        };

        app.load_visible_textures(&cc.egui_ctx);
        app
    }

    fn load_visible_textures(&mut self, ctx: &EguiContext) {
        for asset in self.registry.assets() {
            if self.thumbnails.contains_key(&asset.id) {
                continue;
            }

            match load_thumbnail(ctx, asset) {
                Ok(texture) => {
                    self.thumbnails.insert(asset.id.clone(), texture);
                }
                Err(error) => {
                    eprintln!(
                        "failed to load thumbnail {}: {error:?}",
                        asset.relative_path
                    );
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &EguiContext) {
        ctx.input_mut(|input| {
            if input.consume_key(Modifiers::CTRL, Key::S) {
                self.save_map();
            }
            if input.consume_key(Modifiers::CTRL, Key::Z) {
                self.undo();
            }
            if input.consume_key(Modifiers::CTRL, Key::Y) {
                self.redo();
            }
            if input.consume_key(Modifiers::CTRL, Key::C) {
                self.copy_selected_item();
            }
            if input.consume_key(Modifiers::CTRL, Key::V) {
                self.paste_clipboard();
            }
            if input.consume_key(Modifiers::CTRL, Key::D) {
                self.duplicate_selected_item();
            }
            if input.key_pressed(Key::Delete) {
                self.delete_current_selection();
            }
            if input.key_pressed(Key::Num1) {
                self.tool = ToolKind::Select;
            }
            if input.key_pressed(Key::Num2) {
                self.tool = ToolKind::Brush;
            }
            if input.key_pressed(Key::Num3) {
                self.tool = ToolKind::Bucket;
            }
            if input.key_pressed(Key::Num4) {
                self.tool = ToolKind::Rectangle;
            }
            if input.key_pressed(Key::Num5) {
                self.tool = ToolKind::Erase;
            }
            if input.key_pressed(Key::Num6) {
                self.tool = ToolKind::Eyedropper;
            }
            if input.key_pressed(Key::Num7) {
                self.tool = ToolKind::Collision;
                self.active_layer = LayerKind::Collision;
            }
            if input.key_pressed(Key::Escape) && !self.zone_draft_points.is_empty() {
                self.zone_draft_points.clear();
                self.status = "已取消区域绘制".to_owned();
            }
        });
    }

    fn save_map(&mut self) {
        self.validation_issues = self.validate_current_map();
        if self
            .validation_issues
            .iter()
            .any(|issue| issue.severity == MapValidationSeverity::Error)
        {
            self.show_validation_panel = true;
            self.status = "保存失败：地图校验有错误".to_owned();
            return;
        }

        match self.document.save(&self.map_path) {
            Ok(()) => {
                self.dirty = false;
                self.pending_open_path = None;
                self.open_confirm_path = None;
                self.refresh_map_entries();
                self.push_recent_map(self.map_path.clone());
                self.status = format!(
                    "Saved {}",
                    display_project_path(&self.project_root, &self.map_path)
                );
            }
            Err(error) => {
                self.status = format!("Save failed: {error:#}");
            }
        }
    }

    fn save_map_as(&mut self) {
        let Some(id) = sanitize_map_id(&self.save_as_id) else {
            self.status = "Save As failed: map id is empty".to_owned();
            return;
        };

        self.push_undo_snapshot();
        self.document.id = id.clone();
        self.save_as_id = id.clone();
        self.map_path = maps_dir(&self.project_root).join(format!("{id}.ron"));
        self.selected_map_path = self.map_path.clone();
        self.save_map();
    }

    fn open_selected_map(&mut self) {
        let path = self.selected_map_path.clone();
        if self.dirty && path != self.map_path {
            self.open_confirm_path = Some(path);
            return;
        }

        self.open_map(path);
    }

    fn open_map(&mut self, path: PathBuf) {
        match MapDocument::load(&path) {
            Ok(document) => {
                self.map_path = path.clone();
                self.document = document;
                self.save_as_id = self.document.id.clone();
                self.clear_selection();
                self.selected_asset = None;
                self.pending_open_path = None;
                self.open_confirm_path = None;
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.active_layer = LayerKind::Ground;
                self.dirty = false;
                self.push_recent_map(path.clone());
                self.status = format!(
                    "Opened {}",
                    display_project_path(&self.project_root, &self.map_path)
                );
            }
            Err(error) => {
                self.status = format!(
                    "Open failed for {}: {error:#}",
                    display_project_path(&self.project_root, &path)
                );
            }
        }
    }

    fn refresh_map_entries(&mut self) {
        self.map_entries = scan_map_entries(&self.project_root);
        if !self
            .map_entries
            .iter()
            .any(|entry| entry.path == self.selected_map_path)
        {
            self.selected_map_path = self.map_path.clone();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.pending_open_path = None;
        self.validation_issues.clear();
    }

    fn push_undo_snapshot(&mut self) {
        if self.undo_stack.last() == Some(&self.document) {
            return;
        }
        self.undo_stack.push(self.document.clone());
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        let Some(previous) = self.undo_stack.pop() else {
            self.status = "没有可撤销的操作".to_owned();
            return;
        };
        self.redo_stack.push(self.document.clone());
        self.document = previous;
        self.clear_selection();
        self.mark_dirty();
        self.status = "已撤销".to_owned();
    }

    fn redo(&mut self) {
        let Some(next) = self.redo_stack.pop() else {
            self.status = "没有可重做的操作".to_owned();
            return;
        };
        self.undo_stack.push(self.document.clone());
        self.document = next;
        self.clear_selection();
        self.mark_dirty();
        self.status = "已重做".to_owned();
    }

    fn validate_current_map(&self) -> Vec<MapValidationIssue> {
        validate_map(&self.document, &self.asset_database)
    }

    fn asset_db_path(&self) -> PathBuf {
        self.project_root.join(DEFAULT_ASSET_DB_PATH)
    }

    fn reload_asset_database(&mut self, ctx: &EguiContext) {
        match AssetDatabase::load(&self.asset_db_path()) {
            Ok(database) => {
                self.asset_database = database;
                self.asset_db_dirty = false;
                self.rebuild_asset_registry(ctx);
                self.status = "素材 metadata 已重新加载".to_owned();
            }
            Err(error) => {
                self.status = format!("素材 metadata 读取失败：{error:#}");
            }
        }
    }

    fn save_asset_database(&mut self) {
        self.asset_database.reindex();
        match self.asset_database.save(&self.asset_db_path()) {
            Ok(()) => {
                self.asset_db_dirty = false;
                self.status = "素材库已保存".to_owned();
            }
            Err(error) => {
                self.status = format!("素材库保存失败：{error:#}");
            }
        }
    }

    fn rebuild_asset_registry(&mut self, ctx: &EguiContext) {
        self.asset_database.reindex();
        self.registry =
            AssetRegistry::from_database(&self.project_root, self.asset_database.clone());
        self.load_visible_textures(ctx);
    }

    fn open_add_asset_dialog(&mut self) {
        self.asset_editing_id = None;
        self.asset_draft = AssetDraft::default();
        self.show_asset_dialog = true;
    }

    fn open_edit_asset_dialog(&mut self, asset_id: &str) {
        let Some(asset) = self.registry.get(asset_id) else {
            self.status = format!("找不到素材 {asset_id}");
            return;
        };
        self.asset_editing_id = Some(asset.id.clone());
        self.asset_draft = AssetDraft::from_entry(asset);
        self.show_asset_dialog = true;
    }

    fn apply_asset_draft(&mut self, ctx: &EguiContext) {
        let Some(asset) = self.asset_draft.to_definition() else {
            self.status = "素材保存失败：id 或 path 为空".to_owned();
            return;
        };
        if !self.project_root.join(&asset.path).exists() {
            self.status = format!("素材保存失败：图片不存在 {}", asset.path.to_string_lossy());
            return;
        }

        let editing_id = self.asset_editing_id.clone();
        let duplicate = self.asset_database.assets.iter().any(|existing| {
            existing.id == asset.id && editing_id.as_deref() != Some(existing.id.as_str())
        });
        if duplicate {
            self.status = format!("素材保存失败：id {} 已存在", asset.id);
            return;
        }

        if let Some(editing_id) = editing_id {
            if let Some(existing) = self
                .asset_database
                .assets
                .iter_mut()
                .find(|existing| existing.id == editing_id)
            {
                *existing = asset.clone();
            } else {
                self.asset_database.assets.push(asset.clone());
            }
        } else {
            self.asset_database.assets.push(asset.clone());
        }

        self.asset_database.assets.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| left.id.cmp(&right.id))
        });
        self.selected_asset = Some(asset.id.clone());
        self.asset_db_dirty = true;
        self.show_asset_dialog = false;
        self.rebuild_asset_registry(ctx);
        self.status = format!("素材已登记：{}", asset.id);
    }

    fn delete_selected_asset_definition(&mut self, ctx: &EguiContext) {
        let Some(asset_id) = self.selected_asset.clone() else {
            self.status = "请先选择素材".to_owned();
            return;
        };
        let before = self.asset_database.assets.len();
        self.asset_database
            .assets
            .retain(|asset| asset.id != asset_id);
        if self.asset_database.assets.len() == before {
            self.status = format!("素材不存在：{asset_id}");
            return;
        }
        self.selected_asset = None;
        self.asset_db_dirty = true;
        self.rebuild_asset_registry(ctx);
        self.status = format!("已从素材库移除 {asset_id}，保存地图前校验会检查引用");
    }

    fn fill_asset_draft_from_path(&mut self, relative_path: &str) {
        self.asset_draft = infer_asset_draft_from_path(&self.project_root, relative_path);
        self.asset_editing_id = None;
        self.show_asset_dialog = true;
    }

    fn add_asset_definitions_from_paths(&mut self, paths: Vec<PathBuf>, ctx: &EguiContext) {
        let mut added = 0usize;
        let mut skipped = 0usize;
        let mut last_added_id = None;

        for path in paths {
            let Some(relative_path) = project_relative_path(&self.project_root, &path) else {
                skipped += 1;
                continue;
            };
            if !relative_path.to_ascii_lowercase().ends_with(".png") {
                skipped += 1;
                continue;
            }
            if self.registry.contains_path(&relative_path)
                || self
                    .asset_database
                    .assets
                    .iter()
                    .any(|asset| asset.path.to_string_lossy().replace('\\', "/") == relative_path)
            {
                skipped += 1;
                continue;
            }

            let mut draft = infer_asset_draft_from_path(&self.project_root, &relative_path);
            draft.id = unique_asset_id(&self.asset_database, &draft.id);
            let Some(asset) = draft.to_definition() else {
                skipped += 1;
                continue;
            };
            last_added_id = Some(asset.id.clone());
            self.asset_database.assets.push(asset);
            added += 1;
        }

        if added > 0 {
            self.asset_database.assets.sort_by(|left, right| {
                left.category
                    .cmp(&right.category)
                    .then_with(|| left.id.cmp(&right.id))
            });
            self.selected_asset = last_added_id;
            self.asset_db_dirty = true;
            self.rebuild_asset_registry(ctx);
        }

        self.status = format!("批量登记素材：新增 {added}，跳过 {skipped}");
    }

    fn pick_asset_file_into_draft(&mut self, ctx: &EguiContext) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title("选择 PNG 素材")
            .set_directory(self.project_root.join("assets").join("sprites"))
            .add_filter("PNG 图片", &["png"])
            .pick_files()
        else {
            return;
        };

        if paths.len() > 1 {
            self.add_asset_definitions_from_paths(paths, ctx);
            return;
        }

        let Some(path) = paths.into_iter().next() else {
            return;
        };
        let Some(relative_path) = project_relative_path(&self.project_root, &path) else {
            self.status = "请选择项目目录内的 PNG，或先把图片放进 assets/sprites".to_owned();
            return;
        };
        if !relative_path.to_ascii_lowercase().ends_with(".png") {
            self.status = "请选择 PNG 图片".to_owned();
            return;
        }

        self.asset_draft = infer_asset_draft_from_path(&self.project_root, &relative_path);
        self.asset_editing_id = None;
        self.status = format!("已选择素材文件 {relative_path}");
    }

    fn pick_and_add_asset_files(&mut self, ctx: &EguiContext) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title("批量选择 PNG 素材")
            .set_directory(self.project_root.join("assets").join("sprites"))
            .add_filter("PNG 图片", &["png"])
            .pick_files()
        else {
            return;
        };
        self.add_asset_definitions_from_paths(paths, ctx);
    }

    fn pick_and_add_asset_folder(&mut self, ctx: &EguiContext) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("选择要批量登记的素材文件夹")
            .set_directory(self.project_root.join("assets").join("sprites"))
            .pick_folder()
        else {
            return;
        };
        if project_relative_path(&self.project_root, &path).is_none() {
            self.status = "请选择项目目录内的素材文件夹".to_owned();
            return;
        }
        let mut paths = Vec::new();
        collect_png_paths(&path, &mut paths);
        self.add_asset_definitions_from_paths(paths, ctx);
    }

    fn pick_asset_scan_folder(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("选择素材扫描文件夹")
            .set_directory(self.project_root.join("assets").join("sprites"))
            .pick_folder()
        else {
            return;
        };

        if project_relative_path(&self.project_root, &path).is_none() {
            self.status = "请选择项目目录内的素材文件夹".to_owned();
            return;
        }
        self.asset_scan_root = path;
        self.status = format!(
            "扫描文件夹：{}",
            display_project_path(&self.project_root, &self.asset_scan_root)
        );
    }

    fn unregistered_sprite_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        collect_png_paths(&self.asset_scan_root, &mut paths);
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| Some(project_relative_path(&self.project_root, &path)?))
            .filter(|path| {
                path.contains("/overworld/")
                    && !path.contains("overworld_originals")
                    && !self.registry.contains_path(path)
            })
            .collect()
    }

    fn push_recent_map(&mut self, path: PathBuf) {
        let label = display_project_path(&self.project_root, &path);
        self.config.recent_maps.retain(|entry| entry != &label);
        self.config.recent_maps.insert(0, label);
        self.config.recent_maps.truncate(10);
        let _ = save_editor_config(&self.project_root, &self.config);
    }

    fn delete_map_file(&mut self, path: PathBuf) {
        match fs::remove_file(&path) {
            Ok(()) => {
                let label = display_project_path(&self.project_root, &path);
                self.config.recent_maps.retain(|entry| entry != &label);
                let _ = save_editor_config(&self.project_root, &self.config);
                self.refresh_map_entries();
                self.delete_confirm_path = None;

                if path == self.map_path {
                    if let Some(next) = self
                        .map_entries
                        .iter()
                        .find(|entry| entry.path != path)
                        .map(|entry| entry.path.clone())
                    {
                        self.open_map(next);
                    } else {
                        self.document = MapDocument::new_landing_site();
                        let id = unique_map_id(&self.project_root, &self.document.id);
                        self.document.id = id.clone();
                        self.map_path = maps_dir(&self.project_root).join(format!("{id}.ron"));
                        self.selected_map_path = self.map_path.clone();
                        self.save_as_id = id;
                        self.clear_selection();
                        self.selected_asset = None;
                        self.undo_stack.clear();
                        self.redo_stack.clear();
                        self.dirty = true;
                    }
                }

                self.status = format!("已删除地图 {label}");
            }
            Err(error) => {
                self.status = format!(
                    "删除地图失败 {}：{error:#}",
                    display_project_path(&self.project_root, &path)
                );
            }
        }
    }

    fn autosave_if_needed(&mut self) {
        if !self.dirty || self.last_autosave.elapsed() < Duration::from_secs(60) {
            return;
        }
        self.last_autosave = Instant::now();
        let path = maps_dir(&self.project_root)
            .join(".autosave")
            .join(format!("{}.ron", self.document.id));
        match self.document.save(&path) {
            Ok(()) => {
                self.status = format!(
                    "Autosaved {}",
                    display_project_path(&self.project_root, &path)
                );
            }
            Err(error) => {
                self.status = format!("Autosave failed: {error:#}");
            }
        }
    }

    fn layer_state(&self, layer: LayerKind) -> LayerUiState {
        self.layer_states.get(&layer).copied().unwrap_or_default()
    }

    fn active_layer_locked(&self) -> bool {
        self.layer_state(self.active_layer).locked
    }

    fn set_single_selection(&mut self, selection: Option<SelectedItem>) {
        self.selected_item = selection.clone();
        self.selected_items = selection.into_iter().collect();
    }

    fn set_selection(&mut self, mut selections: Vec<SelectedItem>) {
        let mut deduped = Vec::with_capacity(selections.len());
        for selection in selections.drain(..) {
            if !deduped.contains(&selection) {
                deduped.push(selection);
            }
        }
        self.selected_item = deduped.first().cloned();
        self.selected_items = deduped;
    }

    fn clear_selection(&mut self) {
        self.selected_item = None;
        self.selected_items.clear();
    }

    fn current_selection_list(&self) -> Vec<SelectedItem> {
        if !self.selected_items.is_empty() {
            self.selected_items.clone()
        } else {
            self.selected_item.clone().into_iter().collect()
        }
    }

    fn toggle_selection(&mut self, selection: SelectedItem) {
        let mut selections = self.current_selection_list();
        if let Some(index) = selections.iter().position(|item| item == &selection) {
            selections.remove(index);
        } else {
            selections.push(selection);
        }
        self.set_selection(selections);
    }

    fn copy_selected_item(&mut self) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            self.status = "请先选择对象".to_owned();
            return;
        }

        self.clipboard = selections
            .iter()
            .filter_map(|selection| self.clipboard_for_selection(selection))
            .collect();
        self.status = if self.clipboard.is_empty() {
            "当前选择不能复制".to_owned()
        } else if self.clipboard.len() == 1 {
            format!("已复制 {}", selections[0].label())
        } else {
            format!("已复制 {} 个对象", self.clipboard.len())
        };
    }

    fn paste_clipboard(&mut self) {
        if self.clipboard.is_empty() {
            self.status = "剪贴板为空".to_owned();
            return;
        }
        self.push_undo_snapshot();
        let mut next_selection = Vec::new();
        for item in self.clipboard.clone() {
            let selection = match item {
                ClipboardItem::Ground(mut tile) => {
                    tile.x = (tile.x + 1).clamp(0, self.document.width.saturating_sub(1) as i32);
                    tile.y = (tile.y + 1).clamp(0, self.document.height.saturating_sub(1) as i32);
                    self.document.layers.ground.push(tile.clone());
                    SelectedItem {
                        layer: LayerKind::Ground,
                        id: ground_selection_id(tile.x, tile.y),
                    }
                }
                ClipboardItem::Decal(mut instance) => {
                    instance.id = next_editor_object_id("decal", &self.document.layers.decals);
                    instance.x += 1.0;
                    instance.y += 1.0;
                    self.document.layers.decals.push(instance.clone());
                    SelectedItem {
                        layer: LayerKind::Decals,
                        id: instance.id,
                    }
                }
                ClipboardItem::Object(mut instance) => {
                    instance.id = next_editor_object_id("obj", &self.document.layers.objects);
                    instance.x += 1.0;
                    instance.y += 1.0;
                    self.document.layers.objects.push(instance.clone());
                    SelectedItem {
                        layer: LayerKind::Objects,
                        id: instance.id,
                    }
                }
                ClipboardItem::Entity(mut instance) => {
                    instance.id = next_editor_entity_id("ent", &self.document.layers.entities);
                    instance.x += 1.0;
                    instance.y += 1.0;
                    self.document.layers.entities.push(instance.clone());
                    SelectedItem {
                        layer: LayerKind::Entities,
                        id: instance.id,
                    }
                }
                ClipboardItem::Zone(mut zone) => {
                    zone.id = next_editor_zone_id("zone", &self.document.layers.zones);
                    for point in &mut zone.points {
                        point[0] += 1.0;
                        point[1] += 1.0;
                    }
                    self.document.layers.zones.push(zone.clone());
                    SelectedItem {
                        layer: LayerKind::Zones,
                        id: zone.id,
                    }
                }
            };
            next_selection.push(selection);
        }
        self.set_selection(next_selection);
        self.mark_dirty();
        self.status = format!("已粘贴 {} 个对象", self.selected_items.len().max(1));
    }

    fn duplicate_selected_item(&mut self) {
        self.copy_selected_item();
        self.paste_clipboard();
    }

    fn delete_current_selection(&mut self) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            self.status = "请先选择对象".to_owned();
            return;
        }

        let editable = selections
            .into_iter()
            .filter(|selection| !self.layer_state(selection.layer).locked)
            .collect::<Vec<_>>();
        if editable.is_empty() {
            self.status = "所选图层已锁定".to_owned();
            return;
        }

        self.push_undo_snapshot();
        for selection in &editable {
            self.delete_selected_item(selection);
        }
        self.clear_selection();
        self.mark_dirty();
        self.status = format!("已删除 {} 个对象", editable.len());
    }

    fn clipboard_for_selection(&self, selection: &SelectedItem) -> Option<ClipboardItem> {
        match selection.layer {
            LayerKind::Ground => {
                let [x, y] = parse_ground_selection_id(&selection.id)?;
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .cloned()
                    .map(ClipboardItem::Ground)
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .cloned()
                .map(ClipboardItem::Decal),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .cloned()
                .map(ClipboardItem::Object),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .cloned()
                .map(ClipboardItem::Entity),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .iter()
                .find(|zone| zone.id == selection.id)
                .cloned()
                .map(ClipboardItem::Zone),
            LayerKind::Collision => None,
        }
    }

    fn asset_for_selection(&self, selection: &SelectedItem) -> Option<String> {
        match selection.layer {
            LayerKind::Ground => {
                let [x, y] = parse_ground_selection_id(&selection.id)?;
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .map(|tile| tile.asset.clone())
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| instance.asset.clone()),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| instance.asset.clone()),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| instance.asset.clone()),
            LayerKind::Zones | LayerKind::Collision => None,
        }
    }

    fn selected_asset(&self) -> Option<&AssetEntry> {
        self.selected_asset
            .as_deref()
            .and_then(|id| self.registry.get(id))
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        self.draw_menu_bar(ui);
        ui.separator();
        self.draw_tool_bar(ui);
    }

    fn draw_menu_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("文件", |ui| {
                if ui.button("新建地图").clicked() {
                    self.new_map_draft = NewMapDraft::default();
                    self.show_new_map_dialog = true;
                    ui.close();
                }
                ui.menu_button("打开地图", |ui| {
                    for entry in self.map_entries.clone() {
                        if ui.button(&entry.label).clicked() {
                            self.selected_map_path = entry.path.clone();
                            self.open_selected_map();
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("刷新列表").clicked() {
                        self.refresh_map_entries();
                        ui.close();
                    }
                });
                ui.menu_button("最近地图", |ui| {
                    if self.config.recent_maps.is_empty() {
                        ui.label("暂无");
                    }
                    for recent in self.config.recent_maps.clone() {
                        if ui.button(&recent).clicked() {
                            let path = self.project_root.join(&recent);
                            if self.dirty && path != self.map_path {
                                self.open_confirm_path = Some(path);
                            } else {
                                self.open_map(path);
                            }
                            ui.close();
                        }
                    }
                });
                if ui.button("保存").clicked() {
                    self.save_map();
                    ui.close();
                }
                if ui.button("另存为").clicked() {
                    self.save_map_as();
                    ui.close();
                }
                if ui.button("删除地图").clicked() {
                    self.delete_confirm_path = Some(self.selected_map_path.clone());
                    ui.close();
                }
                if ui.button("还原").clicked() {
                    if self.dirty {
                        self.open_confirm_path = Some(self.map_path.clone());
                    }
                    ui.close();
                }
            });

            ui.menu_button("编辑", |ui| {
                if ui
                    .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("撤销"))
                    .clicked()
                {
                    self.undo();
                    ui.close();
                }
                if ui
                    .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("重做"))
                    .clicked()
                {
                    self.redo();
                    ui.close();
                }
                ui.separator();
                if ui.button("复制").clicked() {
                    self.copy_selected_item();
                    ui.close();
                }
                if ui
                    .add_enabled(!self.clipboard.is_empty(), egui::Button::new("粘贴"))
                    .clicked()
                {
                    self.paste_clipboard();
                    ui.close();
                }
                if ui.button("复制一份").clicked() {
                    self.duplicate_selected_item();
                    ui.close();
                }
                if ui.button("删除").clicked() {
                    self.delete_current_selection();
                    ui.close();
                }
            });

            ui.menu_button("视图", |ui| {
                ui.checkbox(&mut self.show_grid, "网格");
                ui.checkbox(&mut self.show_collision, "碰撞");
                ui.checkbox(&mut self.show_entity_bounds, "实体边界");
                ui.checkbox(&mut self.show_zones, "区域");
                ui.checkbox(&mut self.show_zone_labels, "区域标签");
                ui.separator();
                if ui.button("重置视图").clicked() {
                    self.pan = vec2(48.0, 48.0);
                    self.zoom = 1.0;
                    ui.close();
                }
            });

            ui.menu_button("地图", |ui| {
                if ui.button("校验地图").clicked() {
                    self.validation_issues = self.validate_current_map();
                    self.show_validation_panel = true;
                    self.status = validation_summary(&self.validation_issues);
                    ui.close();
                }
                ui.label(format!(
                    "{} / {}x{} / {}px",
                    self.document.id,
                    self.document.width,
                    self.document.height,
                    self.document.tile_size
                ));
            });

            ui.menu_button("图层", |ui| {
                for layer in LayerKind::ALL {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_layer, layer, layer.zh_label());
                        let state = self.layer_states.entry(layer).or_default();
                        ui.checkbox(&mut state.visible, "显示");
                        ui.checkbox(&mut state.locked, "锁定");
                    });
                }
            });

            ui.menu_button("工具", |ui| {
                for tool in ToolKind::ALL {
                    if ui
                        .selectable_value(&mut self.tool, tool, tool.label())
                        .clicked()
                    {
                        if tool == ToolKind::Collision {
                            self.active_layer = LayerKind::Collision;
                        } else if tool == ToolKind::Zone {
                            self.active_layer = LayerKind::Zones;
                        }
                        ui.close();
                    }
                }
            });

            ui.menu_button("素材", |ui| {
                let ctx = ui.ctx().clone();
                if ui.button("添加素材").clicked() {
                    self.open_add_asset_dialog();
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.selected_asset.is_some(),
                        egui::Button::new("编辑当前素材"),
                    )
                    .clicked()
                {
                    if let Some(asset_id) = self.selected_asset.clone() {
                        self.open_edit_asset_dialog(&asset_id);
                    }
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.selected_asset.is_some(),
                        egui::Button::new("移除当前素材"),
                    )
                    .clicked()
                {
                    self.delete_selected_asset_definition(&ctx);
                    ui.close();
                }
                if ui
                    .add_enabled(self.asset_db_dirty, egui::Button::new("保存素材库"))
                    .clicked()
                {
                    self.save_asset_database();
                    ui.close();
                }
                if ui.button("未登记图片").clicked() {
                    self.show_unregistered_assets = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("重新扫描 Metadata").clicked() {
                    self.reload_asset_database(&ctx);
                    ui.close();
                }
                ui.label(format!(
                    "{} 个素材{}",
                    self.registry.assets().len(),
                    if self.asset_db_dirty { " *" } else { "" }
                ));
            });

            ui.menu_button("帮助", |ui| {
                ui.label("Ctrl+S 保存");
                ui.label("Ctrl+Z 撤销 / Ctrl+Y 重做");
                ui.label("1-6 切换常用工具");
                ui.label("空格拖拽平移，滚轮缩放");
            });
        });
    }

    fn draw_tool_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("工具");
            for tool in ToolKind::ALL {
                if ui
                    .selectable_value(&mut self.tool, tool, tool.label())
                    .clicked()
                {
                    if tool == ToolKind::Collision {
                        self.active_layer = LayerKind::Collision;
                    } else if tool == ToolKind::Zone {
                        self.active_layer = LayerKind::Zones;
                    }
                }
            }

            ui.separator();
            ui.label("图层");
            egui::ComboBox::from_id_salt("active_layer")
                .selected_text(self.active_layer.zh_label())
                .show_ui(ui, |ui| {
                    for layer in LayerKind::ALL {
                        ui.selectable_value(&mut self.active_layer, layer, layer.zh_label());
                    }
                });

            if self.active_layer == LayerKind::Ground {
                ui.separator();
                ui.label("画笔尺寸");
                ui.add(
                    egui::DragValue::new(&mut self.ground_footprint_w)
                        .range(1..=self.document.width as i32)
                        .speed(0.1)
                        .prefix("W "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.ground_footprint_h)
                        .range(1..=self.document.height as i32)
                        .speed(0.1)
                        .prefix("H "),
                );
            }
            if self.tool == ToolKind::Rectangle {
                ui.separator();
                ui.checkbox(&mut self.rectangle_erase_mode, "矩形擦除");
            }

            ui.separator();
            if ui.button("水平翻转").clicked() {
                self.flip_selected_item();
            }
            if ui.button("左转").clicked() {
                self.rotate_selected_item(-90);
            }
            if ui.button("右转").clicked() {
                self.rotate_selected_item(90);
            }
            if ui.button("重置变换").clicked() {
                self.reset_selected_transform();
            }

            if let Some([mut width, mut height]) = self.ground_size_for_selection() {
                ui.separator();
                ui.label("选中地块");
                let width_changed = ui
                    .add(
                        egui::DragValue::new(&mut width)
                            .range(1..=self.document.width as i32)
                            .speed(0.1)
                            .prefix("W "),
                    )
                    .changed();
                let height_changed = ui
                    .add(
                        egui::DragValue::new(&mut height)
                            .range(1..=self.document.height as i32)
                            .speed(0.1)
                            .prefix("H "),
                    )
                    .changed();
                if width_changed || height_changed {
                    self.set_ground_size_for_selection(width, height);
                }
            }

            ui.separator();
            ui.checkbox(&mut self.show_grid, "网格");
            ui.checkbox(&mut self.show_collision, "碰撞");
            ui.checkbox(&mut self.show_entity_bounds, "实体边界");
        });
    }

    fn draw_asset_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("资源库");
        ui.small(format!("{} 个 metadata 素材", self.registry.assets().len()));
        ui.add(
            egui::TextEdit::singleline(&mut self.asset_search)
                .hint_text("搜索 id / tag / path")
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.asset_kind_filter, None, "全部");
            for kind in AssetKind::ALL {
                ui.selectable_value(&mut self.asset_kind_filter, Some(kind), kind.zh_label());
            }
        });
        ui.separator();

        let search = self.asset_search.to_ascii_lowercase();
        let categories = self
            .registry
            .categories()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for category in categories {
            egui::CollapsingHeader::new(category_label(&category))
                .default_open(category == "tiles" || category == "props")
                .show(ui, |ui| {
                    let assets = self
                        .registry
                        .in_category(&category)
                        .filter(|asset| {
                            !self
                                .asset_kind_filter
                                .is_some_and(|kind| kind != asset.kind)
                                && (search.is_empty() || asset_matches_search(asset, &search))
                        })
                        .cloned()
                        .collect::<Vec<_>>();

                    if category == "tiles" {
                        self.draw_tile_asset_grid(ui, &assets);
                        return;
                    }

                    for asset in assets {
                        self.draw_asset_list_row(ui, &asset);
                    }
                });
        }
    }

    fn draw_tile_asset_grid(&mut self, ui: &mut egui::Ui, assets: &[AssetEntry]) {
        let slot = vec2(56.0, 64.0);
        let columns = (ui.available_width() / slot.x).floor().max(1.0) as usize;
        egui::Grid::new("tile_asset_grid")
            .num_columns(columns)
            .spacing(vec2(6.0, 6.0))
            .show(ui, |ui| {
                for (index, asset) in assets.iter().enumerate() {
                    let selected = self.selected_asset.as_deref() == Some(asset.id.as_str());
                    let (rect, response) = ui.allocate_exact_size(slot, Sense::click());
                    let fill = if selected {
                        Color32::from_rgb(62, 88, 82)
                    } else {
                        Color32::from_rgb(31, 35, 37)
                    };
                    ui.painter().rect_filled(rect, 3.0, fill);
                    ui.painter().rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(
                            if selected { 2.0 } else { 1.0 },
                            if selected {
                                Color32::from_rgb(120, 235, 170)
                            } else {
                                Color32::from_rgb(65, 72, 75)
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    let image_slot =
                        Rect::from_min_size(rect.min + vec2(8.0, 4.0), vec2(40.0, 40.0));
                    if let Some(texture) = self.thumbnails.get(&asset.id) {
                        let image_rect = fit_centered_rect(image_slot, texture.size_vec2());
                        ui.painter().image(
                            texture.id(),
                            image_rect,
                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    let label_rect =
                        Rect::from_min_size(rect.min + vec2(4.0, 46.0), vec2(48.0, 14.0));
                    ui.painter().text(
                        label_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        compact_asset_label(&asset.id),
                        egui::TextStyle::Small.resolve(ui.style()),
                        Color32::from_rgb(220, 228, 224),
                    );

                    if response.clicked() {
                        self.select_asset(asset);
                    }
                    response.on_hover_text(&asset.id);

                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn draw_asset_list_row(&mut self, ui: &mut egui::Ui, asset: &AssetEntry) {
        if self
            .asset_kind_filter
            .is_some_and(|kind| kind != asset.kind)
        {
            return;
        }
        let selected = self.selected_asset.as_deref() == Some(asset.id.as_str());
        let response = ui
            .horizontal(|ui| {
                if let Some(texture) = self.thumbnails.get(&asset.id) {
                    let (slot_rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                    let image_rect = fit_centered_rect(slot_rect, texture.size_vec2());
                    ui.painter().image(
                        texture.id(),
                        image_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, Color32::DARK_GRAY);
                }

                ui.selectable_label(selected, &asset.id)
            })
            .inner;

        if response.clicked() {
            self.select_asset(asset);
        }
    }

    fn select_asset(&mut self, asset: &AssetEntry) {
        self.selected_asset = Some(asset.id.clone());
        self.clear_selection();
        self.active_layer = asset.default_layer;
        self.tool = ToolKind::Brush;
        if asset.kind == AssetKind::Tile {
            let footprint = self.asset_tile_footprint(asset);
            self.ground_footprint_w = footprint[0];
            self.ground_footprint_h = footprint[1];
        }
        self.status = format!("Selected {}", asset.id);
    }

    fn draw_layer_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("图层");
        ui.separator();
        for layer in LayerKind::ALL {
            let count = self.layer_item_count(layer);
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_layer,
                    layer,
                    format!("{} ({count})", layer.zh_label()),
                );
                let state = self.layer_states.entry(layer).or_default();
                ui.checkbox(&mut state.visible, "显");
                ui.checkbox(&mut state.locked, "锁");
            });
        }
    }

    fn layer_item_count(&self, layer: LayerKind) -> usize {
        match layer {
            LayerKind::Ground => self.document.layers.ground.len(),
            LayerKind::Decals => self.document.layers.decals.len(),
            LayerKind::Objects => self.document.layers.objects.len(),
            LayerKind::Entities => self.document.layers.entities.len(),
            LayerKind::Zones => self.document.layers.zones.len(),
            LayerKind::Collision => self.document.layers.collision.len(),
        }
    }

    fn draw_inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();

        let selections = self.current_selection_list();
        if selections.len() > 1 {
            self.draw_multi_selection_inspector(ui, selections);
        } else if let Some(selection) = selections.into_iter().next() {
            self.draw_selection_inspector(ui, selection);
        } else if let Some(asset) = self.selected_asset().cloned() {
            self.draw_asset_inspector(ui, &asset);
        } else {
            self.draw_map_inspector(ui);
        }
    }

    fn draw_multi_selection_inspector(&mut self, ui: &mut egui::Ui, selections: Vec<SelectedItem>) {
        ui.label(format!("多选：{} 个对象", selections.len()));
        for layer in LayerKind::ALL {
            let count = selections
                .iter()
                .filter(|selection| selection.layer == layer)
                .count();
            if count > 0 {
                ui.label(format!("{}：{}", layer.zh_label(), count));
            }
        }

        if selections
            .iter()
            .any(|selection| self.layer_state(selection.layer).locked)
        {
            ui.colored_label(Color32::YELLOW, "部分所选图层已锁定，批量编辑会跳过它们");
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("复制").clicked() {
                self.copy_selected_item();
            }
            if ui.button("复制一份").clicked() {
                self.duplicate_selected_item();
            }
            if ui.button("删除").clicked() {
                self.delete_current_selection();
            }
        });

        let object_like_count = selections
            .iter()
            .filter(|selection| {
                matches!(
                    selection.layer,
                    LayerKind::Decals | LayerKind::Objects | LayerKind::Entities
                ) && !self.layer_state(selection.layer).locked
            })
            .count();
        if object_like_count > 0 {
            ui.separator();
            ui.label(format!("批量层级：{} 个可调对象", object_like_count));
            let mut z_index = 0;
            if ui
                .add(egui::DragValue::new(&mut z_index).prefix("层级 "))
                .changed()
            {
                self.push_undo_snapshot();
                for selection in &selections {
                    self.set_z_index_for_selection(selection, z_index);
                }
                self.mark_dirty();
            }
        }
    }

    fn draw_map_inspector(&mut self, ui: &mut egui::Ui) {
        ui.label("地图属性");
        let mut next = self.document.clone();
        let mut changed = false;
        changed |= ui.text_edit_singleline(&mut next.id).changed();
        changed |= ui.text_edit_singleline(&mut next.mode).changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut next.width)
                    .range(1..=512)
                    .prefix("宽 "),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut next.height)
                    .range(1..=512)
                    .prefix("高 "),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut next.tile_size)
                    .range(1..=256)
                    .prefix("格 "),
            )
            .changed();

        ui.separator();
        ui.label("出生点");
        for spawn in &mut next.spawns {
            ui.horizontal(|ui| {
                changed |= ui.text_edit_singleline(&mut spawn.id).changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut spawn.x).speed(0.1).prefix("x "))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut spawn.y).speed(0.1).prefix("y "))
                    .changed();
            });
        }

        if changed {
            self.push_undo_snapshot();
            self.document = next;
            self.save_as_id = self.document.id.clone();
            self.mark_dirty();
        }
    }

    fn draw_asset_inspector(&mut self, ui: &mut egui::Ui, asset: &AssetEntry) {
        ui.label("素材默认属性");
        ui.monospace(&asset.id);
        ui.label(format!("类型：{}", asset.kind.zh_label()));
        ui.label(format!("分类：{}", category_label(&asset.category)));
        ui.label(format!("默认图层：{}", asset.default_layer.zh_label()));
        ui.label(format!("锚点：{}", anchor_label(asset.anchor)));
        ui.label(format!("吸附：{}", snap_label(asset.snap)));
        ui.label(format!(
            "默认尺寸：{:.1} x {:.1}",
            asset.default_size[0], asset.default_size[1]
        ));
        if asset.kind == AssetKind::Tile {
            let footprint = self.asset_tile_footprint(asset);
            ui.label(format!("占格：{} x {}", footprint[0], footprint[1]));
        }
        if let Some(entity_type) = &asset.entity_type {
            ui.label(format!("实体类型：{entity_type}"));
        }
        if let Some(codex_id) = &asset.codex_id {
            ui.label(format!("图鉴：{codex_id}"));
        }
        if !asset.tags.is_empty() {
            ui.label(format!("Tags：{}", asset.tags.join(", ")));
        }
        ui.small(&asset.relative_path);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("编辑素材").clicked() {
                self.open_edit_asset_dialog(&asset.id);
            }
            if ui.button("从素材库移除").clicked() {
                let ctx = ui.ctx().clone();
                self.delete_selected_asset_definition(&ctx);
            }
        });
        if self.asset_db_dirty {
            ui.colored_label(Color32::YELLOW, "素材库有未保存修改");
            if ui.button("保存素材库").clicked() {
                self.save_asset_database();
            }
        }
    }

    fn draw_selection_inspector(&mut self, ui: &mut egui::Ui, selection: SelectedItem) {
        ui.label(format!("选中：{}", selection.label()));
        if self.layer_state(selection.layer).locked {
            ui.colored_label(Color32::YELLOW, "当前图层已锁定");
        }

        let mut next = self.document.clone();
        let mut changed = false;
        let mut next_selection = selection.clone();

        match selection.layer {
            LayerKind::Ground => {
                if let Some([x, y]) = parse_ground_selection_id(&selection.id) {
                    if let Some(tile) = next
                        .layers
                        .ground
                        .iter_mut()
                        .find(|tile| tile.x == x && tile.y == y)
                    {
                        changed |= ui.text_edit_singleline(&mut tile.asset).changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut tile.x).prefix("x "))
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut tile.y).prefix("y "))
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut tile.w)
                                    .range(1..=512)
                                    .prefix("w "),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut tile.h)
                                    .range(1..=512)
                                    .prefix("h "),
                            )
                            .changed();
                        changed |= ui.checkbox(&mut tile.flip_x, "水平翻转").changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut tile.rotation).prefix("旋转 "))
                            .changed();
                        next_selection.id = ground_selection_id(tile.x, tile.y);
                    }
                }
            }
            LayerKind::Decals => {
                if let Some(instance) = next
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    let default_size = self
                        .registry
                        .get(&instance.asset)
                        .map(|asset| asset.default_size);
                    changed |= object_instance_editor(
                        ui,
                        instance,
                        default_size,
                        &mut self.lock_aspect_ratio,
                    );
                    next_selection.id = instance.id.clone();
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = next
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    let default_size = self
                        .registry
                        .get(&instance.asset)
                        .map(|asset| asset.default_size);
                    changed |= object_instance_editor(
                        ui,
                        instance,
                        default_size,
                        &mut self.lock_aspect_ratio,
                    );
                    next_selection.id = instance.id.clone();
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = next
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    changed |= ui.text_edit_singleline(&mut instance.id).changed();
                    changed |= ui.text_edit_singleline(&mut instance.asset).changed();
                    changed |= ui.text_edit_singleline(&mut instance.entity_type).changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut instance.x)
                                .speed(0.1)
                                .prefix("x "),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut instance.y)
                                .speed(0.1)
                                .prefix("y "),
                        )
                        .changed();
                    let default_size = self
                        .registry
                        .get(&instance.asset)
                        .map(|asset| asset.default_size);
                    changed |= instance_size_editor(
                        ui,
                        &mut instance.scale_x,
                        &mut instance.scale_y,
                        default_size,
                        &mut self.lock_aspect_ratio,
                    );
                    changed |= ui
                        .add(egui::DragValue::new(&mut instance.z_index).prefix("层级 "))
                        .changed();
                    entity_rect_editor(ui, "碰撞范围", &mut instance.collision_rect, &mut changed);
                    entity_rect_editor(
                        ui,
                        "交互范围",
                        &mut instance.interaction_rect,
                        &mut changed,
                    );
                    changed |= ui.checkbox(&mut instance.flip_x, "水平翻转").changed();
                    changed |= ui
                        .add(egui::DragValue::new(&mut instance.rotation).prefix("旋转 "))
                        .changed();
                    next_selection.id = instance.id.clone();
                }
            }
            LayerKind::Zones => {
                if let Some(zone) = next
                    .layers
                    .zones
                    .iter_mut()
                    .find(|zone| zone.id == selection.id)
                {
                    changed |= ui.text_edit_singleline(&mut zone.id).changed();
                    changed |= ui.text_edit_singleline(&mut zone.zone_type).changed();
                    next_selection.id = zone.id.clone();
                    ui.label(format!("点数：{}", zone.points.len()));
                    for (index, point) in zone.points.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("#{index}"));
                            changed |= ui
                                .add(egui::DragValue::new(&mut point[0]).speed(0.05).prefix("x "))
                                .changed();
                            changed |= ui
                                .add(egui::DragValue::new(&mut point[1]).speed(0.05).prefix("y "))
                                .changed();
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("加点").clicked() {
                            let point = zone.points.last().copied().unwrap_or([0.0, 0.0]);
                            zone.points.push([point[0] + 1.0, point[1]]);
                            changed = true;
                        }
                        if ui.button("删末点").clicked() && zone.points.len() > 3 {
                            zone.points.pop();
                            changed = true;
                        }
                        if ui.button("反向").clicked() {
                            zone.points.reverse();
                            changed = true;
                        }
                    });
                }
            }
            LayerKind::Collision => {
                ui.label("碰撞格请用碰撞工具绘制或擦除");
            }
        }

        if changed && !self.layer_state(selection.layer).locked {
            self.push_undo_snapshot();
            self.document = next;
            self.set_single_selection(Some(next_selection));
            self.mark_dirty();
        }
    }

    fn draw_dialogs(&mut self, ctx: &EguiContext) {
        if self.show_new_map_dialog {
            egui::Window::new("新建地图")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.new_map_draft.id);
                    ui.text_edit_singleline(&mut self.new_map_draft.mode);
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.width)
                            .range(1..=512)
                            .prefix("宽 "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.height)
                            .range(1..=512)
                            .prefix("高 "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.tile_size)
                            .range(1..=256)
                            .prefix("格 "),
                    );
                    ui.separator();
                    ui.label("出生点");
                    ui.text_edit_singleline(&mut self.new_map_draft.spawn_id);
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.spawn_x)
                            .speed(0.1)
                            .prefix("x "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.spawn_y)
                            .speed(0.1)
                            .prefix("y "),
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("创建").clicked() {
                            self.create_new_map_from_draft();
                        }
                        if ui.button("取消").clicked() {
                            self.show_new_map_dialog = false;
                        }
                    });
                });
        }

        if let Some(path) = self.open_confirm_path.clone() {
            egui::Window::new("未保存的修改")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "当前地图 {} 有未保存修改。",
                        display_project_path(&self.project_root, &self.map_path)
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("保存并打开").clicked() {
                            self.save_map();
                            if !self.dirty {
                                self.open_map(path.clone());
                            }
                        }
                        if ui.button("放弃修改").clicked() {
                            self.open_map(path.clone());
                        }
                        if ui.button("取消").clicked() {
                            self.open_confirm_path = None;
                        }
                    });
                });
        }

        if let Some(path) = self.delete_confirm_path.clone() {
            egui::Window::new("删除地图")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "确定删除 {}？",
                        display_project_path(&self.project_root, &path)
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("删除").clicked() {
                            self.delete_map_file(path.clone());
                        }
                        if ui.button("取消").clicked() {
                            self.delete_confirm_path = None;
                        }
                    });
                });
        }

        if self.show_validation_panel {
            egui::Window::new("地图校验")
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label(validation_summary(&self.validation_issues));
                    ui.separator();
                    if self.validation_issues.is_empty() {
                        ui.label("没有问题");
                    }
                    for issue in &self.validation_issues {
                        let color = match issue.severity {
                            MapValidationSeverity::Error => Color32::RED,
                            MapValidationSeverity::Warning => Color32::YELLOW,
                        };
                        ui.colored_label(color, &issue.message);
                    }
                    if ui.button("关闭").clicked() {
                        self.show_validation_panel = false;
                    }
                });
        }

        if self.show_asset_dialog {
            egui::Window::new(if self.asset_editing_id.is_some() {
                "编辑素材"
            } else {
                "添加素材"
            })
            .default_width(460.0)
            .show(ctx, |ui| {
                self.draw_asset_draft_editor(ui, ctx);
            });
        }

        if self.show_unregistered_assets {
            egui::Window::new("未登记图片")
                .default_width(620.0)
                .default_height(520.0)
                .show(ctx, |ui| {
                    ui.label(
                        "这里扫描 PNG 只是为了辅助登记，未登记图片不会自动进入游戏或地图编辑。",
                    );
                    ui.horizontal(|ui| {
                        ui.label("扫描文件夹");
                        ui.monospace(display_project_path(
                            &self.project_root,
                            &self.asset_scan_root,
                        ));
                        if ui.button("选择文件夹").clicked() {
                            self.pick_asset_scan_folder();
                        }
                        if ui.button("默认").clicked() {
                            self.asset_scan_root = self.project_root.join("assets").join("sprites");
                        }
                        if ui.button("登记全部").clicked() {
                            let mut paths = Vec::new();
                            collect_png_paths(&self.asset_scan_root, &mut paths);
                            self.add_asset_definitions_from_paths(paths, ctx);
                        }
                    });
                    ui.separator();
                    let unregistered = self.unregistered_sprite_paths();
                    if unregistered.is_empty() {
                        ui.label("没有发现未登记 PNG。");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for path in unregistered {
                                ui.horizontal(|ui| {
                                    ui.monospace(&path);
                                    if ui.button("登记").clicked() {
                                        self.fill_asset_draft_from_path(&path);
                                    }
                                });
                            }
                        });
                    }
                    ui.separator();
                    if ui.button("关闭").clicked() {
                        self.show_unregistered_assets = false;
                    }
                });
        }
    }

    fn draw_asset_draft_editor(&mut self, ui: &mut egui::Ui, ctx: &EguiContext) {
        ui.label("素材 id / 路径");
        ui.text_edit_singleline(&mut self.asset_draft.id);
        ui.text_edit_singleline(&mut self.asset_draft.path);
        ui.horizontal(|ui| {
            if ui.button("选择 PNG 文件").clicked() {
                self.pick_asset_file_into_draft(ctx);
            }
            if ui.button("批量选择文件").clicked() {
                self.pick_and_add_asset_files(ctx);
            }
            if ui.button("批量选择文件夹").clicked() {
                self.pick_and_add_asset_folder(ctx);
            }
            if ui.button("从图片尺寸读取").clicked() {
                match image_dimensions(&self.project_root.join(&self.asset_draft.path)) {
                    Some([width, height]) => {
                        self.asset_draft.default_size = [width, height];
                        if self.asset_draft.kind == AssetKind::Tile {
                            self.asset_draft.footprint =
                                infer_tile_footprint([width, height], self.document.tile_size)
                                    .unwrap_or(self.asset_draft.footprint);
                        }
                        self.status = "已读取图片尺寸".to_owned();
                    }
                    None => {
                        self.status = "图片尺寸读取失败".to_owned();
                    }
                }
            }
            if ui.button("按路径推断").clicked() {
                let path = self.asset_draft.path.clone();
                if path.trim().is_empty() {
                    self.status = "请先填写图片路径".to_owned();
                } else {
                    self.asset_draft = infer_asset_draft_from_path(&self.project_root, &path);
                }
            }
        });

        ui.separator();
        ui.label("分类 / 类型");
        ui.text_edit_singleline(&mut self.asset_draft.category);
        egui::ComboBox::from_id_salt("asset_draft_kind")
            .selected_text(self.asset_draft.kind.zh_label())
            .show_ui(ui, |ui| {
                for kind in AssetKind::ALL {
                    if ui
                        .selectable_value(&mut self.asset_draft.kind, kind, kind.zh_label())
                        .clicked()
                    {
                        apply_kind_defaults(&mut self.asset_draft);
                    }
                }
            });
        egui::ComboBox::from_id_salt("asset_draft_layer")
            .selected_text(self.asset_draft.default_layer.zh_label())
            .show_ui(ui, |ui| {
                for layer in LayerKind::ALL {
                    ui.selectable_value(
                        &mut self.asset_draft.default_layer,
                        layer,
                        layer.zh_label(),
                    );
                }
            });

        ui.separator();
        ui.label("默认尺寸 / 放置规则");
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.asset_draft.default_size[0])
                    .range(1.0..=4096.0)
                    .speed(1.0)
                    .prefix("宽 "),
            );
            ui.add(
                egui::DragValue::new(&mut self.asset_draft.default_size[1])
                    .range(1.0..=4096.0)
                    .speed(1.0)
                    .prefix("高 "),
            );
        });
        if self.asset_draft.kind == AssetKind::Tile {
            if let Some(footprint) =
                infer_tile_footprint(self.asset_draft.default_size, self.document.tile_size)
            {
                self.asset_draft.footprint = footprint;
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.asset_draft.footprint[0])
                        .range(1..=64)
                        .speed(0.1)
                        .prefix("占格 W "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.asset_draft.footprint[1])
                        .range(1..=64)
                        .speed(0.1)
                        .prefix("占格 H "),
                );
            });
            if infer_tile_footprint(self.asset_draft.default_size, self.document.tile_size)
                .is_none()
            {
                ui.colored_label(
                    Color32::YELLOW,
                    "尺寸不是当前地图格子的整数倍，请手动确认占格",
                );
            }
        }
        egui::ComboBox::from_id_salt("asset_draft_anchor")
            .selected_text(anchor_label(self.asset_draft.anchor))
            .show_ui(ui, |ui| {
                for anchor in [
                    AnchorKind::TopLeft,
                    AnchorKind::Center,
                    AnchorKind::BottomCenter,
                ] {
                    ui.selectable_value(&mut self.asset_draft.anchor, anchor, anchor_label(anchor));
                }
            });
        egui::ComboBox::from_id_salt("asset_draft_snap")
            .selected_text(snap_label(self.asset_draft.snap))
            .show_ui(ui, |ui| {
                for snap in [SnapMode::Grid, SnapMode::HalfGrid, SnapMode::Free] {
                    ui.selectable_value(&mut self.asset_draft.snap, snap, snap_label(snap));
                }
            });

        ui.separator();
        ui.label("Tags / 额外属性");
        ui.text_edit_singleline(&mut self.asset_draft.tags);
        ui.text_edit_singleline(&mut self.asset_draft.entity_type);
        ui.text_edit_singleline(&mut self.asset_draft.codex_id);

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("应用").clicked() {
                self.apply_asset_draft(ctx);
            }
            if ui.button("应用并保存").clicked() {
                self.apply_asset_draft(ctx);
                if self.asset_db_dirty {
                    self.save_asset_database();
                }
            }
            if ui.button("取消").clicked() {
                self.show_asset_dialog = false;
            }
        });
    }

    fn create_new_map_from_draft(&mut self) {
        let Some(id) = sanitize_map_id(&self.new_map_draft.id) else {
            self.status = "新建失败：地图 id 为空".to_owned();
            return;
        };
        let id = unique_map_id(&self.project_root, &id);
        self.document = MapDocument {
            id: id.clone(),
            mode: self.new_map_draft.mode.clone(),
            tile_size: self.new_map_draft.tile_size.max(1),
            width: self.new_map_draft.width.max(1),
            height: self.new_map_draft.height.max(1),
            layers: Default::default(),
            spawns: vec![content::SpawnPoint {
                id: self.new_map_draft.spawn_id.clone(),
                x: self.new_map_draft.spawn_x,
                y: self.new_map_draft.spawn_y,
            }],
        };
        self.map_path = maps_dir(&self.project_root).join(format!("{id}.ron"));
        self.selected_map_path = self.map_path.clone();
        self.save_as_id = id;
        self.clear_selection();
        self.selected_asset = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = true;
        self.show_new_map_dialog = false;
        self.status = "已创建新地图".to_owned();
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &EguiContext) {
        let desired_size = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());
        let rect = response.rect;

        painter.rect_filled(rect, 0.0, Color32::from_rgb(18, 22, 25));
        self.apply_canvas_input(&response, ctx);

        let tile_size = self.document.tile_size as f32;
        let map_size = vec2(
            self.document.width as f32 * tile_size,
            self.document.height as f32 * tile_size,
        );
        let map_rect = Rect::from_min_size(
            self.world_to_screen(rect, vec2(0.0, 0.0)),
            map_size * self.zoom,
        );
        painter.rect_filled(map_rect, 0.0, Color32::from_rgb(24, 28, 29));

        self.draw_layers(rect, &painter);

        if self.show_grid {
            draw_grid(
                &painter,
                rect,
                map_rect,
                self.document.width,
                self.document.height,
                tile_size * self.zoom,
            );
        }

        if self.show_collision {
            self.draw_collision(rect, &painter);
        }

        if self.show_entity_bounds {
            self.draw_entity_bounds(rect, &painter);
        }

        self.draw_selection_bounds(rect, &painter);
        self.handle_canvas_context_menu(&response, rect, ctx);

        if response.hovered() {
            if let Some(mouse) = response.hover_pos() {
                self.mouse_tile = self.screen_to_tile(rect, mouse);
            }
        } else {
            self.mouse_tile = None;
        }

        self.draw_ground_footprint_preview(rect, &painter);
        self.draw_rectangle_preview(rect, &painter);
        self.draw_zone_draft(rect, &painter, response.hover_pos());
        self.draw_selection_marquee(&painter);
        self.handle_canvas_selection(&response, rect, ctx);
        self.handle_canvas_placement(&response, rect, ctx);
    }

    fn handle_canvas_context_menu(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        if response.secondary_clicked() {
            if let Some(pointer_pos) = ctx
                .input(|input| input.pointer.interact_pos())
                .or_else(|| response.hover_pos())
            {
                if let Some(selection) = self.hit_test_placed_item(canvas_rect, pointer_pos) {
                    if !self.current_selection_list().contains(&selection) {
                        self.set_single_selection(Some(selection));
                    }
                    if let Some(selection) = &self.selected_item {
                        self.active_layer = selection.layer;
                    }
                }
            }
        }

        response.context_menu(|ui| {
            let selections = self.current_selection_list();
            let Some(selection) = self.selected_item.clone() else {
                ui.label("No selection");
                return;
            };

            if selections.len() > 1 {
                ui.label(format!("已选 {} 个对象", selections.len()));
            } else {
                ui.label(selection.label());
                if ui.button("水平翻转").clicked() {
                    self.flip_selected_item();
                    ui.close();
                }
                if ui.button("右转 90").clicked() {
                    self.rotate_selected_item(90);
                    ui.close();
                }
                if ui.button("左转 90").clicked() {
                    self.rotate_selected_item(-90);
                    ui.close();
                }
                if ui.button("重置变换").clicked() {
                    self.reset_selected_transform();
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("复制").clicked() {
                self.copy_selected_item();
                ui.close();
            }
            if ui.button("复制一份").clicked() {
                self.duplicate_selected_item();
                ui.close();
            }
            if ui.button("删除").clicked() {
                self.delete_current_selection();
                ui.close();
            }
            if selection.layer == LayerKind::Zones {
                ui.separator();
                if ui.button("删除附近顶点").clicked() {
                    if let Some(pointer_pos) = ctx
                        .input(|input| input.pointer.interact_pos())
                        .or_else(|| response.hover_pos())
                    {
                        self.delete_zone_vertex_near(canvas_rect, &selection.id, pointer_pos);
                    }
                    ui.close();
                }
            }
        });
    }

    fn apply_canvas_input(&mut self, response: &egui::Response, ctx: &EguiContext) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        let panning = self.tool == ToolKind::Pan || space_down;

        if panning && response.dragged() {
            let pointer_delta = ctx.input(|input| input.pointer.delta());
            self.pan += pointer_delta;
        }

        if response.hovered() {
            let scroll = ctx.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                let factor = (1.0_f32 + scroll * 0.001).clamp(0.75, 1.25);
                self.zoom = (self.zoom * factor).clamp(0.25, 4.0);
            }
        }
    }

    fn handle_canvas_placement(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        if matches!(self.tool, ToolKind::Select | ToolKind::Pan | ToolKind::Zoom) || space_down {
            return;
        }

        let primary_down = ctx.input(|input| input.pointer.primary_down());
        let continuous_paint = self.tool == ToolKind::Erase
            || self.tool == ToolKind::Collision
            || (self.tool == ToolKind::Brush
                && matches!(self.active_layer, LayerKind::Ground | LayerKind::Collision));
        let should_place = if self.tool == ToolKind::Rectangle {
            response.drag_started()
                || response.dragged()
                || response.drag_stopped()
                || response.clicked()
        } else if continuous_paint {
            primary_down && (response.hovered() || response.dragged())
        } else {
            response.clicked()
        };

        if !should_place {
            return;
        }

        let Some(mouse) = ctx
            .input(|input| input.pointer.interact_pos())
            .or_else(|| response.hover_pos())
        else {
            return;
        };
        let Some([tile_x, tile_y]) = self.screen_to_tile(canvas_rect, mouse) else {
            return;
        };
        let raw_map_pos = self
            .screen_to_map_position(canvas_rect, mouse)
            .unwrap_or([tile_x as f32, tile_y as f32]);

        if self.tool == ToolKind::Zone {
            self.handle_zone_tool(response, raw_map_pos, ctx.input(|input| input.modifiers));
            return;
        }

        if self.tool == ToolKind::Rectangle {
            self.handle_rectangle_tool(response, tile_x, tile_y);
            return;
        }

        if self.tool == ToolKind::Eyedropper {
            if let Some(selection) = self.hit_test_placed_item(canvas_rect, mouse) {
                if let Some(asset_id) = self.asset_for_selection(&selection) {
                    self.selected_asset = Some(asset_id.clone());
                    self.clear_selection();
                    self.active_layer = selection.layer;
                    self.tool = ToolKind::Brush;
                    if selection.layer == LayerKind::Ground {
                        if let Some([width, height]) =
                            self.ground_size_for_selection_id(&selection.id)
                        {
                            self.ground_footprint_w = width;
                            self.ground_footprint_h = height;
                        }
                    }
                    self.status = format!("吸取素材 {}", asset_id);
                }
            }
            return;
        }

        if self.active_layer_locked() {
            self.status = format!("{} 已锁定", self.active_layer.zh_label());
            return;
        }

        if self.tool == ToolKind::Erase {
            self.push_undo_snapshot();
            self.erase_brush_at(tile_x, tile_y);
            self.mark_dirty();
            self.status = format!("Erased {}, {}", tile_x, tile_y);
            return;
        }

        if self.tool == ToolKind::Collision || self.active_layer == LayerKind::Collision {
            self.push_undo_snapshot();
            self.paint_collision_brush(tile_x, tile_y);
            self.mark_dirty();
            self.status = format!("Collision {}, {}", tile_x, tile_y);
            return;
        }

        if self.tool == ToolKind::Bucket {
            if self.active_layer != LayerKind::Ground {
                self.status = "油漆桶目前用于地表图层".to_owned();
                return;
            }
            let Some(asset_id) = self.selected_asset().map(|asset| asset.id.clone()) else {
                self.status = "油漆桶需要先选择地表素材".to_owned();
                return;
            };
            self.push_undo_snapshot();
            let filled = self.bucket_fill_ground(tile_x, tile_y, &asset_id);
            if filled > 0 {
                self.mark_dirty();
            }
            self.status = format!("油漆桶填充 {} 格", filled);
            return;
        }

        let modifiers = ctx.input(|input| input.modifiers);
        let Some((asset_id, entity_type, place_pos)) = self.selected_asset().map(|asset| {
            (
                asset.id.clone(),
                asset.entity_type.clone(),
                self.snapped_map_position(raw_map_pos, Some(asset), modifiers),
            )
        }) else {
            self.status = "Select an asset first".to_owned();
            return;
        };
        self.clear_selection();
        self.push_undo_snapshot();

        let placed_ground_size = match self.active_layer {
            LayerKind::Ground => {
                let [width, height] = self.selected_ground_footprint_at(tile_x, tile_y);
                self.paint_ground_brush(tile_x, tile_y, &asset_id);
                Some([width, height])
            }
            LayerKind::Decals => {
                self.document
                    .place_decal(&asset_id, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Objects => {
                self.document
                    .place_object(&asset_id, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Entities => {
                let entity_type = entity_type.unwrap_or_else(|| "Decoration".to_owned());
                self.document
                    .place_entity(&asset_id, &entity_type, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Zones => {
                self.document
                    .place_decal(&asset_id, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Collision => unreachable!(),
        };

        self.mark_dirty();
        self.status = if let Some([width, height]) = placed_ground_size {
            format!(
                "Placed {} {}x{} at {}, {}",
                asset_id, width, height, tile_x, tile_y
            )
        } else {
            format!("Placed {} at {}, {}", asset_id, tile_x, tile_y)
        };
    }

    fn handle_canvas_selection(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        if self.tool != ToolKind::Select || space_down {
            return;
        }

        let Some(pointer_pos) = ctx
            .input(|input| input.pointer.interact_pos())
            .or_else(|| response.hover_pos())
        else {
            return;
        };

        if response.drag_started() {
            let modifiers = ctx.input(|input| input.modifiers);
            let current = self.current_selection_list();
            self.resize_drag = None;
            self.zone_vertex_drag = None;
            self.selection_marquee = None;
            self.multi_move_drag = None;

            if current.len() == 1 {
                let selection = current[0].clone();
                if selection.layer == LayerKind::Zones {
                    if let Some(vertex_index) =
                        self.zone_vertex_hit(canvas_rect, &selection.id, pointer_pos)
                    {
                        self.zone_vertex_drag = Some(ZoneVertexDrag {
                            zone_id: selection.id.clone(),
                            vertex_index,
                        });
                        if !self.layer_state(LayerKind::Zones).locked {
                            self.push_undo_snapshot();
                        }
                        return;
                    }
                }

                if self.resize_handle_hit(canvas_rect, &selection, pointer_pos) {
                    self.resize_drag = Some(ResizeDrag {
                        selection: selection.clone(),
                    });
                    if !self.layer_state(selection.layer).locked {
                        self.push_undo_snapshot();
                    }
                    return;
                }
            }

            let hit = self.hit_test_placed_item(canvas_rect, pointer_pos);
            let additive = modifiers.shift || modifiers.command || modifiers.ctrl;
            if let Some(selection) = hit {
                self.active_layer = selection.layer;
                if additive {
                    self.toggle_selection(selection);
                    self.status = format!("已选中 {} 个对象", self.current_selection_list().len());
                    return;
                }

                if !current.contains(&selection) {
                    self.set_single_selection(Some(selection));
                }

                let selections = self.current_selection_list();
                let origins = self.move_origins_for_selection(&selections);
                if origins
                    .iter()
                    .any(|origin| !self.layer_state(origin.layer()).locked)
                {
                    if let Some(start) = self.screen_to_map_position(canvas_rect, pointer_pos) {
                        self.push_undo_snapshot();
                        self.multi_move_drag = Some(MultiMoveDrag { start, origins });
                    }
                }
                self.status = if selections.len() > 1 {
                    format!("已选中 {} 个对象", selections.len())
                } else {
                    format!("Selected {}", selections[0].label())
                };
            } else {
                if !additive {
                    self.clear_selection();
                }
                self.selection_marquee = Some(SelectionMarquee {
                    start: pointer_pos,
                    current: pointer_pos,
                    additive,
                });
                self.status = "框选中".to_owned();
            }
        }

        if response.dragged() && ctx.input(|input| input.pointer.primary_down()) {
            if let Some(marquee) = &mut self.selection_marquee {
                marquee.current = pointer_pos;
                return;
            }

            if let Some(drag) = self.zone_vertex_drag.clone() {
                if !self.layer_state(LayerKind::Zones).locked {
                    if let Some(raw) = self.screen_to_map_position(canvas_rect, pointer_pos) {
                        let point = self.snapped_map_position(
                            raw,
                            None,
                            ctx.input(|input| input.modifiers),
                        );
                        self.move_zone_vertex(&drag.zone_id, drag.vertex_index, point);
                        self.mark_dirty();
                        self.status = format!("移动区域顶点 #{}", drag.vertex_index);
                    }
                }
                return;
            }

            if let Some(drag) = self.multi_move_drag.clone() {
                if let Some(raw_pos) = self.screen_to_map_position(canvas_rect, pointer_pos) {
                    let modifiers = ctx.input(|input| input.modifiers);
                    let raw_delta = [raw_pos[0] - drag.start[0], raw_pos[1] - drag.start[1]];
                    let delta = snapped_delta(raw_delta, modifiers);
                    self.apply_multi_move(&drag.origins, delta);
                    self.mark_dirty();
                    self.status = format!("移动 {} 个对象", drag.origins.len());
                }
                return;
            }

            if let Some(resize) = self.resize_drag.clone() {
                self.resize_selected_item(canvas_rect, &resize.selection, pointer_pos, ctx);
                self.mark_dirty();
                self.status = format!("Resized {}", resize.selection.label());
                return;
            }
            let Some(selection) = self.selected_item.clone() else {
                return;
            };
            if self.layer_state(selection.layer).locked {
                self.status = format!("{} 已锁定", selection.layer.zh_label());
                return;
            }
            let raw_pos = self
                .screen_to_map_position(canvas_rect, pointer_pos)
                .unwrap_or([0.0, 0.0]);
            let modifiers = ctx.input(|input| input.modifiers);
            let snapped_pos = if selection.layer == LayerKind::Ground {
                self.screen_to_tile(canvas_rect, pointer_pos)
                    .map(|[x, y]| [x as f32, y as f32])
                    .unwrap_or(raw_pos)
            } else {
                let asset_id = self.asset_for_selection(&selection);
                let asset = asset_id.as_deref().and_then(|id| self.registry.get(id));
                self.snapped_map_position(raw_pos, asset, modifiers)
            };

            let moved_ground = self.move_selected_item(&selection, snapped_pos[0], snapped_pos[1]);
            if selection.layer == LayerKind::Ground {
                if let Some([new_x, new_y]) = moved_ground {
                    self.set_single_selection(Some(SelectedItem {
                        layer: LayerKind::Ground,
                        id: ground_selection_id(new_x, new_y),
                    }));
                    self.mark_dirty();
                    self.status = format!("Moved {} to {}, {}", selection.label(), new_x, new_y);
                    return;
                }
            }
            self.mark_dirty();
            self.status = format!(
                "Moved {} to {:.2}, {:.2}",
                selection.label(),
                snapped_pos[0],
                snapped_pos[1]
            );
            return;
        }

        if response.drag_stopped() {
            if let Some(marquee) = self.selection_marquee.take() {
                let rect = Rect::from_two_pos(marquee.start, marquee.current);
                if rect.width().abs() > 4.0 || rect.height().abs() > 4.0 {
                    let mut selections = if marquee.additive {
                        self.current_selection_list()
                    } else {
                        Vec::new()
                    };
                    selections.extend(self.selections_in_screen_rect(canvas_rect, rect));
                    self.set_selection(selections);
                    self.status = format!("框选 {} 个对象", self.current_selection_list().len());
                }
            }
            self.multi_move_drag = None;
            self.resize_drag = None;
            self.zone_vertex_drag = None;
        }

        if response.clicked() {
            let modifiers = ctx.input(|input| input.modifiers);
            if let Some(selection) = self.hit_test_placed_item(canvas_rect, pointer_pos) {
                if modifiers.shift || modifiers.command || modifiers.ctrl {
                    self.toggle_selection(selection);
                } else {
                    self.set_single_selection(Some(selection));
                }
                if let Some(selection) = self.selected_item.clone() {
                    self.active_layer = selection.layer;
                }
                let count = self.current_selection_list().len();
                self.status = if count > 1 {
                    format!("已选中 {count} 个对象")
                } else {
                    self.selected_item
                        .as_ref()
                        .map(|selection| format!("Selected {}", selection.label()))
                        .unwrap_or_else(|| "No object selected".to_owned())
                };
            } else {
                if !(modifiers.shift || modifiers.command || modifiers.ctrl) {
                    self.clear_selection();
                }
                self.status = "No object selected".to_owned();
            }
        }
    }

    fn handle_rectangle_tool(&mut self, response: &egui::Response, tile_x: i32, tile_y: i32) {
        if !matches!(self.active_layer, LayerKind::Ground | LayerKind::Collision) {
            self.status = "矩形工具目前用于地表和碰撞图层".to_owned();
            return;
        }
        if self.active_layer_locked() {
            self.status = format!("{} 已锁定", self.active_layer.zh_label());
            return;
        }

        if response.drag_started() || self.rectangle_drag_start.is_none() {
            self.rectangle_drag_start = Some([tile_x, tile_y]);
        }
        self.rectangle_drag_current = Some([tile_x, tile_y]);

        if response.drag_stopped() || response.clicked() {
            let start = self.rectangle_drag_start.unwrap_or([tile_x, tile_y]);
            let end = self.rectangle_drag_current.unwrap_or([tile_x, tile_y]);
            self.apply_rectangle_tool(start, end);
            self.rectangle_drag_start = None;
            self.rectangle_drag_current = None;
        }
    }

    fn handle_zone_tool(
        &mut self,
        response: &egui::Response,
        raw_pos: [f32; 2],
        modifiers: Modifiers,
    ) {
        if self.layer_state(LayerKind::Zones).locked {
            self.status = "区域图层已锁定".to_owned();
            return;
        }
        if !response.clicked() && !response.double_clicked() {
            return;
        }

        let point = self.snapped_map_position(raw_pos, None, modifiers);
        if response.double_clicked() {
            if self.zone_draft_points.len() >= 3 {
                self.finish_zone_draft();
            }
            return;
        }

        if self.zone_draft_points.len() >= 3
            && distance_sq(point, self.zone_draft_points[0]) <= 0.20 * 0.20
        {
            self.finish_zone_draft();
            return;
        }

        self.zone_draft_points.push(point);
        self.active_layer = LayerKind::Zones;
        self.status = format!("区域点 {}", self.zone_draft_points.len());
    }

    fn finish_zone_draft(&mut self) {
        if self.zone_draft_points.len() < 3 {
            self.status = "区域至少需要 3 个点".to_owned();
            return;
        }
        self.push_undo_snapshot();
        let id = next_editor_zone_id("zone", &self.document.layers.zones);
        self.document.layers.zones.push(content::ZoneInstance {
            id: id.clone(),
            zone_type: "Trigger".to_owned(),
            points: self.zone_draft_points.clone(),
        });
        self.zone_draft_points.clear();
        self.set_single_selection(Some(SelectedItem {
            layer: LayerKind::Zones,
            id,
        }));
        self.mark_dirty();
        self.status = "区域已创建".to_owned();
    }

    fn apply_rectangle_tool(&mut self, start: [i32; 2], end: [i32; 2]) {
        let min_x = start[0]
            .min(end[0])
            .clamp(0, self.document.width as i32 - 1);
        let max_x = start[0]
            .max(end[0])
            .clamp(0, self.document.width as i32 - 1);
        let min_y = start[1]
            .min(end[1])
            .clamp(0, self.document.height as i32 - 1);
        let max_y = start[1]
            .max(end[1])
            .clamp(0, self.document.height as i32 - 1);
        self.push_undo_snapshot();

        match self.active_layer {
            LayerKind::Ground => {
                let asset_id = self.selected_asset().map(|asset| asset.id.clone());
                let [step_w, step_h] = asset_id
                    .as_deref()
                    .and_then(|asset_id| self.registry.get(asset_id))
                    .map(|asset| self.asset_tile_footprint(asset))
                    .unwrap_or([1, 1]);
                for y in (min_y..=max_y).step_by(step_h.max(1) as usize) {
                    for x in (min_x..=max_x).step_by(step_w.max(1) as usize) {
                        if self.rectangle_erase_mode {
                            self.document.erase_at(LayerKind::Ground, x, y);
                        } else if let Some(asset_id) = &asset_id {
                            self.paint_ground_brush(x, y, asset_id);
                        } else {
                            self.status = "矩形填充需要先选择地表素材".to_owned();
                            return;
                        }
                    }
                }
                self.status = format!(
                    "矩形{}: {}x{}",
                    if self.rectangle_erase_mode {
                        "擦除"
                    } else {
                        "填充"
                    },
                    max_x - min_x + 1,
                    max_y - min_y + 1
                );
            }
            LayerKind::Collision => {
                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        if self.rectangle_erase_mode {
                            self.document.erase_at(LayerKind::Collision, x, y);
                        } else {
                            self.document.place_collision(x, y);
                        }
                    }
                }
                self.status = format!(
                    "矩形碰撞{}: {}x{}",
                    if self.rectangle_erase_mode {
                        "擦除"
                    } else {
                        "填充"
                    },
                    max_x - min_x + 1,
                    max_y - min_y + 1
                );
            }
            _ => {}
        }
        self.mark_dirty();
    }

    fn paint_ground_brush(&mut self, x: i32, y: i32, asset_id: &str) {
        let [width, height] = self
            .registry
            .get(asset_id)
            .map(|asset| self.clamped_tile_footprint_at(asset, x, y))
            .unwrap_or_else(|| self.clamped_ground_footprint_at(x, y));
        for yy in y..y + height {
            for xx in x..x + width {
                self.document.erase_at(LayerKind::Ground, xx, yy);
            }
        }
        self.document
            .place_tile_sized(asset_id, x, y, width, height);
    }

    fn paint_collision_brush(&mut self, x: i32, y: i32) {
        let [width, height] = self.selected_ground_footprint_at(x, y);
        for yy in y..y + height {
            for xx in x..x + width {
                self.document.place_collision(xx, yy);
            }
        }
    }

    fn erase_brush_at(&mut self, x: i32, y: i32) {
        let [width, height] = self.clamped_ground_footprint_at(x, y);
        for yy in y..y + height {
            for xx in x..x + width {
                self.document.erase_at(self.active_layer, xx, yy);
            }
        }
    }

    fn bucket_fill_ground(&mut self, x: i32, y: i32, asset_id: &str) -> usize {
        let target = self.ground_asset_at_cell(x, y);
        if target.as_deref() == Some(asset_id) {
            return 0;
        }

        let mut visited = vec![false; self.document.width as usize * self.document.height as usize];
        let mut queue = VecDeque::from([[x, y]]);
        let mut filled = 0;

        while let Some([cx, cy]) = queue.pop_front() {
            if cx < 0
                || cy < 0
                || cx >= self.document.width as i32
                || cy >= self.document.height as i32
            {
                continue;
            }
            let index = cy as usize * self.document.width as usize + cx as usize;
            if visited[index] {
                continue;
            }
            visited[index] = true;

            if self.ground_asset_at_cell(cx, cy) != target {
                continue;
            }

            self.document.erase_at(LayerKind::Ground, cx, cy);
            self.paint_ground_brush(cx, cy, asset_id);
            filled += 1;

            queue.extend([[cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]]);
        }

        filled
    }

    fn ground_asset_at_cell(&self, x: i32, y: i32) -> Option<String> {
        self.document
            .layers
            .ground
            .iter()
            .rev()
            .find(|tile| {
                let width = tile.w.max(1);
                let height = tile.h.max(1);
                x >= tile.x && x < tile.x + width && y >= tile.y && y < tile.y + height
            })
            .map(|tile| tile.asset.clone())
    }

    fn resize_handle_hit(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
        pointer_pos: Pos2,
    ) -> bool {
        self.selection_screen_rect(canvas_rect, selection)
            .is_some_and(|rect| {
                resize_handle_rects(rect)
                    .iter()
                    .any(|handle| handle.contains(pointer_pos))
            })
    }

    fn resize_selected_item(
        &mut self,
        canvas_rect: Rect,
        selection: &SelectedItem,
        pointer_pos: Pos2,
        ctx: &EguiContext,
    ) {
        if self.layer_state(selection.layer).locked {
            return;
        }
        let Some((anchor, asset_id)) = self.selection_anchor_and_asset(canvas_rect, selection)
        else {
            return;
        };
        let Some(asset) = self.registry.get(&asset_id) else {
            return;
        };

        let delta_x = (pointer_pos.x - anchor.x).abs();
        let delta_y = (pointer_pos.y - anchor.y).abs();
        let screen_size = match asset.anchor {
            AnchorKind::TopLeft => vec2(delta_x.max(1.0), delta_y.max(1.0)),
            AnchorKind::Center => vec2((delta_x * 2.0).max(1.0), (delta_y * 2.0).max(1.0)),
            AnchorKind::BottomCenter => vec2((delta_x * 2.0).max(1.0), delta_y.max(1.0)),
        };
        let world_size = screen_size / self.zoom.max(0.01);
        let mut scale_x = (world_size.x / asset.default_size[0].max(1.0)).max(0.05);
        let mut scale_y = (world_size.y / asset.default_size[1].max(1.0)).max(0.05);
        let keep_aspect = self.lock_aspect_ratio || ctx.input(|input| input.modifiers.shift);
        if keep_aspect {
            let uniform = scale_x.max(scale_y);
            scale_x = uniform;
            scale_y = uniform;
        }

        self.set_scale_for_selection(selection, scale_x, scale_y);
    }

    fn selection_anchor_and_asset(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
    ) -> Option<(Pos2, String)> {
        let tile_size = self.document.tile_size as f32;
        match selection.layer {
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    let asset = self.registry.get(&instance.asset)?;
                    Some((
                        self.world_to_screen(
                            canvas_rect,
                            anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor),
                        ),
                        instance.asset.clone(),
                    ))
                }),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    let asset = self.registry.get(&instance.asset)?;
                    Some((
                        self.world_to_screen(
                            canvas_rect,
                            anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor),
                        ),
                        instance.asset.clone(),
                    ))
                }),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    let asset = self.registry.get(&instance.asset)?;
                    Some((
                        self.world_to_screen(
                            canvas_rect,
                            anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor),
                        ),
                        instance.asset.clone(),
                    ))
                }),
            _ => None,
        }
    }

    fn set_scale_for_selection(&mut self, selection: &SelectedItem, scale_x: f32, scale_y: f32) {
        match selection.layer {
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.scale_x = scale_x;
                    instance.scale_y = scale_y;
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.scale_x = scale_x;
                    instance.scale_y = scale_y;
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.scale_x = scale_x;
                    instance.scale_y = scale_y;
                }
            }
            _ => {}
        }
    }

    fn zone_vertex_hit(
        &self,
        canvas_rect: Rect,
        zone_id: &str,
        pointer_pos: Pos2,
    ) -> Option<usize> {
        let zone = self
            .document
            .layers
            .zones
            .iter()
            .find(|zone| zone.id == zone_id)?;
        let tile_size = self.document.tile_size as f32;
        zone.points.iter().position(|point| {
            let screen = self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            );
            Rect::from_center_size(screen, vec2(12.0, 12.0)).contains(pointer_pos)
        })
    }

    fn move_zone_vertex(&mut self, zone_id: &str, vertex_index: usize, point: [f32; 2]) {
        if let Some(zone) = self
            .document
            .layers
            .zones
            .iter_mut()
            .find(|zone| zone.id == zone_id)
        {
            if let Some(vertex) = zone.points.get_mut(vertex_index) {
                *vertex = point;
            }
        }
    }

    fn delete_zone_vertex_near(&mut self, canvas_rect: Rect, zone_id: &str, pointer_pos: Pos2) {
        if self.layer_state(LayerKind::Zones).locked {
            self.status = "区域图层已锁定".to_owned();
            return;
        }
        let Some(index) = self.zone_vertex_hit(canvas_rect, zone_id, pointer_pos) else {
            self.status = "附近没有区域顶点".to_owned();
            return;
        };
        let Some(zone) = self
            .document
            .layers
            .zones
            .iter()
            .find(|zone| zone.id == zone_id)
        else {
            return;
        };
        if zone.points.len() <= 3 {
            self.status = "区域至少需要保留 3 个点".to_owned();
            return;
        }
        self.push_undo_snapshot();
        if let Some(zone) = self
            .document
            .layers
            .zones
            .iter_mut()
            .find(|zone| zone.id == zone_id)
        {
            zone.points.remove(index);
            self.mark_dirty();
            self.status = format!("已删除区域顶点 #{index}");
        }
    }

    fn move_origins_for_selection(&self, selections: &[SelectedItem]) -> Vec<MoveOrigin> {
        selections
            .iter()
            .filter_map(|selection| match selection.layer {
                LayerKind::Ground => {
                    let [x, y] = parse_ground_selection_id(&selection.id)?;
                    self.document
                        .layers
                        .ground
                        .iter()
                        .find(|tile| tile.x == x && tile.y == y)
                        .map(|tile| MoveOrigin::Ground {
                            selection: selection.clone(),
                            x: tile.x,
                            y: tile.y,
                        })
                }
                LayerKind::Decals => self
                    .document
                    .layers
                    .decals
                    .iter()
                    .find(|instance| instance.id == selection.id)
                    .map(|instance| MoveOrigin::ObjectLike {
                        selection: selection.clone(),
                        x: instance.x,
                        y: instance.y,
                    }),
                LayerKind::Objects => self
                    .document
                    .layers
                    .objects
                    .iter()
                    .find(|instance| instance.id == selection.id)
                    .map(|instance| MoveOrigin::ObjectLike {
                        selection: selection.clone(),
                        x: instance.x,
                        y: instance.y,
                    }),
                LayerKind::Entities => self
                    .document
                    .layers
                    .entities
                    .iter()
                    .find(|instance| instance.id == selection.id)
                    .map(|instance| MoveOrigin::ObjectLike {
                        selection: selection.clone(),
                        x: instance.x,
                        y: instance.y,
                    }),
                LayerKind::Zones => self
                    .document
                    .layers
                    .zones
                    .iter()
                    .find(|zone| zone.id == selection.id)
                    .map(|zone| MoveOrigin::Zone {
                        selection: selection.clone(),
                        points: zone.points.clone(),
                    }),
                LayerKind::Collision => None,
            })
            .collect()
    }

    fn apply_multi_move(&mut self, origins: &[MoveOrigin], delta: [f32; 2]) {
        let max_x = self.document.width as f32;
        let max_y = self.document.height as f32;
        let mut next_selection = Vec::with_capacity(origins.len());

        for origin in origins {
            if self.layer_state(origin.layer()).locked {
                continue;
            }

            match origin {
                MoveOrigin::Ground { selection, x, y } => {
                    let new_x = (*x as f32 + delta[0])
                        .round()
                        .clamp(0.0, self.document.width.saturating_sub(1) as f32)
                        as i32;
                    let new_y = (*y as f32 + delta[1])
                        .round()
                        .clamp(0.0, self.document.height.saturating_sub(1) as f32)
                        as i32;
                    let updated = self
                        .move_selected_item(selection, new_x as f32, new_y as f32)
                        .unwrap_or([new_x, new_y]);
                    next_selection.push(SelectedItem {
                        layer: LayerKind::Ground,
                        id: ground_selection_id(updated[0], updated[1]),
                    });
                }
                MoveOrigin::ObjectLike { selection, x, y } => {
                    let new_x = (x + delta[0]).clamp(0.0, max_x);
                    let new_y = (y + delta[1]).clamp(0.0, max_y);
                    self.move_selected_item(selection, new_x, new_y);
                    next_selection.push(selection.clone());
                }
                MoveOrigin::Zone { selection, points } => {
                    if let Some(zone) = self
                        .document
                        .layers
                        .zones
                        .iter_mut()
                        .find(|zone| zone.id == selection.id)
                    {
                        zone.points = points
                            .iter()
                            .map(|point| {
                                [
                                    (point[0] + delta[0]).clamp(0.0, max_x),
                                    (point[1] + delta[1]).clamp(0.0, max_y),
                                ]
                            })
                            .collect();
                    }
                    next_selection.push(selection.clone());
                }
            }
        }

        self.set_selection(next_selection);
    }

    fn selections_in_screen_rect(
        &self,
        canvas_rect: Rect,
        selection_rect: Rect,
    ) -> Vec<SelectedItem> {
        let mut selections = Vec::new();
        for layer in LayerKind::ALL {
            if !self.layer_state(layer).visible {
                continue;
            }
            match layer {
                LayerKind::Ground => {
                    for tile in &self.document.layers.ground {
                        let selection = SelectedItem {
                            layer,
                            id: ground_selection_id(tile.x, tile.y),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Decals => {
                    for instance in &self.document.layers.decals {
                        let selection = SelectedItem {
                            layer,
                            id: instance.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Objects => {
                    for instance in &self.document.layers.objects {
                        let selection = SelectedItem {
                            layer,
                            id: instance.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Entities => {
                    for instance in &self.document.layers.entities {
                        let selection = SelectedItem {
                            layer,
                            id: instance.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Zones => {
                    for zone in &self.document.layers.zones {
                        let selection = SelectedItem {
                            layer,
                            id: zone.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Collision => {}
            }
        }
        selections
    }

    fn hit_test_placed_item(&self, canvas_rect: Rect, pointer_pos: Pos2) -> Option<SelectedItem> {
        for entity in self.document.layers.entities.iter().rev() {
            if self
                .entity_instance_screen_rect(canvas_rect, entity)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Entities,
                    id: entity.id.clone(),
                });
            }
        }

        for object in self.document.layers.objects.iter().rev() {
            if self
                .object_instance_screen_rect(canvas_rect, object)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Objects,
                    id: object.id.clone(),
                });
            }
        }

        for decal in self.document.layers.decals.iter().rev() {
            if self
                .object_instance_screen_rect(canvas_rect, decal)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Decals,
                    id: decal.id.clone(),
                });
            }
        }

        for zone in self.document.layers.zones.iter().rev() {
            if self
                .zone_screen_rect(canvas_rect, zone)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Zones,
                    id: zone.id.clone(),
                });
            }
        }

        for tile in self.document.layers.ground.iter().rev() {
            if self
                .tile_screen_rect(canvas_rect, tile.x, tile.y, tile.w, tile.h)
                .contains(pointer_pos)
            {
                return Some(SelectedItem {
                    layer: LayerKind::Ground,
                    id: ground_selection_id(tile.x, tile.y),
                });
            }
        }

        None
    }

    fn move_selected_item(&mut self, selection: &SelectedItem, x: f32, y: f32) -> Option<[i32; 2]> {
        match selection.layer {
            LayerKind::Ground => {
                let Some([old_x, old_y]) = parse_ground_selection_id(&selection.id) else {
                    return None;
                };
                let Some(index) = self
                    .document
                    .layers
                    .ground
                    .iter()
                    .position(|tile| tile.x == old_x && tile.y == old_y)
                else {
                    return None;
                };
                let asset = self.document.layers.ground[index].asset.clone();
                let width = self.document.layers.ground[index].w.max(1);
                let height = self.document.layers.ground[index].h.max(1);
                let new_x = (x as i32).clamp(0, (self.document.width as i32 - width).max(0));
                let new_y = (y as i32).clamp(0, (self.document.height as i32 - height).max(0));
                let flip_x = self.document.layers.ground[index].flip_x;
                let rotation = self.document.layers.ground[index].rotation;
                self.document.layers.ground.remove(index);
                self.document.place_tile(&asset, new_x, new_y);
                if let Some(tile) = self
                    .document
                    .layers
                    .ground
                    .iter_mut()
                    .find(|tile| tile.x == new_x && tile.y == new_y)
                {
                    tile.w = width;
                    tile.h = height;
                    tile.flip_x = flip_x;
                    tile.rotation = rotation;
                }
                Some([new_x, new_y])
            }
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.x = x;
                    instance.y = y;
                }
                None
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.x = x;
                    instance.y = y;
                }
                None
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.x = x;
                    instance.y = y;
                }
                None
            }
            LayerKind::Zones | LayerKind::Collision => None,
        }
    }

    fn delete_selected_item(&mut self, selection: &SelectedItem) {
        match selection.layer {
            LayerKind::Ground => {
                if let Some([x, y]) = parse_ground_selection_id(&selection.id) {
                    self.document
                        .layers
                        .ground
                        .retain(|tile| tile.x != x || tile.y != y);
                }
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .retain(|instance| instance.id != selection.id),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .retain(|instance| instance.id != selection.id),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .retain(|instance| instance.id != selection.id),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .retain(|instance| instance.id != selection.id),
            LayerKind::Collision => {}
        }
    }

    fn flip_selected_item(&mut self) {
        let Some(selection) = self.selected_item.clone() else {
            self.status = "Select an item before flipping".to_owned();
            return;
        };
        let Some((flip_x, rotation)) = self.transform_for_selection(&selection) else {
            self.status = format!("No transform target for {}", selection.label());
            return;
        };

        if self.layer_state(selection.layer).locked {
            self.status = format!("{} 已锁定", selection.layer.zh_label());
            return;
        }
        self.push_undo_snapshot();
        self.set_transform_for_selection(&selection, !flip_x, rotation);
        self.mark_dirty();
        self.status = format!("Flipped {}", selection.label());
    }

    fn rotate_selected_item(&mut self, delta: i32) {
        let Some(selection) = self.selected_item.clone() else {
            self.status = "Select an item before rotating".to_owned();
            return;
        };
        let Some((flip_x, rotation)) = self.transform_for_selection(&selection) else {
            self.status = format!("No transform target for {}", selection.label());
            return;
        };

        if self.layer_state(selection.layer).locked {
            self.status = format!("{} 已锁定", selection.layer.zh_label());
            return;
        }
        self.push_undo_snapshot();
        self.set_transform_for_selection(&selection, flip_x, normalize_rotation(rotation + delta));
        self.mark_dirty();
        self.status = format!("Rotated {}", selection.label());
    }

    fn reset_selected_transform(&mut self) {
        let Some(selection) = self.selected_item.clone() else {
            self.status = "Select an item before resetting transform".to_owned();
            return;
        };

        if self.layer_state(selection.layer).locked {
            self.status = format!("{} 已锁定", selection.layer.zh_label());
            return;
        }
        self.push_undo_snapshot();
        self.set_transform_for_selection(&selection, false, 0);
        self.mark_dirty();
        self.status = format!("Reset transform for {}", selection.label());
    }

    fn transform_for_selection(&self, selection: &SelectedItem) -> Option<(bool, i32)> {
        match selection.layer {
            LayerKind::Ground => {
                let [x, y] = parse_ground_selection_id(&selection.id)?;
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .map(|tile| (tile.flip_x, tile.rotation))
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| (instance.flip_x, instance.rotation)),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| (instance.flip_x, instance.rotation)),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| (instance.flip_x, instance.rotation)),
            LayerKind::Zones | LayerKind::Collision => None,
        }
    }

    fn set_transform_for_selection(
        &mut self,
        selection: &SelectedItem,
        flip_x: bool,
        rotation: i32,
    ) {
        let rotation = normalize_rotation(rotation);
        match selection.layer {
            LayerKind::Ground => {
                if let Some([x, y]) = parse_ground_selection_id(&selection.id) {
                    if let Some(tile) = self
                        .document
                        .layers
                        .ground
                        .iter_mut()
                        .find(|tile| tile.x == x && tile.y == y)
                    {
                        tile.flip_x = flip_x;
                        tile.rotation = rotation;
                    }
                }
            }
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.flip_x = flip_x;
                    instance.rotation = rotation;
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.flip_x = flip_x;
                    instance.rotation = rotation;
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.flip_x = flip_x;
                    instance.rotation = rotation;
                }
            }
            LayerKind::Zones | LayerKind::Collision => {}
        }
    }

    fn set_z_index_for_selection(&mut self, selection: &SelectedItem, z_index: i32) {
        if self.layer_state(selection.layer).locked {
            return;
        }
        match selection.layer {
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.z_index = z_index;
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.z_index = z_index;
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.z_index = z_index;
                }
            }
            LayerKind::Ground | LayerKind::Zones | LayerKind::Collision => {}
        }
    }

    fn ground_size_for_selection(&self) -> Option<[i32; 2]> {
        if self.current_selection_list().len() != 1 {
            return None;
        }
        let selection = self.selected_item.as_ref()?;
        if selection.layer != LayerKind::Ground {
            return None;
        }

        self.ground_size_for_selection_id(&selection.id)
    }

    fn ground_size_for_selection_id(&self, id: &str) -> Option<[i32; 2]> {
        let [x, y] = parse_ground_selection_id(id)?;
        self.document
            .layers
            .ground
            .iter()
            .find(|tile| tile.x == x && tile.y == y)
            .map(|tile| [tile.w.max(1), tile.h.max(1)])
    }

    fn set_ground_size_for_selection(&mut self, width: i32, height: i32) {
        let Some(selection) = self.selected_item.clone() else {
            return;
        };
        if selection.layer != LayerKind::Ground {
            return;
        }
        let Some([x, y]) = parse_ground_selection_id(&selection.id) else {
            return;
        };

        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);
        let mut resized_to = None;

        if self.layer_state(LayerKind::Ground).locked {
            self.status = "地表图层已锁定".to_owned();
            return;
        }
        self.push_undo_snapshot();
        if let Some(tile) = self
            .document
            .layers
            .ground
            .iter_mut()
            .find(|tile| tile.x == x && tile.y == y)
        {
            tile.w = width.clamp(1, max_width);
            tile.h = height.clamp(1, max_height);
            resized_to = Some([tile.w, tile.h]);
        }

        if let Some([width, height]) = resized_to {
            self.mark_dirty();
            self.status = format!("Resized Ground:{x},{y} to {width} x {height}");
        }
    }

    fn clamped_ground_footprint_at(&self, x: i32, y: i32) -> [i32; 2] {
        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);

        [
            self.ground_footprint_w.clamp(1, max_width),
            self.ground_footprint_h.clamp(1, max_height),
        ]
    }

    fn selected_ground_footprint_at(&self, x: i32, y: i32) -> [i32; 2] {
        self.selected_asset()
            .map(|asset| self.clamped_tile_footprint_at(asset, x, y))
            .unwrap_or_else(|| self.clamped_ground_footprint_at(x, y))
    }

    fn clamped_tile_footprint_at(&self, asset: &AssetEntry, x: i32, y: i32) -> [i32; 2] {
        let [width, height] = self.asset_tile_footprint(asset);
        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);
        [width.clamp(1, max_width), height.clamp(1, max_height)]
    }

    fn asset_tile_footprint(&self, asset: &AssetEntry) -> [i32; 2] {
        asset
            .footprint
            .or_else(|| infer_tile_footprint(asset.default_size, self.document.tile_size))
            .unwrap_or([1, 1])
    }

    fn draw_layers(&self, canvas_rect: Rect, painter: &egui::Painter) {
        if self.layer_state(LayerKind::Ground).visible {
            for tile in &self.document.layers.ground {
                let rect = self.tile_screen_rect(canvas_rect, tile.x, tile.y, tile.w, tile.h);
                self.draw_asset_image(painter, &tile.asset, rect, tile.flip_x, tile.rotation);
            }
        }

        if self.layer_state(LayerKind::Decals).visible {
            for decal in &self.document.layers.decals {
                self.draw_object_like(
                    canvas_rect,
                    painter,
                    &decal.asset,
                    decal.x,
                    decal.y,
                    decal.scale_x,
                    decal.scale_y,
                    decal.flip_x,
                    decal.rotation,
                );
            }
        }

        if self.layer_state(LayerKind::Objects).visible {
            let mut objects = self.document.layers.objects.iter().collect::<Vec<_>>();
            objects.sort_by(|left, right| {
                left.z_index
                    .cmp(&right.z_index)
                    .then_with(|| left.y.total_cmp(&right.y))
            });
            for object in objects {
                self.draw_object_like(
                    canvas_rect,
                    painter,
                    &object.asset,
                    object.x,
                    object.y,
                    object.scale_x,
                    object.scale_y,
                    object.flip_x,
                    object.rotation,
                );
            }
        }

        if self.layer_state(LayerKind::Entities).visible {
            let mut entities = self.document.layers.entities.iter().collect::<Vec<_>>();
            entities.sort_by(|left, right| {
                left.z_index
                    .cmp(&right.z_index)
                    .then_with(|| left.y.total_cmp(&right.y))
            });
            for entity in entities {
                self.draw_object_like(
                    canvas_rect,
                    painter,
                    &entity.asset,
                    entity.x,
                    entity.y,
                    entity.scale_x,
                    entity.scale_y,
                    entity.flip_x,
                    entity.rotation,
                );
            }
        }

        if self.show_zones && self.layer_state(LayerKind::Zones).visible {
            self.draw_zones(canvas_rect, painter);
        }
    }

    fn draw_zones(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let tile_size = self.document.tile_size as f32;
        for zone in &self.document.layers.zones {
            let (stroke_color, fill_color) = zone_colors(&zone.zone_type);
            let points = zone
                .points
                .iter()
                .map(|point| {
                    self.world_to_screen(
                        canvas_rect,
                        vec2(point[0] * tile_size, point[1] * tile_size),
                    )
                })
                .collect::<Vec<_>>();
            if points.len() < 2 {
                continue;
            }
            painter.add(Shape::convex_polygon(
                points.clone(),
                fill_color,
                Stroke::new(1.5, stroke_color),
            ));
            if self.show_zone_labels {
                let center = polygon_screen_center(&points);
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("{}\\n{}", zone.id, zone.zone_type),
                    egui::TextStyle::Small.resolve(&egui::Style::default()),
                    Color32::from_rgb(235, 245, 255),
                );
            }
        }
    }

    fn draw_object_like(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        asset_id: &str,
        x: f32,
        y: f32,
        scale_x: f32,
        scale_y: f32,
        flip_x: bool,
        rotation: i32,
    ) {
        if let Some(rect) =
            self.object_screen_rect_scaled(canvas_rect, asset_id, x, y, scale_x, scale_y)
        {
            self.draw_asset_image(painter, asset_id, rect, flip_x, rotation);
        }
    }

    fn object_screen_rect_scaled(
        &self,
        canvas_rect: Rect,
        asset_id: &str,
        x: f32,
        y: f32,
        scale_x: f32,
        scale_y: f32,
    ) -> Option<Rect> {
        let asset = self.registry.get(asset_id)?;
        let tile_size = self.document.tile_size as f32;
        let anchor = self.world_to_screen(
            canvas_rect,
            anchor_grid_to_world(tile_size, x, y, asset.anchor),
        );
        let size = vec2(
            asset.default_size[0] * scale_x.max(0.05),
            asset.default_size[1] * scale_y.max(0.05),
        ) * self.zoom;

        Some(screen_rect_from_anchor(anchor, size, asset.anchor))
    }

    fn object_instance_screen_rect(
        &self,
        canvas_rect: Rect,
        instance: &content::ObjectInstance,
    ) -> Option<Rect> {
        self.object_screen_rect_scaled(
            canvas_rect,
            &instance.asset,
            instance.x,
            instance.y,
            instance.scale_x,
            instance.scale_y,
        )
    }

    fn entity_instance_screen_rect(
        &self,
        canvas_rect: Rect,
        instance: &content::EntityInstance,
    ) -> Option<Rect> {
        self.object_screen_rect_scaled(
            canvas_rect,
            &instance.asset,
            instance.x,
            instance.y,
            instance.scale_x,
            instance.scale_y,
        )
    }

    fn zone_screen_rect(&self, canvas_rect: Rect, zone: &content::ZoneInstance) -> Option<Rect> {
        let tile_size = self.document.tile_size as f32;
        let mut points = zone.points.iter().map(|point| {
            self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            )
        });
        let first = points.next()?;
        let mut min = first;
        let mut max = first;
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
        Some(Rect::from_min_max(min, max))
    }

    fn draw_asset_image(
        &self,
        painter: &egui::Painter,
        asset_id: &str,
        rect: Rect,
        flip_x: bool,
        rotation: i32,
    ) {
        if let Some(texture) = self.thumbnails.get(asset_id) {
            let image_rect = fit_centered_rect(rect, texture.size_vec2());
            paint_transformed_image(
                painter,
                texture.id(),
                image_rect,
                flip_x,
                rotation,
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(rect, 1.0, Color32::from_rgb(80, 80, 90));
        }
    }

    fn tile_screen_rect(&self, canvas_rect: Rect, x: i32, y: i32, w: i32, h: i32) -> Rect {
        let tile_size = self.document.tile_size as f32;
        let world = vec2(x as f32 * tile_size, y as f32 * tile_size);
        let size = vec2(w.max(1) as f32 * tile_size, h.max(1) as f32 * tile_size);
        Rect::from_min_size(self.world_to_screen(canvas_rect, world), size * self.zoom)
    }

    fn draw_collision(&self, canvas_rect: Rect, painter: &egui::Painter) {
        if !self.layer_state(LayerKind::Collision).visible {
            return;
        }
        let tile_size = self.document.tile_size as f32;
        for cell in &self.document.layers.collision {
            if !cell.solid {
                continue;
            }

            let world = vec2(cell.x as f32 * tile_size, cell.y as f32 * tile_size);
            let rect = Rect::from_min_size(
                self.world_to_screen(canvas_rect, world),
                vec2(tile_size, tile_size) * self.zoom,
            );
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(255, 48, 48, 80));
        }
    }

    fn draw_entity_bounds(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let tile_size = self.document.tile_size as f32;
        for entity in &self.document.layers.entities {
            let world = vec2(entity.x * tile_size, entity.y * tile_size);
            let rect = Rect::from_min_size(
                self.world_to_screen(canvas_rect, world),
                vec2(tile_size, tile_size) * self.zoom,
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.5, Color32::from_rgb(120, 210, 255)),
                StrokeKind::Inside,
            );
        }
    }

    fn draw_selection_bounds(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            return;
        }

        let mut group_rect: Option<Rect> = None;
        for (index, selection) in selections.iter().enumerate() {
            let Some(rect) = self.selection_screen_rect(canvas_rect, selection) else {
                continue;
            };
            group_rect = Some(match group_rect {
                Some(group) => group.union(rect),
                None => rect,
            });
            let color = if index == 0 {
                Color32::YELLOW
            } else {
                Color32::from_rgb(95, 210, 255)
            };
            painter.rect_stroke(
                rect.expand(3.0),
                2.0,
                Stroke::new(2.0, color),
                StrokeKind::Inside,
            );
        }

        if selections.len() == 1 {
            let selection = &selections[0];
            let Some(rect) = self.selection_screen_rect(canvas_rect, selection) else {
                return;
            };
            if matches!(
                selection.layer,
                LayerKind::Decals | LayerKind::Objects | LayerKind::Entities
            ) && !self.layer_state(selection.layer).locked
            {
                for handle in resize_handle_rects(rect) {
                    painter.rect_filled(handle, 1.5, Color32::from_rgb(255, 230, 90));
                    painter.rect_stroke(
                        handle,
                        1.5,
                        Stroke::new(1.0, Color32::from_rgb(70, 55, 0)),
                        StrokeKind::Inside,
                    );
                }
            }

            if selection.layer == LayerKind::Zones {
                self.draw_zone_vertex_handles(canvas_rect, selection, painter);
            }
        } else if let Some(rect) = group_rect {
            painter.rect_stroke(
                rect.expand(8.0),
                2.0,
                Stroke::new(1.5, Color32::from_rgb(140, 235, 255)),
                StrokeKind::Inside,
            );
        }
    }

    fn draw_selection_marquee(&self, painter: &egui::Painter) {
        let Some(marquee) = &self.selection_marquee else {
            return;
        };
        let rect = Rect::from_two_pos(marquee.start, marquee.current);
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(90, 180, 255, 32));
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.5, Color32::from_rgb(100, 210, 255)),
            StrokeKind::Inside,
        );
    }

    fn draw_zone_vertex_handles(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
        painter: &egui::Painter,
    ) {
        let Some(zone) = self
            .document
            .layers
            .zones
            .iter()
            .find(|zone| zone.id == selection.id)
        else {
            return;
        };
        let tile_size = self.document.tile_size as f32;
        for (index, point) in zone.points.iter().enumerate() {
            let screen = self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            );
            let rect = Rect::from_center_size(screen, vec2(9.0, 9.0));
            painter.rect_filled(rect, 2.0, Color32::from_rgb(255, 245, 120));
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, Color32::from_rgb(70, 60, 0)),
                StrokeKind::Inside,
            );
            painter.text(
                screen + vec2(8.0, -8.0),
                egui::Align2::LEFT_CENTER,
                index.to_string(),
                egui::TextStyle::Small.resolve(&egui::Style::default()),
                Color32::WHITE,
            );
        }
    }

    fn selection_screen_rect(&self, canvas_rect: Rect, selection: &SelectedItem) -> Option<Rect> {
        match selection.layer {
            LayerKind::Ground => parse_ground_selection_id(&selection.id).and_then(|[x, y]| {
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .map(|tile| self.tile_screen_rect(canvas_rect, x, y, tile.w, tile.h))
            }),
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| self.object_instance_screen_rect(canvas_rect, instance)),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| self.object_instance_screen_rect(canvas_rect, instance)),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| self.entity_instance_screen_rect(canvas_rect, instance)),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .iter()
                .find(|zone| zone.id == selection.id)
                .and_then(|zone| self.zone_screen_rect(canvas_rect, zone)),
            LayerKind::Collision => None,
        }
    }

    fn draw_ground_footprint_preview(&self, canvas_rect: Rect, painter: &egui::Painter) {
        if !matches!(self.tool, ToolKind::Brush | ToolKind::Rectangle)
            || self.active_layer != LayerKind::Ground
        {
            return;
        }
        if self.selected_asset.is_none() {
            return;
        }
        let Some([x, y]) = self.mouse_tile else {
            return;
        };

        let [width, height] = self.clamped_ground_footprint_at(x, y);
        let rect = self.tile_screen_rect(canvas_rect, x, y, width, height);
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(120, 220, 150, 26),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(130, 235, 165)),
            StrokeKind::Inside,
        );
    }

    fn draw_rectangle_preview(&self, canvas_rect: Rect, painter: &egui::Painter) {
        if self.tool != ToolKind::Rectangle {
            return;
        }
        let Some(start) = self.rectangle_drag_start else {
            return;
        };
        let Some(end) = self.rectangle_drag_current.or(self.mouse_tile) else {
            return;
        };

        let min_x = start[0].min(end[0]);
        let max_x = start[0].max(end[0]);
        let min_y = start[1].min(end[1]);
        let max_y = start[1].max(end[1]);
        let rect = self.tile_screen_rect(
            canvas_rect,
            min_x,
            min_y,
            max_x - min_x + 1,
            max_y - min_y + 1,
        );
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(80, 160, 255, 30));
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(80, 180, 255)),
            StrokeKind::Inside,
        );
    }

    fn draw_zone_draft(&self, canvas_rect: Rect, painter: &egui::Painter, hover_pos: Option<Pos2>) {
        if self.zone_draft_points.is_empty() {
            return;
        }
        let tile_size = self.document.tile_size as f32;
        let mut points = self
            .zone_draft_points
            .iter()
            .map(|point| {
                self.world_to_screen(
                    canvas_rect,
                    vec2(point[0] * tile_size, point[1] * tile_size),
                )
            })
            .collect::<Vec<_>>();
        if let Some(hover) = hover_pos {
            points.push(hover);
        }

        for pair in points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(2.0, Color32::from_rgb(110, 220, 255)),
            );
        }
        for point in points {
            painter.circle_filled(point, 4.5, Color32::from_rgb(110, 220, 255));
        }
    }

    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mouse = self
                .mouse_tile
                .map(|tile| format!("{}, {}", tile[0], tile[1]))
                .unwrap_or_else(|| "-".to_owned());
            let asset = self.selected_asset.as_deref().unwrap_or("none");
            let selections = self.current_selection_list();
            let selected_item = if selections.len() > 1 {
                format!("{} items", selections.len())
            } else {
                selections
                    .first()
                    .map(SelectedItem::label)
                    .unwrap_or_else(|| "none".to_owned())
            };
            let transform = self
                .selected_item
                .as_ref()
                .and_then(|selection| self.transform_for_selection(selection))
                .map(|(flip_x, rotation)| format!("flip_x={}, rot={}deg", flip_x, rotation))
                .unwrap_or_else(|| "-".to_owned());
            let dirty_marker = if self.dirty { "*" } else { "" };
            let current_file = format!(
                "{}{}",
                display_project_path(&self.project_root, &self.map_path),
                dirty_marker
            );

            ui.label(format!("File: {current_file}"));
            ui.separator();
            ui.label(format!("Mouse Tile: {mouse}"));
            ui.separator();
            ui.label(format!("Selected: {asset}"));
            ui.separator();
            ui.label(format!("Selection: {selected_item}"));
            ui.separator();
            ui.label(format!("Transform: {transform}"));
            ui.separator();
            ui.label(format!("Layer: {}", self.active_layer.zh_label()));
            ui.separator();
            ui.label(format!(
                "Ground Size: {}x{}",
                self.ground_footprint_w.max(1),
                self.ground_footprint_h.max(1)
            ));
            ui.separator();
            ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
            ui.separator();
            ui.label(&self.status);
        });
    }

    fn world_to_screen(&self, canvas_rect: Rect, world: Vec2) -> Pos2 {
        canvas_rect.min + self.pan + world * self.zoom
    }

    fn screen_to_tile(&self, canvas_rect: Rect, screen: Pos2) -> Option<[i32; 2]> {
        let local = (screen - canvas_rect.min - self.pan) / self.zoom;
        let tile_size = self.document.tile_size as f32;
        let x = (local.x / tile_size).floor() as i32;
        let y = (local.y / tile_size).floor() as i32;

        if x < 0 || y < 0 || x >= self.document.width as i32 || y >= self.document.height as i32 {
            None
        } else {
            Some([x, y])
        }
    }

    fn screen_to_map_position(&self, canvas_rect: Rect, screen: Pos2) -> Option<[f32; 2]> {
        let local = (screen - canvas_rect.min - self.pan) / self.zoom;
        let tile_size = self.document.tile_size as f32;
        let x = local.x / tile_size;
        let y = local.y / tile_size;

        if x < 0.0 || y < 0.0 || x >= self.document.width as f32 || y >= self.document.height as f32
        {
            None
        } else {
            Some([x, y])
        }
    }

    fn snapped_map_position(
        &self,
        raw: [f32; 2],
        asset: Option<&AssetEntry>,
        modifiers: Modifiers,
    ) -> [f32; 2] {
        let snap = if modifiers.alt {
            SnapMode::Free
        } else if modifiers.shift {
            SnapMode::HalfGrid
        } else {
            asset.map(|asset| asset.snap).unwrap_or(SnapMode::Grid)
        };
        match snap {
            SnapMode::Grid => [raw[0].floor(), raw[1].floor()],
            SnapMode::HalfGrid => [(raw[0] * 2.0).round() * 0.5, (raw[1] * 2.0).round() * 0.5],
            SnapMode::Free => [raw[0], raw[1]],
        }
    }
}

impl eframe::App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);
        self.autosave_if_needed();

        egui::Panel::top("top_bar").show_inside(ui, |ui| self.draw_top_bar(ui));
        egui::Panel::bottom("status_bar").show_inside(ui, |ui| self.draw_status_bar(ui));
        egui::Panel::left("asset_panel")
            .resizable(true)
            .default_size(280.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.draw_asset_panel(ui));
            });
        egui::Panel::left("layer_panel")
            .resizable(true)
            .default_size(180.0)
            .show_inside(ui, |ui| self.draw_layer_panel(ui));
        egui::Panel::right("inspector_panel")
            .resizable(true)
            .default_size(300.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.draw_inspector_panel(ui));
            });
        egui::CentralPanel::default().show_inside(ui, |ui| self.draw_canvas(ui, &ctx));
        self.draw_dialogs(&ctx);
    }
}

fn draw_grid(
    painter: &egui::Painter,
    clip_rect: Rect,
    map_rect: Rect,
    width: u32,
    height: u32,
    tile_size: f32,
) {
    if tile_size < 4.0 {
        return;
    }

    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 220, 220, 34));
    let clipped = map_rect.intersect(clip_rect);
    if clipped.is_negative() {
        return;
    }

    for x in 0..=width {
        let screen_x = map_rect.min.x + x as f32 * tile_size;
        if screen_x < clip_rect.left() || screen_x > clip_rect.right() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(screen_x, clipped.top()),
                Pos2::new(screen_x, clipped.bottom()),
            ],
            stroke,
        );
    }

    for y in 0..=height {
        let screen_y = map_rect.min.y + y as f32 * tile_size;
        if screen_y < clip_rect.top() || screen_y > clip_rect.bottom() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(clipped.left(), screen_y),
                Pos2::new(clipped.right(), screen_y),
            ],
            stroke,
        );
    }
}

fn load_thumbnail(ctx: &EguiContext, asset: &AssetEntry) -> Result<TextureHandle> {
    let image = image::ImageReader::open(&asset.path)
        .with_context(|| format!("failed to open {}", asset.path.display()))?
        .decode()
        .with_context(|| format!("failed to decode {}", asset.path.display()))?;
    let thumbnail = image.thumbnail(128, 128).to_rgba8();
    let (width, height) = thumbnail.dimensions();
    let pixels = thumbnail.into_raw();
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels);

    Ok(ctx.load_texture(&asset.id, color_image, TextureOptions::NEAREST))
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn project_relative_path(project_root: &Path, path: &Path) -> Option<String> {
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

fn maps_dir(project_root: &Path) -> PathBuf {
    project_root.join("assets").join("data").join("maps")
}

fn configure_editor_fonts(ctx: &EguiContext) {
    let mut fonts = FontDefinitions::default();
    let font_name = "alien_archive_ui".to_owned();
    fonts.font_data.insert(
        font_name.clone(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/ui.ttf"
        ))),
    );

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, font_name.clone());
    }

    ctx.set_fonts(fonts);
}

fn editor_config_path(project_root: &Path) -> PathBuf {
    project_root.join(".editor").join("editor_config.ron")
}

fn load_editor_config(project_root: &Path) -> EditorConfig {
    let path = editor_config_path(project_root);
    let Ok(source) = fs::read_to_string(&path) else {
        return EditorConfig::default();
    };
    ron::from_str(&source).unwrap_or_default()
}

fn save_editor_config(project_root: &Path, config: &EditorConfig) -> Result<()> {
    let path = editor_config_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let source = ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::new())
        .context("failed to serialize editor config")?;
    fs::write(&path, source).with_context(|| format!("failed to write {}", path.display()))
}

fn default_layer_states() -> HashMap<LayerKind, LayerUiState> {
    LayerKind::ALL
        .into_iter()
        .map(|layer| (layer, LayerUiState::default()))
        .collect()
}

fn scan_map_entries(project_root: &Path) -> Vec<MapListEntry> {
    let Ok(entries) = fs::read_dir(maps_dir(project_root)) else {
        return Vec::new();
    };
    let mut maps = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
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

fn sanitize_map_id(id: &str) -> Option<String> {
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

fn sanitize_asset_id(id: &str) -> Option<String> {
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

fn sanitize_category(category: &str) -> Option<String> {
    sanitize_asset_id(category)
}

fn sanitize_relative_path(path: &str) -> Option<String> {
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

fn parse_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn unique_map_id(project_root: &Path, base_id: &str) -> String {
    let base = sanitize_map_id(base_id).unwrap_or_else(|| "untitled_overworld".to_owned());
    let map_dir = maps_dir(project_root);
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

fn unique_asset_id(database: &AssetDatabase, base_id: &str) -> String {
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

fn fit_centered_rect(bounds: Rect, source_size: Vec2) -> Rect {
    let width = source_size.x.max(1.0);
    let height = source_size.y.max(1.0);
    let scale = (bounds.width() / width).min(bounds.height() / height);

    Rect::from_center_size(bounds.center(), vec2(width * scale, height * scale))
}

fn anchor_grid_to_world(tile_size: f32, x: f32, y: f32, anchor: AnchorKind) -> Vec2 {
    match anchor {
        AnchorKind::TopLeft => vec2(x * tile_size, y * tile_size),
        AnchorKind::Center => vec2((x + 0.5) * tile_size, (y + 0.5) * tile_size),
        AnchorKind::BottomCenter => vec2((x + 0.5) * tile_size, (y + 1.0) * tile_size),
    }
}

fn screen_rect_from_anchor(anchor: Pos2, size: Vec2, anchor_kind: AnchorKind) -> Rect {
    let min = match anchor_kind {
        AnchorKind::TopLeft => anchor,
        AnchorKind::Center => Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y * 0.5),
        AnchorKind::BottomCenter => Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y),
    };
    Rect::from_min_size(min, size)
}

fn resize_handle_rects(rect: Rect) -> [Rect; 4] {
    const SIZE: f32 = 9.0;
    [
        Rect::from_center_size(rect.left_top(), vec2(SIZE, SIZE)),
        Rect::from_center_size(rect.right_top(), vec2(SIZE, SIZE)),
        Rect::from_center_size(rect.left_bottom(), vec2(SIZE, SIZE)),
        Rect::from_center_size(rect.right_bottom(), vec2(SIZE, SIZE)),
    ]
}

fn paint_transformed_image(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    rect: Rect,
    flip_x: bool,
    rotation: i32,
    tint: Color32,
) {
    let center = rect.center();
    let half_size = rect.size() * 0.5;
    let rotation = (normalize_rotation(rotation) as f32).to_radians();
    let cos = rotation.cos();
    let sin = rotation.sin();
    let corners = [
        vec2(-half_size.x, -half_size.y),
        vec2(half_size.x, -half_size.y),
        vec2(half_size.x, half_size.y),
        vec2(-half_size.x, half_size.y),
    ];
    let [uv_left, uv_right] = if flip_x { [1.0, 0.0] } else { [0.0, 1.0] };
    let uvs = [
        Pos2::new(uv_left, 0.0),
        Pos2::new(uv_right, 0.0),
        Pos2::new(uv_right, 1.0),
        Pos2::new(uv_left, 1.0),
    ];

    let mut mesh = Mesh::with_texture(texture_id);
    for (corner, uv) in corners.into_iter().zip(uvs) {
        let rotated = vec2(
            corner.x * cos - corner.y * sin,
            corner.x * sin + corner.y * cos,
        );
        mesh.vertices.push(Vertex {
            pos: center + rotated,
            uv,
            color: tint,
        });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(Shape::mesh(mesh));
}

fn normalize_rotation(rotation: i32) -> i32 {
    rotation.rem_euclid(360)
}

fn category_label(category: &str) -> &str {
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

fn anchor_label(anchor: AnchorKind) -> &'static str {
    match anchor {
        AnchorKind::TopLeft => "左上",
        AnchorKind::Center => "中心",
        AnchorKind::BottomCenter => "底部中心",
    }
}

fn snap_label(snap: SnapMode) -> &'static str {
    match snap {
        SnapMode::Grid => "网格",
        SnapMode::HalfGrid => "半格",
        SnapMode::Free => "自由",
    }
}

fn infer_asset_draft_from_path(project_root: &Path, relative_path: &str) -> AssetDraft {
    let normalized = relative_path.trim().replace('\\', "/");
    let file_stem = Path::new(&normalized)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("asset");
    let id = sanitize_asset_id(file_stem).unwrap_or_else(|| "asset".to_owned());
    let category = normalized
        .split('/')
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|pair| (pair[1] == "overworld").then(|| pair[0].to_owned()))
        .unwrap_or_else(|| "props".to_owned());
    let kind = infer_asset_kind(&category, &id);
    let mut draft = AssetDraft {
        id,
        category: sanitize_category(&category).unwrap_or_else(|| "props".to_owned()),
        path: normalized.clone(),
        kind,
        default_layer: LayerKind::Objects,
        default_size: image_dimensions(&project_root.join(&normalized)).unwrap_or([72.0, 72.0]),
        footprint: [1, 1],
        anchor: AnchorKind::BottomCenter,
        snap: SnapMode::Grid,
        tags: category.replace('_', ", "),
        entity_type: String::new(),
        codex_id: String::new(),
    };
    apply_kind_defaults(&mut draft);
    draft
}

fn infer_asset_kind(category: &str, id: &str) -> AssetKind {
    if category == "tiles" || id.contains("_tile_") {
        AssetKind::Tile
    } else if category == "decals" || id.contains("_decal_") {
        AssetKind::Decal
    } else if category == "fauna" || category == "pickups" || id.contains("_fauna_") {
        AssetKind::Entity
    } else {
        AssetKind::Object
    }
}

fn apply_kind_defaults(draft: &mut AssetDraft) {
    match draft.kind {
        AssetKind::Tile => {
            draft.default_layer = LayerKind::Ground;
            if draft.default_size[0] <= 1.0 || draft.default_size[1] <= 1.0 {
                draft.default_size = [32.0, 32.0];
            }
            draft.footprint = infer_tile_footprint(draft.default_size, 32).unwrap_or([1, 1]);
            draft.anchor = AnchorKind::TopLeft;
            draft.snap = SnapMode::Grid;
        }
        AssetKind::Decal => {
            draft.default_layer = LayerKind::Decals;
            draft.anchor = AnchorKind::Center;
            draft.snap = SnapMode::HalfGrid;
        }
        AssetKind::Object => {
            draft.default_layer = LayerKind::Objects;
            draft.anchor = AnchorKind::BottomCenter;
            draft.snap = SnapMode::Grid;
        }
        AssetKind::Entity => {
            draft.default_layer = LayerKind::Entities;
            draft.anchor = AnchorKind::BottomCenter;
            draft.snap = SnapMode::Grid;
            if draft.entity_type.trim().is_empty() {
                draft.entity_type = "Decoration".to_owned();
            }
        }
        AssetKind::Zone => {
            draft.default_layer = LayerKind::Zones;
            draft.anchor = AnchorKind::TopLeft;
            draft.snap = SnapMode::Grid;
        }
    }
}

fn image_dimensions(path: &Path) -> Option<[f32; 2]> {
    image::image_dimensions(path)
        .ok()
        .map(|(width, height)| [width as f32, height as f32])
}

fn infer_tile_footprint(default_size: [f32; 2], tile_size: u32) -> Option<[i32; 2]> {
    let tile_size = tile_size.max(1) as f32;
    let width_units = default_size[0] / tile_size;
    let height_units = default_size[1] / tile_size;
    let width = width_units.round();
    let height = height_units.round();
    ((width_units - width).abs() < 0.01 && (height_units - height).abs() < 0.01)
        .then_some([width.max(1.0) as i32, height.max(1.0) as i32])
}

fn collect_png_paths(dir: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_png_paths(&path, output);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            output.push(path);
        }
    }
}

fn zone_colors(zone_type: &str) -> (Color32, Color32) {
    match zone_type {
        "ScanArea" => (
            Color32::from_rgb(90, 210, 150),
            Color32::from_rgba_unmultiplied(90, 210, 150, 32),
        ),
        "MapTransition" => (
            Color32::from_rgb(255, 185, 85),
            Color32::from_rgba_unmultiplied(255, 185, 85, 32),
        ),
        "NoSpawn" => (
            Color32::from_rgb(235, 95, 95),
            Color32::from_rgba_unmultiplied(235, 95, 95, 32),
        ),
        "CameraBounds" => (
            Color32::from_rgb(175, 130, 255),
            Color32::from_rgba_unmultiplied(175, 130, 255, 32),
        ),
        _ => (
            Color32::from_rgb(80, 180, 255),
            Color32::from_rgba_unmultiplied(80, 180, 255, 32),
        ),
    }
}

fn polygon_screen_center(points: &[Pos2]) -> Pos2 {
    if points.is_empty() {
        return Pos2::ZERO;
    }
    let sum = points
        .iter()
        .fold(vec2(0.0, 0.0), |sum, point| sum + point.to_vec2());
    Pos2::new(sum.x / points.len() as f32, sum.y / points.len() as f32)
}

fn distance_sq(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

fn snapped_delta(delta: [f32; 2], modifiers: Modifiers) -> [f32; 2] {
    if modifiers.alt {
        delta
    } else if modifiers.shift {
        [
            (delta[0] * 2.0).round() * 0.5,
            (delta[1] * 2.0).round() * 0.5,
        ]
    } else {
        [delta[0].round(), delta[1].round()]
    }
}

fn asset_matches_search(asset: &AssetEntry, search: &str) -> bool {
    asset.id.to_ascii_lowercase().contains(search)
        || asset.relative_path.to_ascii_lowercase().contains(search)
        || asset
            .tags
            .iter()
            .any(|tag| tag.to_ascii_lowercase().contains(search))
}

fn compact_asset_label(id: &str) -> String {
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

fn object_instance_editor(
    ui: &mut egui::Ui,
    instance: &mut content::ObjectInstance,
    default_size: Option<[f32; 2]>,
    lock_aspect_ratio: &mut bool,
) -> bool {
    let mut changed = false;
    changed |= ui.text_edit_singleline(&mut instance.id).changed();
    changed |= ui.text_edit_singleline(&mut instance.asset).changed();
    changed |= ui
        .add(
            egui::DragValue::new(&mut instance.x)
                .speed(0.1)
                .prefix("x "),
        )
        .changed();
    changed |= ui
        .add(
            egui::DragValue::new(&mut instance.y)
                .speed(0.1)
                .prefix("y "),
        )
        .changed();
    changed |= instance_size_editor(
        ui,
        &mut instance.scale_x,
        &mut instance.scale_y,
        default_size,
        lock_aspect_ratio,
    );
    changed |= ui
        .add(egui::DragValue::new(&mut instance.z_index).prefix("层级 "))
        .changed();
    changed |= ui.checkbox(&mut instance.flip_x, "水平翻转").changed();
    changed |= ui
        .add(egui::DragValue::new(&mut instance.rotation).prefix("旋转 "))
        .changed();
    changed
}

fn instance_size_editor(
    ui: &mut egui::Ui,
    scale_x: &mut f32,
    scale_y: &mut f32,
    default_size: Option<[f32; 2]>,
    lock_aspect_ratio: &mut bool,
) -> bool {
    let Some([base_width, base_height]) = default_size else {
        let mut changed = false;
        changed |= ui
            .add(
                egui::DragValue::new(scale_x)
                    .range(0.05..=8.0)
                    .speed(0.01)
                    .prefix("宽缩放 "),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(scale_y)
                    .range(0.05..=8.0)
                    .speed(0.01)
                    .prefix("高缩放 "),
            )
            .changed();
        return changed;
    };

    let mut width = base_width * scale_x.max(0.05);
    let mut height = base_height * scale_y.max(0.05);
    ui.checkbox(lock_aspect_ratio, "锁定比例");
    let width_changed = ui
        .add(
            egui::DragValue::new(&mut width)
                .range(1.0..=4096.0)
                .speed(1.0)
                .prefix("宽 "),
        )
        .changed();
    let height_changed = ui
        .add(
            egui::DragValue::new(&mut height)
                .range(1.0..=4096.0)
                .speed(1.0)
                .prefix("高 "),
        )
        .changed();

    if width_changed {
        *scale_x = (width / base_width.max(1.0)).max(0.05);
        if *lock_aspect_ratio {
            *scale_y = *scale_x;
        }
    }
    if height_changed {
        *scale_y = (height / base_height.max(1.0)).max(0.05);
        if *lock_aspect_ratio {
            *scale_x = *scale_y;
        }
    }

    width_changed || height_changed
}

fn entity_rect_editor(
    ui: &mut egui::Ui,
    label: &str,
    rect: &mut Option<InstanceRect>,
    changed: &mut bool,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(label);
        if rect.is_none() && ui.button("添加").clicked() {
            *rect = Some(InstanceRect {
                offset: [0.0, 0.0],
                size: [1.0, 1.0],
            });
            *changed = true;
        }
        if rect.is_some() && ui.button("清除").clicked() {
            *rect = None;
            *changed = true;
        }
    });

    let Some(rect) = rect else {
        return;
    };

    ui.horizontal(|ui| {
        *changed |= ui
            .add(
                egui::DragValue::new(&mut rect.offset[0])
                    .speed(0.05)
                    .prefix("x "),
            )
            .changed();
        *changed |= ui
            .add(
                egui::DragValue::new(&mut rect.offset[1])
                    .speed(0.05)
                    .prefix("y "),
            )
            .changed();
    });
    ui.horizontal(|ui| {
        *changed |= ui
            .add(
                egui::DragValue::new(&mut rect.size[0])
                    .range(0.05..=32.0)
                    .speed(0.05)
                    .prefix("w "),
            )
            .changed();
        *changed |= ui
            .add(
                egui::DragValue::new(&mut rect.size[1])
                    .range(0.05..=32.0)
                    .speed(0.05)
                    .prefix("h "),
            )
            .changed();
    });
}

fn validation_summary(issues: &[MapValidationIssue]) -> String {
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

fn next_editor_object_id(prefix: &str, instances: &[content::ObjectInstance]) -> String {
    for index in 1.. {
        let candidate = format!("{prefix}_{index:03}");
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded id scan should always find a candidate")
}

fn next_editor_entity_id(prefix: &str, instances: &[content::EntityInstance]) -> String {
    for index in 1.. {
        let candidate = format!("{prefix}_{index:03}");
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded id scan should always find a candidate")
}

fn next_editor_zone_id(prefix: &str, instances: &[content::ZoneInstance]) -> String {
    for index in 1.. {
        let candidate = format!("{prefix}_{index:03}");
        if instances.iter().all(|instance| instance.id != candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded id scan should always find a candidate")
}

fn ground_selection_id(x: i32, y: i32) -> String {
    format!("{x},{y}")
}

fn parse_ground_selection_id(id: &str) -> Option<[i32; 2]> {
    let (x, y) = id.split_once(',')?;
    Some([x.parse().ok()?, y.parse().ok()?])
}
