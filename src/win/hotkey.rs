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
