use crate::{LayerKind, MenuCommand, ToolKind};

#[cfg(target_os = "macos")]
pub const NATIVE_MENU_ENABLED: bool = true;
#[cfg(not(target_os = "macos"))]
pub const NATIVE_MENU_ENABLED: bool = false;

#[cfg(target_os = "macos")]
pub struct NativeMenu {
    _menu: Option<muda::Menu>,
}

#[cfg(not(target_os = "macos"))]
pub struct NativeMenu;

#[cfg(target_os = "macos")]
impl NativeMenu {
    pub fn install() -> Self {
        match build_menu() {
            Ok(menu) => {
                menu.init_for_nsapp();
                Self { _menu: Some(menu) }
            }
            Err(error) => {
                eprintln!("failed to install native macOS menu: {error}");
                Self { _menu: None }
            }
        }
    }

    pub fn poll_command(&self) -> Option<MenuCommand> {
        while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            if let Some(command) = command_for_id(event.id.as_ref()) {
                return Some(command);
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
impl NativeMenu {
    pub fn install() -> Self {
        Self
    }

    pub fn poll_command(&self) -> Option<MenuCommand> {
        None
    }
}

#[cfg(target_os = "macos")]
fn build_menu() -> muda::Result<muda::Menu> {
    use muda::{AboutMetadata, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

    let app_menu = Submenu::with_items(
        "Alien Archive",
        true,
        &[
            &PredefinedMenuItem::about(
                Some("关于 Alien Archive Editor"),
                Some(AboutMetadata {
                    name: Some("Alien Archive Editor".to_owned()),
                    version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                    short_version: None,
                    authors: None,
                    comments: Some("Overworld map editor".to_owned()),
                    copyright: None,
                    license: None,
                    website: None,
                    website_label: None,
                    credits: None,
                    icon: None,
                }),
            ),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(Some("隐藏 Alien Archive Editor")),
            &PredefinedMenuItem::hide_others(Some("隐藏其他")),
            &PredefinedMenuItem::show_all(Some("全部显示")),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(Some("退出 Alien Archive Editor")),
        ],
    )?;

    let file_menu = Submenu::with_items(
        "文件",
        true,
        &[
            &menu_item("file.new", "新建地图", Some("CmdOrCtrl+n")),
            &menu_item(
                "file.open_selected",
                "打开当前选择地图",
                Some("CmdOrCtrl+o"),
            ),
            &menu_item("file.refresh_maps", "刷新地图列表", None),
            &PredefinedMenuItem::separator(),
            &menu_item("file.save", "保存", Some("CmdOrCtrl+s")),
            &menu_item("file.save_as", "另存为", Some("CmdOrCtrl+Shift+s")),
            &PredefinedMenuItem::separator(),
            &menu_item("file.delete_map", "删除地图", None),
            &menu_item("file.revert", "还原", None),
        ],
    )?;

    let edit_menu = Submenu::with_items(
        "编辑",
        true,
        &[
            &menu_item("edit.undo", "撤销", Some("CmdOrCtrl+z")),
            &menu_item("edit.redo", "重做", Some("CmdOrCtrl+Shift+z")),
            &PredefinedMenuItem::separator(),
            &menu_item("edit.copy", "复制", Some("CmdOrCtrl+c")),
            &menu_item("edit.paste", "粘贴", Some("CmdOrCtrl+v")),
            &menu_item("edit.duplicate", "复制一份", Some("CmdOrCtrl+d")),
            &menu_item("edit.delete", "删除", None),
        ],
    )?;

    let view_menu = Submenu::with_items(
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
    )?;

    let map_menu = Submenu::with_items(
        "地图",
        true,
        &[&menu_item(
            "map.validate",
            "校验地图",
            Some("CmdOrCtrl+Shift+v"),
        )],
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
            &menu_item("tool.collision", "碰撞", Some("7")),
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
            &menu_item("asset.reload", "重新扫描 Metadata", None),
        ],
    )?;

    let help_menu = Submenu::with_items(
        "帮助",
        true,
        &[&MenuItem::new("快捷键见窗口状态栏", false, None)],
    )?;

    Menu::with_items(&[
        &app_menu as &dyn IsMenuItem,
        &file_menu,
        &edit_menu,
        &view_menu,
        &map_menu,
        &layer_menu,
        &tool_menu,
        &asset_menu,
        &help_menu,
    ])
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
        "file.open_selected" => MenuCommand::OpenSelectedMap,
        "file.refresh_maps" => MenuCommand::RefreshMaps,
        "file.save" => MenuCommand::Save,
        "file.save_as" => MenuCommand::SaveAs,
        "file.delete_map" => MenuCommand::DeleteMap,
        "file.revert" => MenuCommand::RevertMap,
        "edit.undo" => MenuCommand::Undo,
        "edit.redo" => MenuCommand::Redo,
        "edit.copy" => MenuCommand::Copy,
        "edit.paste" => MenuCommand::Paste,
        "edit.duplicate" => MenuCommand::Duplicate,
        "edit.delete" => MenuCommand::DeleteSelection,
        "view.grid" => MenuCommand::ToggleGrid,
        "view.collision" => MenuCommand::ToggleCollision,
        "view.entity_bounds" => MenuCommand::ToggleEntityBounds,
        "view.zones" => MenuCommand::ToggleZones,
        "view.zone_labels" => MenuCommand::ToggleZoneLabels,
        "view.reset" => MenuCommand::ResetView,
        "map.validate" => MenuCommand::ValidateMap,
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
        "tool.collision" => MenuCommand::SetTool(ToolKind::Collision),
        "tool.zone" => MenuCommand::SetTool(ToolKind::Zone),
        "tool.pan" => MenuCommand::SetTool(ToolKind::Pan),
        "tool.zoom" => MenuCommand::SetTool(ToolKind::Zoom),
        "asset.add" => MenuCommand::AddAsset,
        "asset.edit" => MenuCommand::EditSelectedAsset,
        "asset.remove" => MenuCommand::RemoveSelectedAsset,
        "asset.save_db" => MenuCommand::SaveAssetDatabase,
        "asset.unregistered" => MenuCommand::ShowUnregisteredAssets,
        "asset.reload" => MenuCommand::ReloadAssetDatabase,
        _ => return None,
    })
}
