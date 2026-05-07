# Alien Archive 项目状态

最后更新：2026-05-07

## 项目目标

`Alien Archive / 星尘档案` 是一款计划长期迭代的 2D 系统型探索游戏。

核心方向：

- 探索地图
- 扫描物体
- 记录图鉴数据
- 管理背包和外勤状态
- 解锁区域
- 逐步理解外星生态和遗迹文明

长期目标不是做一次性 Demo，而是做一个可以持续演化的系统世界。

## 当前阶段

当前仍处于 V0.1 初始化阶段，但已经从“只验证窗口/地图/移动”推进到“核心循环雏形”。

现在的可运行骨架包括：

- 原生窗口和 GPU 渲染
- Scene 栈与覆盖层菜单
- 俯视角 Overworld 和横版 Facility
- 自定义 RON 地图加载
- 玩家移动、摄像机跟随、基础碰撞
- 游戏内菜单
- 简单扫描状态机
- 本地 RON 存档和自动保存

还没有完成的核心循环部分：

- 多地图入口/出口目标字段和区域脚本执行
- 更完整的交互面板和日志历史
- 存档选择/新建/删除界面

## 今天更新重点

今天主要推进了几条线：

1. 游戏内菜单

- 新增/重构 `GameMenuScene`，作为覆盖层压在当前游戏场景上，而不是完全切走底层场景。
- `SceneStack` 支持 overlay scene 渲染：底层场景先按自己的 camera 渲染，菜单再用 UI camera 叠上去。
- 菜单分为 `Profile` / `Inventory` / `Codex` / `Map` / `Quests` / `Settings` 六个页签。
- `Esc` 打开/关闭游戏内菜单，`I` / `Tab` 直达背包页，`C` 直达属性页。
- 菜单支持鼠标点击左侧页签、返回按钮、设置里的语言切换。
- 非背包页可以用方向键切换页签；背包页方向键移动选中格子。
- 菜单文字使用中文/英文两套文本，不再把两种语言同时堆在界面里。
- 菜单 UI 拆出了 `game_menu_content`、`layout`、`menu_style`、`menu_widgets`、`text` 等模块，方便继续迭代。
- 菜单皮肤、导航图标、底部动作图标、属性图标、图鉴缩略图集中登记在 `menu_style::TEXTURES`。
- `tools/generate_menu_assets.py` 可以生成当前菜单需要的 AI 风格图标和缩略图。

2. 简单物品/目标扫描

- 新增 `crates/game/src/scenes/scan_system.rs`。
- `ScanState` 管理当前目标、扫描进度、完成提示时间。
- 玩家靠近带 `codex_id` 的地图实体后，按住 `Space` 进行扫描。
- 扫描时间默认是 1.25 秒，也可以从 Codex 条目的 `scan_time` 覆盖。
- 扫描完成后把 `codex_id` 写入 `GameContext::scanned_codex_ids`。
- 新扫描到的 `codex_id` 会写入本地存档，退出重进后 Codex 页仍保持解锁状态。
- 扫描 UI 会读取真实 Codex 条目，显示当前目标标题、分类和记录状态。
- 扫描目标不再只看 `ScanTarget` 类型，而是通过 `World::codex_entities()` 查找所有带 `codex_id` 的实体，因此 Decoration 等物件也可以成为扫描候选。
- Facility 里 `Space` 同时有跳跃含义；当附近有未完成扫描目标时，扫描系统会捕获 `Space`，避免按住扫描时触发跳跃。
- 游戏内 Codex 页已接入 `assets/data/codex/overworld_codex.ron`：未扫描条目显示锁定，扫描后显示标题、分类和描述。
- 首次完成扫描会推进人物档案里的研究进度和 XP，并按 Codex 类型给少量背包奖励。
- 入口/门现在优先读取地图实体上的显式 `unlock` 规则；旧地图里带 `codex_id` 的入口/门仍会兼容为“需要先扫描”。

3. 本地存档

- 新增 `crates/game/src/save.rs`，默认存档路径是 `saves/profile_01.ron`。
- 存档使用 RON，包含 schema version、人物档案、背包槽位、快捷栏、当前地图/场景/出生点/玩家位置、已扫描 Codex ID、已收集实体、语言设置。
- `GameContext` 启动时读取存档；不存在或读取失败时使用默认存档。
- 主菜单“开始游戏”会从存档里的当前场景继续，而不是固定进 Overworld。
- Overworld / Facility 会持续记录玩家位置和当前地图，移动后按 5 秒间隔自动保存；退出时若有未保存状态会补写一次。
- 扫描完成、语言切换、背包选择/分类状态变化会立即请求保存。
- Profile 页和游戏内 Profile 面板读取 `SaveData.profile`；背包页和游戏内 Inventory 面板读取 `SaveData.inventory`。

