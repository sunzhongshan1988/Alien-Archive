mod app;
mod asset_registry;
mod assets;
mod canvas;
mod dialogs;
mod inspector;
mod native_menu;
mod panels;
mod terrain;
mod tools;
mod ui;
mod util;

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    fs,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use app::{
    commands::MenuCommand,
    config::{load_editor_config, save_editor_config},
    maps::{display_project_path, maps_dir, project_relative_path, project_root, scan_map_entries},
    model::{
        DEFAULT_ENTITY_TYPES, DEFAULT_UNLOCK_ITEM_IDS, launch_scene_for_mode, load_codex_database,
        validation_summary,
    },
    outliner::{
        EDITOR_KNOWN_ZONE_TYPES, OUTLINER_GROUPS, outliner_entry, outliner_matches,
        unlock_search_text, zone_focus_world,
    },
    state::{
        ClipboardItem, EditorApp, LayerUiState, LeftSidebarTab, MoveOrigin, MultiMoveDrag,
        NewMapDraft, OutlinerBadge, OutlinerEntry, ResizeDrag, SelectedItem, SelectionMarquee,
        ZoneVertexDrag, default_layer_states,
    },
};
use asset_registry::{AssetEntry, AssetRegistry};
use assets::{
    AssetDraft, apply_kind_defaults, asset_matches_search, category_label, collect_png_paths,
    compact_asset_label, image_dimensions, infer_asset_draft_from_path, infer_tile_footprint,
    load_thumbnail,
};
use canvas::rendering::{draw_grid, paint_transformed_image, zone_colors};
use content::{
    AnchorKind, AssetDatabase, AssetKind, CodexDatabase, DEFAULT_ASSET_DB_PATH,
    DEFAULT_CODEX_DB_PATH, DEFAULT_MAP_PATH, InstanceRect, LayerKind, MapDocument,
    MapValidationIssue, MapValidationSeverity, SnapMode, UnlockRule, validate_map_with_codex,
};
use eframe::egui::{
    self, Color32, Context as EguiContext, Key, Modifiers, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Vec2, vec2,
};
use terrain::{TerrainMask, TerrainRules};
use tools::ToolKind;
use ui::asset_grid::{asset_grid_columns, asset_list_row, asset_tile};
use ui::buttons::editor_icon_button;
use ui::fields::{property_options, property_text_edit};
use ui::filter_bar::filter_bar;
use ui::header::panel_header;
use ui::layer_row::layer_row;
use ui::search::search_field;
use ui::sections::inspector_section;
use ui::side_rail::collapsed_side_rail;
use ui::tabs::EditorTabs;
use ui::theme::*;
use ui::toolbar::{
    TOOLBAR_HEIGHT, configure_tool_icons, toolbar_centered, toolbar_command_button, toolbar_label,
    toolbar_tool_button,
};
use ui::tree::{TreeBadge, tree_row};
use util::{geometry::*, ids::*, sanitize::*};

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

