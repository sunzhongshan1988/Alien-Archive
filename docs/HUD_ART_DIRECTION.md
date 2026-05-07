# Alien Archive HUD 素材说明

最后更新：2026-05-08

## 当前方向

HUD 改成更正常的一体化游戏面板：

- 左上统一面板：人物状态、时间、天气集成在 `hud_player_panel.png` 里，圆形头像单独读取 `hud_player_avatar.png`。
- 底部快捷栏：仍保留单独的快捷栏底座和槽位 overlay。
- 动态内容由代码绘制：状态条填充、时间、天气文字、快捷栏数字、数量、物品图标。

视觉风格继续贴近现有菜单素材：深蓝黑金属底、多层硬边框、青色发光描边、少量琥珀色角标。

## 运行时读取的 5 张

目录固定为：

```txt
assets/images/ui/hud/
```

当前只保留这 5 张：

```txt
hud_player_panel.png        # 一体化状态 / 时间 / 天气面板，当前 1678x454
hud_player_avatar.png       # HUD 圆形头像贴图，当前 256x256
hud_quickbar_dock.png       # 底部快捷栏底座，当前 1024x240
hud_quick_slot_empty.png    # 普通快捷槽，当前 146x146
hud_quick_slot_selected.png # 选中快捷槽高亮，当前 154x154
```

## 代码绘制范围

这些不要烘焙进素材：

- 生命、体力、外骨骼、负重的彩色填充。
- 时间和天气文字。
- 快捷栏数字键。
- 物品数量。
- 物品图标。

## 注意

- `hud_player_panel.png` 当前已经裁成真实 HUD 面板，运行时代码按整张图读取；后续不要再保留大面积黑边留白。
- `hud_player_avatar.png` 由运行时代码塞进左侧头像框，后续只换同名文件即可。
- 后续如果重新裁图，文件名保持不变即可。
- 如果 `hud_player_panel.png` 的内部布局大改，需要同步调整 `crates/game/src/scenes/field_hud.rs` 里的源坐标。
