# Window Rectangle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Windows tray app in Rust that resizes the active window via global hotkeys (halve, quarter, maximize, center, restore), with halve repeat-cycle across adjacent monitors.

**Architecture:** Single-binary Rust app. Pure-Rust geometry/adjacency/state-machine logic in a platform-agnostic module so it tests on any OS. Win32-only modules are gated by `#[cfg(target_os = "windows")]` and isolate `windows` crate calls. `winit` event loop hosts a hidden message window; `tray-icon` provides the system tray. `RegisterHotKey` for global hotkeys, `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` to cache the real foreground window so tray clicks know which window to act on.

**Tech Stack:** Rust 2021, `windows` 0.58, `tray-icon` 0.19, `winit` 0.30, `anyhow` 1, `embed-resource` 2 (build dep).

**Source design doc:** `docs/plans/2026-05-05-window-rectangle-design.md`

---

## Notes for the Implementing Engineer

- The host machine for this plan may be macOS or Linux. The Win32-dependent code does not compile there. Pure-logic modules (`geometry`, `adjacency`, `cycle_state`) are written so they compile and test on **any** OS via `cargo test --lib`. The Win32 modules are gated `#[cfg(target_os = "windows")]` with a stub `main` for other targets that prints "Windows only" and exits.
- Use `RTK` if available — `git status` etc. should already be transparently rewritten to `rtk git status`. You don't need to do anything extra.
- Commit messages: do **not** add Claude as co-author. Avoid the words "medi", "de", "verbund" in commit messages (per user CLAUDE.md).
- Use descriptive variable names (per user CLAUDE.md). E.g., `current_monitor_rect` not `mr`.
- Run tests after each implementation step. Commit after each task.

---

## Task 1: Initialize Cargo project

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src/main.rs`
- Create: `src/lib.rs`

**Step 1: Run cargo init**

Run:
```
cargo init --name window-rectangle --vcs none
```
Expected: creates `Cargo.toml` and `src/main.rs`.

**Step 2: Replace `Cargo.toml`**

```toml
[package]
name = "window-rectangle"
version = "0.1.0"
edition = "2021"
description = "Windows tray app to resize active window via hotkeys"

[lib]
name = "window_rectangle"
path = "src/lib.rs"

[[bin]]
name = "window-rectangle"
path = "src/main.rs"

[dependencies]
anyhow = "1"

[target.'cfg(target_os = "windows")'.dependencies]
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

[target.'cfg(target_os = "windows")'.build-dependencies]
embed-resource = "2"

