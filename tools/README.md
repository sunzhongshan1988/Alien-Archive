# Tools

这个目录放项目开发期使用的本地工具。这里的工具不属于游戏运行时逻辑，主要服务于素材整理、地图编辑、数据检查等工作流。

## 当前工具

- [`sprite_sheet_compactor`](sprite_sheet_compactor/README.md)

  用来处理 AI 生成的 sprite sheet 和 tile：按帧裁掉过大的空白间隙，清理透明或浅色棋盘格背景，并可把每帧缩放到指定分辨率，例如 `64x64`。地形 tile 可以用 `--fill-transparent --stretch-to-cell` 生成铺满格子的方形图。

## 运行方式

这些工具作为 Rust workspace package 维护，从项目根目录运行：

```powershell
cargo run -p sprite_sheet_compactor -- --help
```

以后新增工具时，优先放在 `tools/<tool_name>` 下，并在这里补一条入口说明。