4. 拾取和奖励

- 新增 `scenes::rewards`，集中把 pickup asset / codex id 映射到背包奖励和研究分类。
- 地图实体现在保留运行时 `asset_id`，用于判断 `ow_pickup_*` 是否可采集。
- 玩家靠近 pickup 实体按 `E` 会把物品写进 `SaveData.inventory`，并把该地图实体写进 `SaveData.world.collected_entities`。
- 已收集实体在重新进入地图时会从运行时 World 移除，避免重复领取。

5. 交互反馈

- 新增 `scenes::notice_system`，用于在游戏画面上方显示轻量提示。
- 拾取成功会显示“获得物品 x 数量”。
- 背包无空位时会显示背包已满。
- 被 `unlock` 锁住的入口/出口会根据条件显示扫描目标、所需物品或自定义锁定提示。
- 首次扫描完成会显示扫描完成提示，提示研究和奖励已经记录。

6. 显式解锁规则

- `content::MapDocument` 的 `EntityInstance` 和 `ZoneInstance` 增加 `unlock` 字段。
- `unlock` 当前支持 `requires_codex_id`、`requires_item_id`、`locked_message`。
- 运行时 `MapEntity` 会携带 `MapUnlockRule`，Overworld 入口和 Facility 出口交互时会检查已扫描 Codex 和背包物品。
- 编辑器 Inspector 已能给实体/区域启用并编辑解锁条件，字段前有明确标签，并提供 Codex ID / 常用 item id 选择。
- Outliner 会给带解锁规则的实体/区域显示 `unlock` badge，并可通过解锁字段搜索对象。
- `validate_map_with_codex` 会检查显式 unlock 的 Codex 引用、空条件和可疑 item id；旧的“入口/门素材 codex_id 隐式锁门”会提示改成显式字段。

## 已完成

- 初始化 Rust workspace。
- Workspace 当前成员：
  - `crates/runtime`
  - `crates/content`
  - `crates/game`
  - `crates/editor`
  - `tools/sprite_sheet_compactor`
- 使用 `winit` 打开窗口。
- 使用 `wgpu` 渲染矩形和 PNG 图片。
- 最小窗口尺寸已调整为 `1280x720`。
- 添加启动界面，读取 `assets/images/startup/startup_background.png`。
- 添加基础 Scene 系统和 `SceneCommand`：`Switch` / `Push` / `Pop` / `Quit`。
- 当前主要场景流：`MainMenuScene -> OverworldScene -> FacilityScene`。
- `GameMenuScene` 和 `PauseScene` 属于覆盖层 scene。
- `OverworldScene` 是俯视角 2D，使用四方向移动。
- `FacilityScene` 是侧视横版 2D，已有左右移动、重力和跳跃。
- 游戏可以通过 `--scene overworld|facility --map <path> --spawn <id>` 直接启动到指定地图；不传参数时仍进入主菜单。
- 游戏默认从 `saves/profile_01.ron` 读取/写入本地存档，也可以用 `ALIEN_ARCHIVE_SAVE_PATH` 指定测试存档路径。
- 俯视地图里的 `FacilityEntrance` 实体按 `E` 进入设施；设施里的 `FacilityExit` 实体按 `E` 返回俯视地图。
- 主菜单支持键盘选择、鼠标悬停选择和左键确认。
- 菜单文字会预渲染为纹理，优先使用 `assets/fonts/ui.ttf`。
- 实现基础输入系统，支持 `WASD`、方向键、鼠标左键、`E`、`Space`、`Esc`、`I` / `Tab`、`C`。
- 实现基础 `Camera2d`，摄像机跟随玩家。
- 实现 Demo 玩家矩形和俯视角 sprite sheet 播放。
- 实现自定义地图文件加载。
- 实现 `SaveData` 本地存档层，保存语言、场景、地图、玩家位置、Codex 解锁、已收集实体、人物档案、背包和快捷栏。
- `MapDocument` 支持实体/区域级 `unlock` 规则，运行时入口/出口可按扫描记录或背包物品放行。
- 运行时 `Map::load` 支持新的编辑器 RON 地图格式，并保留旧版 legacy RON 地图解析。
- `World::solid_rects()` 提供 tile/entity/collision 碰撞矩形。
- `World::codex_entities()` 提供扫描候选实体。
- 运行时启动时加载 `content::CodexDatabase`，供扫描 UI 和游戏内 Codex 菜单共同使用。
- 运行时启动时加载 `SaveData`，用于恢复语言、Codex 解锁、背包、人物档案和上次所在地图位置。
- `crates/editor` 已作为专用 Overworld 地图编辑器入口存在，读写 `assets/data/maps/*.ron`。
- 编辑器菜单和工具栏支持“保存并运行当前地图”，用于快速验证当前 RON 和出生点。

