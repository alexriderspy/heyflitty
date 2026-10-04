//! Display geometry, in physical pixels and global desktop coordinates.

use tauri::AppHandle;

#[derive(Clone, Debug)]
pub struct Screen {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

impl Screen {
    pub fn contains(&self, global_x: f64, global_y: f64) -> bool {
        global_x >= self.x as f64
            && global_x < (self.x + self.width as i32) as f64
            && global_y >= self.y as f64
            && global_y < (self.y + self.height as i32) as f64
    }

    /// Converts a global physical point into CSS pixels inside this screen's overlay.
    pub fn to_local_css(&self, global_x: f64, global_y: f64) -> (f64, f64) {
        ((global_x - self.x as f64) / self.scale, (global_y - self.y as f64) / self.scale)
    }

    pub fn css_width(&self) -> f64 {
        self.width as f64 / self.scale
    }
}

pub fn all(app: &AppHandle) -> Vec<Screen> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, monitor)| Screen {
            index,
            x: monitor.position().x,
            y: monitor.position().y,
            width: monitor.size().width,
            height: monitor.size().height,
            scale: monitor.scale_factor(),
        })
        .collect()
}

pub fn under_cursor(app: &AppHandle, screens: &[Screen]) -> Option<Screen> {
    let position = app.cursor_position().ok()?;
    screens.iter().find(|screen| screen.contains(position.x, position.y)).cloned()
}
