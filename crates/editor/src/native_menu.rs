#[cfg(target_os = "macos")]
use content::LayerKind;

use crate::app::commands::MenuCommand;
#[cfg(target_os = "macos")]
use crate::app::state::{BatchAlignMode, BatchDistributeMode, EditorWorkspace};
#[cfg(target_os = "macos")]
use crate::tools::ToolKind;
#[cfg(target_os = "macos")]
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[cfg(target_os = "macos")]
pub const NATIVE_MENU_ENABLED: bool = true;
#[cfg(not(target_os = "macos"))]
pub const NATIVE_MENU_ENABLED: bool = false;

#[cfg(target_os = "macos")]
pub struct NativeMenu {
    _menu: Option<muda::Menu>,
    commands: Arc<Mutex<VecDeque<MenuCommand>>>,
}

#[cfg(not(target_os = "macos"))]
pub struct NativeMenu;

#[cfg(target_os = "macos")]
impl NativeMenu {
    pub fn install(workspace: EditorWorkspace, ctx: eframe::egui::Context) -> Self {
        let commands = Arc::new(Mutex::new(VecDeque::new()));
        let handler_commands = Arc::clone(&commands);
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if let Some(command) = command_for_id(event.id.as_ref()) {
                if let Ok(mut commands) = handler_commands.lock() {
                    commands.push_back(command);
                }
                ctx.request_repaint();
            }
        }));

        match build_menu(workspace) {
            Ok(menu) => {
                menu.init_for_nsapp();
                Self {
                    _menu: Some(menu),
                    commands,
                }
            }
            Err(error) => {
                eprintln!("failed to install native macOS menu: {error}");
                Self {
                    _menu: None,
                    commands,
                }
            }
        }
    }

    pub fn poll_command(&self) -> Option<MenuCommand> {
        self.commands
            .lock()
            .ok()
            .and_then(|mut commands| commands.pop_front())
    }

    pub fn set_workspace(&mut self, workspace: EditorWorkspace) {
        if let Some(menu) = self._menu.take() {
            menu.remove_for_nsapp();
        }

        match build_menu(workspace) {
            Ok(menu) => {
                menu.init_for_nsapp();
                self._menu = Some(menu);
            }
            Err(error) => {
                eprintln!("failed to rebuild native macOS menu: {error}");
                self._menu = None;
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl NativeMenu {
    pub fn install(
        _workspace: crate::app::state::EditorWorkspace,
        _ctx: eframe::egui::Context,
    ) -> Self {
        Self
    }

    pub fn poll_command(&self) -> Option<MenuCommand> {
        None
    }

    pub fn set_workspace(&mut self, _workspace: crate::app::state::EditorWorkspace) {}
}

#[cfg(target_os = "macos")]
fn build_menu(workspace: EditorWorkspace) -> muda::Result<muda::Menu> {
    use muda::{AboutMetadata, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

    let app_menu = Submenu::with_items(
        "Alien Archive",
        true,
        &[
            &PredefinedMenuItem::about(
                Some("关于 Alien Archive 游戏编辑器"),
                Some(AboutMetadata {
                    name: Some("Alien Archive 游戏编辑器".to_owned()),
                    version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                    short_version: None,
                    authors: None,
                    comments: Some("Alien Archive 游戏内容编辑器".to_owned()),
                    copyright: None,
                    license: None,
                    website: None,
                    website_label: None,
                    credits: None,
                    icon: None,
                }),
            ),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(Some("隐藏 Alien Archive 游戏编辑器")),
            &PredefinedMenuItem::hide_others(Some("隐藏其他")),
            &PredefinedMenuItem::show_all(Some("全部显示")),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(Some("退出 Alien Archive 游戏编辑器")),
        ],
    )?;

    let file_menu = match workspace {
        EditorWorkspace::OverworldMap => Submenu::with_items(
            "文件",
            true,
            &[
                &menu_item("file.new", "新建地图", Some("CmdOrCtrl+n")),
                &menu_item("file.open_dialog", "打开地图...", Some("CmdOrCtrl+o")),
                &menu_item("file.refresh_maps", "刷新地图列表", None),
                &PredefinedMenuItem::separator(),
                &menu_item("file.save", "保存地图", Some("CmdOrCtrl+s")),
                &menu_item("file.save_run", "保存并运行当前地图", None),
                &menu_item("file.save_as", "另存为", Some("CmdOrCtrl+Shift+s")),
                &PredefinedMenuItem::separator(),
                &menu_item("file.delete_map", "删除地图", None),
                &menu_item("file.revert", "还原地图", None),
            ],
        )?,
        EditorWorkspace::Cutscenes => Submenu::with_items(
            "文件",
            true,
            &[
                &menu_item("cutscene.save", "保存过场", Some("CmdOrCtrl+s")),
                &menu_item("cutscene.reload", "重新加载过场", None),
            ],
        )?,
        EditorWorkspace::Events => Submenu::with_items(
            "文件",
            true,
            &[
                &menu_item("event.save", "保存事件", Some("CmdOrCtrl+s")),
                &menu_item("event.reload", "重新加载事件", None),
            ],
        )?,
    };

    let edit_menu = match workspace {
        EditorWorkspace::OverworldMap => Submenu::with_items(
            "编辑",
            true,
            &[
                &menu_item("edit.undo", "撤销", Some("CmdOrCtrl+z")),
                &menu_item("edit.redo", "重做", Some("CmdOrCtrl+Shift+z")),
                &PredefinedMenuItem::separator(),
                &menu_item("edit.copy", "复制", Some("CmdOrCtrl+c")),
                &menu_item("edit.paste", "粘贴", Some("CmdOrCtrl+v")),
                &menu_item("edit.duplicate", "复制一份", Some("CmdOrCtrl+d")),
                &menu_item("edit.toggle_hidden", "隐藏/显示所选", None),
                &menu_item("edit.delete", "删除", None),
                &PredefinedMenuItem::separator(),
                &menu_item("edit.align_left", "左对齐", None),
                &menu_item("edit.align_center_x", "水平居中", None),
                &menu_item("edit.align_right", "右对齐", None),
                &menu_item("edit.align_top", "顶对齐", None),
                &menu_item("edit.align_center_y", "垂直居中", None),
                &menu_item("edit.align_bottom", "底对齐", None),
                &PredefinedMenuItem::separator(),
                &menu_item("edit.distribute_horizontal", "水平分布", None),
                &menu_item("edit.distribute_vertical", "垂直分布", None),
                &menu_item("edit.replace_asset", "用当前素材替换所选", None),
            ],
        )?,
        EditorWorkspace::Cutscenes => Submenu::with_items(
            "编辑",
            true,
            &[
                &menu_item("cutscene.new", "新增过场", Some("CmdOrCtrl+n")),
                &menu_item("cutscene.duplicate", "复制过场", None),
                &menu_item("cutscene.delete", "删除过场", None),
            ],
        )?,
        EditorWorkspace::Events => Submenu::with_items(
            "编辑",
            true,
            &[
                &menu_item("event.new", "新增事件", Some("CmdOrCtrl+n")),
                &menu_item("event.duplicate", "复制事件", None),
                &menu_item("event.delete", "删除事件", None),
            ],
        )?,
    };

    let view_menu = match workspace {
        EditorWorkspace::OverworldMap => Submenu::with_items(
            "视图",
            true,
            &[
                &menu_item("view.grid", "切换网格", Some("CmdOrCtrl+g")),
                &menu_item("view.collision", "切换碰撞", None),
                &menu_item("view.entity_bounds", "切换实体边界", None),
                &menu_item("view.zones", "切换区域", None),
                &menu_item("view.zone_labels", "切换区域标签", None),
                &PredefinedMenuItem::separator(),
                &menu_item("view.reset", "重置视图", Some("CmdOrCtrl+0")),
            ],
        )?,
        EditorWorkspace::Cutscenes => Submenu::with_items(
            "视图",
            true,
            &[&MenuItem::new(
                "过场工作区没有地图画布视图选项",
                false,
                None,
            )],
        )?,
        EditorWorkspace::Events => Submenu::with_items(
            "视图",
            true,
            &[&MenuItem::new(
                "事件工作区没有地图画布视图选项",
                false,
                None,
            )],
        )?,
    };

    let workspace_menu = Submenu::with_items(
        "工作区",
        true,
        &[
            &menu_item("workspace.map", "开放世界地图", None),
            &menu_item("workspace.cutscenes", "过场", None),
            &menu_item("workspace.events", "事件", None),
        ],
    )?;

    let settings_menu = Submenu::with_items(
        "设置",
        true,
        &[&menu_item(
            "settings.global",
            "全局设置",
            Some("CmdOrCtrl+,"),
        )],
    )?;

    let map_menu = Submenu::with_items(
        "地图",
        true,
        &[
            &menu_item("map.validate", "校验地图", Some("CmdOrCtrl+Shift+v")),
            &menu_item("file.save_run", "保存并运行当前地图", None),
            &menu_item("map.reload_codex", "重新加载图鉴", None),
        ],
    )?;

    let layer_menu = Submenu::with_items(
        "图层",
        true,
        &[
            &menu_item("layer.ground", "地表", None),
            &menu_item("layer.decals", "贴花", None),
            &menu_item("layer.objects", "物件", None),
            &menu_item("layer.entities", "实体", None),
            &menu_item("layer.zones", "区域", None),
            &menu_item("layer.collision", "碰撞", None),
        ],
    )?;

    let tool_menu = Submenu::with_items(
        "工具",
        true,
        &[
            &menu_item("tool.select", "选择", Some("1")),
            &menu_item("tool.brush", "画笔", Some("2")),
            &menu_item("tool.bucket", "油漆桶", Some("3")),
            &menu_item("tool.rectangle", "矩形", Some("4")),
            &menu_item("tool.erase", "橡皮", Some("5")),
            &menu_item("tool.eyedropper", "吸管", Some("6")),
            &menu_item("tool.stamp", "盖章", Some("7")),
            &menu_item("tool.collision", "碰撞", Some("8")),
            &menu_item("tool.zone", "区域", None),
            &menu_item("tool.pan", "平移", None),
            &menu_item("tool.zoom", "缩放", None),
        ],
    )?;

    let asset_menu = Submenu::with_items(
        "素材",
        true,
        &[
            &menu_item("asset.add", "添加素材", None),
            &menu_item("asset.edit", "编辑当前素材", None),
            &menu_item("asset.remove", "移除当前素材", None),
            &menu_item("asset.save_db", "保存素材库", None),
            &PredefinedMenuItem::separator(),
            &menu_item("asset.unregistered", "未登记图片", None),
            &menu_item("asset.dependency_report", "资产依赖报告", None),
            &menu_item("asset.reload", "重新扫描元数据", None),
        ],
    )?;

    let cutscene_menu = Submenu::with_items(
        "过场",
        true,
        &[
            &menu_item("cutscene.menu_new", "新增", None),
            &menu_item("cutscene.menu_duplicate", "复制当前", None),
            &menu_item("cutscene.menu_delete", "删除当前", None),
            &menu_item("cutscene.menu_save", "保存", None),
            &menu_item("cutscene.menu_reload", "重新加载", None),
            &menu_item("cutscene.menu_validate", "校验", None),
        ],
    )?;

    let event_menu = Submenu::with_items(
        "事件",
        true,
        &[
            &menu_item("event.menu_new", "新增", None),
            &menu_item("event.menu_duplicate", "复制当前", None),
            &menu_item("event.menu_delete", "删除当前", None),
            &menu_item("event.menu_save", "保存", None),
            &menu_item("event.menu_reload", "重新加载", None),
            &menu_item("event.menu_validate", "校验", None),
        ],
    )?;

    let help_menu = Submenu::with_items(
        "帮助",
        true,
        &[&MenuItem::new(
            match workspace {
                EditorWorkspace::OverworldMap => "地图快捷键见窗口状态栏",
                EditorWorkspace::Cutscenes => "过场：Cmd/Ctrl+S 保存，Cmd/Ctrl+N 新增",
                EditorWorkspace::Events => "事件：Cmd/Ctrl+S 保存，Cmd/Ctrl+N 新增",
            },
            false,
            None,
        )],
    )?;

    let menu = Menu::new();
    menu.append(&app_menu as &dyn IsMenuItem)?;
    menu.append(&file_menu)?;
    menu.append(&edit_menu)?;
    menu.append(&view_menu)?;
    menu.append(&workspace_menu)?;
    match workspace {
        EditorWorkspace::OverworldMap => {
            menu.append(&map_menu)?;
            menu.append(&layer_menu)?;
            menu.append(&tool_menu)?;
            menu.append(&asset_menu)?;
        }
        EditorWorkspace::Cutscenes => {
            menu.append(&cutscene_menu)?;
        }
        EditorWorkspace::Events => {
            menu.append(&event_menu)?;
        }
    }
    menu.append(&settings_menu)?;
    menu.append(&help_menu)?;
    Ok(menu)
}

#[cfg(target_os = "macos")]
fn menu_item(id: &str, text: &str, accelerator: Option<&str>) -> muda::MenuItem {
    let accelerator = accelerator.and_then(|accelerator| accelerator.parse().ok());
    muda::MenuItem::with_id(id, text, true, accelerator)
}

#[cfg(target_os = "macos")]
fn command_for_id(id: &str) -> Option<MenuCommand> {
    Some(match id {
        "file.new" => MenuCommand::NewMap,
        "file.open_dialog" => MenuCommand::OpenMapDialog,
        "file.open_selected" => MenuCommand::OpenSelectedMap,
        "file.refresh_maps" => MenuCommand::RefreshMaps,
        "file.save" => MenuCommand::Save,
        "file.save_run" => MenuCommand::SaveAndRun,
        "file.save_as" => MenuCommand::SaveAs,
        "file.delete_map" => MenuCommand::DeleteMap,
        "file.revert" => MenuCommand::RevertMap,
        "edit.undo" => MenuCommand::Undo,
        "edit.redo" => MenuCommand::Redo,
        "edit.copy" => MenuCommand::Copy,
        "edit.paste" => MenuCommand::Paste,
        "edit.duplicate" => MenuCommand::Duplicate,
        "edit.toggle_hidden" => MenuCommand::ToggleSelectionHidden,
        "edit.delete" => MenuCommand::DeleteSelection,
        "edit.align_left" => MenuCommand::AlignSelection(BatchAlignMode::Left),
        "edit.align_center_x" => MenuCommand::AlignSelection(BatchAlignMode::CenterX),
        "edit.align_right" => MenuCommand::AlignSelection(BatchAlignMode::Right),
        "edit.align_top" => MenuCommand::AlignSelection(BatchAlignMode::Top),
        "edit.align_center_y" => MenuCommand::AlignSelection(BatchAlignMode::CenterY),
        "edit.align_bottom" => MenuCommand::AlignSelection(BatchAlignMode::Bottom),
        "edit.distribute_horizontal" => {
            MenuCommand::DistributeSelection(BatchDistributeMode::Horizontal)
        }
        "edit.distribute_vertical" => {
            MenuCommand::DistributeSelection(BatchDistributeMode::Vertical)
        }
        "edit.replace_asset" => MenuCommand::ReplaceSelectionAsset,
        "view.grid" => MenuCommand::ToggleGrid,
        "view.collision" => MenuCommand::ToggleCollision,
        "view.entity_bounds" => MenuCommand::ToggleEntityBounds,
        "view.zones" => MenuCommand::ToggleZones,
        "view.zone_labels" => MenuCommand::ToggleZoneLabels,
        "view.reset" => MenuCommand::ResetView,
        "workspace.map" => MenuCommand::SetWorkspace(EditorWorkspace::OverworldMap),
        "workspace.cutscenes" => MenuCommand::SetWorkspace(EditorWorkspace::Cutscenes),
        "workspace.events" => MenuCommand::SetWorkspace(EditorWorkspace::Events),
        "settings.global" => MenuCommand::OpenGlobalSettings,
        "map.validate" => MenuCommand::ValidateMap,
        "map.reload_codex" => MenuCommand::ReloadCodexDatabase,
        "layer.ground" => MenuCommand::SetLayer(LayerKind::Ground),
        "layer.decals" => MenuCommand::SetLayer(LayerKind::Decals),
        "layer.objects" => MenuCommand::SetLayer(LayerKind::Objects),
        "layer.entities" => MenuCommand::SetLayer(LayerKind::Entities),
        "layer.zones" => MenuCommand::SetLayer(LayerKind::Zones),
        "layer.collision" => MenuCommand::SetLayer(LayerKind::Collision),
        "tool.select" => MenuCommand::SetTool(ToolKind::Select),
        "tool.brush" => MenuCommand::SetTool(ToolKind::Brush),
        "tool.bucket" => MenuCommand::SetTool(ToolKind::Bucket),
        "tool.rectangle" => MenuCommand::SetTool(ToolKind::Rectangle),
        "tool.erase" => MenuCommand::SetTool(ToolKind::Erase),
        "tool.eyedropper" => MenuCommand::SetTool(ToolKind::Eyedropper),
        "tool.stamp" => MenuCommand::SetTool(ToolKind::Stamp),
        "tool.collision" => MenuCommand::SetTool(ToolKind::Collision),
        "tool.zone" => MenuCommand::SetTool(ToolKind::Zone),
        "tool.pan" => MenuCommand::SetTool(ToolKind::Pan),
        "tool.zoom" => MenuCommand::SetTool(ToolKind::Zoom),
        "asset.add" => MenuCommand::AddAsset,
        "asset.edit" => MenuCommand::EditSelectedAsset,
        "asset.remove" => MenuCommand::RemoveSelectedAsset,
        "asset.save_db" => MenuCommand::SaveAssetDatabase,
        "asset.unregistered" => MenuCommand::ShowUnregisteredAssets,
        "asset.dependency_report" => MenuCommand::ShowAssetDependencyReport,
        "asset.reload" => MenuCommand::ReloadAssetDatabase,
        "cutscene.new" => MenuCommand::NewCutscene,
        "cutscene.duplicate" => MenuCommand::DuplicateCutscene,
        "cutscene.delete" => MenuCommand::DeleteCutscene,
        "cutscene.save" => MenuCommand::SaveCutscenes,
        "cutscene.reload" => MenuCommand::ReloadCutscenes,
        "cutscene.validate" => MenuCommand::ValidateCutscenes,
        "cutscene.menu_new" => MenuCommand::NewCutscene,
        "cutscene.menu_duplicate" => MenuCommand::DuplicateCutscene,
        "cutscene.menu_delete" => MenuCommand::DeleteCutscene,
        "cutscene.menu_save" => MenuCommand::SaveCutscenes,
        "cutscene.menu_reload" => MenuCommand::ReloadCutscenes,
        "cutscene.menu_validate" => MenuCommand::ValidateCutscenes,
        "event.new" => MenuCommand::NewEvent,
        "event.duplicate" => MenuCommand::DuplicateEvent,
        "event.delete" => MenuCommand::DeleteEvent,
        "event.save" => MenuCommand::SaveEvents,
        "event.reload" => MenuCommand::ReloadEvents,
        "event.validate" => MenuCommand::ValidateEvents,
        "event.menu_new" => MenuCommand::NewEvent,
        "event.menu_duplicate" => MenuCommand::DuplicateEvent,
        "event.menu_delete" => MenuCommand::DeleteEvent,
        "event.menu_save" => MenuCommand::SaveEvents,
        "event.menu_reload" => MenuCommand::ReloadEvents,
        "event.menu_validate" => MenuCommand::ValidateEvents,
        _ => return None,
    })
}
