# Alien Archive 地图编辑器改进路线

最后更新：2026-05-09

本文只讨论 `Alien Archive` 专用 Overworld 地图编辑器。当前方向仍然是：

- 编辑器代码放在 `crates/editor`
- 资源索引放在 `assets/data/assets/overworld_assets.ron`
- 地图保存为 `assets/data/maps/*.ron`
- 运行时代码直接读取编辑器产出的 RON
- 不把它扩成通用地图编辑器、通用引擎或 JSON 管线

## 对照对象

主流 2D 游戏编辑器的共同点不是“功能越多越好”，而是把重复劳动和容易填错的字段尽量前置成工具能力：

- Tiled：Layer / Object Layer / Group Layer、对象模板、Custom Properties、Terrain Brush、Automapping。
- LDtk：强类型 Entity、Custom Fields、Enum、IntGrid、Auto layer、World/Level 组织。
- Godot：TileMap / TileSet、Terrain、Tile 自定义数据、碰撞/导航/遮挡形状、批量属性绘制。
- Unity：Tile Palette、Active Target、Brush / Box Fill / Flood Fill / Picker / Eraser、Brush Preview、Tilemap Focus。

这些不是要照抄，而是提醒我们：编辑器后续应该优先减少手动填字段、手动对齐、手动检查、手动切游戏验证。

参考资料：

- Tiled layers: https://docs.mapeditor.org/en/stable/manual/layers/
- Tiled custom properties: https://docs.mapeditor.org/en/stable/manual/custom-properties/
- Tiled templates: https://docs.mapeditor.org/en/stable/manual/using-templates/
- Tiled automapping: https://docs.mapeditor.org/en/stable/manual/automapping/
- Tiled terrains: https://docs.mapeditor.org/en/stable/manual/terrain/
- LDtk layers: https://ldtk.io/docs/general/editor-components/layers/
- LDtk entities: https://ldtk.io/docs/general/editor-components/entities/
- LDtk enums: https://ldtk.io/docs/general/editor-components/enumerations-enums/
- LDtk auto layers: https://ldtk.io/docs/general/auto-layers/
- LDtk world: https://ldtk.io/docs/general/world/
- Godot TileMaps: https://docs.godotengine.org/en/stable/tutorials/2d/using_tilemaps.html
- Godot TileSets: https://docs.godotengine.org/en/stable/tutorials/2d/using_tilesets.html
- Unity Tilemap painting: https://docs.unity.cn/Manual/Tilemap-Painting.html
- Unity Tilemap component: https://docs.unity.cn/Manual/class-Tilemap.html

## 当前已有能力

从现有代码看，编辑器已经具备一套可用的专用生产管线：

- 文件工作流：新建、打开、保存、另存、删除、还原、最近地图、自动保存。
- 地图格式：`MapDocument` 包含 `Ground` / `Decals` / `Objects` / `Entities` / `Zones` / `Collision` 六类层。
- 资产格式：`AssetDefinition` 已有 `kind`、`default_layer`、`default_size`、`footprint`、`anchor`、`snap`、`tags`、`entity_type`、`codex_id`。
- 画布工具：选择、画笔、油漆桶、矩形、橡皮、吸管、碰撞、区域、平移、缩放。
- 编辑能力：撤销/重做、复制/粘贴、duplicate、拖拽移动、框选、多选、翻转、旋转、重置、z-index。
- 资源面板：扫描 overworld PNG、导入未登记资产、编辑资产草稿、保存资产数据库、显示缩略图。
- 层面板：按层显示/隐藏、锁定、选择当前层。
- Inspector：地图属性、资产属性、单选实例属性、多选属性、碰撞/实体/区域相关属性。
- 验证：能检查空地图 id、无效尺寸、越界、重复 id、未知资产、资产类型/默认层不匹配、实体类型为空、scale 非法、区域点数过少等。
- UI 组件层：`crates/editor/src/ui` 是 Alien Archive Editor 自己的组件层。`egui` 只作为底层 immediate-mode GUI，长期不要在业务面板里反复手写 tabs、toolbar button、property row 等常见控件。
- 面板拆分：左侧 `资源库 / 图层 / 对象` 面板已经从 `main.rs` 移到 `crates/editor/src/panels.rs`，右侧 Inspector 已移到 `crates/editor/src/inspector.rs`，弹窗和素材草稿编辑已移到 `crates/editor/src/dialogs.rs`。
- Canvas 拆分：画布绘制、输入处理、hit test、工具操作和坐标转换已经从 `main.rs` 移到 `crates/editor/src/canvas/editor.rs`，纯绘制 helper 保留在 `crates/editor/src/canvas/rendering.rs`。Canvas 绘制已引入可见区域裁剪：Ground/Collision 按当前 tile bounds 跳过视口外单元，Decals/Objects/Entities/Zones/Entity Bounds 按 screen rect 跳过视口外实例，对象和实体会先裁剪再排序。下一步在 canvas 内继续拆 `viewport / input / hit_test / tools / overlays`。

