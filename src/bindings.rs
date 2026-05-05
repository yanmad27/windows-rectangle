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
