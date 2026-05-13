# Agent Architecture Map

Audience: AI code agents working in this repo. Prefer this file over broad search when choosing where to edit. Verify live code before making risky changes.

Update rule: if a code change invalidates a route, ownership boundary, data flow, or command in this file, update this file in the same change.

## Read Order

1. `docs/AGENT_ARCHITECTURE.md`
2. `docs/PROJECT_STATE.md`
3. Task-specific docs:
   - Game menu: `docs/GAME_MENU_ARCHITECTURE.md`
   - Editor roadmap/status: `docs/EDITOR_ROADMAP.md`
   - HUD art/layout: `docs/HUD_ART_DIRECTION.md`
4. Relevant entry files:
   - Game runtime: `crates/game/src/main.rs`
   - Scene/runtime state: `crates/game/src/scenes/mod.rs`
   - Content schemas: `crates/content/src/lib.rs`
   - World adapter: `crates/game/src/world.rs`
   - Editor app: `crates/editor/src/main.rs`

## Workspace Graph

```text
runtime
  no project-domain dependency
  owns app loop, wgpu renderer, input, camera, math, collision, audio, SceneCommand

content
  no runtime/game/editor dependency
  owns RON schemas, constants, data loaders, semantic ids, validation

game -> runtime + content
  owns executable game, save model, player, world runtime adapter, scene stack, gameplay systems, UI

editor -> content
  owns eframe game editor, current Overworld workspace, asset registry, canvas tools, inspector, validation UI

tools
  owns asset generation/maintenance helpers; not runtime gameplay code
```

Do not add a dependency from `content` to `game`, `runtime`, or `editor`. `content` must stay schema/validation focused so both game and editor can share it.

## Data Locations

```text
assets/data/maps/*.ron                         map documents loaded by game/editor
assets/data/assets/overworld_assets.ron        asset database used by game/editor
assets/data/codex/overworld_codex.ron          codex database loaded by game/editor
assets/data/ui/localization.ron                runtime UI localization dictionary
crates/content/data/cutscenes.ron              bundled cutscene/sequence database
crates/content/data/items.ron                  item database fallback/source
crates/content/data/objectives.ron             objective database fallback/source
saves/profile_01.ron                           default save file
```

Format rule: authored gameplay/editor data is RON unless a file already proves otherwise.

## Runtime Flow

```text
crates/game/src/main.rs
  GameLaunchOptions::from_env_and_args
  SaveData::load_or_default
  CodexDatabase::load
  CutsceneDatabase::load
  GameContext::from_save
  SceneStack::new_main_menu OR SceneStack::new_scene
  runtime::run

runtime::Game callbacks
  setup  -> SceneStack::setup_current
  update -> SceneStack::update -> GameContext::update_save
  render -> SceneStack::render
  camera -> SceneStack::camera
```

`GameContext` is the shared mutable runtime/save bridge. Any gameplay mutation that must persist should usually go through `GameContext` methods so derived profile state, activity log, inventory load, codex/objective progress, and save dirty flags stay synchronized.

## Scene Stack

Core files:

```text
crates/game/src/scenes/mod.rs
crates/runtime/src/scene.rs
```

Scene contract:

```text
Scene::id
Scene::debug_name
Scene::setup
Scene::update -> SceneCommand<SceneId>
Scene::render
Scene::camera
Scene::render_debug_geometry
Scene::debug_snapshot
```

Overlay scenes:

```text
SceneId::GameMenu
SceneId::Pause
SceneId::Cutscene
```

Overlay render order:

```text
base scene camera/render
optional base debug geometry
FieldHud over base scene
overlay scene camera/render
DebugOverlay final pass
```

If changing overlay behavior, inspect `SceneStack::render`, `SceneStack::camera`, `SceneStack::debug_snapshot`, and `is_overlay_scene`.

## Game Module Ownership

```text
crates/game/src/main.rs
  executable boot, args/env launch options, save/codex loading

crates/game/src/save.rs
  SaveData schema, profile/inventory/world/codex/objectives/activity/settings saves, normalization, disk IO

crates/game/src/player.rs
  player movement, animation, collision footprint, side/top-down behavior

crates/game/src/world.rs
  public facade over game world map runtime

crates/game/src/world/map.rs
  runtime map cache, draw order, collisions, walk surfaces, depth sorting, debug geometry

crates/game/src/objectives.rs
  runtime objective menu rows/checkpoints on top of content objective database

crates/game/src/ui/*
  reusable UI rendering/text/localization/layout/menu style primitives

crates/game/src/scenes/*
  scene implementations and scene-local systems
```

## Scene-Local Systems