这个基础已经足够继续做内容，但如果进入长期地图生产，下面这些缺口会很快变成效率和质量问题。

## 主要缺口

### P0：建立 Editor UI 组件库

编辑器会长期迭代，不能每个面板都直接拼裸 `egui` 控件。约定如下：

- `crates/editor/src/ui` 是专用设计系统层，不追求通用 GUI 框架，只服务 Alien Archive Editor。
- 业务代码优先调用 `ui/*` 组件；只有临时验证或一次性布局才直接写裸 `egui::Button` / `selectable_value` / 自绘 painter。
- 已有 `ui/tabs.rs`，用于 editor 风格 tab bar。左侧栏的 `资源库 / 图层 / 对象` 已使用它。
- 已有 `ui/buttons.rs`、`ui/header.rs`、`ui/fields.rs`、`ui/search.rs`、`ui/tree.rs`、`ui/sections.rs`，分别承担紧凑按钮、面板标题、属性行、搜索框、对象列表行和 Inspector 分段。
- 已有 `ui/asset_grid.rs`、`ui/badge.rs`、`ui/layer_row.rs`，资源缩略图/资源行、状态标签和图层行不再散落在业务面板里手写。
- 已有 `ui/filter_bar.rs` 和 `ui/side_rail.rs`，资源类型筛选和折叠侧栏也走组件层。
- 后续新增常见组件时先进入 `ui/`，例如更完整的 `asset_grid` 拖拽/多选变体、`panel_surface`、`splitter`。
- 视觉风格集中走 `ui/theme.rs`，不要在业务代码里散落新的颜色常量。
- 每次新增 UI 组件，要在 `crates/editor/src/ui/README.md` 里记录用途和使用约定。

### P0：先补游戏语义和防错

1. 扫描目标可视化

当前游戏里“能不能扫描”依赖实体最终是否带 `codex_id`。编辑器需要在资源面板和地图实例上直接显示扫描状态：

- 资产是否有 `codex_id`
- 放到地图上后是否会成为扫描候选
- 是否已经有对应图鉴条目
- 扫描半径/交互矩形大概覆盖哪里
- 当前选中实体在运行时会显示什么名称

建议做成 Inspector 里的专门区块，而不是只显示一行文本。

2. `codex_id` / `entity_type` 从自由输入升级为可选字段

现在字段已经存在，但手动输入很容易拼错。参考 Tiled/LDtk 的 typed properties 和 enums，应当增加项目内枚举/注册表：

- `entity_type` 下拉选择，仍允许高级手动输入。
- `codex_id` 下拉选择，来源可以先从 `assets/data/codex` 或现有代码常量扫描。
- 选择 `codex_id` 后自动提示名称、分类、是否存在图鉴数据。
- 资产草稿和已放置实体都用同一套字段编辑器。

3. 地图验证补齐游戏规则

现有 `validate_map` 主要验证结构正确。下一步要验证“这张图在游戏里是否合理”：

