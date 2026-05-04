# Alien Archive

`Alien Archive` is the starting point for a long-running 2D sci-fi exploration game.

Current milestone:

- opens a native window with `winit`
- initializes `wgpu`
- renders rectangles through a small renderer abstraction
- loads and renders PNG images through the renderer
- shows `assets/images/startup/startup_background.png` as the startup screen
- has a basic scene stack: `MainMenuScene -> GameScene`, plus a minimal `PauseScene`
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
- In game: `WASD` or arrow keys to move
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

tools/        # future local web map editor
```

More project memory:

- [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md)
