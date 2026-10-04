use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

/// Tauri's always-on-top is the floating level, below menus and full-screen apps.
/// Use the screen-saver level and join every Space, including full-screen ones.
pub fn make_overlay_unobtrusive(window: &tauri::WebviewWindow) {
    let Ok(ns_window_pointer) = window.ns_window() else { return };
    let pointer_address = ns_window_pointer as usize;
    let _ = window.run_on_main_thread(move || {
        // SAFETY: Tauri hands out a live NSWindow pointer, used only on the main thread.
        let ns_window: &NSWindow = unsafe { &*(pointer_address as *const NSWindow) };
        // NSScreenSaverWindowLevel.
        ns_window.setLevel(1000);
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        ns_window.setHasShadow(false);
    });
}