- 带 `codex_id` 的实体必须有非空 `entity_type`。
- 可扫描实体应该有交互范围，或者能从默认尺寸推导合理范围。
- `codex_id` 必须能在图鉴数据中找到。
- 同一地图中重复 `codex_id` 是允许、警告还是错误，需要明确规则。
- player spawn 不能落在 collision 上。
- spawn 附近最好有最小可走空间。
- 出入口/传送区域必须指向存在的地图和 spawn。
- Zone 的 `zone_type` 应该来自固定列表。
- Entity / Zone 的 `unlock` 规则必须引用存在的 Codex 条目；空条件和可疑 item id 要提示。

4. 属性面板继续补标签和字段分组

输入框前面必须有属性名，这是已经暴露出来的 UX 问题。后续 Inspector 要按语义分组：

- Identity：id、asset、type
- Transform：x/y、scale、rotation、flip、z-index
- Gameplay：codex id、entity type、zone type、spawn id、unlock 条件
- Collision/Interaction：rect offset、size、solid
- Asset defaults：anchor、snap、footprint、default layer

每个字段都应该能看出单位：tile、pixel、world cell、ratio。

### P1：提高铺图和摆物效率

5. Terrain / Autotile / Rule Brush

Tiled、Godot、LDtk 都把“根据邻居自动选择边角 tile”作为核心能力。我们现在有画笔和矩形，但还缺少：

- 地形集定义：sand、grass、road、wall 等。
- 边/角匹配规则：根据邻接格自动换成 corner/edge/center。
- 变化权重：同类 tile 随机挑选，避免大面积重复。
- 绘制时修正邻居。
- 一键重算当前可见区域或整张地图。

这个功能优先级很高，因为 Overworld 地表手铺会很快变成苦工。

6. Prefab / Template

Tiled 的模板和 Godot 的 Scene tile 都说明一个问题：常用组合不应该每次重搭。

建议先做轻量模板：

- 从选中对象保存为模板。
- 模板包含 asset、transform、entity_type、codex_id、collision_rect、interaction_rect。
- 模板包含 unlock 规则，避免常见门/区域每次重新配置。
- 放置模板时自动生成唯一实例 id。
- 模板更新是否影响已放置实例，先不做继承也可以，第一版可以只是“一键生成配置完整的对象”。

7. Brush Preview 和 Placement Preview

Unity Tile Palette 有光标预览。我们也需要：

- 画笔悬停时显示即将放置的 tile/object ghost。
- 多格 footprint 显示占地范围。
- 当前层锁定或 asset kind 不匹配时用警告色。
- 放置会越界时给出明显预警。

8. 多格 Stamp / Pattern

当前 ground 已支持一个实例占多格，但还缺少“选一片再盖章”的高效流程：

- 从地图框选一片 ground/object 保存为 pattern。
- 从 tileset/地图中拾取多格 stamp。
- 支持旋转/翻转 stamp。
- 支持随机 variation stamp。

9. 对齐和批量操作

对象多了以后，只能手动拖很痛苦。需要：

- 对齐到左/右/上/下/中心。
- 均匀分布。
- 批量修改 z-index。
- 批量替换 asset。
- 批量设置 tags、codex_id、entity_type。
- 方向键微调，Shift/Ctrl 改变步长。

### P1：提升地图管理和导航

10. Outliner / 搜索

主流编辑器都有层级或对象列表。我们现在有 Layer panel，但缺对象级列表：

- 按层列出所有 objects/entities/zones/spawns。
- 搜索 id、asset、tag、entity_type、codex_id。
- 点击列表项直接定位并选中。
- 显示重复 id、未知资产、可扫描状态、缺字段状态。

11. 小地图 / Bookmark

大地图编辑时，平移缩放不足够：

- 小地图显示当前视口。
- bookmark 保存常用区域。
- 双击 zone/entity 跳转。
- 记录最近编辑位置。

12. World / 多地图关系

LDtk 的 World 功能提醒我们：后面不只会有一张地图。

Alien Archive 不一定需要完整世界编辑器，但至少需要：

- 地图列表视图。
- 地图间入口/出口关系检查。
- 从 Zone 或 Door 跳转到目标地图。
- 显示当前地图依赖的资产数量和缺失项。

### P2：增强资源和运行时联动

