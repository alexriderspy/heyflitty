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
    let panel = match app.get_webview_window("panel") {
        Some(panel) => panel,
        None => match WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
            .title("Flitty")
            .inner_size(420.0, 560.0)
            .resizable(false)
            .center()
            .build()
        {
            Ok(panel) => panel,
            Err(_) => return,
        },
    };
    let _ = panel.unminimize();
    let _ = panel.show();
    bring_to_front(&panel);
}

/// Windows refuses focus requests from background apps (the tray, or a second
/// launch handing off to us), so the panel would open behind other windows.
/// Briefly making it topmost brings it forward reliably.
fn bring_to_front(panel: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        let _ = panel.set_always_on_top(true);
        let _ = panel.set_always_on_top(false);
    }
    let _ = panel.set_focus();
}