```text
activity_log.rs
  typed builders for log entries; use instead of hand-building activity log strings

profile_derived.rs
  derives profile state from inventory, scans, objectives, world facts

profile_status.rs
  field-time, stamina/health/environment deltas, status alert throttling

world_runtime.rs
  map transition resolution, save/restore world location, scoped progress/entity keys

quick_items.rs
  quickbar slot mapping and consumable use rules

rewards.rs
  pickup/codex reward mapping to inventory/research

scan_system.rs
  nearby codex target detection, scan progress, scan UI, completion handoff to GameContext

cutscene_scene.rs
  fullscreen/overlay cutscene playback; consumes pending cutscenes from GameContext and marks seen/flags

zone_system.rs
  hazard/prompt/objective zone execution and once-per-map scoped progress

notice_system.rs
  transient in-world notices

field_hud.rs
  gameplay HUD and quickbar hit testing

debug_overlay.rs
  debug text overlay and scene snapshots
```

## Main Scenes

```text
main_menu.rs
  save slots, language settings, new/continue/load, app-level menu

overworld_scene.rs
  top-down world exploration, pickups, scan, zones, transitions to facility/menu/profile/inventory

facility_scene.rs
  side-view facility map, pickup/scan/zones, jump/stamina, transitions

game_menu_scene.rs
  game menu overlay orchestrator only; see docs/GAME_MENU_ARCHITECTURE.md for component split

cutscene_scene.rs
  blocking sequence scene for fades, text panels, waits, flags, and completion scene switches

inventory_scene.rs
  standalone inventory scene; older/larger than game menu inventory helpers

profile_scene.rs
  standalone profile scene and shared profile overview data

pause_scene.rs
  pause overlay
```

## Game Menu Split

Before editing game menu, read `docs/GAME_MENU_ARCHITECTURE.md`.

Current route:

```text
game_menu_scene.rs      overlay scene orchestration, state, input, text upload coordination
game_menu_activity.rs   log rows, activity scroll math, quest/log panel helpers
game_menu_art.rs        nav/action icons and fallback glyphs
game_menu_codex.rs      codex snapshot/text/card/glyph rendering
game_menu_feedback.rs   toast and save/action feedback
game_menu_inventory.rs  inventory capacity/text/module slots/slot rendering
game_menu_map.rs        map page rendering
game_menu_profile.rs    profile/equipment status visual primitives
```

## Content Crate Ownership

```text
assets.rs
  AssetDatabase, AssetDefinition, footprint/anchor/snap metadata

map.rs
  MapDocument, MapLayers, Tile/Object/Entity/Zone instances, transition/unlock/hazard/prompt/objective rules

codex.rs
  CodexDatabase and CodexEntry

cutscenes.rs
  CutsceneDatabase, CutsceneDefinition, CutsceneStep, localized text, completion action

items.rs
  ItemDatabase, item definitions, consumable effects, equipment module checks

objectives.rs
  ObjectiveDatabase and objective/checkpoint text

semantics.rs
  stable ids and aliases for scenes, entities, layers, meters, zones

validation.rs
  map/content validation shared by tests and editor
```

Content crate rule: add schema fields here first, add validation second, adapt game/editor third.

## Editor Ownership

```text
crates/editor/src/main.rs
  eframe app wiring, workspace switching, and many UI flows; large legacy coordinator

app/
  commands, config, maps, model, outliner, state structs

assets/
  draft/import/labels/thumbnails for asset database editing

cutscenes.rs
  Cutscenes workspace UI; edits crates/content/data/cutscenes.ron through content::CutsceneDatabase; author source text only, not per-language translation fields

canvas/
  editing operations and canvas rendering

ui/
  small reusable egui components

terrain.rs
  terrain autotile/rule logic

inspector.rs, panels.rs, dialogs.rs
  editor side panels and modal flows
```

Editor scope rule: this is the Alien Archive Game Editor. The shipped workspaces are Overworld Map and Cutscenes. New authoring surfaces such as dialogues, events, actors, and assets should live under the same game editor shell when they share runtime content. Do not turn it into a generic engine editor; prefer game-readback-compatible RON changes over schema-only UI.

## Change Routing