[profile.release]
opt-level = "z"
lto = true
strip = true
```

**Step 3: Create `.gitignore`**

```
/target
**/*.rs.bk
Cargo.lock
*.exe
```

**Step 4: Create empty `src/lib.rs`**

```rust
pub mod geometry;
```

(`geometry` module added in next task — file must exist for `lib.rs` to compile after Task 2.)

**Step 5: Replace `src/main.rs` with stub**

```rust
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("window-rectangle is Windows-only");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    Ok(())
}
```

**Step 6: Verify it compiles**

Run: `cargo check`
Expected: compiles with warnings about unused module (acceptable for now).

This will fail because `src/lib.rs` references `geometry` which doesn't exist yet. Comment out the `pub mod geometry;` line for this task only — re-enable in Task 2.

Run: `cargo check`
Expected: clean compile.

**Step 7: Commit**

```bash
git add Cargo.toml .gitignore src/main.rs src/lib.rs
git commit -m "chore: scaffold cargo project with windows-only target gates"
```

---

## Task 2: Geometry primitives (TDD)

Pure-logic types and functions. Compiles on any OS.

**Files:**
- Create: `src/geometry.rs`
- Modify: `src/lib.rs` (re-enable `pub mod geometry;`)

**Step 1: Write failing tests**

Create `src/geometry.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Halve(Half),
    Quarter(Corner),
    Maximize,
    Center,
    RestoreOriginal,
}

/// Compute target rect for a given action within a work area.
/// `current` is needed for Center (preserve size) and is otherwise unused.
pub fn compute_target_rect(work_area: Rect, action: Action, current: Rect) -> Option<Rect> {
    match action {
        Action::Halve(Half::Left) => Some(Rect::new(
            work_area.x,
            work_area.y,
            work_area.width / 2,
            work_area.height,
        )),
        Action::Halve(Half::Right) => Some(Rect::new(
            work_area.x + work_area.width / 2,
            work_area.y,
            work_area.width - work_area.width / 2,
            work_area.height,
        )),
        Action::Halve(Half::Top) => Some(Rect::new(
            work_area.x,
            work_area.y,
            work_area.width,
            work_area.height / 2,
        )),
        Action::Halve(Half::Bottom) => Some(Rect::new(
            work_area.x,
            work_area.y + work_area.height / 2,
            work_area.width,
            work_area.height - work_area.height / 2,
        )),
        Action::Quarter(Corner::TopLeft) => Some(Rect::new(
            work_area.x,
            work_area.y,
            work_area.width / 2,
            work_area.height / 2,
        )),
        Action::Quarter(Corner::TopRight) => Some(Rect::new(
            work_area.x + work_area.width / 2,
            work_area.y,
            work_area.width - work_area.width / 2,
            work_area.height / 2,
        )),
        Action::Quarter(Corner::BottomLeft) => Some(Rect::new(
            work_area.x,
            work_area.y + work_area.height / 2,
            work_area.width / 2,
            work_area.height - work_area.height / 2,
        )),
        Action::Quarter(Corner::BottomRight) => Some(Rect::new(
            work_area.x + work_area.width / 2,
            work_area.y + work_area.height / 2,
            work_area.width - work_area.width / 2,
            work_area.height - work_area.height / 2,
        )),
        Action::Center => Some(Rect::new(
            work_area.x + (work_area.width - current.width) / 2,
            work_area.y + (work_area.height - current.height) / 2,
            current.width,
            current.height,
        )),
        // Maximize and RestoreOriginal are not pure-geometry — they need IsZoomed / restore stack.
        Action::Maximize | Action::RestoreOriginal => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work_area_1920_1080() -> Rect {
        Rect::new(0, 0, 1920, 1040) // taskbar takes 40px
    }

    #[test]
    fn halve_left_takes_left_half() {
        let work_area = work_area_1920_1080();
        let result = compute_target_rect(work_area, Action::Halve(Half::Left), work_area).unwrap();
        assert_eq!(result, Rect::new(0, 0, 960, 1040));
    }

    #[test]
    fn halve_right_takes_right_half_with_remainder() {
        let work_area = Rect::new(0, 0, 1921, 1080); // odd width
        let result = compute_target_rect(work_area, Action::Halve(Half::Right), work_area).unwrap();
        assert_eq!(result, Rect::new(960, 0, 961, 1080));
        assert_eq!(result.x + result.width, 1921, "must reach right edge");
    }

    #[test]
    fn halve_bottom_starts_at_midpoint() {
        let work_area = Rect::new(0, 0, 1920, 1080);
        let result = compute_target_rect(work_area, Action::Halve(Half::Bottom), work_area).unwrap();
        assert_eq!(result, Rect::new(0, 540, 1920, 540));
    }

    #[test]
    fn quarter_top_left() {
        let work_area = work_area_1920_1080();
        let result = compute_target_rect(work_area, Action::Quarter(Corner::TopLeft), work_area).unwrap();
        assert_eq!(result, Rect::new(0, 0, 960, 520));
    }

    #[test]
    fn quarter_bottom_right_reaches_corners() {
        let work_area = Rect::new(0, 0, 1921, 1081); // odd both
        let result = compute_target_rect(work_area, Action::Quarter(Corner::BottomRight), work_area).unwrap();
        assert_eq!(result.x + result.width, 1921);
        assert_eq!(result.y + result.height, 1081);
    }

    #[test]
    fn center_preserves_size() {
        let work_area = Rect::new(0, 0, 1920, 1080);
        let current = Rect::new(100, 100, 800, 600);
        let result = compute_target_rect(work_area, Action::Center, current).unwrap();
        assert_eq!(result, Rect::new(560, 240, 800, 600));
    }

    #[test]
    fn center_on_offset_work_area() {
        // Second monitor at x=1920
        let work_area = Rect::new(1920, 0, 1920, 1080);
        let current = Rect::new(0, 0, 800, 600);
        let result = compute_target_rect(work_area, Action::Center, current).unwrap();
        assert_eq!(result, Rect::new(1920 + 560, 240, 800, 600));
    }

    #[test]
    fn maximize_returns_none_pure_geometry() {
        let work_area = Rect::new(0, 0, 1920, 1080);
        assert!(compute_target_rect(work_area, Action::Maximize, work_area).is_none());
    }
}
```

Re-enable in `src/lib.rs`:

```rust
pub mod geometry;
```

**Step 2: Run tests, expect all pass**

Run: `cargo test --lib geometry`
Expected: 8 passed.

**Step 3: Commit**

```bash
git add src/geometry.rs src/lib.rs
git commit -m "feat(geometry): add Rect, Action enum, and compute_target_rect with tests"
```

---

## Task 3: Monitor adjacency (TDD)

Pure logic for picking the next monitor in a given direction.

**Files:**
- Create: `src/adjacency.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing tests**

Create `src/adjacency.rs`:

```rust
use crate::geometry::{Half, Rect};

/// Find the monitor adjacent to `current` in the direction implied by `direction`.
/// Returns the index into `monitors`, or None if no neighbor.
///
/// Rules:
/// - Left: another monitor whose `right` edge equals `current.left`, with vertical overlap > 0.
/// - Right: symmetric with `left == current.right`.
/// - Top: another monitor whose `bottom` edge equals `current.top`, with horizontal overlap > 0.
/// - Bottom: symmetric with `top == current.bottom`.
/// - On multiple candidates, pick the one with greatest perpendicular-axis overlap.
pub fn find_adjacent_monitor(monitors: &[Rect], current: Rect, direction: Half) -> Option<usize> {
    let mut best_index: Option<usize> = None;
    let mut best_overlap: i32 = 0;

    for (index, candidate) in monitors.iter().enumerate() {
        if *candidate == current {
            continue;
        }
        let (edge_match, overlap) = match direction {
            Half::Left => (
                candidate.x + candidate.width == current.x,
                vertical_overlap(*candidate, current),
            ),
            Half::Right => (
                candidate.x == current.x + current.width,
                vertical_overlap(*candidate, current),
            ),
            Half::Top => (
                candidate.y + candidate.height == current.y,
                horizontal_overlap(*candidate, current),
            ),
            Half::Bottom => (
                candidate.y == current.y + current.height,
                horizontal_overlap(*candidate, current),
            ),
        };

        if edge_match && overlap > 0 && overlap > best_overlap {
            best_index = Some(index);
            best_overlap = overlap;
        }
    }

    best_index
}

fn vertical_overlap(a: Rect, b: Rect) -> i32 {
    let top = a.y.max(b.y);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (bottom - top).max(0)
}

fn horizontal_overlap(a: Rect, b: Rect) -> i32 {
    let left = a.x.max(b.x);
    let right = (a.x + a.width).min(b.x + b.width);
    (right - left).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_left_monitor_in_horizontal_setup() {
        let monitor_left = Rect::new(-1920, 0, 1920, 1080);
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_left, monitor_main];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn finds_right_monitor() {
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitor_right = Rect::new(1920, 0, 1920, 1080);
        let monitors = vec![monitor_main, monitor_right];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Right);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn no_left_neighbor_returns_none() {
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitor_right = Rect::new(1920, 0, 1920, 1080);
        let monitors = vec![monitor_main, monitor_right];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, None);
    }

    #[test]
    fn picks_greatest_overlap_among_multiple_left_candidates() {
        // Two stacked monitors on the left, both share the right-edge x.
        // Top one barely overlaps current vertically; bottom one fully overlaps.
        let monitor_top_left = Rect::new(-1920, -1000, 1920, 1080);   // overlaps y=0..80
        let monitor_bottom_left = Rect::new(-1920, 0, 1920, 1080);    // overlaps y=0..1080
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_top_left, monitor_bottom_left, monitor_main];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn finds_top_monitor() {
        let monitor_top = Rect::new(0, -1080, 1920, 1080);
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_top, monitor_main];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Top);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn touching_corner_only_does_not_count() {
        // Top-left monitor touches main only at the corner — no overlap on either axis.
        let monitor_corner = Rect::new(-1920, -1080, 1920, 1080);
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_corner, monitor_main];

        // Looking left: edge matches (right = -1920+1920 = 0 = current.x) but vertical overlap is 0.
        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, None);
    }
}
```

Add to `src/lib.rs`:

```rust
pub mod adjacency;
pub mod geometry;
```

**Step 2: Run tests**

Run: `cargo test --lib adjacency`
Expected: 6 passed.

**Step 3: Commit**

```bash
git add src/adjacency.rs src/lib.rs
git commit -m "feat(adjacency): add find_adjacent_monitor with overlap-based selection"
```

---

## Task 4: Repeat-cycle state machine (TDD)

Tracks the last action so a second press of `Halve(Left)` moves to the prev monitor.

**Files:**
- Create: `src/cycle_state.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing tests**

Create `src/cycle_state.rs`:

```rust
use crate::geometry::Action;

/// Identifies a window for cycle-state purposes. The Win32 layer passes
/// HWND.0 as i64 here; tests use any unique integer.
pub type WindowId = i64;

/// Identifies a monitor for cycle-state purposes. The Win32 layer passes
/// HMONITOR.0 as i64; tests use any unique integer.
pub type MonitorId = i64;

#[derive(Debug, Default)]
pub struct CycleState {
    last: Option<(WindowId, Action, MonitorId)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CycleDecision {
    /// First press, or different window/action: apply on the current monitor.
    ApplyHere,
    /// Repeat press on the same window+action+monitor: move to the adjacent monitor.
    MoveToAdjacent,
}

impl CycleState {
    pub fn new() -> Self {
        Self { last: None }
    }

    /// Decide what to do for an incoming action on `window` currently on `monitor`.
    /// Only `Halve` actions cycle; everything else returns `ApplyHere`.
    pub fn decide(&self, window: WindowId, action: Action, monitor: MonitorId) -> CycleDecision {
        if !matches!(action, Action::Halve(_)) {
            return CycleDecision::ApplyHere;
        }
        match self.last {
            Some((prev_window, prev_action, prev_monitor))
                if prev_window == window
                    && prev_action == action
                    && prev_monitor == monitor =>
            {
                CycleDecision::MoveToAdjacent
            }
            _ => CycleDecision::ApplyHere,
        }
    }

    /// Record that `action` was just applied to `window` on `monitor`.
    pub fn record(&mut self, window: WindowId, action: Action, monitor: MonitorId) {
        if matches!(action, Action::Halve(_)) {
            self.last = Some((window, action, monitor));
        } else {
            self.last = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Half;

    const WINDOW_A: WindowId = 100;
    const WINDOW_B: WindowId = 200;
    const MONITOR_MAIN: MonitorId = 1;
    const MONITOR_LEFT: MonitorId = 2;

    #[test]
    fn first_halve_press_applies_here() {
        let state = CycleState::new();
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn repeat_same_action_same_monitor_moves_to_adjacent() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::MoveToAdjacent
        );
    }

    #[test]
    fn different_window_resets_cycle() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_B, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn different_direction_resets_cycle() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Right), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn quarter_action_does_not_cycle() {
        use crate::geometry::Corner;
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Quarter(Corner::TopLeft), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_A, Action::Quarter(Corner::TopLeft), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn non_halve_action_clears_cycle_record() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        state.record(WINDOW_A, Action::Center, MONITOR_MAIN);
        // Now even repeating the prior halve should apply here, not move.
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn moving_to_adjacent_monitor_then_repeating_continues_cycle() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        // After dispatcher acts on adjacent and records:
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_LEFT);
        // Next press should request another move (to monitor further left).
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_LEFT),
            CycleDecision::MoveToAdjacent
        );
    }
}
```

Add to `src/lib.rs`:

```rust
pub mod adjacency;
pub mod cycle_state;
pub mod geometry;
```

**Step 2: Run tests**

Run: `cargo test --lib cycle_state`
Expected: 7 passed.

**Step 3: Run full test suite**

Run: `cargo test --lib`
Expected: all 21 tests pass.

**Step 4: Commit**

```bash
git add src/cycle_state.rs src/lib.rs
git commit -m "feat(cycle): add CycleState for halve repeat-to-adjacent-monitor logic"
```

---

## Task 5: Bindings table

Static table of `(modifiers, virtual key, action, label)` consumed by both hotkey registration and tray menu.

**Files:**
- Create: `src/bindings.rs`
- Modify: `src/lib.rs`

**Step 1: Write the module**

Create `src/bindings.rs`:

```rust
use crate::geometry::{Action, Corner, Half};

/// Win32 modifier flags (matches MOD_* constants used by RegisterHotKey).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers(pub u32);

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;

/// Win32 virtual-key codes used by our bindings.
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_C: u32 = 0x43;
pub const VK_I: u32 = 0x49;
pub const VK_J: u32 = 0x4A;
pub const VK_K: u32 = 0x4B;
pub const VK_R: u32 = 0x52;
pub const VK_U: u32 = 0x55;

#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub modifiers: u32,
    pub virtual_key: u32,
    pub action: Action,
    pub menu_label: &'static str,
    pub hotkey_label: &'static str,
}

const CTRL_ALT: u32 = MOD_CONTROL | MOD_ALT;

pub const BINDINGS: &[Binding] = &[
    Binding { modifiers: CTRL_ALT, virtual_key: VK_LEFT,   action: Action::Halve(Half::Left),    menu_label: "Halve Left",          hotkey_label: "Ctrl+Alt+Left" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_RIGHT,  action: Action::Halve(Half::Right),   menu_label: "Halve Right",         hotkey_label: "Ctrl+Alt+Right" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_UP,     action: Action::Halve(Half::Top),     menu_label: "Halve Top",           hotkey_label: "Ctrl+Alt+Up" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_DOWN,   action: Action::Halve(Half::Bottom),  menu_label: "Halve Bottom",        hotkey_label: "Ctrl+Alt+Down" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_U,      action: Action::Quarter(Corner::TopLeft),     menu_label: "Quarter Top-Left",     hotkey_label: "Ctrl+Alt+U" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_I,      action: Action::Quarter(Corner::TopRight),    menu_label: "Quarter Top-Right",    hotkey_label: "Ctrl+Alt+I" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_J,      action: Action::Quarter(Corner::BottomLeft),  menu_label: "Quarter Bottom-Left",  hotkey_label: "Ctrl+Alt+J" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_K,      action: Action::Quarter(Corner::BottomRight), menu_label: "Quarter Bottom-Right", hotkey_label: "Ctrl+Alt+K" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_RETURN, action: Action::Maximize,             menu_label: "Maximize",            hotkey_label: "Ctrl+Alt+Enter" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_C,      action: Action::Center,               menu_label: "Center",              hotkey_label: "Ctrl+Alt+C" },
    Binding { modifiers: CTRL_ALT, virtual_key: VK_R,      action: Action::RestoreOriginal,      menu_label: "Restore",             hotkey_label: "Ctrl+Alt+R" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_count_matches_design() {
        assert_eq!(BINDINGS.len(), 11);
    }

    #[test]
    fn no_duplicate_modifier_plus_key() {
        for (i, a) in BINDINGS.iter().enumerate() {
            for b in BINDINGS.iter().skip(i + 1) {
                assert!(
                    !(a.modifiers == b.modifiers && a.virtual_key == b.virtual_key),
                    "duplicate binding: {:?} and {:?}",
                    a.menu_label,
                    b.menu_label
                );
            }
        }
    }

    #[test]
    fn every_action_has_a_binding() {
        let actions: std::collections::HashSet<_> = BINDINGS.iter().map(|b| b.action).collect();
        assert!(actions.contains(&Action::Halve(Half::Left)));
        assert!(actions.contains(&Action::Halve(Half::Right)));
        assert!(actions.contains(&Action::Halve(Half::Top)));
        assert!(actions.contains(&Action::Halve(Half::Bottom)));
        assert!(actions.contains(&Action::Quarter(Corner::TopLeft)));
        assert!(actions.contains(&Action::Quarter(Corner::TopRight)));
        assert!(actions.contains(&Action::Quarter(Corner::BottomLeft)));
        assert!(actions.contains(&Action::Quarter(Corner::BottomRight)));
        assert!(actions.contains(&Action::Maximize));
        assert!(actions.contains(&Action::Center));
        assert!(actions.contains(&Action::RestoreOriginal));
    }
}
```

Add to `src/lib.rs`:

```rust
pub mod adjacency;
pub mod bindings;
pub mod cycle_state;
pub mod geometry;
```

**Step 2: Run tests**

Run: `cargo test --lib bindings`
Expected: 3 passed.

**Step 3: Commit**

```bash
git add src/bindings.rs src/lib.rs
git commit -m "feat(bindings): add static BINDINGS table mapping hotkeys to actions"
```

---

## Task 6: App manifest + build script (Windows-only)

DPI awareness manifest and embedding via `embed-resource`. From here on, tasks compile **only** on Windows. Engineer should switch to a Windows machine (or Windows VM) to continue.

**Files:**
- Create: `app.manifest`
- Create: `build.rs`
- Create: `app.rc` (resource file)

**Step 1: Create `app.manifest`**

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
```

**Step 2: Create `app.rc`**

```
#define RT_MANIFEST 24
1 RT_MANIFEST "app.manifest"
```

(Icon is added in Task 11 once the tray code needs it.)

**Step 3: Create `build.rs`**

```rust
fn main() {
    #[cfg(target_os = "windows")]
    {
        embed_resource::compile("app.rc", embed_resource::NONE);
    }
}
```

**Step 4: Verify build still works**

On Windows: `cargo build`
Expected: clean compile.

On non-Windows: `cargo check` should still pass (build script no-ops on non-Windows).

**Step 5: Commit**

```bash
git add app.manifest app.rc build.rs
git commit -m "build: add PerMonitorV2 DPI manifest and resource embedding"
```

---

## Task 7: Win32 monitor enumeration

**Files:**
- Create: `src/win/mod.rs`
- Create: `src/win/monitor.rs`
- Modify: `src/lib.rs`

**Step 1: Add Win32 module to lib.rs**

```rust
pub mod adjacency;
pub mod bindings;
pub mod cycle_state;
pub mod geometry;

#[cfg(target_os = "windows")]
pub mod win;
```

**Step 2: Create `src/win/mod.rs`**

```rust
pub mod monitor;
```

**Step 3: Create `src/win/monitor.rs`**

```rust
use crate::geometry::Rect;
use anyhow::{anyhow, Result};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};

pub struct MonitorEntry {
    pub handle: HMONITOR,
    pub work_area: Rect,
    pub monitor_area: Rect,
}

pub fn enumerate_monitors() -> Result<Vec<MonitorEntry>> {
    let mut collected: Vec<MonitorEntry> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_proc),
            LPARAM(&mut collected as *mut Vec<MonitorEntry> as isize),
        )
        .ok()
        .map_err(|e| anyhow!("EnumDisplayMonitors failed: {e}"))?;
    }
    Ok(collected)
}

unsafe extern "system" fn enum_proc(
    monitor_handle: HMONITOR,
    _hdc: HDC,
    _clip_rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if GetMonitorInfoW(monitor_handle, &mut info).as_bool() {
        let collected = &mut *(lparam.0 as *mut Vec<MonitorEntry>);
        collected.push(MonitorEntry {
            handle: monitor_handle,
            work_area: rect_from_win32(info.rcWork),
            monitor_area: rect_from_win32(info.rcMonitor),
        });
    }
    BOOL(1)
}

pub fn monitor_from_window(window: HWND) -> HMONITOR {
    unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) }
}

pub fn work_area_for(monitor_handle: HMONITOR) -> Result<Rect> {
    unsafe {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor_handle, &mut info)
            .ok()
            .map_err(|e| anyhow!("GetMonitorInfoW failed: {e}"))?;
        Ok(rect_from_win32(info.rcWork))
    }
}

fn rect_from_win32(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}
```

**Step 4: Verify Windows build**

On Windows: `cargo build`
Expected: clean compile.

On macOS/Linux: `cargo check` (no `--target`) — clean compile, win module gated out.

**Step 5: Commit**

```bash
git add src/win/mod.rs src/win/monitor.rs src/lib.rs
git commit -m "feat(win): add monitor enumeration via EnumDisplayMonitors"
```

---

## Task 8: Win32 window operations

`SetWindowPos` wrapper, restore stack, `IsZoomed` toggle.

**Files:**
- Create: `src/win/window_ops.rs`
- Modify: `src/win/mod.rs`

**Step 1: Add module**

`src/win/mod.rs`:

```rust
pub mod monitor;
pub mod window_ops;
```

**Step 2: Create `src/win/window_ops.rs`**

```rust
use crate::geometry::Rect;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, IsWindow, IsZoomed, SetWindowPos, ShowWindow,
    HWND_TOP, SWP_NOACTIVATE, SWP_NOZORDER, SW_MAXIMIZE, SW_RESTORE,
};

fn hwnd_to_key(window: HWND) -> isize {
    window.0 as isize
}

static RESTORE_STACK: Mutex<Option<HashMap<isize, Rect>>> = Mutex::new(None);

fn restore_stack() -> std::sync::MutexGuard<'static, Option<HashMap<isize, Rect>>> {
    let mut guard = RESTORE_STACK.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

pub fn foreground_window() -> Option<HWND> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        None
    } else {
        Some(hwnd)
    }
}

pub fn is_valid_window(window: HWND) -> bool {
    unsafe { IsWindow(window).as_bool() }
}

pub fn current_rect(window: HWND) -> Result<Rect> {
    let mut raw = RECT::default();
    unsafe {
        GetWindowRect(window, &mut raw)
            .map_err(|e| anyhow!("GetWindowRect failed: {e}"))?;
    }
    Ok(Rect::new(
        raw.left,
        raw.top,
        raw.right - raw.left,
        raw.bottom - raw.top,
    ))
}

pub fn is_maximized(window: HWND) -> bool {
    unsafe { IsZoomed(window).as_bool() }
}

pub fn is_minimized(window: HWND) -> bool {
    unsafe { IsIconic(window).as_bool() }
}

pub fn apply_rect(window: HWND, target: Rect) -> Result<()> {
    if is_maximized(window) || is_minimized(window) {
        unsafe {
            let _ = ShowWindow(window, SW_RESTORE);
        }
    }
    capture_for_restore(window)?;
    unsafe {
        SetWindowPos(
            window,
            HWND_TOP,
            target.x,
            target.y,
            target.width,
            target.height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|e| anyhow!("SetWindowPos failed: {e}"))?;
    }
    Ok(())
}

pub fn maximize_toggle(window: HWND) -> Result<()> {
    if is_maximized(window) {
        unsafe {
            let _ = ShowWindow(window, SW_RESTORE);
        }
    } else {
        capture_for_restore(window)?;
        unsafe {
            let _ = ShowWindow(window, SW_MAXIMIZE);
        }
    }
    Ok(())
}

pub fn restore_original(window: HWND) -> Result<bool> {
    let mut guard = restore_stack();
    let map = guard.as_mut().unwrap();
    if let Some(saved) = map.remove(&hwnd_to_key(window)) {
        if is_maximized(window) || is_minimized(window) {
            unsafe {
                let _ = ShowWindow(window, SW_RESTORE);
            }
        }
        unsafe {
            SetWindowPos(
                window,
                HWND_TOP,
                saved.x,
                saved.y,
                saved.width,
                saved.height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .map_err(|e| anyhow!("SetWindowPos failed: {e}"))?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

fn capture_for_restore(window: HWND) -> Result<()> {
    let rect_now = current_rect(window)?;
    let mut guard = restore_stack();
    let map = guard.as_mut().unwrap();
    map.insert(hwnd_to_key(window), rect_now);
    Ok(())
}
```

**Step 3: Verify**

On Windows: `cargo build`
Expected: clean compile.

**Step 4: Commit**

```bash
git add src/win/mod.rs src/win/window_ops.rs
git commit -m "feat(win): add window_ops with SetWindowPos, restore stack, maximize toggle"
```

---

## Task 9: Foreground hook

Caches the real foreground window so tray clicks know which window to act on.

**Files:**
- Create: `src/win/foreground.rs`
- Modify: `src/win/mod.rs`

**Step 1: Add module**

`src/win/mod.rs`:

```rust
pub mod foreground;
pub mod monitor;
pub mod window_ops;
```

**Step 2: Create `src/win/foreground.rs`**

```rust
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongW, IsWindowVisible, EVENT_SYSTEM_FOREGROUND, GWL_STYLE,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WS_CHILD,
};

static LAST_REAL_FOREGROUND: AtomicIsize = AtomicIsize::new(0);
static OWN_WINDOWS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
// HWINEVENTHOOK wraps *mut c_void which is not Send/Sync, so we cannot keep it
// in a Mutex<Option<HWINEVENTHOOK>> static. Store the raw pointer value as
// isize via an AtomicIsize and reconstruct the handle when unhooking.
static HOOK_HANDLE_RAW: AtomicIsize = AtomicIsize::new(0);

pub fn register_own_window(window: HWND) {
    OWN_WINDOWS.lock().unwrap().push(window.0 as isize);
}

pub fn install_hook() -> Result<()> {
    // Seed the cache with whatever is foreground right now.
    unsafe {
        let initial = GetForegroundWindow();
        if !is_own(initial) && is_top_level_visible(initial) {
            LAST_REAL_FOREGROUND.store(initial.0 as isize, Ordering::Relaxed);
        }
    }

    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(foreground_change_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };

    if hook.is_invalid() {
        return Err(anyhow!("SetWinEventHook returned null"));
    }

    HOOK_HANDLE_RAW.store(hook.0 as isize, Ordering::Relaxed);
    Ok(())
}

pub fn uninstall_hook() {
    let raw = HOOK_HANDLE_RAW.swap(0, Ordering::Relaxed);
    if raw != 0 {
        let handle = HWINEVENTHOOK(raw as *mut std::ffi::c_void);
        unsafe {
            let _ = UnhookWinEvent(handle);
        }
    }
}

pub fn last_real_foreground() -> Option<HWND> {
    let raw = LAST_REAL_FOREGROUND.load(Ordering::Relaxed);
    if raw == 0 {
        None
    } else {
        Some(HWND(raw as *mut std::ffi::c_void))
    }
}

fn is_own(window: HWND) -> bool {
    OWN_WINDOWS.lock().unwrap().contains(&(window.0 as isize))
}

fn is_top_level_visible(window: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(window).as_bool() {
            return false;
        }
        let style = GetWindowLongW(window, GWL_STYLE) as u32;
        style & WS_CHILD.0 == 0
    }
}

unsafe extern "system" fn foreground_change_callback(
    _hook: HWINEVENTHOOK,
    _event: u32,
    window: HWND,
    id_object: i32,
    id_child: i32,
    _thread_id: u32,
    _event_time: u32,
) {
    // OBJID_WINDOW is 0; child events have nonzero id_child or non-window id_object.
    if id_object != 0 || id_child != 0 {
        return;
    }
    if window.is_invalid() || is_own(window) || !is_top_level_visible(window) {
        return;
    }
    LAST_REAL_FOREGROUND.store(window.0 as isize, Ordering::Relaxed);
}
```

**Step 3: Verify**

On Windows: `cargo build`
Expected: clean compile.

**Step 4: Commit**

```bash
git add src/win/mod.rs src/win/foreground.rs
git commit -m "feat(win): cache real foreground window via SetWinEventHook"
```

---

## Task 10: Hotkey registration + dispatcher

Wire `RegisterHotKey` for each binding, and a single `apply_action` dispatcher that uses geometry + adjacency + cycle_state + window_ops.

**Files:**
- Create: `src/win/hotkey.rs`
- Create: `src/win/dispatcher.rs`
- Modify: `src/win/mod.rs`

**Step 1: Add modules**

`src/win/mod.rs`:

```rust
pub mod dispatcher;
pub mod foreground;
pub mod hotkey;
pub mod monitor;
pub mod window_ops;
```

**Step 2: Create `src/win/hotkey.rs`**

```rust
use crate::bindings::BINDINGS;
use anyhow::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS};

/// Register every binding. Returns a Vec of binding indices that registered
/// successfully. Conflicts are logged and skipped.
pub fn register_all(message_window: HWND) -> Result<Vec<usize>> {
    let mut registered = Vec::new();
    for (index, binding) in BINDINGS.iter().enumerate() {
        let id = index as i32;
        let ok = unsafe {
            RegisterHotKey(
                message_window,
                id,
                HOT_KEY_MODIFIERS(binding.modifiers),
                binding.virtual_key,
            )
        };
        if ok.is_ok() {
            registered.push(index);
        } else {
            eprintln!(
                "hotkey conflict: {} ({})",
                binding.menu_label, binding.hotkey_label
            );
        }
    }
    Ok(registered)
}

pub fn unregister_all(message_window: HWND, registered: &[usize]) {
    for &index in registered {
        unsafe {
            let _ = UnregisterHotKey(message_window, index as i32);
        }
    }
}
```

**Step 3: Create `src/win/dispatcher.rs`**

```rust
use crate::adjacency::find_adjacent_monitor;
use crate::bindings::BINDINGS;
use crate::cycle_state::{CycleDecision, CycleState};
use crate::geometry::{compute_target_rect, Action};
use crate::win::foreground::last_real_foreground;
use crate::win::monitor::{enumerate_monitors, monitor_from_window, work_area_for};
use crate::win::window_ops::{
    apply_rect, current_rect, foreground_window, is_valid_window, maximize_toggle, restore_original,
};
use anyhow::Result;
use std::sync::Mutex;
use windows::Win32::Foundation::HWND;

static CYCLE_STATE: Mutex<Option<CycleState>> = Mutex::new(None);

fn cycle_state() -> std::sync::MutexGuard<'static, Option<CycleState>> {
    let mut guard = CYCLE_STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(CycleState::new());
    }
    guard
}

#[derive(Debug, Clone, Copy)]
pub enum DispatchSource {
    Hotkey,
    TrayMenu,
}

pub fn dispatch_binding_index(index: usize, source: DispatchSource) -> Result<()> {
    let action = BINDINGS[index].action;
    let target_window = match source {
        DispatchSource::Hotkey => foreground_window(),
        DispatchSource::TrayMenu => last_real_foreground(),
    };
    let Some(target_window) = target_window else {
        return Ok(()); // no target, silently no-op
    };
    if !is_valid_window(target_window) {
        return Ok(());
    }
    apply_action(target_window, action)
}

pub fn apply_action(window: HWND, action: Action) -> Result<()> {
    match action {
        Action::Maximize => {
            maximize_toggle(window)?;
            cycle_state().as_mut().unwrap().record(window.0 as i64, action, 0);
            return Ok(());
        }
        Action::RestoreOriginal => {
            restore_original(window)?;
            cycle_state().as_mut().unwrap().record(window.0 as i64, action, 0);
            return Ok(());
        }
        _ => {}
    }

    let current_monitor = monitor_from_window(window);
    let monitor_id = current_monitor.0 as i64;
    let window_id = window.0 as i64;

    let decision = {
        let guard = cycle_state();
        guard.as_ref().unwrap().decide(window_id, action, monitor_id)
    };

    let (target_work_area, recorded_monitor_id) = match (decision, action) {
        (CycleDecision::MoveToAdjacent, Action::Halve(direction)) => {
            let monitors = enumerate_monitors()?;
            let cur_work_area = work_area_for(current_monitor)?;
            let monitor_rects: Vec<_> = monitors.iter().map(|m| m.work_area).collect();
            match find_adjacent_monitor(&monitor_rects, cur_work_area, direction) {
                Some(adjacent_index) => (
                    monitors[adjacent_index].work_area,
                    monitors[adjacent_index].handle.0 as i64,
                ),
                None => return Ok(()), // no neighbor; no-op (no wrap) per design
            }
        }
        _ => (work_area_for(current_monitor)?, monitor_id),
    };

    let cur_rect = current_rect(window)?;
    let Some(target_rect) = compute_target_rect(target_work_area, action, cur_rect) else {
        return Ok(());
    };

    apply_rect(window, target_rect)?;

    cycle_state()
        .as_mut()
        .unwrap()
        .record(window_id, action, recorded_monitor_id);

    Ok(())
}
```

**Step 4: Verify build**

On Windows: `cargo build`
Expected: clean compile.

**Step 5: Commit**

```bash
git add src/win/mod.rs src/win/hotkey.rs src/win/dispatcher.rs
git commit -m "feat(win): add hotkey registration and action dispatcher with cycle logic"
```

---

## Task 11: Tray icon

System tray with menu items for each binding plus Quit.

**Files:**
- Create: `src/win/tray.rs`
- Create: `icon.ico` (placeholder — see step 1)
- Modify: `src/win/mod.rs`

**Step 1: Provide an icon**

For MVP, use any 32x32 .ico file as `icon.ico` in repo root. A blank colored square is fine. (Engineer can replace later.) On Windows:

```
# Quick way: copy any ico
copy %SystemRoot%\System32\notepad.exe nul   # placeholder demo
```

Or create one with imagemagick:
```
magick -size 32x32 xc:#3b82f6 icon.ico
```

If neither available, skip — `tray-icon` accepts a runtime-generated icon (see code below uses `tray_icon::Icon::from_rgba` with a solid blue square).

**Step 2: Add module**

`src/win/mod.rs`:

```rust
pub mod dispatcher;
pub mod foreground;
pub mod hotkey;
pub mod monitor;
pub mod tray;
pub mod window_ops;
```

**Step 3: Create `src/win/tray.rs`**

```rust
use crate::bindings::BINDINGS;
use anyhow::{anyhow, Result};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayHandle {
    _tray: TrayIcon,
    pub binding_menu_ids: Vec<MenuId>,
    pub quit_menu_id: MenuId,
}

pub fn build_tray() -> Result<TrayHandle> {
    let menu = Menu::new();
    let mut binding_menu_ids = Vec::with_capacity(BINDINGS.len());

    let mut last_kind: Option<&'static str> = None;
    for binding in BINDINGS {
        let kind = group_of(binding.menu_label);
        if last_kind.is_some() && last_kind != Some(kind) {
            menu.append(&PredefinedMenuItem::separator())?;
        }
        last_kind = Some(kind);

        let label = format!("{:<22} {}", binding.menu_label, binding.hotkey_label);
        let item = MenuItem::new(label, true, None);
        binding_menu_ids.push(item.id().clone());
        menu.append(&item)?;
    }

    menu.append(&PredefinedMenuItem::separator())?;
    let quit_item = MenuItem::new("Quit", true, None);
    let quit_menu_id = quit_item.id().clone();
    menu.append(&quit_item)?;

    let icon = solid_color_icon(0x3B, 0x82, 0xF6); // blue
    let tray = TrayIconBuilder::new()
        .with_tooltip("Window Rectangle")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .map_err(|e| anyhow!("tray build failed: {e}"))?;

    Ok(TrayHandle {
        _tray: tray,
        binding_menu_ids,
        quit_menu_id,
    })
}

pub fn next_menu_event() -> Option<MenuEvent> {
    MenuEvent::receiver().try_recv().ok()
}

fn group_of(label: &str) -> &'static str {
    if label.starts_with("Halve") {
        "halve"
    } else if label.starts_with("Quarter") {
        "quarter"
    } else {
        "misc"
    }
}

fn solid_color_icon(r: u8, g: u8, b: u8) -> Icon {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for _ in 0..(SIZE * SIZE) {
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(255);
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid rgba buffer")
}
```

**Step 4: Verify build**

On Windows: `cargo build`
Expected: clean compile.

**Step 5: Commit**

```bash
git add src/win/mod.rs src/win/tray.rs
git commit -m "feat(win): add tray icon with menu items per binding"
```

---

## Task 12: Wire up `main.rs`

Use `winit` to host a hidden message window. Receive `WM_HOTKEY` via raw event handling, check tray menu events each tick, dispatch.

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace `src/main.rs`**

```rust
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("window-rectangle is Windows-only");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    use anyhow::anyhow;
    use std::time::Duration;
    use window_rectangle::win::{
        dispatcher::{dispatch_binding_index, DispatchSource},
        foreground::{install_hook, register_own_window, uninstall_hook},
        hotkey::{register_all, unregister_all},
        tray::{build_tray, next_menu_event},
    };
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_HOTKEY, WM_QUIT,
    };
    use winit::event::Event;
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::platform::windows::EventLoopBuilderExtWindows;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::{WindowAttributes, WindowLevel};

    let event_loop = EventLoop::builder().with_any_thread(false).build()?;
    let attributes = WindowAttributes::default()
        .with_title("window-rectangle-msg")
        .with_visible(false)
        .with_window_level(WindowLevel::AlwaysOnBottom);

    let window = event_loop.create_window(attributes)?;
    let RawWindowHandle::Win32(raw) = window.window_handle()?.as_raw() else {
        return Err(anyhow!("expected Win32 window handle"));
    };
    let message_window_hwnd = HWND(raw.hwnd.get() as *mut std::ffi::c_void);

    register_own_window(message_window_hwnd);
    install_hook()?;

    let registered_indices = register_all(message_window_hwnd)?;
    let tray = build_tray()?;

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + Duration::from_millis(50),
        ));

        // Pump our own thread message queue for WM_HOTKEY (winit does not surface it).
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, message_window_hwnd, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_HOTKEY {
                    let binding_index = msg.wParam.0 as usize;
                    if let Err(err) = dispatch_binding_index(binding_index, DispatchSource::Hotkey) {
                        eprintln!("hotkey dispatch error: {err}");
                    }
                } else {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if msg.message == WM_QUIT {
                    target.exit();
                }
            }
        }

        // Drain tray menu events.
        while let Some(menu_event) = next_menu_event() {
            if menu_event.id == tray.quit_menu_id {
                target.exit();
                break;
            }
            if let Some(binding_index) = tray
                .binding_menu_ids
                .iter()
                .position(|id| *id == menu_event.id)
            {
                if let Err(err) = dispatch_binding_index(binding_index, DispatchSource::TrayMenu) {
                    eprintln!("tray dispatch error: {err}");
                }
            }
        }

        if let Event::LoopExiting = event {
            unregister_all(message_window_hwnd, &registered_indices);
            uninstall_hook();
        }
    })?;

    Ok(())
}
```

**Step 2: Build the release exe on Windows**

Run: `cargo build --release`
Expected: produces `target/release/window-rectangle.exe` around 1–2 MB.

**Step 3: Run the exe**

Run: `target\release\window-rectangle.exe`
Expected: tray icon appears (blue square). No console window in release. Hotkeys active.

If a console window appears, add to `src/main.rs` at the top (above `#[cfg]` attributes):

```rust
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
```

**Step 4: Smoke-test on Windows**

- Open Notepad.
- Press `Ctrl+Alt+Left` → Notepad takes left half.
- Press `Ctrl+Alt+Right` → right half.
- Press `Ctrl+Alt+Enter` → maximize. Press again → restore.
- Press `Ctrl+Alt+C` → centered.
- Right-click tray icon → menu lists all bindings → click one → applies to last-focused window.
- Click tray Quit → app exits, hotkeys release.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire winit event loop, hotkey pump, and tray menu dispatch"
```

---

## Task 13: Manual test checklist document

**Files:**
- Create: `TESTING.md`

**Step 1: Create `TESTING.md`**

```markdown
# Manual Test Checklist

Run these tests on a Windows 10/11 machine after every release-build.

## Single monitor

- [ ] `Ctrl+Alt+Left` halves active window to left half (taskbar still visible)
- [ ] `Ctrl+Alt+Right` halves to right half
- [ ] `Ctrl+Alt+Up` halves to top half
- [ ] `Ctrl+Alt+Down` halves to bottom half
- [ ] `Ctrl+Alt+U/I/J/K` produce correct quarter rects with no gaps at edges
- [ ] `Ctrl+Alt+Enter` toggles maximize ↔ restore
- [ ] `Ctrl+Alt+C` centers without resizing
- [ ] `Ctrl+Alt+R` returns the window to its pre-action rect

## Dual monitor (side-by-side)

- [ ] On main monitor, `Ctrl+Alt+Left` halves on main monitor
- [ ] Press `Ctrl+Alt+Left` again → window moves to left monitor and halves left there
- [ ] Press `Ctrl+Alt+Left` a third time → no-op (no further left)
- [ ] `Ctrl+Alt+Right` symmetric on the right edge

## Mixed DPI (e.g. 100% + 150%)

- [ ] Halve a window across each monitor — width/height look correct in both

## Tray menu

- [ ] Right-click tray → menu lists every binding with its hotkey label
- [ ] Alt-tab to Notepad, click tray, click "Halve Left" → Notepad halves left (not the tray)
- [ ] Click "Quit" → tray icon disappears, app exits, hotkeys no longer fire
```

**Step 2: Commit**

```bash
git add TESTING.md
git commit -m "docs: add manual test checklist for Windows verification"
```

---

## Done

After Task 13:
- All unit tests pass on any OS (`cargo test --lib` → 21 tests).
- Release build produces a single ~1–2 MB exe.
- Manual testing on Windows confirms behavior.

Out-of-scope items from design (config file, GUI settings, autostart, etc.) are deliberately not in this plan.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-05-05-window-rectangle-implementation.md`. Two execution options:

1. **Subagent-Driven (this session)** — fresh subagent per task, review between tasks, fast iteration.
2. **Parallel Session (separate)** — open new session with executing-plans, batch execution with checkpoints.

Which approach?