13. 资产依赖检查

现在能扫描未登记 PNG，但还可以继续做：

- 资源文件丢失检测。
- 资产数据库里有但地图未使用的资产。
- PNG 存在但未登记的资产。
- asset id 与文件名不一致提醒。
- OneDrive/symlink 场景下显示资产根目录是否正常。

14. TileSet 属性绘制

Godot 的 TileSet property painting 对我们很有参考价值：

- 批量给 tile 设置 footprint。
- 批量给 tile 设置 collision 默认值。
- 批量给 object/entity 设置 anchor、snap、default_size。
- 批量给可扫描物设置 `codex_id` 前缀或分类 tag。

15. 一键运行/预览

编辑器需要能更快验证地图：

- 保存并启动游戏到当前地图。
- 传入 spawn id。
- 可选打开 debug overlay。
- 游戏启动后显示当前地图是否由编辑器 RON 加载。

这比“保存、切终端、cargo run、走过去看”省太多时间。

16. Autosave 恢复界面

已有 autosave，但需要让恢复过程更直观：

- 启动时检测 autosave 比主文件新。
- 显示主文件时间、autosave 时间、差异概要。
- 一键恢复/丢弃。

17. 快捷键和工具提示

工具栏已有快捷键，但后续应系统化：

- 每个图标 hover 显示工具名和快捷键。
- 菜单里列出所有编辑器快捷键。
- 冲突快捷键检测。
- 输入框聚焦时不要触发全局工具切换。

18. 性能和大地图

地图变大后可能需要：

- 只绘制可见区域。
- 缩略图缓存。
- 大量对象的空间索引。
- 验证按需增量运行。
- undo 历史限制和压缩。

## 推荐实现顺序

第一轮应该围绕“扫描和图鉴内容能稳定生产”：

1. Inspector 字段彻底补标签、单位、分组。
2. 资源面板和实例 Inspector 增加扫描状态。
3. `codex_id` / `entity_type` 改成下拉选择加手动兜底。
4. `validate_map` 增加扫描、图鉴、spawn、zone 类型检查。
5. 增加对象 Outliner 和搜索，方便找到所有扫描点。
6. 增加一键保存并运行当前地图。

第二轮围绕“铺地图不痛苦”：

1. Brush Preview。
2. Terrain / Autotile 第一版。
3. Variation 权重和随机绘制。
4. Pattern / Stamp。
5. 批量对齐、替换、设置字段。

第三轮围绕“长期维护多地图内容”：

1. 地图列表和地图关系检查。
2. 入口/出口跳转。
3. 资产依赖报告。
4. Autosave 恢复界面。
5. 大地图性能优化。

## 素材生成和裁切规则

后续继续用 AI 生成 Overworld 素材时，必须保留可复现的源图到素材库流程，不能只手工修几张最终 PNG：

- 生成源图统一保存在 `assets/sprites/_sources/overworld/`，文件名带 pack 名和日期。
- 最终 PNG 进入对应分类的 `assets/sprites/<category>/overworld/generated/`。
- 每个 pack 保留 `<pack>_manifest.txt` 和 `<pack>_preview.png`，方便回查裁切结果。
- 新素材必须登记进 `assets/data/assets/overworld_assets.ron`，并同步 `default_size` 到真实 PNG 尺寸。

裁切时不要盲信整张图的固定等分网格。源图先做布局检查，再决定每一行/每一组的格子数量；如果某一行数量不同，要按行单独切，例如 Landing Site 贴花源图是 `6/6/6/7`，不是四行都按 6 列。

宽道具、高交互物和容易跨格的素材，格子只负责定位大概归属，真正裁切要基于源图里非背景像素的联通区域边界：

- 先用粗格子把素材分配到目标 id。
- 对目标格里的非背景联通区域取整体 bbox，并加少量 padding。
- 对 alpha bbox 再收紧一次，避免留下无意义透明边。
- 按分类目标高度和最大宽度缩放；当前 Landing Site 验证值为：`props` 高 `72`、最大宽 `132`；`interactables` 高 `96`、最大宽 `132`；`decals` 高 `64`、最大宽 `132`。
- chroma key 绿底要全图清理，不只 flood-fill 外部背景；物体内部孔洞也可能残留绿色。

