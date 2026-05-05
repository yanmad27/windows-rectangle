# Windows Rectangle — Design

**Date:** 2026-05-05
**Status:** Design approved, ready for implementation plan

## Goal

Windows desktop utility that resizes and positions the active window via global hotkeys. Inspired by macOS Rectangle. Tray-only UI, hardcoded default bindings, no config UI in MVP.

## Stack

- **Language:** Rust (edition 2021)
- **Win32 bindings:** `windows` crate (official Microsoft)
- **Tray:** `tray-icon` crate
- **Event loop host:** `winit` (hidden message window)
- **Build:** `cargo build --release`, single ~1 MB exe, no installer

## Architecture

```
+-------------------+
|  main.rs          |  winit message loop, receive WM_HOTKEY
|  hidden window    |  + tray menu events → Action dispatch
+---------+---------+
          |
          v
+-------------------+        +------------------+
|  hotkey.rs        |        |  tray.rs         |
|  RegisterHotKey   |        |  tray-icon menu  |
|  id → Action map  |        |  (built from     |
|                   |        |  BINDINGS table) |
+---------+---------+        +------------------+
          |                          |
          +-------------+------------+
                        v
+-------------------+
|  action.rs        |  enum Action + execute()
+---------+---------+
          |
          +---> foreground.rs   (SetWinEventHook → LAST_FG cache)
          +---> window_ops.rs   (SetWindowPos, restore stack)
          +---> monitor.rs      (EnumDisplayMonitors, work area, adjacency)
```

Single thread, message-loop driven. No async.

## Action Enum

```rust
enum Half { Left, Right, Top, Bottom }
enum Corner { TopLeft, TopRight, BottomLeft, BottomRight }

enum Action {
    Halve(Half),         // repeat-cycle to adjacent monitor
    Quarter(Corner),     // single-shot, no cycle
    Maximize,            // toggle: maximize ↔ restore
    Center,
    RestoreOriginal,
}
```

## Default Hotkeys

| Hotkey | Action |
|---|---|
| `Ctrl+Alt+Left` | Halve(Left) — repeat → prev monitor |
| `Ctrl+Alt+Right` | Halve(Right) — repeat → next monitor |
| `Ctrl+Alt+Up` | Halve(Top) — repeat → monitor above |
| `Ctrl+Alt+Down` | Halve(Bottom) — repeat → monitor below |
| `Ctrl+Alt+U` | Quarter(TopLeft) |
| `Ctrl+Alt+I` | Quarter(TopRight) |
| `Ctrl+Alt+J` | Quarter(BottomLeft) |
| `Ctrl+Alt+K` | Quarter(BottomRight) |
| `Ctrl+Alt+Enter` | Maximize (toggle) |
| `Ctrl+Alt+C` | Center |
| `Ctrl+Alt+R` | RestoreOriginal |

Single source of truth: `const BINDINGS: &[(VK, Modifiers, Action, &str)]` consumed by both hotkey registration and tray menu builder.

## Repeat-Cycle Logic (Halves Only)

State: `last_action: Option<(HWND, Action, MonitorId)>` (per-process global).

```
on Halve(dir):
    target_hwnd = active window
    cur_monitor = MonitorFromWindow(target_hwnd)
    if last_action == Some((target_hwnd, Halve(dir), cur_monitor)):
        adjacent = monitor_adjacent_to(cur_monitor, dir)
        if adjacent.is_some():
            apply Halve(dir) on adjacent
            last_action = (target_hwnd, Halve(dir), adjacent)
        else:
            no-op (no wrap)
    else:
        apply Halve(dir) on cur_monitor
        last_action = (target_hwnd, Halve(dir), cur_monitor)
```

Quarters do not cycle. Other actions clear `last_action`.

## Active Window Detection

**Two paths:**

1. **Hotkey path** — `GetForegroundWindow()` directly. Hotkeys do not steal focus.
2. **Tray menu path** — clicking the tray icon steals focus, so the foreground at click time is the tray window. Use a cached value instead.

**Cache via `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`:**

```rust
static LAST_FOREGROUND_HWND: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn foreground_change_hook(
    _hook: HWINEVENTHOOK, _event: u32, hwnd: HWND,
    _id_object: i32, _id_child: i32, _thread_id: u32, _event_time: u32,
) {
    if !is_own_window(hwnd) && is_top_level_visible(hwnd) {
        LAST_FOREGROUND_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
    }
}
```

`is_own_window` filters tray HWND + hidden message-window HWND. Before applying any action, validate via `IsWindow(hwnd)` and bail if invalid.

## Window Mutation

- All resize/move via `SetWindowPos(hwnd, null, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE)`.
- If window is currently maximized and action is non-Maximize: `ShowWindow(SW_RESTORE)` first, otherwise `SetWindowPos` misbehaves.
- Maximize toggle: `IsZoomed(hwnd)` → `SW_RESTORE`, else `SW_MAXIMIZE`.
- Center: keep current size, position `(work.x + (work.w - win.w)/2, work.y + (work.h - win.h)/2)`.

