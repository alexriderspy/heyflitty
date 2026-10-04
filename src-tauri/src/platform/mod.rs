//! The few things Tauri does not expose: window levels and activation behaviour.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::make_overlay_unobtrusive;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{keep_overlay_on_top, make_overlay_unobtrusive};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn make_overlay_unobtrusive(_window: &tauri::WebviewWindow) {}

/// macOS keeps the screen-saver level on its own; only Windows needs re-raising.
#[cfg(not(target_os = "windows"))]
pub fn keep_overlay_on_top(_window: &tauri::WebviewWindow) {}
