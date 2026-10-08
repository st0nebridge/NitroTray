//! @module tray::raster
//! @description Minimal RGBA canvas for tray icons: fills, anti-aliased lines and even-odd polygons.
//!
//! @input  Drawing commands in pixel coordinates (0,0 = top-left of the top-left pixel).
//! @output A straight-alpha RGBA8 buffer suitable for `tray_icon::Icon::from_rgba`.
//! @dependencies none (pure)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba(0, 0, 0, 0);

    /// Parses `#RRGGBB` (alpha 255).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let h = hex.strip_prefix('#')?;
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255))
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Rgba(self.0, self.1, self.2, a)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Canvas {
    width: u32,
    height: u32,
    px: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, px: vec![0; (width * height * 4) as usize] }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        let i = ((y * self.width + x) * 4) as usize;
        Rgba(self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3])
    }

    pub fn into_rgba(self) -> Vec<u8> {
        self.px
    }

    pub fn fill(&mut self, c: Rgba) {
        for p in self.px.chunks_exact_mut(4) {
            p.copy_from_slice(&[c.0, c.1, c.2, c.3]);
        }
    }

    /// Source-over blend of `c` scaled by `coverage` (0..1) into one pixel; out of bounds is ignored.
    pub fn blend(&mut self, x: i32, y: i32, c: Rgba, coverage: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let a = (f32::from(c.3) / 255.0) * coverage.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let da = f32::from(self.px[i + 3]) / 255.0;
        let out_a = a + da * (1.0 - a);
        for (k, sc) in [c.0, c.1, c.2].into_iter().enumerate() {
            let dc = f32::from(self.px[i + k]);
            let v = (f32::from(sc) * a + dc * da * (1.0 - a)) / out_a.max(f32::EPSILON);
            self.px[i + k] = v.round().clamp(0.0, 255.0) as u8;
        }
        self.px[i + 3] = (out_a * 255.0).round() as u8;
    }

    /// Vertical span in one column, from `y_top` (fractional, anti-aliased) down to the bottom.
    pub fn fill_column_from(&mut self, x: i32, y_top: f32, c: Rgba) {
        let h = self.height as i32;
        let top = y_top.clamp(0.0, h as f32);
        let first = top.floor() as i32;
        for y in first..h {
            let coverage = if y == first { 1.0 - (top - first as f32) } else { 1.0 };
            self.blend(x, y, c, coverage);
        }
    }

    /// Anti-aliased line of the given width (distance-to-segment coverage).
    pub fn line(&mut self, (x0, y0): (f32, f32), (x1, y1): (f32, f32), width: f32, c: Rgba) {
        let half = width / 2.0;
        let (minx, maxx) = (x0.min(x1) - half - 1.0, x0.max(x1) + half + 1.0);
        let (miny, maxy) = (y0.min(y1) - half - 1.0, y0.max(y1) + half + 1.0);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = dx * dx + dy * dy;
        for py in (miny.floor() as i32)..=(maxy.ceil() as i32) {
            for px in (minx.floor() as i32)..=(maxx.ceil() as i32) {
                let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
                let t = if len2 > 0.0 { (((cx - x0) * dx + (cy - y0) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
                let (qx, qy) = (x0 + t * dx, y0 + t * dy);
                let d = ((cx - qx).powi(2) + (cy - qy).powi(2)).sqrt();
                let coverage = (half + 0.5 - d).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    self.blend(px, py, c, coverage);
                }
            }
        }
    }

    /// Fills polygons with the even-odd rule (so an inner ring cuts a hole), 4×4 supersampled.
    pub fn fill_polygons(&mut self, rings: &[Vec<(f32, f32)>], c: Rgba) {
        const S: usize = 4;
        for py in 0..self.height as i32 {
            for px in 0..self.width as i32 {
                let mut inside = 0;
                for sy in 0..S {
                    for sx in 0..S {
                        let x = px as f32 + (sx as f32 + 0.5) / S as f32;
                        let y = py as f32 + (sy as f32 + 0.5) / S as f32;
                        if even_odd(rings, x, y) {
                            inside += 1;
                        }
                    }
                }
                if inside > 0 {
                    self.blend(px, py, c, inside as f32 / (S * S) as f32);
                }
            }
        }
    }
}

fn even_odd(rings: &[Vec<(f32, f32)>], x: f32, y: f32) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (xi, yi) = ring[i];
            let (xj, yj) = ring[(i + n - 1) % n];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                inside = !inside;
            }
        }
    }
    inside
}
