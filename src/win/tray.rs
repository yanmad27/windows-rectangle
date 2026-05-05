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
