//! @module tray::app_icon
//! @description The NitroTray application icon: a red inverted-shield chevron in the NitroSense style.
//!
//! @input  Pixel size.
//! @output RGBA pixels (transparent background), drawn from a 32-unit vector design.
//! @dependencies tray::raster
use crate::tray::raster::{Canvas, Rgba};

pub const NITRO_RED: Rgba = Rgba(0xFF, 0x25, 0x2D, 0xFF);

/// Design in a 32×32 unit box: an inverted-triangle outline with a chevron inside.
fn design() -> Vec<Vec<(f32, f32)>> {
    vec![
        // Outer triangle
        vec![(2.5, 5.0), (29.5, 5.0), (16.0, 29.0)],
        // Inner cut-out (even-odd makes it a hole → outline)
        vec![(8.2, 8.3), (23.8, 8.3), (16.0, 22.2)],
        // Chevron inside the hole
        vec![(10.6, 10.2), (13.9, 10.2), (16.0, 14.1), (18.1, 10.2), (21.4, 10.2), (16.0, 19.8)],
    ]
}

pub fn render(size: u32) -> Vec<u8> {
    let size = size.max(8);
    let k = size as f32 / 32.0;
    let rings: Vec<Vec<(f32, f32)>> =
        design().into_iter().map(|ring| ring.into_iter().map(|(x, y)| (x * k, y * k)).collect()).collect();
    let mut canvas = Canvas::new(size, size);
    canvas.fill_polygons(&rings, NITRO_RED);
    canvas.into_rgba()
}