impl EditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let project_root = project_root();
        configure_editor_fonts(&cc.egui_ctx);
        configure_editor_theme(&cc.egui_ctx);
        configure_tool_icons(&cc.egui_ctx);
        let map_path = project_root.join(DEFAULT_MAP_PATH);
        let map_entries = scan_map_entries(&project_root);
        let config = load_editor_config(&project_root);
        let asset_database = AssetDatabase::load(&project_root.join(DEFAULT_ASSET_DB_PATH))
            .unwrap_or_else(|error| {
                eprintln!("asset database load failed: {error:?}");
                AssetDatabase::new("Overworld")
            });
        let registry = AssetRegistry::from_database(&project_root, asset_database.clone());
        let (codex_database, codex_db_status) = load_codex_database(&project_root);
        let document =
            MapDocument::load(&map_path).unwrap_or_else(|_| MapDocument::new_landing_site());
        let save_as_id = document.id.clone();
        let mut app = Self {
            native_menu: native_menu::NativeMenu::install(),
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
            codex_database,
            codex_db_status,
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
            terrain_autotile: true,
            collision_brush_w: 1,
            collision_brush_h: 1,
            rectangle_erase_mode: false,
            asset_search: String::new(),
            outliner_search: String::new(),
            asset_kind_filter: None,
            show_grid: true,
            show_collision: true,
            show_entity_bounds: false,
            show_zones: true,
            show_left_sidebar: true,
            active_left_tab: LeftSidebarTab::Assets,
            show_right_sidebar: true,
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
            pending_focus_world: None,
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

    fn set_tool(&mut self, tool: ToolKind) {
        self.tool = tool;
        if tool == ToolKind::Collision {
            self.active_layer = LayerKind::Collision;
        } else if tool == ToolKind::Zone {
            self.active_layer = LayerKind::Zones;
        }
        self.status = format!("工具：{}", tool.label());
    }

    fn set_layer_shortcut(&mut self, layer: LayerKind) {
        self.active_layer = layer;
        self.tool = match layer {
            LayerKind::Collision => ToolKind::Collision,
            LayerKind::Zones => ToolKind::Zone,
            _ => ToolKind::Brush,
        };
        self.status = format!("图层：{}", layer.zh_label());
    }

    fn cancel_current_operation_or_select(&mut self) {
        let mut cancelled = false;
        if !self.zone_draft_points.is_empty() {
            self.zone_draft_points.clear();
            cancelled = true;
        }
        if self.rectangle_drag_start.take().is_some()
            || self.rectangle_drag_current.take().is_some()
            || self.selection_marquee.take().is_some()
            || self.resize_drag.take().is_some()
            || self.multi_move_drag.take().is_some()
            || self.zone_vertex_drag.take().is_some()
        {
            cancelled = true;
        }

        if cancelled {
            self.status = "已取消当前操作".to_owned();
        } else {
            self.set_tool(ToolKind::Select);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &EguiContext) {
        let wants_keyboard_input = ctx.egui_wants_keyboard_input();
        ctx.input_mut(|input| {
            if input.consume_key(Modifiers::COMMAND, Key::O) {
                self.open_map_dialog();
            }
            if input.consume_key(Modifiers::COMMAND, Key::S) {
                self.save_map();
            }

            if wants_keyboard_input {
                return;
            }
            if input.consume_key(Modifiers::COMMAND, Key::Y)
                || input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
            {
                self.redo();
            }
            if input.consume_key(Modifiers::COMMAND, Key::Z) {
                self.undo();
            }
            if input.consume_key(Modifiers::COMMAND, Key::C) {
                self.copy_selected_item();
            }
            if input.consume_key(Modifiers::COMMAND, Key::V) {
                self.paste_clipboard();
            }
            if input.consume_key(Modifiers::COMMAND, Key::D) {
                self.duplicate_selected_item();
            }
            if input.key_pressed(Key::Delete) {
                self.delete_current_selection();
            }

            if input.key_pressed(Key::Escape) {
                self.cancel_current_operation_or_select();
            }

            let unmodified = !input.modifiers.alt
                && !input.modifiers.ctrl
                && !input.modifiers.command
                && !input.modifiers.mac_cmd;

            if !unmodified {
                return;
            }

            if input.key_pressed(Key::V) {
                self.set_tool(ToolKind::Select);
            }
            if input.key_pressed(Key::B) {
                self.set_tool(ToolKind::Brush);
            }
            if input.key_pressed(Key::G) {
                self.set_tool(ToolKind::Bucket);
            }
            if input.key_pressed(Key::R) {
                self.set_tool(ToolKind::Rectangle);
            }
            if input.key_pressed(Key::E) {
                self.set_tool(ToolKind::Erase);
            }
            if input.key_pressed(Key::I) {
                self.set_tool(ToolKind::Eyedropper);
            }
            if input.key_pressed(Key::C) {
                self.set_tool(ToolKind::Collision);
            }
            if input.key_pressed(Key::A) {
                self.set_tool(ToolKind::Zone);
            }
            if input.key_pressed(Key::H) {
                self.set_tool(ToolKind::Pan);
            }
            if input.key_pressed(Key::Z) {
                self.set_tool(ToolKind::Zoom);
            }

            if input.key_pressed(Key::Num1) {
                self.set_layer_shortcut(LayerKind::Ground);
            }
            if input.key_pressed(Key::Num2) {
                self.set_layer_shortcut(LayerKind::Decals);
            }
            if input.key_pressed(Key::Num3) {
                self.set_layer_shortcut(LayerKind::Objects);
            }
            if input.key_pressed(Key::Num4) {
                self.set_layer_shortcut(LayerKind::Entities);
            }
            if input.key_pressed(Key::Num5) {
                self.set_layer_shortcut(LayerKind::Collision);
            }
            if input.key_pressed(Key::Num6) {
                self.set_layer_shortcut(LayerKind::Zones);
            }
        });
    }

    fn handle_native_menu_commands(&mut self, ctx: &EguiContext) {
        while let Some(command) = self.native_menu.poll_command() {
            self.execute_menu_command(command, ctx);
        }
    }

    fn execute_menu_command(&mut self, command: MenuCommand, ctx: &EguiContext) {
        match command {
            MenuCommand::NewMap => {
                self.new_map_draft = NewMapDraft::default();
                self.show_new_map_dialog = true;
            }
            MenuCommand::OpenMapDialog => self.open_map_dialog(),
            MenuCommand::OpenSelectedMap => self.open_selected_map(),
            MenuCommand::RefreshMaps => self.refresh_map_entries(),
            MenuCommand::Save => {
                self.save_map();
            }
            MenuCommand::SaveAs => self.save_map_as(),
            MenuCommand::SaveAndRun => self.save_and_run_current_map(),
            MenuCommand::DeleteMap => {
                self.delete_confirm_path = Some(self.selected_map_path.clone());
            }
            MenuCommand::RevertMap => {
                if self.dirty {
                    self.open_confirm_path = Some(self.map_path.clone());
                } else {
                    self.open_map(self.map_path.clone());
                }
            }
            MenuCommand::Undo => self.undo(),
            MenuCommand::Redo => self.redo(),
            MenuCommand::Copy => self.copy_selected_item(),
            MenuCommand::Paste => self.paste_clipboard(),
            MenuCommand::Duplicate => self.duplicate_selected_item(),
            MenuCommand::DeleteSelection => self.delete_current_selection(),
            MenuCommand::ToggleGrid => self.show_grid = !self.show_grid,
            MenuCommand::ToggleCollision => self.show_collision = !self.show_collision,
            MenuCommand::ToggleEntityBounds => self.show_entity_bounds = !self.show_entity_bounds,
            MenuCommand::ToggleZones => self.show_zones = !self.show_zones,
            MenuCommand::ToggleZoneLabels => self.show_zone_labels = !self.show_zone_labels,
            MenuCommand::ResetView => {
                self.pan = vec2(48.0, 48.0);
                self.zoom = 1.0;
            }
            MenuCommand::ValidateMap => {
                self.validation_issues = self.validate_current_map();
                self.show_validation_panel = true;
                self.status = validation_summary(&self.validation_issues);
            }
            MenuCommand::SetLayer(layer) => self.active_layer = layer,
            MenuCommand::SetTool(tool) => self.set_tool(tool),
            MenuCommand::AddAsset => self.open_add_asset_dialog(),
            MenuCommand::EditSelectedAsset => {
                if let Some(asset_id) = self.selected_asset.clone() {
                    self.open_edit_asset_dialog(&asset_id);
                } else {
                    self.status = "请先选择素材".to_owned();
                }
            }
            MenuCommand::RemoveSelectedAsset => self.delete_selected_asset_definition(ctx),
            MenuCommand::SaveAssetDatabase => self.save_asset_database(),
            MenuCommand::ShowUnregisteredAssets => self.show_unregistered_assets = true,
            MenuCommand::ReloadAssetDatabase => self.reload_asset_database(ctx),
        }
    }

    fn save_map(&mut self) -> bool {
        self.validation_issues = self.validate_current_map();
        if self
            .validation_issues
            .iter()
            .any(|issue| issue.severity == MapValidationSeverity::Error)
        {
            self.show_validation_panel = true;
            self.status = "保存失败：地图校验有错误".to_owned();
            return false;
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
                true
            }
            Err(error) => {
                self.status = format!("Save failed: {error:#}");
                false
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

    fn save_and_run_current_map(&mut self) {
        if !self.save_map() {
            return;
        }

        let map_arg = self.current_map_launch_path();
        let spawn_id = self
            .document
            .spawns
            .iter()
            .map(|spawn| spawn.id.trim())
            .find(|id| !id.is_empty())
            .map(str::to_owned);
        let scene_arg = launch_scene_for_mode(&self.document.mode);

        let mut command = Command::new("cargo");
        command
            .current_dir(&self.project_root)
            .arg("run")
            .arg("-p")
            .arg("alien_archive")
            .arg("--")
            .arg("--scene")
            .arg(scene_arg)
            .arg("--map")
            .arg(&map_arg);
        if let Some(spawn_id) = spawn_id.as_deref() {
            command.arg("--spawn").arg(spawn_id);
        }

        match command.spawn() {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                let spawn_note = spawn_id
                    .as_deref()
                    .map(|id| format!(" / spawn {id}"))
                    .unwrap_or_default();
                self.status = format!("已启动游戏预览：{map_arg}{spawn_note}");
            }
            Err(error) => {
                self.status = format!("启动游戏失败：{error}");
            }
        }
    }

    fn current_map_launch_path(&self) -> String {
        project_relative_path(&self.project_root, &self.map_path)
            .unwrap_or_else(|| self.map_path.to_string_lossy().replace('\\', "/"))
    }

    fn open_map_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("打开地图")
            .set_directory(maps_dir(&self.project_root))
            .add_filter("RON 地图", &["ron"])
            .pick_file()
        else {
            return;
        };

        self.selected_map_path = path.clone();
        if self.dirty && path != self.map_path {
            self.open_confirm_path = Some(path);
        } else {
            self.open_map(path);
        }
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
        validate_map_with_codex(
            &self.document,
            &self.asset_database,
            self.codex_database.as_ref(),
        )
    }

    fn asset_db_path(&self) -> PathBuf {
        self.project_root.join(DEFAULT_ASSET_DB_PATH)
    }

    fn codex_db_path(&self) -> PathBuf {
        self.project_root.join(DEFAULT_CODEX_DB_PATH)
    }

    fn reload_codex_database(&mut self) {
        let (database, status) = load_codex_database(&self.project_root);
        self.codex_database = database;
        self.codex_db_status = status.clone();
        self.status = status;
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

    fn entity_type_options(&self) -> Vec<String> {
        let mut values = DEFAULT_ENTITY_TYPES
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<BTreeSet<_>>();

        for asset in self.registry.assets() {
            if let Some(entity_type) = &asset.entity_type {
                if !entity_type.trim().is_empty() {
                    values.insert(entity_type.clone());
                }
            }
        }
        for entity in &self.document.layers.entities {
            if !entity.entity_type.trim().is_empty() {
                values.insert(entity.entity_type.clone());
            }
        }

        values.into_iter().collect()
    }

    fn codex_id_options(&self) -> Vec<String> {
        let mut values = BTreeSet::new();
        if let Some(database) = &self.codex_database {
            for id in database.ids() {
                values.insert(id.to_owned());
            }
        }
        for asset in self.registry.assets() {
            if let Some(codex_id) = &asset.codex_id {
                if !codex_id.trim().is_empty() {
                    values.insert(codex_id.clone());
                }
            }
        }
        values.into_iter().collect()
    }

    fn item_id_options(&self) -> Vec<String> {
        let mut values = DEFAULT_UNLOCK_ITEM_IDS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<BTreeSet<_>>();

        for entity in &self.document.layers.entities {
            if let Some(unlock) = &entity.unlock {
                if let Some(item_id) = &unlock.requires_item_id {
                    if !item_id.trim().is_empty() {
                        values.insert(item_id.clone());
                    }
                }
            }
        }
        for zone in &self.document.layers.zones {
            if let Some(unlock) = &zone.unlock {
                if let Some(item_id) = &unlock.requires_item_id {
                    if !item_id.trim().is_empty() {
                        values.insert(item_id.clone());
                    }
                }
            }
        }

        values.into_iter().collect()
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        if !native_menu::NATIVE_MENU_ENABLED {
            self.draw_menu_bar(ui);
            ui.separator();
        }
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
                if ui.button("打开...").clicked() {
                    self.open_map_dialog();
                    ui.close();
                }
                ui.menu_button("从列表打开", |ui| {
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
                if ui.button("保存并运行当前地图").clicked() {
                    self.save_and_run_current_map();
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
                ui.checkbox(&mut self.show_left_sidebar, "左侧栏");
                ui.checkbox(&mut self.show_right_sidebar, "右侧栏");
                ui.separator();
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
                if ui.button("保存并运行当前地图").clicked() {
                    self.save_and_run_current_map();
                    ui.close();
                }
                if ui.button("重新加载 Codex").clicked() {
                    self.reload_codex_database();
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
                        if ui
                            .selectable_value(
                                &mut self.active_layer,
                                layer,
                                format!("{} ({})", layer.zh_label(), layer_shortcut(layer)),
                            )
                            .clicked()
                        {
                            ui.close();
                        }
                        let state = self.layer_states.entry(layer).or_default();
                        ui.checkbox(&mut state.visible, "显示");
                        ui.checkbox(&mut state.locked, "锁定");
                    });
                }
            });

            ui.menu_button("工具", |ui| {
                for tool in ToolKind::ALL {
                    if ui
                        .selectable_label(self.tool == tool, tool.menu_label())
                        .clicked()
                    {
                        self.set_tool(tool);
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
                ui.label("Cmd/Ctrl+S 保存");
                ui.label("Cmd/Ctrl+Z 撤销 / Cmd/Ctrl+Shift+Z 重做");
                ui.label("V/B/G/R/E/I/C/A/H/Z 切换工具");
                ui.label("1-6 切换图层");
                ui.label("空格拖拽平移，滚轮缩放");
                ui.label("鼠标中键拖拽平移");
            });
        });
    }

    fn draw_tool_bar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
        ui.horizontal_centered(|ui| {
            ui.set_height(TOOLBAR_HEIGHT);
            toolbar_label(ui, "工具");
            for tool in ToolKind::ALL {
                if toolbar_tool_button(ui, self.tool == tool, tool).clicked() {
                    self.set_tool(tool);
                }
            }

            ui.separator();
            toolbar_label(ui, "图层");
            toolbar_centered(ui, vec2(96.0, 26.0), |ui| {
                egui::ComboBox::from_id_salt("active_layer")
                    .selected_text(self.active_layer.zh_label())
                    .width(92.0)
                    .show_ui(ui, |ui| {
                        for layer in LayerKind::ALL {
                            ui.selectable_value(&mut self.active_layer, layer, layer.zh_label());
                        }
                    });
            });

            if self.active_layer == LayerKind::Ground {
                ui.separator();
                toolbar_label(ui, "画笔尺寸");
                toolbar_centered(ui, vec2(54.0, 26.0), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.ground_footprint_w)
                            .range(1..=self.document.width as i32)
                            .speed(0.1)
                            .prefix("W "),
                    );
                });
                toolbar_centered(ui, vec2(54.0, 26.0), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.ground_footprint_h)
                            .range(1..=self.document.height as i32)
                            .speed(0.1)
                            .prefix("H "),
                    );
                });
                toolbar_centered(ui, vec2(86.0, 26.0), |ui| {
                    ui.checkbox(&mut self.terrain_autotile, "自动接边")
                })
                .inner
                .on_hover_text("刷地、矩形填充或擦除后，自动重算周围同族地形素材");
            } else if self.active_layer == LayerKind::Collision || self.tool == ToolKind::Collision
            {
                ui.separator();
                toolbar_label(ui, "碰撞尺寸");
                toolbar_centered(ui, vec2(54.0, 26.0), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.collision_brush_w)
                            .range(1..=self.document.width as i32)
                            .speed(0.1)
                            .prefix("W "),
                    );
                });
                toolbar_centered(ui, vec2(54.0, 26.0), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.collision_brush_h)
                            .range(1..=self.document.height as i32)
                            .speed(0.1)
                            .prefix("H "),
                    );
                });
            }
            if self.tool == ToolKind::Rectangle {
                ui.separator();
                toolbar_centered(ui, vec2(86.0, 26.0), |ui| {
                    ui.checkbox(&mut self.rectangle_erase_mode, "矩形擦除");
                });
            }

            ui.separator();
            if toolbar_command_button(ui, "运行地图", 68.0)
                .on_hover_text("保存当前地图并启动游戏到第一个出生点")
                .clicked()
            {
                self.save_and_run_current_map();
            }
            ui.separator();
            if toolbar_command_button(ui, "水平翻转", 72.0).clicked() {
                self.flip_selected_item();
            }
            if toolbar_command_button(ui, "左转", 48.0).clicked() {
                self.rotate_selected_item(-90);
            }
            if toolbar_command_button(ui, "右转", 48.0).clicked() {
                self.rotate_selected_item(90);
            }
            if toolbar_command_button(ui, "重置变换", 78.0).clicked() {
                self.reset_selected_transform();
            }

            if let Some([mut width, mut height]) = self.ground_size_for_selection() {
                ui.separator();
                toolbar_label(ui, "选中地块");
                let width_changed = toolbar_centered(ui, vec2(54.0, 26.0), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut width)
                            .range(1..=self.document.width as i32)
                            .speed(0.1)
                            .prefix("W "),
                    )
                    .changed()
                })
                .inner;
                let height_changed = toolbar_centered(ui, vec2(54.0, 26.0), |ui| {
                    ui.add(
                        egui::DragValue::new(&mut height)
                            .range(1..=self.document.height as i32)
                            .speed(0.1)
                            .prefix("H "),
                    )
                    .changed()
                })
                .inner;
                if width_changed || height_changed {
                    self.set_ground_size_for_selection(width, height);
                }
            }

            ui.separator();
            toolbar_centered(ui, vec2(58.0, 26.0), |ui| {
                ui.checkbox(&mut self.show_grid, "网格");
            });
            toolbar_centered(ui, vec2(58.0, 26.0), |ui| {
                ui.checkbox(&mut self.show_collision, "碰撞");
            });
            toolbar_centered(ui, vec2(82.0, 26.0), |ui| {
                ui.checkbox(&mut self.show_entity_bounds, "实体边界");
            });
        });
    }
}

