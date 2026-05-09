# Alien Archive HUD 素材说明

最后更新：2026-05-09

## 当前方向

HUD 改成新的像素风宇航服状态面板：

- 左上状态面板：使用 `hud_player_panel.png` 作为完整底图，运行时覆盖绘制时间、天气和 4 条真实状态槽，基准尺寸为 428x192。
- 底部快捷栏：改为运行时代码绘制的 6 个独立小槽，不再使用整张快捷栏底座背景。
- 动态内容由代码绘制：状态条填充、时间、天气文字、快捷栏数量、物品图标。

视觉风格继续贴近现有菜单素材：深蓝黑金属底、多层硬边框、青色发光描边、少量琥珀色角标。

## 运行时读取的可选贴图

目录固定为：

```txt
assets/images/ui/hud/
```

当前左上面板读取整张新底图；底部快捷栏不需要整张大底图：

```txt
hud_player_panel.png        # 左上完整 HUD 面板，当前 1284x576
hud_player_avatar.png       # HUD 小头像贴图，建议 256x256
```

## 代码绘制范围

这些不要烘焙进素材：

- 生命、体力、外骨骼、负重的彩色填充。
- 时间和天气文字，覆盖在新面板左下时间底座里。
- 快捷栏槽位底框和选中高亮。
- 物品数量，使用纯文字，不加徽章背景和边框。
- 物品图标。
- 快捷栏不显示数字键提示，也不显示选中物品名提示。

## 注意

- `hud_player_panel.png` 是左上 HUD 的主要视觉资产；如果内部槽位位置变化，需要同步调整 `PLAYER_PANEL_METER_SOURCES` 和 `PLAYER_PANEL_TIME_SOURCE`。
- `hud_player_avatar.png` 仅作为新面板缺失时的代码 fallback。
- 快捷栏布局现在集中在 `QUICKBAR_SLOT_SIZE`、`QUICKBAR_SLOT_GAP`、`quickbar_slot_at_position` 和 `draw_quick_slot_frame` 中。
- 左上状态面板布局现在集中在 `crates/game/src/scenes/field_hud.rs` 的 `PLAYER_PANEL_BASE_SIZE`、`PLAYER_PANEL_TIME_SOURCE` 和 `PLAYER_PANEL_METER_SOURCES` 中。
