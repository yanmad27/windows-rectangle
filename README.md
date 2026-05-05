# window-rectangle

Windows tray app that resizes the active window via global hotkeys. Inspired by macOS Rectangle.

## Features

- Halve window left / right / top / bottom
- Quarter into any corner
- Maximize toggle, center, restore
- Halve repeat-cycle: pressing the same direction again moves the window to the adjacent monitor and halves it there
- System tray menu lists every binding; clicking acts on the previously focused window
- Per-monitor DPI aware
- Single ~1–2 MB exe, no installer

## Default hotkeys

| Hotkey | Action |
|---|---|
| `Ctrl+Alt+Left` | Halve Left (repeat → previous monitor) |
| `Ctrl+Alt+Right` | Halve Right (repeat → next monitor) |
| `Ctrl+Alt+Up` | Halve Top |
| `Ctrl+Alt+Down` | Halve Bottom |
| `Ctrl+Alt+U` | Quarter Top-Left |
| `Ctrl+Alt+I` | Quarter Top-Right |
| `Ctrl+Alt+J` | Quarter Bottom-Left |
| `Ctrl+Alt+K` | Quarter Bottom-Right |
| `Ctrl+Alt+Enter` | Maximize toggle |
| `Ctrl+Alt+C` | Center |
| `Ctrl+Alt+R` | Restore previous size |

Hotkeys are hardcoded in this MVP. Config file / settings GUI are out of scope.

## Install

Download the latest `window-rectangle-*.exe` from the [Releases](https://github.com/yanmad27/window-rectangle/releases) page. Double-click to run. A blue square appears in the system tray.

To start on login, drop a shortcut into `shell:startup`.

To quit, right-click the tray icon → Quit.

## Build from source

Requires Rust stable and the `x86_64-pc-windows-msvc` target.

```cmd
cargo build --release --target x86_64-pc-windows-msvc
```

Output: `target\x86_64-pc-windows-msvc\release\window-rectangle.exe`.

Cross-target check from non-Windows hosts also works:

```sh
cargo check --target x86_64-pc-windows-msvc
```

Pure-logic modules run on any OS:

```sh
cargo test --lib
```

## Architecture

- Pure-Rust geometry, adjacency, repeat-cycle state, and bindings table — platform-agnostic, fully unit-tested.
- Win32 modules (`monitor`, `window_ops`, `foreground`, `hotkey`, `dispatcher`, `tray`) gated under `#[cfg(target_os = "windows")]`.
- `winit` hosts a hidden message window. `RegisterHotKey` posts `WM_HOTKEY` for hotkey path. `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` caches the real foreground window so tray clicks dispatch to the prior app, not the tray.

See `docs/plans/` for the design and implementation plan.

## License

MIT