impl eframe::App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        configure_editor_theme(&ctx);
        self.handle_native_menu_commands(&ctx);
        self.handle_shortcuts(&ctx);
        self.autosave_if_needed();

        egui::Panel::top("top_bar").show_inside(ui, |ui| self.draw_top_bar(ui));
        egui::Panel::bottom("status_bar").show_inside(ui, |ui| self.draw_status_bar(ui));
        if self.show_left_sidebar {
            egui::Panel::left("left_sidebar")
                .resizable(true)
                .default_size(320.0)
                .show_inside(ui, |ui| self.draw_left_sidebar(ui));
        } else {
            egui::Panel::left("left_sidebar_collapsed")
                .resizable(false)
                .default_size(34.0)
                .show_inside(ui, |ui| {
                    if collapsed_side_rail(ui, ">", "展开左侧栏") {
                        self.show_left_sidebar = true;
                    }
                });
        }
        if self.show_right_sidebar {
            egui::Panel::right("inspector_panel")
                .resizable(true)
                .default_size(300.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.draw_inspector_panel(ui));
                });
        } else {
            egui::Panel::right("right_sidebar_collapsed")
                .resizable(false)
                .default_size(34.0)
                .show_inside(ui, |ui| {
                    if collapsed_side_rail(ui, "<", "展开右侧栏") {
                        self.show_right_sidebar = true;
                    }
                });
        }
        egui::CentralPanel::default().show_inside(ui, |ui| self.draw_canvas(ui, &ctx));
        self.draw_dialogs(&ctx);
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

