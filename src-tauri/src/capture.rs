//! Screenshots of every display, downscaled and JPEG-encoded for a vision model.

use std::io::Cursor;
use std::time::Instant;

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};

/// Longest edge sent to the model. Larger images cost more and point no better.
const MAX_EDGE_PIXELS: u32 = 1280;

pub struct ScreenCapture {
    pub screen_index: usize,
    /// Size of the image the model sees; pointing coordinates come back in this space.
    pub image_width: u32,
    pub image_height: u32,
    pub jpeg: Vec<u8>,
}

pub fn capture_all() -> Result<Vec<ScreenCapture>, String> {
    let started = Instant::now();
    let monitors = xcap::Monitor::all().map_err(|error| error.to_string())?;
    let mut captures = Vec::with_capacity(monitors.len());
    for (screen_index, monitor) in monitors.iter().enumerate() {
        let raw = monitor.capture_image().map_err(|error| error.to_string())?;
        let image = DynamicImage::ImageRgba8(raw);
        let resized = if image.width().max(image.height()) > MAX_EDGE_PIXELS {
            image.resize(MAX_EDGE_PIXELS, MAX_EDGE_PIXELS, FilterType::Triangle)
        } else {
            image
        };
        let mut jpeg = Vec::new();
        DynamicImage::ImageRgb8(resized.to_rgb8())
            .write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg)
            .map_err(|error| error.to_string())?;
        captures.push(ScreenCapture {
            screen_index,
            image_width: resized.width(),
            image_height: resized.height(),
            jpeg,
        });
    }
    println!("[flitty] captured {} screen(s) in {:?}", captures.len(), started.elapsed());
    Ok(captures)
}
