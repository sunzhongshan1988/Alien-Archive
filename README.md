# Alien Archive

`Alien Archive` is the starting point for a long-running 2D sci-fi exploration game.

Current milestone:

- opens a native window with `winit`
- initializes `wgpu`
- renders rectangles through a small renderer abstraction
- loads and renders PNG images through the renderer
- shows `assets/images/startup/startup_background.png` as the startup screen
- has a basic scene stack: `MainMenuScene -> OverworldScene -> FacilityScene`, plus a minimal `PauseScene`
- switches from the top-down overworld into a side-view facility through map entities
- draws a focused Start/Quit main menu over the background image
- loads the top-down idle player sprite sheet and draws the current frame in game
- moves the player with keyboard input
- loads custom RON maps from `assets/data/maps/overworld.ron` and `assets/data/maps/facility_ruin_01.ron`

Map strategy:

- Alien Archive uses its own game-facing map schema.
- External editors are optional tools, not runtime architecture.
- A future local web editor can read/write the same map files under `assets/data/maps`.

Run it:

```powershell
cargo run -p alien_archive
```

Font:

- Recommended: `Noto Sans SC` or `Source Han Sans SC`
- Current UI font: `assets/fonts/ui.ttf`
- Original font file name before normalization: `SourceHanSans-VF.ttf`
- Local `SourceHan*.ttf` / `Noto*.ttf` files in `assets/fonts` are also auto-detected as fallback

Controls:

- Main menu: `WASD` / arrow keys / mouse hover to select, `Enter` / `Space` / left click to confirm
- Overworld: `WASD` or arrow keys for four-direction movement, `E` near the purple entrance to enter the facility
- Facility: `A/D` or left/right to move, `W` or `Space` to jump, `E` near the yellow exit to return
- `Esc` in game: pause
- `Esc` in the main menu: quit

Project layout:

```txt
crates/
  runtime/   # window, input, renderer, camera, future platform services
  game/      # game state, player, world, future scanner/codex/save systems

assets/
  fonts/    # optional UI font override: ui.ttf
  images/
    startup/ # startup/title background images
  sprites/
    player/
      topdown/
        idle_down.png
        walk_down.png
        walk_left.png
        walk_right.png
        walk_up.png
  maps/
  audio/
  data/
    maps/     # custom Alien Archive map files
      overworld.ron
      facility_ruin_01.ron

tools/        # asset and content maintenance helpers
```

Player sprite sheet convention:

- `assets/sprites/player/topdown/{idle_down,walk_down,walk_left,walk_right,walk_up}.png` are real `512x128` sheets
- The runtime treats each sheet as 4 horizontal frames, each `128x128`
- The top-down player renderer selects `walk_*` by movement direction and draws one source frame instead of stretching the full sheet

Tools:

- Compact oversized AI sprite sheets:

```powershell
cargo run -p sprite_sheet_compactor -- `
  --input d:\Downloads\raw_sheet.png `
  --output assets\sprites\player\topdown\walk_down_compact.png `
  --frames 4 `
  --input-columns 4 `
  --padding 8 `
  --cell-size 64x64
```

The tool trims each frame separately, clears detected transparent or edge-connected
white checkerboard background, then repacks frames into uniform bottom-center
aligned cells. Omit `--cell-size` to keep the compact natural size; use
`--filter lanczos3` if a downscaled AI sheet looks too jagged.
For terrain tiles that must fill the whole map cell, add
`--fill-transparent --stretch-to-cell` with a square `--cell-size`.

More project memory:

- [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md)
