use crate::geometry::Rect;
use anyhow::{anyhow, Result};
use std::cell::RefCell;
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
    thread_local! {
        static COLLECTED: RefCell<Vec<MonitorEntry>> = RefCell::new(Vec::new());
    }

    COLLECTED.with(|cell| cell.borrow_mut().clear());

    unsafe extern "system" fn enum_proc(
        monitor_handle: HMONITOR,
        _hdc: HDC,
        _clip_rect: *mut RECT,
        _lparam: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor_handle, &mut info).as_bool() {
            COLLECTED.with(|cell| {
                cell.borrow_mut().push(MonitorEntry {
                    handle: monitor_handle,
                    work_area: rect_from_win32(info.rcWork),
                    monitor_area: rect_from_win32(info.rcMonitor),
                });
            });
        }
        BOOL(1)
    }

    unsafe {
        let ok = EnumDisplayMonitors(None, None, Some(enum_proc), LPARAM(0));
        if !ok.as_bool() {
            return Err(anyhow!("EnumDisplayMonitors failed"));
        }
    }

    Ok(COLLECTED.with(|cell| cell.replace(Vec::new())))
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
        if !GetMonitorInfoW(monitor_handle, &mut info).as_bool() {
            return Err(anyhow!("GetMonitorInfoW failed"));
        }
        Ok(rect_from_win32(info.rcWork))
    }
}

fn rect_from_win32(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}
