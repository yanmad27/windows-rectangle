#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

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
