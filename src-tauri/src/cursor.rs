//! Streams the mouse position to every overlay at ~60 Hz so the buddy can follow it.

use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::overlay::label_for;
use crate::screens::Screen;

#[derive(Clone, Serialize)]
struct CursorPayload {
    x: f64,
    y: f64,
    inside: bool,
}

pub fn start_stream(app: AppHandle, screens: Vec<Screen>) {
    thread::spawn(move || loop {
        let frame_started = Instant::now();
        if let Ok(position) = app.cursor_position() {
            for screen in &screens {
                let (x, y) = screen.to_local_css(position.x, position.y);
                let payload = CursorPayload { x, y, inside: screen.contains(position.x, position.y) };
                let _ = app.emit_to(label_for(screen.index), "cursor", payload);
            }
        }
        thread::sleep(Duration::from_millis(16).saturating_sub(frame_started.elapsed()));
    });
}
