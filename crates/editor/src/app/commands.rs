use content::LayerKind;

use super::state::{BatchAlignMode, BatchDistributeMode, EditorWorkspace};
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
    ToggleSelectionHidden,
    DeleteSelection,
    AlignSelection(BatchAlignMode),
    DistributeSelection(BatchDistributeMode),
    ReplaceSelectionAsset,
    ToggleGrid,
    ToggleCollision,
    ToggleEntityBounds,
    ToggleZones,
    ToggleZoneLabels,
    ResetView,
    SetWorkspace(EditorWorkspace),
    ValidateMap,
    SetLayer(LayerKind),
    SetTool(ToolKind),
    AddAsset,
    EditSelectedAsset,
    RemoveSelectedAsset,
    SaveAssetDatabase,
    ShowUnregisteredAssets,
    ShowAssetDependencyReport,
    ReloadAssetDatabase,
}
