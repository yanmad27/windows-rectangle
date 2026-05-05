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