## Monitors

- Always use `MONITORINFO.rcWork` (excludes taskbar). Never `rcMonitor`.
- Adjacency: enumerate via `EnumDisplayMonitors`, compare `RECT` edges:
  - left-of: `other.right == cur.left` AND vertical overlap > 0
  - right-of: `other.left == cur.right` AND vertical overlap > 0
  - above / below: symmetric on Y
- Multiple candidates → pick the one with greatest overlap on the perpendicular axis.

## DPI

- App manifest declares `PerMonitorV2`.
- All Win32 calls then return physical pixels matching each monitor's scale.
- No manual DPI math needed when manifest is set.

## Restore Stack

- `HashMap<isize, RECT>` keyed on HWND raw value.
- Capture pre-action `GetWindowRect` on every action that mutates geometry.
- `RestoreOriginal` looks up and reapplies most recent stored RECT, then removes the entry.
- MVP does not detect manual user resize between actions. The captured rect is overwritten on each new action, which is acceptable.

## Tray Menu

```
Halve Left              Ctrl+Alt+Left
Halve Right             Ctrl+Alt+Right
Halve Top               Ctrl+Alt+Up
Halve Bottom            Ctrl+Alt+Down
─────────────
Quarter Top-Left        Ctrl+Alt+U
Quarter Top-Right       Ctrl+Alt+I
Quarter Bottom-Left     Ctrl+Alt+J
Quarter Bottom-Right    Ctrl+Alt+K
─────────────
Maximize                Ctrl+Alt+Enter
Center                  Ctrl+Alt+C
Restore                 Ctrl+Alt+R
─────────────
Quit
```

Hotkey labels are display-only text. Each action item is built from the same `BINDINGS` table. Click on action item → dispatch through identical Action path as hotkey, using `LAST_FOREGROUND_HWND`.

## Hotkey Registration

- `RegisterHotKey(hwnd, id, modifiers, vk)` per binding, id = index into `BINDINGS`.
- Return value of 0 → conflict with another app. Log warning, skip that binding, continue with rest.
- Unregister all on shutdown.

## File Layout

```
windows-rectangle/
├── Cargo.toml
├── build.rs                # embed manifest + icon via embed-resource
├── app.manifest            # PerMonitorV2 DPI
├── icon.ico
├── src/
│   ├── main.rs             # winit loop, init tray + hotkeys + hook
│   ├── bindings.rs         # const BINDINGS table
│   ├── action.rs           # Action enum, execute(), repeat-cycle state
│   ├── window_ops.rs       # SetWindowPos wrappers, restore stack
│   ├── monitor.rs          # enumeration, work area, adjacency
│   ├── foreground.rs       # SetWinEventHook + LAST_FOREGROUND_HWND
│   ├── hotkey.rs           # RegisterHotKey wrapper, modifier parsing
│   └── tray.rs             # tray-icon menu construction
└── tests/
    └── monitor_adjacency.rs
```

## Cargo.toml

```toml
[package]
name = "windows-rectangle"
version = "0.1.0"
edition = "2021"

[dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_Graphics_Gdi",
    "Win32_UI_Accessibility",
    "Win32_System_LibraryLoader",
] }
tray-icon = "0.19"
winit = "0.30"
anyhow = "1"

[build-dependencies]
embed-resource = "2"

[profile.release]
opt-level = "z"
lto = true
strip = true
```

## Testing

**Unit tests (pure logic):**
- `monitor::find_adjacent(rects, current, direction)` — feed synthetic RECT list, assert correct neighbor.
- `action::compute_target_rect(work_area, action, current_rect)` — math for halve, quarter, center.

**No Win32 mocking.** Win32 calls live in thin wrappers; verify by hand on real Windows.

**Manual test checklist** (lives in `TESTING.md` once implementation starts):
- Single monitor: each halve, each quarter, maximize toggle, center, restore.
- Dual monitor side-by-side: `Ctrl+Alt+Left` once → halves on current monitor; press again → moves and halves on left monitor; press again → no-op (no further left).
- Mixed DPI (100% primary + 150% secondary): window keeps correct size when moved across monitors.
- Taskbar visible: window respects work area, does not cover taskbar.
- Tray click immediately after Alt-tab: action applies to the previously focused app, not the tray.

## Out of Scope (MVP)

- Configurable hotkeys / config file
- GUI settings window
- Auto-start on boot
- Custom layouts beyond halves/quarters
- Snap-on-drag overlays
- Multi-monitor "throw" via dedicated hotkey (folded into halve repeat-cycle)

## Open Questions

None blocking. Ready for implementation plan.