fn scan_badge(asset: &AssetEntry) -> Option<(&'static str, Color32)> {
    asset.codex_id.as_ref()?;
    if asset.kind == AssetKind::Entity {
        Some(("scan", THEME_ACCENT_STRONG))
    } else {
        Some(("codex", THEME_WARNING))
    }
}

fn empty_fallback(value: &str) -> &str {
    if value.trim().is_empty() {
        "未填写"
    } else {
        value
    }
}

fn draw_asset_scan_status(
    ui: &mut egui::Ui,
    asset: &AssetEntry,
    codex_database: Option<&CodexDatabase>,
) {
    ui.separator();
    ui.label("扫描 / 图鉴");
    let Some(codex_id) = &asset.codex_id else {
        ui.colored_label(THEME_MUTED_TEXT, "未设置 Codex ID，不会进入扫描候选。");
        return;
    };

    ui.label(format!("Codex ID：{codex_id}"));
    draw_codex_entry_preview(ui, codex_id, codex_database);
    if asset.kind == AssetKind::Entity {
        ui.colored_label(
            THEME_ACCENT_STRONG,
            "可扫描：是。该素材放到实体层后，运行时会把它加入扫描候选。",
        );
        if asset.entity_type.as_deref().map_or(true, str::is_empty) {
            ui.colored_label(THEME_WARNING, "缺少默认实体类型，放置后需要手动补。");
        }
    } else {
        ui.colored_label(
            THEME_WARNING,
            "可扫描：否。当前运行时只扫描实体层，Object/Decal/Tile 的 Codex ID 只是素材 metadata。",
        );
    }
}

