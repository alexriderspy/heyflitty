//! The few things Tauri does not expose: window levels and activation behaviour.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::make_overlay_unobtrusive;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::make_overlay_unobtrusive;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn make_overlay_unobtrusive(_window: &tauri::WebviewWindow) {}
