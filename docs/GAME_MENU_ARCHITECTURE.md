# Game Menu Architecture

这份文档记录游戏内菜单这轮拆分后的边界。目标不是把 `GameMenuScene` 拆到最碎，而是让后续新增页面、改文案、换图标、接真实数据时，有稳定的位置可放。

## 当前边界

`crates/game/src/scenes/game_menu_scene.rs` 仍然是覆盖层 scene 的编排者，负责：

- 和 `SceneStack` / `GameContext` 对接。
- 维护当前页签、底部子页、选中格子、滚动位置和 toast 状态。
- 同步存档快照、目标快照、Codex 快照，并在数据变化时刷新菜单文字纹理。
- 处理输入：页签切换、背包格子选择、Codex 选择、日志滚动、保存动作、语言切换。
- 组织页面布局，把具体页面组件委托给 `game_menu_*` 模块。

页面与组件模块的职责：

- `game_menu_activity.rs`：外勤日志与任务页里的活动记录，包含行文本上传、日志滚动、可见范围和滚动条绘制。
- `game_menu_art.rs`：菜单导航图标、底部动作图标和无贴图 fallback 绘制。
- `game_menu_codex.rs`：Codex 快照、摘要统计、条目文本上传、Codex 卡片和 glyph 绘制。
- `game_menu_feedback.rs`：菜单 toast、保存成功/失败文案、未接入动作提示。
- `game_menu_inventory.rs`：库存容量、库存格子数量/详情文本、装备模块筛选和库存格子绘制。
- `game_menu_map.rs`：地图页背景、网格、路线点和标签绘制。
- `game_menu_profile.rs`：档案页和装备状态页共享的 XP 比例、属性图标、状态卡、状态行和紧凑进度条。

共享 UI 基础设施仍放在 `crates/game/src/ui/`：

- `game_menu_content.rs`：菜单页签、标题、固定文案和本地化入口。
- `layout.rs`：通用布局工具。
- `menu_style.rs`：菜单颜色、尺寸、贴图登记、slot rect 等样式与几何常量。
- `menu_widgets.rs`：底层绘制积木，例如面板、边框、进度条、九宫格贴图。
- `text.rs`：字体加载、文字纹理上传和文字绘制。

## 新功能放置规则

新增或修改菜单功能时，优先按这几条走：

- 只影响某个页面的纯展示、纯计算或文本上传：放进对应 `game_menu_*` 模块。
- 影响当前页签、输入分发、save/load、语言切换、toast 生命周期：留在 `game_menu_scene.rs`。
- 多个菜单页面共享的绘制原语：先看是否属于现有领域模块，例如库存格子放 `game_menu_inventory.rs`，人物状态放 `game_menu_profile.rs`；只有真正跨领域的低层 widget 才放 `ui/menu_widgets.rs`。
- 文字 key、页面标题、固定标签：放 `ui/game_menu_content.rs`，不要散在 scene 里。
- 新图标或菜单贴图：先登记到 `ui/menu_style.rs::TEXTURES`，再在页面模块里引用 texture id。
- 能用小型纯函数测试锁住的计算边界，随模块补单测；纯绘制函数至少测试它依赖的映射、几何或 fallback id。

## 现在不继续硬拆的部分

`game_menu_scene.rs` 里还可以继续拆，但收益已经开始下降。剩余代码主要集中在三类：

- Shell / nav / bottom bar 绘制：可以拆成 `game_menu_shell.rs`，但它强依赖 `GameMenuText` 和整体布局，先保留在 scene 里更直观。
- `upload_textures`：可以拆成一个文本上传 builder，不过现在它直接填充 `GameMenuText`，强行抽会引入一层没有太多行为的结构。
- 输入处理：可以拆成 reducer 风格的 `game_menu_input.rs`，但它会直接修改 scene 状态并调用 `GameContext` 保存/切页，拆错会让控制流更绕。

后续如果菜单继续增长，比较合理的下一步是：

- 当底部装备页继续扩展时，把装备页绘制和输入从 `GameMenuScene` 拆成 `game_menu_equipment.rs`。
- 当 `Map` 页接入真实世界数据时，把 `game_menu_map.rs` 从静态路线升级为接收 `WorldMapMenuSnapshot`。
- 当 `upload_textures` 继续变长时，把每个页面的文本上传函数移到对应模块，让 `GameMenuScene` 只组合返回值。

## 验证要求

改菜单结构后至少跑：

```powershell
cargo check --workspace
cargo test -p alien_archive game_menu
```

如果改了共享 UI、场景栈、存档同步或输入处理，跑完整：

```powershell
cargo test --workspace
```
