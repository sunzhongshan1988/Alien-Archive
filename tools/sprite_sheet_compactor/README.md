# Sprite Sheet Compactor

`sprite_sheet_compactor` 用来把 AI 生成的宽间隙 sprite sheet 整理成游戏更容易读取的紧凑 sheet。

它做三件事：

- 按输入网格把整张图切成多帧。
- 对每一帧检测角色内容区域，裁掉周围多余空白。
- 重新排版为统一 cell，并按底部居中对齐；需要时可以把每帧缩放到指定分辨率。

## 适用场景

- AI 生成的横向或网格 sprite sheet，角色之间空白太大。
- 背景是假透明棋盘格，或者边缘是大面积白色/浅灰背景。
- 想把素材规范化成游戏中固定帧尺寸，例如 `64x64`、`96x96`、`128x128`。

如果原图已经是真透明 PNG，工具会优先按 alpha 裁剪；如果不是透明图，会从每个 frame 的边缘开始识别浅色背景区域。

## 常用命令

从项目根目录执行。

只缩小间隙，保持自然紧凑尺寸：

```powershell
cargo run -p sprite_sheet_compactor -- `
  --input d:\Downloads\raw_sheet.png `
  --output assets\sprites\player\topdown\walk_down_compact.png `
  --frames 4 `
  --input-columns 4 `
  --padding 8
```

缩小间隙，并把每帧放进 `64x64` cell：

```powershell
cargo run -p sprite_sheet_compactor -- `
  --input d:\Downloads\raw_sheet.png `
  --output assets\sprites\player\topdown\walk_down_64.png `
  --frames 4 `
  --input-columns 4 `
  --padding 2 `
  --cell-size 64x64 `
  --filter lanczos3
```

输入是两行四列，输出仍按四列排：

```powershell
cargo run -p sprite_sheet_compactor -- `
  --input d:\Downloads\raw_sheet.png `
  --output assets\sprites\player\topdown\walk_down_64.png `
  --frames 8 `
  --input-columns 4 `
  --input-rows 2 `
  --output-columns 4 `
  --cell-size 64 `
  --padding 2
```

## 参数说明

- `--input <path>`：输入 PNG。
- `--output <path>`：输出 PNG。父目录不存在时会自动创建。
- `--frames <count>`：要处理的帧数。
- `--input-columns <count>`：输入 sheet 的列数。默认等于 `--frames`，适合单行横向 sheet。
- `--input-rows <count>`：输入 sheet 的行数。默认按帧数和列数自动计算。
- `--output-columns <count>`：输出 sheet 的列数。默认等于输入列数。
- `--padding <pixels>`：每个输出 cell 内保留的透明边距，默认 `4`。
- `--cell-size <WxH>`：输出 cell 尺寸，例如 `64x64`。也可以只写 `64`，表示 `64x64`。
- `--frame-size <WxH>`：`--cell-size` 的别名。
- `--filter <name>`：缩放滤镜。可选 `nearest`、`triangle`、`catmullrom`、`gaussian`、`lanczos3`。默认 `nearest`。
- `--background <mode>`：背景检测模式。可选 `auto`、`transparent`、`white`，默认 `auto`。
- `--alpha-threshold <0-255>`：低于该 alpha 值的像素视为透明，默认 `8`。
- `--white-threshold <0-255>`：浅色背景检测的最低 RGB 通道值，默认 `225`。
- `--white-chroma <0-255>`：浅色背景检测允许的 RGB 通道差，默认 `28`。
- `--preserve-canvas`：只清背景，不裁切每帧画布。适合地形 tile 这类需要保持统一画布尺寸的素材。
- `--keep-background`：保留检测到的边缘背景像素，只裁间隙，不清透明。

## 缩放规则

设置 `--cell-size` 后，工具不会把每一帧分别拉伸到满格，而是会：

- 先找出所有帧里最大的裁剪后角色尺寸。
- 根据目标 cell 和 padding 计算一个共享缩放比例。
- 所有帧使用同一个缩放比例。
- 缩放后仍按底部居中贴入各自 cell。

这样可以避免动画播放时角色大小一帧一帧跳动。

## 滤镜建议

- 像素风原始素材：优先 `nearest`，边缘最硬。
- AI 生成后再缩小的大图：通常 `lanczos3` 更顺眼。
- 需要稍微软一点但不想太糊：可以试 `catmullrom`。

## 当前项目建议

`Alien Archive` 目前玩家占位素材在：

```txt
assets/sprites/player/topdown/
```

如果要替换成固定帧宽的 sprite sheet，推荐先统一为 `64x64` cell：

```powershell
cargo run -p sprite_sheet_compactor -- `
  --input d:\Downloads\raw_walk_down.png `
  --output assets\sprites\player\topdown\walk_down.png `
  --frames 4 `
  --input-columns 4 `
  --padding 2 `
  --cell-size 64x64 `
  --filter lanczos3
```

导出后确认输出尺寸应该是：

```txt
width = frames_per_row * cell_width
height = rows * cell_height
```

例如 4 帧横排、每帧 `64x64`，输出就是 `256x64`。

## 注意事项

- 输入图片宽高必须能被输入网格整除，例如 `2508x627` 配 `--input-columns 4` 时，每格就是 `627x627`。
- 如果浅色装甲被误判成背景，可以提高 `--white-threshold`，或者用 `--background transparent` 只按 alpha 处理。
- 如果棋盘格残留明显，可以降低 `--white-threshold` 或提高 `--white-chroma`。
- 如果只是想缩间隙，不想改背景像素，加 `--keep-background`。