## 当前操作

主菜单：

- `WASD` / 方向键：选择菜单项
- 鼠标悬停：选择菜单项
- `Enter` / `Space` / 鼠标左键：确认
- `Esc`：退出

游戏中：

- `WASD` / 方向键：移动
- `E`：交互入口/出口；靠近 pickup 时收集物品；若入口/出口配置了 `unlock`，会先检查扫描记录或背包物品
- `Space`：扫描；在 Facility 中也可跳跃
- `Esc`：打开/关闭游戏内菜单
- `I` / `Tab`：打开游戏内菜单并切到背包
- `C`：打开游戏内菜单并切到属性
- 存档：移动、扫描、语言切换等会自动保存；当前没有手动存档 UI

游戏内菜单：

- 鼠标点击左侧页签：切换菜单页
- 鼠标点击返回按钮：关闭菜单
- 方向键：切换页签；背包页内移动选中格
- 设置页 `Enter` / `E`：切换语言

## 技术决策

### 地图不用 LDtk

已决定不使用 LDtk 作为地图方案。

原因：

- 项目是长期作品，核心数据结构应该掌握在自己手里。
- 运行时不应该依赖第三方地图编辑器的 schema。
- 编辑器可以围绕 Alien Archive 的真实工作流做窄，而不是做成通用地图编辑器。
- 编码成本相比长期架构锁定风险更低。

当前策略：

```txt
assets/data/maps/*.ron
↓
content::MapDocument 或 legacy map schema
↓
game::world::map::Map
↓
World / Collision / Scan / Codex / Save 使用内部结构
```

### Runtime / Content / Game 分层

`runtime` 负责偏平台和引擎的能力：

- window
- input
- renderer
- camera
- collision
- assets
- audio
- scene

`content` 负责可编辑内容的数据结构：

- asset database
- editor map document
- layer / tile / object / entity / zone / collision schema
- map validation

`game` 负责游戏规则和内容：

- player
- world
- scenes
- menu
- scan
- save
- inventory/profile/codex 的当前雏形
- 未来 doors / relics / ecology

规则：游戏逻辑尽量平台无关，平台能力用 trait 或薄封装隔离。

### Assets 不入库

当前 `.gitignore` 已忽略整个 `/assets` 和本地 `/saves`。

这表示：

- 代码会继续使用稳定路径，例如 `assets/data/maps/...`、`assets/images/ui/...`、`assets/sprites/...`。
- 真实素材目录需要在本机存在，可以是复制出来的本地素材，也可以是指向 OneDrive 素材库的软连接。
- 存档属于本机运行数据，不随代码提交。
- 如果缺少 `assets`，`cargo check` 仍可能通过，但运行游戏、加载菜单纹理、地图读取和部分资源存在性测试会失败。

## 当前目录结构

```txt
Alien-Archive/
  Cargo.toml
  README.md
  docs/
    PROJECT_STATE.md

  crates/
    runtime/
      src/
        app.rs
        input.rs
        camera.rs
        collision.rs
        renderer/

    content/
      src/
        assets.rs
        map.rs
        validation.rs

    game/
      src/
        main.rs
        player.rs
        save.rs
        world.rs
        world/
          map.rs
        scenes/
          main_menu.rs
          overworld_scene.rs
          facility_scene.rs
          game_menu_scene.rs
          scan_system.rs
        ui/
          game_menu_content.rs
          layout.rs
          menu_style.rs
          menu_widgets.rs
          text.rs

    editor/
      src/

  tools/
    generate_menu_assets.py
    sprite_sheet_compactor/

  assets/   # 本地素材目录，不入库；运行时仍依赖这个路径
```

## 当前地图格式

当前运行时支持两类 RON：

1. 新编辑器地图格式

默认地图路径：

