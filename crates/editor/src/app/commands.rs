use content::LayerKind;

use crate::tools::ToolKind;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MenuCommand {
    NewMap,
    OpenMapDialog,
    OpenSelectedMap,
    RefreshMaps,
    Save,
    SaveAs,
    SaveAndRun,
    DeleteMap,
    RevertMap,
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    DeleteSelection,
    ToggleGrid,
    ToggleCollision,
    ToggleEntityBounds,
    ToggleZones,
    ToggleZoneLabels,
    ResetView,
    ValidateMap,
    SetLayer(LayerKind),
    SetTool(ToolKind),
    AddAsset,
    EditSelectedAsset,
    RemoveSelectedAsset,
    SaveAssetDatabase,
    ShowUnregisteredAssets,
    ReloadAssetDatabase,
}
