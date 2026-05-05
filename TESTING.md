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
