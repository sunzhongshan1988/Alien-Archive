# Alien Archive Game Editor UI

`crates/editor/src/ui` is the editor's small design-system layer on top of `egui`.

`egui` remains the low-level immediate-mode GUI toolkit. Editor panels should prefer reusable components from this module instead of hand-rolling common controls in `main.rs` or feature panels.

## Rules

- Put reusable editor controls here first: tabs, panel headers, property rows, search fields, tree rows, inspector sections, and icon buttons.
- Keep components specific to Alien Archive Game Editor. This is not meant to become a general Rust UI library.
- Use colors and spacing from `ui/theme.rs`; avoid scattering ad hoc colors in business UI code.
- Prefer stable dimensions for controls so labels, hover states, and dynamic counts do not shift layouts.
- Keep rendering details inside the component. Call sites should read like editor intent, not painter plumbing.

## Current Components

- `buttons.rs`: fixed-size editor icon/action buttons for compact panel controls.
- `asset_grid.rs`: reusable thumbnail tiles and list rows for asset-like collections.
- `badge.rs`: compact colored status labels.
- `fields.rs`: property rows and text/options fields for inspector-style forms.
- `header.rs`: shared panel header row with a right-side action.
- `layer_row.rs`: layer rows with active state, item count, visibility, and lock toggles.
- `search.rs`: search input with built-in clear action.
- `sections.rs`: compact inspector section headings.
- `side_rail.rs`: collapsed left/right sidebar rails.
- `tabs.rs`: editor-style tab bar with shared bottom edge, selected underline, hover fill, and bounded tab width. Used by the left sidebar for `资源库 / 图层 / 对象`.
- `toolbar.rs`: tool buttons, command buttons, icon loading, and toolbar layout helpers.
- `theme.rs`: editor fonts, color tokens, and base egui visual style.
- `tree.rs`: outliner/tree rows with selection, badges, and detail text.

## Next Candidates

- `splitter`: consistent resizable panel affordances if egui defaults are not enough.
- `asset_grid`: richer variants for drag/drop and multi-select.
- `panel_surface`: common padding/margins for full-height editor panels.