fn draw_asset_draft_scan_status(
    ui: &mut egui::Ui,
    draft: &AssetDraft,
    codex_database: Option<&CodexDatabase>,
) {
    ui.separator();
    ui.label("扫描 / 图鉴预览");
    if draft.codex_id.trim().is_empty() {
        ui.colored_label(THEME_MUTED_TEXT, "未设置 Codex ID，不会进入扫描候选。");
        return;
    }

    draw_codex_entry_preview(ui, draft.codex_id.trim(), codex_database);
    if draft.kind == AssetKind::Entity {
        ui.colored_label(
            THEME_ACCENT_STRONG,
            "可扫描素材：放到实体层后，运行时会读取这个 Codex ID。",
        );
        if draft.entity_type.trim().is_empty() {
            ui.colored_label(THEME_WARNING, "实体类型为空，保存地图时会被校验拦住。");
        }
    } else {
        ui.colored_label(
            THEME_WARNING,
            "当前运行时只扫描实体层；如果这个素材要被扫描，请把类型改成实体。",
        );
    }
}

fn draw_codex_entry_preview(
    ui: &mut egui::Ui,
    codex_id: &str,
    codex_database: Option<&CodexDatabase>,
) {
    let Some(database) = codex_database else {
        ui.colored_label(THEME_WARNING, "Codex 数据库未加载，无法确认图鉴内容。");
        return;
    };
    let Some(entry) = database.get(codex_id) else {
        ui.colored_label(THEME_ERROR, "Codex 数据库中没有这个条目。");
        return;
    };

    ui.label(format!("标题：{}", empty_fallback(&entry.title)));
    ui.label(format!("分类：{}", empty_fallback(&entry.category)));
    ui.label(format!(
        "正文：{}",
        if entry.description.trim().is_empty() {
            "未填写"
        } else {
            "已填写"
        }
    ));
    if let Some(scan_time) = entry.scan_time {
        ui.label(format!("扫描时间：{scan_time:.2}s"));
    }
}

