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
- moves the player rectangle with keyboard input
- loads a custom RON map from `assets/data/maps/demo.ron`

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
  maps/
  audio/
  data/
    maps/     # custom Alien Archive map files
      overworld.ron
      facility_ruin_01.ron

tools/        # future local web map editor
```

More project memory:

- [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md)