```text
Need new map/entity/zone field
  content/src/map.rs -> content/src/validation.rs -> editor UI/state -> game world/map adapter -> tests

Need new authored asset metadata
  content/src/assets.rs -> editor asset draft/inspector -> game world/map rendering or collision use

Need new gameplay item/effect
  content/src/items.rs or crates/content/data/items.ron -> game scenes/quick_items/profile_status/rewards -> save/profile tests

Need new scan/codex behavior
  content/src/codex.rs or data/codex -> game/scenes/scan_system.rs -> GameContext::complete_codex_scan -> rewards/profile/objectives if needed

Need new objective behavior
  content/src/objectives.rs -> game/src/objectives.rs -> zone_system or scan/pickup hooks -> menu rows

Need new cutscene/flow sequence
  content/src/cutscenes.rs or crates/content/data/cutscenes.ron -> editor/src/cutscenes.rs for authoring -> GameContext::request_cutscene_once -> SceneCommand::Push(SceneId::Cutscene) -> cutscene_scene.rs

Need translation/localization editing
  Language workspace should own multi-language translation and proofreading for all authored text; do not add parallel English/Chinese text editors inside feature workspaces such as Cutscenes

Need persistent runtime state
  save.rs schema/default/normalize -> GameContext methods -> call request_save -> tests for load/save/normalize

Need field meter/status behavior
  profile_status.rs for runtime deltas/alerts; profile_derived.rs for derived snapshot; do not duplicate meter math in scenes

Need quickbar/consumable behavior
  quick_items.rs first; FieldHud and menu should consume its API

Need world transition/progress key
  world_runtime.rs; do not build scoped map keys ad hoc in scenes

Need UI text/localization
  assets/data/ui/localization.ron plus ui/game_menu_content.rs fallback for game menu text

Need low-level rendering/input/window behavior
  runtime crate
```

## Persistence Rules

Use `GameContext` for runtime mutations that affect save data:

```text
complete_codex_scan
request_cutscene_once / mark_cutscene_seen / mark_cutscene_flag
add_inventory_item
set_inventory_save
use_selected_quickbar_item path
apply zone/objective updates
request_save / save_now / update_save
```

Avoid mutating nested `ctx.save_data.*` directly in scene code unless the enclosing method also handles:

```text
derived profile sync
inventory load meter sync
activity log entry
objective/codex mirrored state
cutscene seen/flag state
save dirty/requested flags
world location update
```

## World/Map Rules

```text
content::MapDocument
  serialized authored map

game::world::map::Map
  runtime cache derived from MapDocument

game::world::World
  scene-facing facade

editor
  writes MapDocument and validates through content::validation
```

Collision/walk-surface/depth-order behavior lives in `crates/game/src/world/map.rs`; do not reimplement scene-local collision rules unless the player movement mode truly requires it.

Scoped progress keys:

```text
entity_progress_key(map_path, entity_id)
zone_progress_key(map_path, zone_id)
```

Use `world_runtime.rs`; do not concatenate keys locally.

## Validation Commands

Always choose the smallest useful check plus broader checks for shared changes.

```powershell
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
```

Useful scoped tests:

```powershell
cargo test -p alien_archive game_menu
cargo test -p alien_archive cutscene
cargo test -p alien_archive scan_system
cargo test -p alien_archive zone_system
cargo test -p alien_archive quick_items
cargo test -p alien_archive_content cutscene
cargo test -p alien_archive_content validation
cargo test -p editor
cargo check -p editor
```

Launch examples:

```powershell
cargo run -p alien_archive
cargo run -p alien_archive -- --scene overworld --map assets/data/maps/overworld_landing_site.ron --spawn player_start
cargo run -p alien_archive -- --scene facility --map assets/data/maps/facility_ruin_01.ron --spawn entry
cargo run -p editor
```

Env launch overrides:

```text
ALIEN_ARCHIVE_SCENE
ALIEN_ARCHIVE_MAP
ALIEN_ARCHIVE_SPAWN
ALIEN_ARCHIVE_SAVE_PATH
```

## Do Not

- Do not convert authored RON flows to JSON unless the user explicitly changes the format.
- Do not make the editor generic; keep Alien Archive semantics visible.
- Do not bypass `content::validation` for map/editor validity.
- Do not put game-specific schema knowledge into `runtime`.
- Do not put renderer/window/input dependencies into `content`.
- Do not mutate save data in multiple unrelated places for one gameplay event.
- Do not hand-roll parsers for RON content when `ron` + typed schemas are available.
- Do not remove fallback behavior for missing optional assets/localization unless replacing it with a tested equivalent.

## Known Large Files / Future Refactor Targets

```text
crates/editor/src/main.rs
  very large coordinator; good future slices: save/autosave flows, validation/diagnostic panels, asset database workflows

crates/game/src/scenes/main_menu.rs
  can split save slot text/state, rendering buttons, feedback toast

crates/game/src/scenes/inventory_scene.rs
  older standalone inventory scene; compare with game_menu_inventory before extracting shared inventory view model

crates/game/src/scenes/game_menu_scene.rs
  remaining candidates: shell/nav/bottom bar, equipment subpage, per-page text upload builders
```

Refactor rule: extract domain seams with tests. Avoid pure file-count reduction that creates pass-through modules with no stable ownership.