```txt
assets/data/maps/overworld_landing_site.ron
```

核心结构来自 `content::MapDocument`：

- `id`
- `mode`
- `tile_size`
- `width`
- `height`
- `layers`
- `spawns`

`layers` 目前包括：

- `ground`
- `decals`
- `objects`
- `entities`
- `zones`
- `collision`

2. 旧版 legacy 地图格式

旧格式仍可读，用于兼容已有 demo/测试地图：

- `tile_size`
- `origin`
- `palette`
- `tiles`
- `entities`

旧版实体目前包括：

- `PlayerSpawn`
- `FacilityEntrance`
- `FacilityExit`
- `ScanTarget`
- `Door`
- `Decoration`

扫描系统不强依赖实体类型，只要实体最终带有 `codex_id`，就可以成为扫描候选。

## 验证建议

常用轻量验证：

```powershell
cargo fmt --all --check
cargo check -p alien_archive
```

直接启动指定地图：

```powershell
cargo run -p alien_archive -- --scene overworld --map assets/data/maps/overworld_landing_site.ron --spawn player_start
```

使用临时存档启动，避免污染默认进度：

```powershell
$env:ALIEN_ARCHIVE_SAVE_PATH='target/test_profile_01.ron'
cargo run -p alien_archive -- --scene overworld --map assets/data/maps/overworld_landing_site.ron --spawn player_start
Remove-Item Env:\ALIEN_ARCHIVE_SAVE_PATH
```

资源齐全时再跑：

```powershell
cargo test --workspace
$env:ALIEN_ARCHIVE_EXIT_AFTER_FRAMES='3'; cargo run -p alien_archive
```

注意：

- 当前 PowerShell 会输出 Conda 启动噪声，通常不影响 Rust 编译和运行。
- 如果本机没有 `assets`，菜单纹理、背包图标、地图文件和资源存在性测试会失败。

## 下一步建议

推荐下一阶段按这个顺序做：

1. 给主菜单增加新建/继续/删除存档入口。
2. 给门/区域解锁增加更明确的数据字段，而不是只依赖 `codex_id`。
3. 增加更多人物状态变化来源，例如受伤、体力消耗、抗性变化。
4. 做 Debug Overlay，显示 scene、player position、map、collider count、scan target、save path/save dirty。
5. 做交互历史/日志页，保留最近获得物品、解锁和扫描结果。

地图编辑器的长期改进清单单独记录在：

```txt
docs/EDITOR_ROADMAP.md
```

## 暂时不要做

为了不偏离 MVP，暂时不要做：

- 通用游戏引擎
- 重型物理系统
- 过早的网络/联机
- 大量抽象
- 程序生成星球
- 与当前核心循环无关的大地图内容膨胀

当前最重要的是把探索、扫描、图鉴、背包、开门这条核心循环跑通。

## 素材约定

启动/标题背景图放在：

```txt
assets/images/startup/
```

推荐先用：

```txt
assets/images/startup/startup_background.png
```

UI 字体优先文件名：

```txt
assets/fonts/ui.ttf
```

如果没有 `ui.ttf`，项目也会自动查找 `assets/fonts` 下的 `SourceHan*.ttf` / `Noto*.ttf`，最后才使用系统中文字体。

游戏内菜单素材目前集中在：

```txt
assets/images/ui/menu/
assets/images/ui/profile/
assets/images/ui/inventory/
```

菜单图标/缩略图可以用下面的脚本生成：

```powershell
python tools/generate_menu_assets.py
```

游戏内角色、物体、tile 等像素素材仍放在：

```txt
assets/sprites/
```

地图数据放在：

```txt
assets/data/maps/
```

图鉴数据放在：

```txt
assets/data/codex/
```

本地存档放在：

```txt
saves/profile_01.ron
```

`saves/` 不入库。测试或编辑器预览可以通过 `ALIEN_ARCHIVE_SAVE_PATH` 改用临时存档。

俯视角玩家素材路径：

```txt
assets/sprites/player/topdown/
```

当前俯视角玩家文件：

```txt
idle_down.png
walk_down.png
walk_left.png
walk_right.png
walk_up.png
```

这些文件按横向 sprite sheet 读取：

```txt
512x128 = 4 frames * 128x128
```

运行时代码只绘制当前帧，不会把整张 sheet 拉伸成一个角色。俯视角移动时按方向选择 `walk_down` / `walk_left` / `walk_right` / `walk_up`，停止时使用 `idle_down`。
