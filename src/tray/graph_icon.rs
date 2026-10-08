//! @module tray::graph_icon
//! @description System Informer–style tray graph: temperature and fan-speed history in one tiny icon.
//!
//! @input  Icon size (px), a `GraphStyle` from user settings, and oldest-first sample histories.
//! @output RGBA pixels (newest sample at the right edge, one column per sample) and a tooltip.
//! @dependencies tray::raster, config::settings
//!
//! Style follows System Informer's tray graphs: solid dark background, a translucent area fill
//! under the primary series (temperature) and a crisp line on top, with the fan trace as a
//! second line in a complementary colour. Gaps in the data are left blank, never interpolated.
use crate::config::settings::{GraphIconSettings, TemperatureUnit};
use crate::tray::raster::{Canvas, Rgba};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeriesStyle {
    pub color: Rgba,
    pub fill: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphStyle {
    pub background: Rgba,
    pub temperature: Option<SeriesStyle>,
    pub fan: Option<SeriesStyle>,
    pub temp_range: (f32, f32),
    pub fan_max_rpm: f32,
}

/// Opacity of the area fill under a series.
pub const FILL_ALPHA: u8 = 110;
/// Smallest automatic fan scale, so an idle fan does not look like it is at full speed.
pub const MIN_AUTO_FAN_RPM: f32 = 3000.0;

/// Headroom above the session peak so the trace never sits on the top edge.
pub const AUTO_HEADROOM: f32 = 1.1;

/// Full-scale RPM: the configured value, or the session peak plus headroom (never below `MIN_AUTO_FAN_RPM`).
pub fn fan_scale(configured_rpm: u32, session_peak: Option<f32>) -> f32 {
    if configured_rpm > 0 {
        configured_rpm as f32
    } else {
        (session_peak.unwrap_or(0.0) * AUTO_HEADROOM).max(MIN_AUTO_FAN_RPM)
    }
}

pub fn style_from(s: &GraphIconSettings, fan_peak: Option<f32>) -> GraphStyle {
    let color = |hex: &str, fallback: Rgba| Rgba::from_hex(hex).unwrap_or(fallback);
    GraphStyle {
        background: color(&s.background_color, Rgba(0, 0, 0, 255)),
        temperature: s
            .show_temperature
            .then(|| SeriesStyle { color: color(&s.temperature_color, Rgba(255, 42, 42, 255)), fill: s.fill }),
        fan: s.show_fan.then(|| SeriesStyle { color: color(&s.fan_color, Rgba(34, 211, 238, 255)), fill: false }),
        temp_range: (s.temperature_min_c, s.temperature_max_c),
        fan_max_rpm: fan_scale(s.fan_max_rpm, fan_peak),
    }
}

/// Maps a value to a y coordinate: `lo` → bottom edge, `hi` → top edge (clamped).
pub fn y_for(value: f32, lo: f32, hi: f32, size: u32) -> f32 {
    let span = (hi - lo).max(f32::EPSILON);
    let t = ((value - lo) / span).clamp(0.0, 1.0);
    (size as f32 - 0.5) - t * (size as f32 - 1.0)
}

fn draw_series(canvas: &mut Canvas, values: &[Option<f32>], range: (f32, f32), style: SeriesStyle, size: u32) {
    let n = values.len().min(size as usize);
    let tail = &values[values.len() - n..];
    let x0 = size as i32 - n as i32;
    let pts: Vec<Option<(f32, f32)>> = tail
        .iter()
        .enumerate()
        .map(|(i, v)| v.map(|v| ((x0 + i as i32) as f32 + 0.5, y_for(v, range.0, range.1, size))))
        .collect();
    if style.fill {
        let fill = style.color.with_alpha(FILL_ALPHA);
        for (x, y) in pts.iter().flatten() {
            canvas.fill_column_from(*x as i32, *y, fill);
        }
    }
    let width = (size as f32 / 16.0).max(1.0);
    for pair in pts.windows(2) {
        match (pair[0], pair[1]) {
            (Some(a), Some(b)) => canvas.line(a, b, width, style.color),
            (Some(a), None) => canvas.line(a, a, width, style.color),
            _ => {}
        }
    }
    if let Some(Some(last)) = pts.last() {
        canvas.line(*last, *last, width, style.color);
    }
}

/// Renders the icon. `temps` in °C and `fans` in RPM, oldest first.
pub fn render(size: u32, style: &GraphStyle, temps: &[Option<f32>], fans: &[Option<f32>]) -> Vec<u8> {
    let size = size.max(8);
    let mut canvas = Canvas::new(size, size);
    canvas.fill(style.background);
    if let Some(t) = style.temperature {
        draw_series(&mut canvas, temps, style.temp_range, t, size);
    }
    if let Some(f) = style.fan {
        draw_series(&mut canvas, fans, (0.0, style.fan_max_rpm), f, size);
    }
    canvas.into_rgba()
}

/// Tooltip such as "CPU 85°C · Fan 7317 RPM" (≤ 127 chars, the shell limit).
pub fn tooltip(name: &str, temp_c: Option<f32>, unit: TemperatureUnit, rpm: Option<u32>) -> String {
    let temp =
        temp_c.map_or_else(|| format!("--{}", unit.symbol()), |t| format!("{:.0}{}", unit.convert(t), unit.symbol()));
    let fan = rpm.map_or_else(|| "Fan --".to_string(), |r| format!("Fan {r} RPM"));
    let mut s = format!("{name} {temp} · {fan}");
    s.truncate(127);
    s
}
