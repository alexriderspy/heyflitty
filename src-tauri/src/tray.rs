//! Menu-bar (macOS) / system-tray (Windows) icon: the app's only permanent UI.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let show_buddy = CheckMenuItem::with_id(app, "show-buddy", "Show Flitty", true, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Flitty", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_buddy, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;

    let show_buddy_for_events = show_buddy.clone();
    TrayIconBuilder::with_id("flitty")
        .icon(app.default_window_icon().cloned().expect("bundle icon"))
        .tooltip("Flitty")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show-buddy" => {
                let visible = show_buddy_for_events.is_checked().unwrap_or(true);
                let _ = app.emit("buddy-visibility", visible);
            }
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

pub fn open_settings(app: &AppHandle) {
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.show();
        let _ = panel.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
        .title("Flitty")
        .inner_size(420.0, 560.0)
        .resizable(false)
        .center()
        .build();
}
