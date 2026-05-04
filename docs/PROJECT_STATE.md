# Alien Archive 项目状态

最后更新：2026-05-04

## 项目目标

`Alien Archive / 星尘档案` 是一款计划长期迭代的 2D 系统型探索游戏。

核心方向：

- 探索地图
- 扫描物体
- 记录数据
- 解锁区域
- 逐步理解外星生态和遗迹文明

长期目标不是做一个一次性 Demo，而是做一个可以持续演化的系统世界。

## 当前阶段

当前处于 V0.1 初始化阶段。

目标是先做出一个可运行、有核心技术骨架的 Demo：

- 原生窗口
- GPU 渲染
- 玩家移动
- 摄像机跟随
- 自定义地图加载
- 后续接扫描、图鉴、门、存档

## 已完成

- 初始化 Rust workspace。
- 建立 `crates/runtime` 和 `crates/game` 分层。
- 使用 `winit` 打开窗口。
- 使用 `wgpu` 渲染矩形。
- 实现基础 `Renderer` trait，目前支持 `draw_rect`。
- 扩展 `Renderer`，支持加载和绘制 PNG 图片。
- 添加启动界面，读取 `assets/images/startup/startup_background.png`。
- 添加基础 Scene 系统和 `SceneCommand`：`Switch` / `Push` / `Pop` / `Quit`。
- 当前场景流：`MainMenuScene -> OverworldScene -> FacilityScene`，并有最小 `PauseScene`。
- `OverworldScene` 是俯视角 2D，使用四方向移动。
- `FacilityScene` 是侧视横版 2D，当前已有左右移动、重力和跳跃。
- 俯视地图里的 `FacilityEntrance` 实体按 `E` 进入设施；设施里的 `FacilityExit` 实体按 `E` 返回俯视地图。
- 主菜单已收敛为当前阶段需要的两个选项：开始游戏、退出。
- 主菜单支持键盘选择、鼠标悬停选择和左键确认。
- 菜单文字会预渲染为纹理，优先使用 `assets/fonts/ui.ttf`，再自动查找 `assets/fonts` 下的 `SourceHan*.ttf` / `Noto*.ttf`，最后才使用系统中文字体。
- 实现基础输入系统，支持 `WASD` 和方向键移动。
- 实现基础 `Camera2d`，摄像机跟随玩家。
- 实现 Demo 玩家矩形。
- 实现自定义地图文件加载。
- 添加第一张地图：`assets/data/maps/demo.ron`。
- 程序可以从 RON 地图加载 tile 和 entity 并渲染。

验证过的命令：

```powershell
cargo fmt --all --check
cargo check -p alien_archive
$env:ALIEN_ARCHIVE_EXIT_AFTER_FRAMES='3'; cargo run -p alien_archive
```

注意：当前 PowerShell 会输出 Conda 启动噪声，但不影响 Rust 编译和运行。

## 技术决策

### 地图不用 LDtk

已决定不使用 LDtk 作为地图方案。

原因：

- 项目是长期作品，核心数据结构应该掌握在自己手里。
- 地图编辑器以后可以做成本地网页工具。
- 运行时不应该依赖第三方地图编辑器的 schema。
- 编码成本相比长期架构锁定风险更低。

当前策略：

```txt
自定义 RON 地图文件
↓
MapFile 反序列化
↓
转换成游戏内部 Map / Tile / Entity
↓
World / Collision / Scanner / Codex 使用内部结构
```

未来本地网页地图编辑器只需要读写 `assets/data/maps` 下的地图文件。

### Runtime 和 Game 分层

`runtime` 负责偏平台和引擎的能力：

- window
- input
- renderer
- camera
- collision
- assets
- audio
- scene

`game` 负责游戏规则和内容：

- player
- world
- scanner
- codex
- save
- doors / relics / ecology

规则：游戏逻辑尽量平台无关，平台能力用 trait 或薄封装隔离。

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

    game/
      src/
        main.rs
        player.rs
        world.rs
        world/
          map.rs

  assets/
    fonts/
    images/
      startup/
    sprites/
    maps/
    audio/
    data/
      maps/
        demo.ron
        overworld.ron
        facility_ruin_01.ron

  tools/
```

## 当前地图格式

第一版地图文件：

```txt
assets/data/maps/demo.ron
```

它包含：

- `tile_size`
- `origin`
- `palette`
- `tiles`
- `entities`

实体目前包括：

- `PlayerSpawn`
- `ScanTarget`
- `Door`
- `Decoration`

`id` 和 `codex_id` 已经预留，之后扫描和图鉴系统会用到。

当前新增实体类型：

- `FacilityEntrance`
- `FacilityExit`

它们用于 `OverworldScene` 和 `FacilityScene` 之间的固定切换。

## 下一步建议

推荐下一阶段按这个顺序做：

1. 加基础碰撞，让玩家不能穿过 solid tile 和 Door。
2. 加扫描系统，靠近 `ScanTarget` 后按 Scan 键读条。
3. 加内存版图鉴，扫描后记录 `codex_id`。
4. 加存档，把已扫描 `codex_id` 和玩家位置写入本地 JSON。
5. 退出重进后读取存档，确认图鉴记录仍然存在。
6. 加一个遗迹门逻辑，扫描指定对象后解锁 Door。
7. 做 Debug Overlay，显示 scene、player position、map、collider count。

## 暂时不要做

为了不偏离 MVP，暂时不要做：

- 通用游戏引擎
- 复杂编辑器
- 重型物理系统
- 大量抽象
- 美术素材管线自动化
- 程序生成星球

当前最重要的是把探索、扫描、图鉴、开门这条核心循环跑通。

## 素材约定

启动/标题背景图放在：

```txt
assets/images/startup/
```

推荐先用：

```txt
assets/images/startup/startup_background.png
```

游戏内角色、物体、tile 等像素素材仍放在 `assets/sprites`；地图数据放在 `assets/data/maps`。

UI 字体优先文件名：

```txt
assets/fonts/ui.ttf
```

当前 UI 字体已统一命名为：

```txt
assets/fonts/ui.ttf
```

原始字体文件名：

```txt
SourceHanSans-VF.ttf
```

如果没有 `ui.ttf`，项目也会自动查找 `assets/fonts` 下的 `SourceHan*.ttf` / `Noto*.ttf`。

推荐字体：

- `Noto Sans SC`
- `Source Han Sans SC / 思源黑体`

这两类现代中文无衬线字体更适合科幻 UI；后续可以把最终选定字体复制为 `assets/fonts/ui.ttf`，让游戏在不同机器上保持一致。
