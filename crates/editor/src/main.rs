mod app;
mod asset_registry;
mod assets;
mod canvas;
mod dialogs;
mod inspector;
mod native_menu;
mod panels;
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

    fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &EguiContext) {
        let desired_size = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());
        let rect = response.rect;

        if let Some(focus_world) = self.pending_focus_world.take() {
            self.pan = (rect.center() - rect.min) - focus_world * self.zoom;
        }

        painter.rect_filled(rect, 0.0, THEME_CANVAS_BG);
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
        painter.rect_filled(map_rect, 0.0, THEME_MAP_BG);

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
        let middle_pan = ctx.input(|input| input.pointer.button_down(egui::PointerButton::Middle))
            && response.is_pointer_button_down_on();
        let panning = self.tool == ToolKind::Pan || space_down || middle_pan;

        if panning && (response.dragged() || middle_pan) {
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
        let middle_button_active = ctx.input(|input| {
            input.pointer.button_down(egui::PointerButton::Middle)
                || input.pointer.button_released(egui::PointerButton::Middle)
        });
        if matches!(self.tool, ToolKind::Select | ToolKind::Pan | ToolKind::Zoom)
            || space_down
            || middle_button_active
            || response.drag_started_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Middle)
            || response.clicked_by(egui::PointerButton::Middle)
        {
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
        let middle_button_active = ctx.input(|input| {
            input.pointer.button_down(egui::PointerButton::Middle)
                || input.pointer.button_released(egui::PointerButton::Middle)
        });
        if self.tool != ToolKind::Select
            || space_down
            || middle_button_active
            || response.drag_started_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Middle)
            || response.clicked_by(egui::PointerButton::Middle)
        {
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
            unlock: None,
            transition: None,
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
        let [width, height] = self.clamped_collision_brush_at(x, y);
        for yy in y..y + height {
            for xx in x..x + width {
                self.document.place_collision(xx, yy);
            }
        }
    }

    fn erase_brush_at(&mut self, x: i32, y: i32) {
        let [width, height] = if self.active_layer == LayerKind::Collision {
            self.clamped_collision_brush_at(x, y)
        } else {
            self.clamped_ground_footprint_at(x, y)
        };
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

    fn clamped_collision_brush_at(&self, x: i32, y: i32) -> [i32; 2] {
        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);

        [
            self.collision_brush_w.clamp(1, max_width),
            self.collision_brush_h.clamp(1, max_height),
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
                    THEME_TEXT,
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
            painter.rect_filled(rect, 1.0, Color32::from_rgb(68, 72, 64));
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
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(
                    THEME_COLLISION.r(),
                    THEME_COLLISION.g(),
                    THEME_COLLISION.b(),
                    86,
                ),
            );
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
                Stroke::new(1.5, THEME_ACCENT),
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
                THEME_SELECTION
            } else {
                THEME_MULTI_SELECTION
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
                    painter.rect_filled(handle, 1.5, THEME_SELECTION);
                    painter.rect_stroke(
                        handle,
                        1.5,
                        Stroke::new(1.0, THEME_WARNING_BG),
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
                Stroke::new(1.5, THEME_MULTI_SELECTION),
                StrokeKind::Inside,
            );
        }
    }

    fn draw_selection_marquee(&self, painter: &egui::Painter) {
        let Some(marquee) = &self.selection_marquee else {
            return;
        };
        let rect = Rect::from_two_pos(marquee.start, marquee.current);
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(
                THEME_MULTI_SELECTION.r(),
                THEME_MULTI_SELECTION.g(),
                THEME_MULTI_SELECTION.b(),
                34,
            ),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.5, THEME_MULTI_SELECTION),
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
            painter.rect_filled(rect, 2.0, THEME_SELECTION);
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, THEME_WARNING_BG),
                StrokeKind::Inside,
            );
            painter.text(
                screen + vec2(8.0, -8.0),
                egui::Align2::LEFT_CENTER,
                index.to_string(),
                egui::TextStyle::Small.resolve(&egui::Style::default()),
                THEME_TEXT,
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
        if !matches!(
            (self.active_layer, self.tool),
            (LayerKind::Ground, ToolKind::Brush | ToolKind::Rectangle)
                | (
                    LayerKind::Collision,
                    ToolKind::Brush | ToolKind::Rectangle | ToolKind::Collision
                )
        ) {
            return;
        }
        if self.active_layer == LayerKind::Ground && self.selected_asset.is_none() {
            return;
        }
        let Some([x, y]) = self.mouse_tile else {
            return;
        };

        let [width, height] = if self.active_layer == LayerKind::Collision {
            self.clamped_collision_brush_at(x, y)
        } else {
            self.clamped_ground_footprint_at(x, y)
        };
        let rect = self.tile_screen_rect(canvas_rect, x, y, width, height);
        let color = if self.active_layer == LayerKind::Collision {
            THEME_COLLISION
        } else {
            THEME_ACCENT
        };
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 32),
        );
        painter.rect_stroke(rect, 0.0, Stroke::new(2.0, color), StrokeKind::Inside);
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
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(
                THEME_WARNING.r(),
                THEME_WARNING.g(),
                THEME_WARNING.b(),
                30,
            ),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(2.0, THEME_WARNING),
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
            painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, THEME_ACCENT_STRONG));
        }
        for point in points {
            painter.circle_filled(point, 4.5, THEME_ACCENT_STRONG);
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