物件类素材必须保持“干净落地”，不要把地面附件烘进主体图里：

- 植物不要自带根部砾石、土堆、草皮底座。
- 箱子、设备、房屋、模块舱、遗迹主体不要自带底部石头、碎砖、沙土垫、废料裙边。
- 这些压地感和环境融合效果统一通过 Decal、Terrain transition 或 Stamp 叠加完成。
- 这样碰撞框、交互框和遮挡范围可以只围绕主体，不会被美术底座迫使扩大。

裁切后必须做这些检查：

- manifest 条数、缺失路径、重复 id、生成 pack id 数量和分类计数。
- 重新生成 pack preview，并肉眼检查偏移、截断、残留绿底和尺寸异常。
- `overworld_assets.ron` 中的 `default_size` 与 PNG 实际尺寸一致。
- 需要时启动 `target/debug/editor.exe`，确认 `target/editor-startup.stderr.log` 为空。

地面底材优先做真实 `32x32` 单格 tile，不要把 128x128 或 4x4 footprint 的大图当基础地面拉伸使用。中心块需要有可读的像素纹理（砂粒、风纹、裂缝、矿点、遗迹槽线），但边缘不能有高亮外框或阴影；明显焦痕、晶体、道路边缘和大地貌分界应通过 transition tile、decals 或 Stamp 叠加完成。

## 实施记录

### 2026-05-10

- 新增第二轮 Landing Site Expansion Pack：生成、裁切并登记 100 个 Overworld 素材，源图、裁切脚本、manifest 和 preview 位于 `assets/sprites/_sources/overworld/landing_site_expansion_2026_05_10_*`。
- 本轮分类补量：structures 9、ruins 9、props 17、interactables 10、flora 14、fauna 6、pickups 5、decals 30；素材已落到各自 `assets/sprites/<category>/overworld/generated/` 目录，并登记进 `assets/data/assets/overworld_assets.ron`。
- 本轮按“干净落地”规则处理物件：结构、遗迹、箱子、设备、植物和生物不烘底部石头/土堆/碎料底座；需要压地和环境融合的内容单独放到 decal，例如 root soil、contact dust、rubble scatter、burn smear、coolant puddle、crawler tracks。
- 裁切流程增加边界碎片过滤：格子边缘的小联通块会被丢弃，避免相邻 cell 的残片混入宽物件或高物件；chroma-key 处理也加了边缘去绿和 despill。
- 追加 Terrain Landforms Pack：新增 `terrain` 分类 20 个悬崖/山脉/岩脊/台地/洞口主体素材，另补 20 个 cliff/mountain 支撑贴花；源图、裁切脚本、manifest 和 preview 位于 `assets/sprites/_sources/overworld/terrain_landforms_2026_05_10_*`。主体作为 Object 放置，山脚碎石、坡脚尘土、岩屑和矿脉散布仍拆到 Decal。
- Overworld 地形新增第一版“可走表面”语义：Zone 支持 `WalkSurface` + `surface` 参数，人物脚底进入该多边形后会用 `surface.z_index` / `surface.depth_offset` 临时参与 Y 深度排序。悬崖、斜坡、台地仍应由 Object 主体负责遮挡；坡面/台面用 WalkSurface Zone 标出，碰撞只画不可通行的岩壁/边缘，这样人物既能绕到地形背后，也能在坡面或台面上方显示。

### 2026-05-09

