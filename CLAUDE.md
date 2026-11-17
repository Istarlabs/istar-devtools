# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Run
cargo run

# Check without building
cargo check

# Lint
cargo clippy

# Format
cargo fmt
```

There are no tests in this project currently.

The build step compiles Slint UI files via `build.rs`, which uses `slint-build` to compile `ui/app-window.slint` into Rust code. The `lucide-slint` crate provides icon assets as a library path. The compiled UI types are brought into scope via `slint::include_modules!()` in `main.rs`.

This project targets Wayland on Linux. The Slint backend is `backend-winit-wayland` with `renderer-femtovg`. The clipboard crate (`arboard`) is configured with `wayland-data-control`. The file dialog (`rfd`) uses `xdg-portal`.

## Architecture

This is a desktop developer toolbox GUI app built with **Rust + Slint**. The UI is declarative (`.slint` files), and the logic lives in Rust.

### UI ↔ Rust bridge pattern

Slint `global` singletons (defined in `ui/globals.slint`) are the shared state layer between UI and Rust. Each tool page has its own global (e.g. `Base64State`, `HashGeneratorState`). Each global exposes:
- `in-out property` fields for input/output strings and mode toggles
- `callback` declarations for actions (convert, copy, paste, open-from-file)

On the Rust side, each page module lives in `src/pages/<tool>.rs` and exports a single `register(app: &AppWindow)` function. That function:
1. Gets a weak handle to the window (`app.as_weak()`)
2. Attaches closures to the global's callbacks via `on_<callback_name>()`
3. Reads state with `state.get_*()` and writes back with `state.set_*()`

All pages are registered in `main.rs` before `app_window.run()`.

### Adding a new tool

`python3 Scripts/new_tool.py <Name> <module> <slug> "<Menu text>" <LucideIcon> "<Menu group>" ["<search tags>"]` does steps 1-5 and leaves compilable skeletons; the steps below describe what it wires.

1. Create `ui/pages/<tool>.slint` — import and use `PageContainer`, `InputOutputPanel`, or `LabeledField` components as appropriate
2. Add a new global to `ui/globals.slint` with the tool's state + callbacks
3. Export the new global and import the new page in `ui/app-window.slint`; add a new entry to the `AppPages` enum in `ui/types.slint` and a `NavItem` in the side menu model
4. Create `src/pages/<tool>.rs` implementing `pub fn register(app: &AppWindow)`
5. Add `pub mod <tool>;` to `src/pages/mod.rs` and call `pages::<tool>::register(&app_window)` in `main.rs`

### UI structure

```
ui/
  app-window.slint   # Root window; page visibility switching; side menu model
  globals.slint      # All global state singletons (one per tool)
  types.slint        # Shared structs/enums (AppPages, NavItem, DiskInfo, etc.)
  side-menu.slint    # Collapsible nav sidebar
  components/        # Reusable UI pieces: InputOutputPanel, PageContainer, LabeledField, SeparatorLine
  pages/             # One .slint file per tool page
```

### Rust structure

```
src/
  main.rs            # Entry point; registers all pages; runs app
  pages/             # One module per tool — handles callbacks, clipboard, file I/O
  units/             # Utility logic not tied to a specific page (e.g. byte_unit_converter)
```

### Key crates

| Crate | Purpose |
|---|---|
| `slint` | UI framework (Wayland, femtovg renderer) |
| `slint-build` / `lucide-slint` | Build-time UI compilation + Lucide icon set |
| `arboard` | Clipboard (Wayland data-control protocol) |
| `rfd` | Native file dialogs (xdg-portal) |
| `base64`, `hex` | Encoding utilities |
| `md5`, `sha1`, `sha2`, `sha3`, `adler`, `crc` | Hash/checksum algorithms |
| `html-escape` | HTML encode/decode |
| `regex` | Text inspection |
| `sysinfo` | System information (CPU, RAM, disks, OS) |