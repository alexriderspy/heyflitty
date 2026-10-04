//! One transparent, click-through, always-on-top window per display.
//! The buddy is drawn in whichever overlay the mouse is on.

use serde::Serialize;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::platform;
use crate::screens::Screen;

pub fn label_for(index: usize) -> String {
    format!("overlay-{index}")
}

/// What the buddy is doing; drives the waveform, spinner and speaking states.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VoiceState {
    Idle,
    Listening,
    Processing,
    Responding,
}

#[derive(Clone, Serialize)]
pub struct PointTarget {
    /// CSS pixels inside the target overlay.
    pub x: f64,
    pub y: f64,
    pub label: String,
}

pub fn create_all(app: &AppHandle, screens: &[Screen]) -> tauri::Result<()> {
    for screen in screens {
        let window = WebviewWindowBuilder::new(
            app,
            label_for(screen.index),
            WebviewUrl::App(format!("overlay.html?screen={}", screen.index).into()),
        )
        .title("Flitty overlay")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible_on_all_workspaces(true)
        .build()?;

        // Physical units so mixed-DPI setups line up exactly.
        window.set_position(PhysicalPosition::new(screen.x, screen.y))?;
        window.set_size(PhysicalSize::new(screen.width, screen.height))?;
        window.set_ignore_cursor_events(true)?;
        platform::make_overlay_unobtrusive(&window);
    }
    Ok(())
}

pub fn set_voice_state(app: &AppHandle, state: VoiceState) {
    let _ = app.emit("voice-state", state);
}

pub fn point_at(app: &AppHandle, screen: &Screen, target: PointTarget) {
    let _ = app.emit_to(label_for(screen.index), "point", target);
}

/// Periodically re-raises every overlay above other topmost windows.
pub fn start_topmost_guard(app: AppHandle, screens: Vec<Screen>) {
    thread::spawn(move || loop {
        for screen in &screens {
            if let Some(window) = app.get_webview_window(&label_for(screen.index)) {
                platform::keep_overlay_on_top(&window);
            }
        }
        thread::sleep(Duration::from_millis(250));
    });
}