fn draw_entity_scan_status(
    ui: &mut egui::Ui,
    instance: &content::EntityInstance,
    asset: Option<&AssetEntry>,
    codex_database: Option<&CodexDatabase>,
) {
    ui.separator();
    ui.label("Gameplay / 扫描");
    let Some(asset) = asset else {
        ui.colored_label(THEME_ERROR, "找不到素材 metadata，无法判断扫描状态。");
        return;
    };
    let Some(codex_id) = &asset.codex_id else {
        ui.colored_label(THEME_MUTED_TEXT, "该实体素材没有 Codex ID，不会被扫描。");
        return;
    };

    ui.label(format!("Codex ID：{codex_id}"));
    draw_codex_entry_preview(ui, codex_id, codex_database);
    ui.colored_label(THEME_ACCENT_STRONG, "运行时扫描候选：是。");
    if instance.interaction_rect.is_none() {
        ui.colored_label(
            THEME_WARNING,
            "未设置交互范围，运行时会用 1x1 默认扫描区域。",
        );
    }
}

fn draw_object_layer_scan_status(
    ui: &mut egui::Ui,
    layer: LayerKind,
    instance: &content::ObjectInstance,
    asset: Option<&AssetEntry>,
    codex_database: Option<&CodexDatabase>,
) {
    let Some(asset) = asset else {
        return;
    };
    if let Some(codex_id) = &asset.codex_id {
        ui.separator();
        ui.label("Gameplay / 扫描");
        ui.label(format!("Codex ID：{codex_id}"));
        draw_codex_entry_preview(ui, codex_id, codex_database);
        ui.colored_label(
            THEME_WARNING,
            format!(
                "{} 层不会进入当前扫描系统；要扫描 {}，请改为实体素材/实体层。",
                layer.zh_label(),
                instance.id
            ),
        );
    }
}