- 素材生产转入第一轮 Landing Site Starter Pack：生成并裁切 85 个 Overworld 可用素材，覆盖 structures、ruins、props、decals、flora、fauna、interactables 和 pickups。
- 生成源图和裁切清单保存在 `assets/sprites/_sources/overworld/landing_site_pack_2026_05_09_*`；实际素材落在各分类的 `overworld/generated` 目录，并已登记进 `assets/data/assets/overworld_assets.ron`。
- 已修正 Landing Site 生成素材裁切：贴花按 6/6/6/7 行布局重切，宽道具和高交互物改用源图联通区域边界裁切，并清掉内部残留 chroma key 绿底。
- 曾试做 Coherent Terrain Pack 和 Seamless Terrain Pack：前者块边界太明显，后者纹理过平；两批 `ct_*` / `st_*` 4x4 地面素材已被后续 Terrain32 Pack 清理替换。
- 4x4 地面素材实测过平，已清理旧 `tiles` 素材并重建 Terrain32 Pack：`assets/sprites/_sources/overworld/terrain32_pack_2026_05_09_*`；实际 tile 统一为 `assets/sprites/tiles/overworld/generated/ow_tile_32_*`，`default_size: (32,32)`、`footprint: Some((1,1))`。这包覆盖 sand、rock、scorch、ruin 四组，每组 12 个 center、1 个 `edge_n`、1 个 `corner_ne`，并补齐四组材料两两之间的有向 edge/corner transition。
- 当前素材库计数变为：tiles 152、decals 38、props 34、structures 19、ruins 15、flora 18、fauna 10、interactables 11、pickups 14。当前 landing map 的 ground 层已从 117 个 4x4 实例展开为 1872 个 1x1 地块，starter map 已补成 1728 个 1x1 地块；后续地图生产基于 32x32 tile 和 transition tile 继续细化。
- Editor 的“自动接边”已从同族 edge/corner 扩展到跨材质 directed transition：基础族优先读 `material:*`，如 sand 邻接 rock 会优先选 `sand_to_rock_edge_*` / `sand_to_rock_corner_*`，找不到 transition 时才回退到本族 edge/corner 或 center。地表工具栏同时补了“重算可见 / 重算全图”，方便整理旧地图或手动修补后的 transition。

### 2026-05-08

- 已开始 P1 第二轮：画布 Brush Preview 从只显示地表 footprint 框，升级为通用放置预览。
- 地表画笔现在会显示选中素材 ghost 和实际落地 footprint；靠近地图边界时会显示裁切后的尺寸，并用预警色提示。
- 贴花、物件、实体图层现在会在鼠标悬停处显示素材 ghost、锚点格和多格 footprint 辅助框。
- 碰撞画笔和橡皮工具也会显示当前影响范围；锁定图层时直接用错误色提示，不再等点击后才知道不能编辑。
- 预览会提示素材类型/默认层与当前图层不匹配、图像超出地图边界等摆放风险，减少保存后再跑验证才发现的问题。
- Terrain / Autotile 第一版已接入：地表工具栏新增“自动接边”，刷地、矩形填充、地表擦除和油漆桶填充后会重算受影响区域周围的同族地形。
- 第一版规则由素材 id/tag 推断 terrain family 与 center/edge/corner 变体；如果只登记了一个方向的 edge/corner，会用旋转复用该变体；没有变体时安全回退到 center，不会破坏当前素材库。
- 已生成并裁切一组临时地表素材：`assets/sprites/tiles/overworld/generated` 下新增 88 张 128x128 tile，素材数据库里统一归到“地块”分类，覆盖 sand、rock、ruin、scorch 和 transition 五组；原始大图保留为 `terrain_sheet_2026_05_08.png` 方便以后重裁。
- 自动接边规则已调整为保留当前 center 变体；同一 terrain family 下多个中心 tile 不会在重算时被折回第一个素材。
- 普通地表画笔恢复为严格放置当前选中素材；随机变体不再默认介入刷地流程，后续如果需要应作为明确的 Random Brush / Stamp 模式单独实现。
- Pattern / Stamp 第一版已接入：新增“盖章”工具，拖拽地图区域可捕获可见的地表、贴花、物件和实体为临时 Stamp；已有 Stamp 时点击地图会按相对坐标粘贴，地表保留原 tile / size / flip / rotation，不再经过随机变体。
- Stamp 工具栏支持“从选择”生成临时 Stamp 和“清空”；第一版只做当前会话内临时 Stamp，下一步再补保存到预设库、旋转/翻转 Stamp、随机 Stamp 变体。

### 2026-05-07

