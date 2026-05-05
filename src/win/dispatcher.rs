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
