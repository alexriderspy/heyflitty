//! Screenshots of every display, downscaled and JPEG-encoded for a vision model.

use std::io::Cursor;
use std::time::Instant;

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};

/// Longest edge sent to the model. Larger images cost more and point no better.
const MAX_EDGE_PIXELS: u32 = 1280;
/// Side of the full-resolution close-up around the mouse, so small things under it stay legible.
const CLOSEUP_PIXELS: u32 = 480;

pub struct ScreenCapture {
    /// Monitor origin as xcap reports it: physical pixels on Windows, points on macOS.
    pub monitor_x: i32,
    pub monitor_y: i32,
    /// Size of the image the model sees; pointing coordinates come back in this space.
    pub image_width: u32,
    pub image_height: u32,
    pub jpeg: Vec<u8>,
    /// Full-resolution crop centred on the mouse, only on the screen the mouse is on.
    pub closeup: Option<Vec<u8>>,
}

/// `mouse` is the pointer in global physical pixels, for the close-up.
pub fn capture_all(mouse: Option<(f64, f64)>) -> Result<Vec<ScreenCapture>, String> {
    let started = Instant::now();
    let monitors = xcap::Monitor::all().map_err(|error| error.to_string())?;
    let mut captures = Vec::with_capacity(monitors.len());
    for (screen_index, monitor) in monitors.iter().enumerate() {
        let monitor_x = monitor.x().map_err(|error| error.to_string())?;
        let monitor_y = monitor.y().map_err(|error| error.to_string())?;
        let grab_started = Instant::now();
        let raw = monitor.capture_image().map_err(|error| error.to_string())?;
        let grab_time = grab_started.elapsed();
        // xcap reports macOS monitors in points but captures pixels; this ratio maps one to the other.
        let scale = raw.width() as f64 / monitor.width().map_err(|error| error.to_string())?.max(1) as f64;
        let image = DynamicImage::ImageRgba8(raw);
        let closeup = mouse.and_then(|(x, y)| closeup(&image, x - monitor_x as f64 * scale, y - monitor_y as f64 * scale));
        let resized = if image.width().max(image.height()) > MAX_EDGE_PIXELS {
            image.resize(MAX_EDGE_PIXELS, MAX_EDGE_PIXELS, FilterType::Triangle)
        } else {
            image
        };
        let jpeg = encode(&resized)?;
        println!("[flitty] screen {screen_index}: grab {grab_time:?}, total {:?}", grab_started.elapsed());
        captures.push(ScreenCapture {
            monitor_x,
            monitor_y,
            image_width: resized.width(),
            image_height: resized.height(),
            jpeg,
            closeup,
        });
    }
    println!("[flitty] captured {} screen(s) in {:?}", captures.len(), started.elapsed());
    Ok(captures)
}

fn encode(image: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut jpeg = Vec::new();
    DynamicImage::ImageRgb8(image.to_rgb8()).write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg).map_err(|error| error.to_string())?;
    Ok(jpeg)
}

/// Crops around a point (in this image's pixels) and rings the point, since screenshots leave the cursor out.
fn closeup(image: &DynamicImage, x: f64, y: f64) -> Option<Vec<u8>> {
    let (width, height) = (image.width(), image.height());
    if x < 0.0 || y < 0.0 || x >= width as f64 || y >= height as f64 {
        return None;
    }
    let side = CLOSEUP_PIXELS.min(width).min(height);
    let left = (x as i64 - side as i64 / 2).clamp(0, (width - side) as i64) as u32;
    let top = (y as i64 - side as i64 / 2).clamp(0, (height - side) as i64) as u32;
    let mut crop = image.crop_imm(left, top, side, side).to_rgb8();
    let (cx, cy) = (x - left as f64, y - top as f64);
    for step in 0..360 {
        let angle = (step as f64).to_radians();
        for radius in [17.0, 18.0, 19.0] {
            let (px, py) = ((cx + radius * angle.cos()).round(), (cy + radius * angle.sin()).round());
            if px >= 0.0 && py >= 0.0 && (px as u32) < side && (py as u32) < side {
                crop.put_pixel(px as u32, py as u32, image::Rgb([255, 40, 40]));
            }
        }
    }
    encode(&DynamicImage::ImageRgb8(crop)).ok()
}