fn layer_shortcut(layer: LayerKind) -> &'static str {
    match layer {
        LayerKind::Ground => "1",
        LayerKind::Decals => "2",
        LayerKind::Objects => "3",
        LayerKind::Entities => "4",
        LayerKind::Collision => "5",
        LayerKind::Zones => "6",
    }
}

fn object_instance_editor(
    ui: &mut egui::Ui,
    instance: &mut content::ObjectInstance,
    default_size: Option<[f32; 2]>,
    lock_aspect_ratio: &mut bool,
) -> bool {
    let mut changed = false;
    changed |= labeled_text_edit(ui, "实例 ID", &mut instance.id);
    changed |= labeled_text_edit(ui, "素材 ID", &mut instance.asset);
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

fn draw_unlock_rule_editor(
    ui: &mut egui::Ui,
    label: &str,
    id_prefix: &str,
    unlock: &mut Option<UnlockRule>,
    codex_id_options: &[String],
    item_id_options: &[String],
    codex_database: Option<&CodexDatabase>,
    changed: &mut bool,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(label);
        let mut enabled = unlock.is_some();
        if ui.checkbox(&mut enabled, "启用").changed() {
            if enabled {
                *unlock = Some(UnlockRule::default());
            } else {
                *unlock = None;
            }
            *changed = true;
        }
    });

    let Some(rule) = unlock.as_mut() else {
        ui.colored_label(THEME_MUTED_TEXT, "无解锁条件；玩家可直接通过/触发。");
        return;
    };

    let mut codex_id = rule.requires_codex_id.clone().unwrap_or_default();
    if labeled_text_edit_with_options(
        ui,
        "扫描需求",
        format!("{id_prefix}_codex"),
        &mut codex_id,
        codex_id_options,
    ) {
        set_optional_string(&mut rule.requires_codex_id, codex_id);
        *changed = true;
    }
    if let Some(codex_id) = rule.requires_codex_id.as_deref() {
        draw_codex_entry_preview(ui, codex_id, codex_database);
    }

    let mut item_id = rule.requires_item_id.clone().unwrap_or_default();
    if labeled_text_edit_with_options(
        ui,
        "物品需求",
        format!("{id_prefix}_item"),
        &mut item_id,
        item_id_options,
    ) {
        set_optional_string(&mut rule.requires_item_id, item_id);
        *changed = true;
    }

    let mut locked_message = rule.locked_message.clone().unwrap_or_default();
    if labeled_text_edit(ui, "锁定提示", &mut locked_message) {
        set_optional_string(&mut rule.locked_message, locked_message);
        *changed = true;
    }

    if rule.requires_codex_id.is_none() && rule.requires_item_id.is_none() {
        ui.colored_label(THEME_WARNING, "已启用但没有条件；运行时会视为未上锁。");
    } else {
        ui.colored_label(THEME_ACCENT_STRONG, "运行时会保存并检查这些解锁条件。");
    }
}

