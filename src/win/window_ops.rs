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