- 已开始 P0 第一轮：资源列表会标记 `scan` / `codex`，素材 Inspector、资产草稿和实体 Inspector 会显示当前扫描状态。
- `entity_type` 和 `Codex ID` 增加常用值选择，来源于现有素材库和当前地图实体。
- `validate_map` 增加扫描相关规则：Object 层的 Codex 资产不会被运行时扫描、可扫描实体缺少 `interaction_rect` 时提醒、重复 `codex_id` 提醒。
- `validate_map` 增加地图生产防错：出生点压在 solid collision 上报错，Zone 类型为空报错，未知 Zone 类型提醒。
- 已补对象 Outliner：按 Spawns / Entities / Objects / Decals / Zones / Ground 分组，支持按 id、asset、entity_type、codex_id、tag 和状态 badge 搜索。
- Outliner 点击对象会同步选中并把画布定位到对象附近；扫描候选、缺交互范围、重复 Codex、缺素材、出生点压碰撞等状态会直接显示在列表 badge 里。
- 已新增 `content::CodexDatabase` / `CodexEntry`，默认读取 `assets/data/codex/overworld_codex.ron`。
- 编辑器的 Codex ID 下拉会合并真实 Codex 数据库和素材库中已有的 Codex ID；素材/实体 Inspector 会显示真实图鉴标题、分类、正文状态和 scan time。
- `validate_map_with_codex` 会在 Codex 数据库加载后检查 `codex_id` 是否存在，并提醒空标题、空分类、空正文。
- 已在本机素材库里建立 starter Codex RON；注意 `assets/` 被 git 忽略且当前是 OneDrive 软链接，真实内容随素材库同步。
- 游戏运行时支持 `--scene overworld|facility --map <path> --spawn <id>`，也支持 `ALIEN_ARCHIVE_SCENE` / `ALIEN_ARCHIVE_MAP` / `ALIEN_ARCHIVE_SPAWN` 环境变量；不传参数时仍进入主菜单。
- 编辑器已增加“保存并运行当前地图”：保存校验通过后，用当前地图 RON 和第一个出生点直接启动游戏预览。
- 游戏启动时会加载 `assets/data/codex/overworld_codex.ron`，扫描进度条会显示真实 Codex 标题、分类和记录状态。
- 游戏内 Codex 页已接入真实 Codex 数据库和 `scanned_codex_ids`：未扫描条目保持锁定，扫描完成后显示标题、分类和描述。
- 运行时已新增本地 RON 存档：保存当前 scene/map/spawn/player position、已扫描 Codex、语言、人物档案和背包状态，方便编辑器一键运行后验证扫描结果是否能跨会话保留。
- 扫描结果已经开始影响游戏规则：扫描首次完成会给 XP、研究进度和少量物品奖励；`ow_pickup_*` 实体可通过 `E` 收集并写入存档。
- 游戏侧已新增交互反馈 notice：拾取、背包已满、门需要扫描/物品、扫描奖励记录都会有屏幕提示。
- 已新增显式解锁规则：`EntityInstance` / `ZoneInstance` 可保存 `unlock.requires_codex_id`、`unlock.requires_item_id`、`unlock.locked_message`；旧地图里入口/门素材上的 `codex_id` 仍会被运行时兼容为扫描门禁。
- 编辑器 Inspector 已支持实体/区域的 `unlock` 配置，Codex ID 和常用 item id 有下拉选择，Outliner 显示 `unlock` badge 并支持搜索。
- `validate_map_with_codex` 会检查 unlock 的 Codex 引用、空条件和 item id 空白字符；入口/门仍依赖素材 `codex_id` 的旧写法会提示迁移到显式 unlock。

## 非目标

短期不要做：

- 通用地图编辑器。
- 兼容 TMX / LDtk / Unity / Godot 导入导出。
- 完整脚本系统。
- 复杂 node graph。
- 程序化世界生成。
- 不被运行时代码读取的漂亮字段。

编辑器的价值应该始终回到一个问题：它能不能更快、更不容易出错地生产 `Alien Archive` 运行时真实使用的 Overworld RON 地图。