fn draw_transition_target_editor(
    ui: &mut egui::Ui,
    label: &str,
    id_prefix: &str,
    transition: &mut Option<content::TransitionTarget>,
    changed: &mut bool,
) {
    const SCENE_OPTIONS: &[&str] = &["Overworld", "Facility"];

    ui.separator();
    ui.horizontal(|ui| {
        ui.label(label);
        let mut enabled = transition.is_some();
        if ui.checkbox(&mut enabled, "启用").changed() {
            if enabled {
                *transition = Some(content::TransitionTarget::default());
            } else {
                *transition = None;
            }
            *changed = true;
        }
    });

    let Some(target) = transition.as_mut() else {
        ui.colored_label(
            THEME_MUTED_TEXT,
            "无转场目标；入口/出口会使用运行时默认目的地。",
        );
        return;
    };

    let mut scene = target.scene.clone().unwrap_or_default();
    let scene_options = SCENE_OPTIONS
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    if labeled_text_edit_with_options(
        ui,
        "目标场景",
        format!("{id_prefix}_scene"),
        &mut scene,
        &scene_options,
    ) {
        set_optional_string(&mut target.scene, scene);
        *changed = true;
    }

    let mut map_path = target.map_path.clone().unwrap_or_default();
    if labeled_text_edit(ui, "目标地图", &mut map_path) {
        set_optional_string(&mut target.map_path, map_path);
        *changed = true;
    }

    let mut spawn_id = target.spawn_id.clone().unwrap_or_default();
    if labeled_text_edit(ui, "出生点", &mut spawn_id) {
        set_optional_string(&mut target.spawn_id, spawn_id);
        *changed = true;
    }

    if target.scene.is_none() && target.map_path.is_none() && target.spawn_id.is_none() {
        ui.colored_label(
            THEME_WARNING,
            "已启用但没有目标字段；运行时会继续使用默认目的地。",
        );
    } else {
        ui.colored_label(THEME_ACCENT_STRONG, "运行时会优先使用这些转场目标字段。");
    }
}

fn labeled_text_edit(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    property_text_edit(ui, label, value)
}

fn labeled_text_edit_with_options(
    ui: &mut egui::Ui,
    label: &str,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    options: &[String],
) -> bool {
    let changed = labeled_text_edit(ui, label, value);
    if options.is_empty() {
        return changed;
    }

    changed | property_options(ui, "常用", id_salt, value, options)
}

fn set_optional_string(target: &mut Option<String>, value: String) {
    let value = value.trim().to_owned();
    *target = (!value.is_empty()).then_some(value);
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
