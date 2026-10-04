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

/// Sends when the mouse moves, only to overlays it is on or just left, plus a
/// once-a-second refresh so a freshly loaded overlay learns where the mouse is.
pub fn start_stream(app: AppHandle, screens: Vec<Screen>) {
    thread::spawn(move || {
        let mut last_position: Option<(f64, f64)> = None;
        let mut inside_last_frame = vec![false; screens.len()];
        let mut frames_since_send = 0u32;
        loop {
            let frame_started = Instant::now();
            frames_since_send += 1;
            if let Ok(position) = app.cursor_position() {
                if last_position != Some((position.x, position.y)) || frames_since_send >= 60 {
                    frames_since_send = 0;
                    last_position = Some((position.x, position.y));
                    for (slot, screen) in screens.iter().enumerate() {
                        let inside = screen.contains(position.x, position.y);
                        if inside || inside_last_frame[slot] {
                            let (x, y) = screen.to_local_css(position.x, position.y);
                            let _ = app.emit_to(label_for(screen.index), "cursor", CursorPayload { x, y, inside });
                        }
                        inside_last_frame[slot] = inside;
                    }
                }
            }
            thread::sleep(Duration::from_millis(16).saturating_sub(frame_started.elapsed()));
        }
    });
}
