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

    let icon = load_app_icon()?;
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

/// Load the embedded ICON resource (id 2 in app.rc) as the tray icon.
fn load_app_icon() -> Result<Icon> {
    Icon::from_resource(2, None).map_err(|e| anyhow!("icon load failed: {e}"))
}
