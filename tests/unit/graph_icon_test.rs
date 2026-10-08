//! @module graph_icon_test
//! @description System Informer–style tray graph rendering, scaling and tooltips.
use nitrotray::config::settings::{GraphIconSettings, TemperatureUnit};
use nitrotray::tray::graph_icon::*;
use nitrotray::tray::raster::Rgba;

fn px(buf: &[u8], size: u32, x: u32, y: u32) -> Rgba {
    let i = ((y * size + x) * 4) as usize;
    Rgba(buf[i], buf[i + 1], buf[i + 2], buf[i + 3])
}

#[test]
fn fan_scale_prefers_configuration_then_session_peak_with_headroom() {
    assert_eq!(fan_scale(8000, Some(9999.0)), 8000.0);
    assert_eq!(fan_scale(0, Some(7000.0)), 7000.0 * AUTO_HEADROOM);
    assert_eq!(fan_scale(0, Some(500.0)), MIN_AUTO_FAN_RPM);
    assert_eq!(fan_scale(0, None), MIN_AUTO_FAN_RPM);
}

#[test]
fn style_reflects_settings() {
    let mut s = GraphIconSettings::cpu_default();
    let style = style_from(&s, Some(7000.0));
    assert_eq!(style.background, Rgba(0, 0, 0, 255));
    assert_eq!(style.temperature.unwrap().color, Rgba(0xFF, 0x2A, 0x2A, 255));
    assert!(style.temperature.unwrap().fill && !style.fan.unwrap().fill);
    assert_eq!(style.temp_range, (30.0, 100.0));
    s.show_fan = false;
    s.fill = false;
    s.temperature_color = "bogus".into();
    let style = style_from(&s, None);
    assert!(style.fan.is_none());
    assert!(!style.temperature.unwrap().fill);
    assert_eq!(style.temperature.unwrap().color, Rgba(255, 42, 42, 255), "invalid colour falls back");
    s.show_temperature = false;
    assert!(style_from(&s, None).temperature.is_none());
}

#[test]
fn y_mapping_spans_the_icon() {
    assert_eq!(y_for(30.0, 30.0, 100.0, 16), 15.5);
    assert_eq!(y_for(100.0, 30.0, 100.0, 16), 0.5);
    assert_eq!(y_for(200.0, 30.0, 100.0, 16), 0.5, "clamped");
    assert_eq!(y_for(0.0, 30.0, 100.0, 16), 15.5, "clamped");
}

#[test]
fn renders_background_fill_and_lines() {
    let style = style_from(&GraphIconSettings::cpu_default(), Some(6000.0));
    let temps = vec![Some(65.0); 16];
    let fans = vec![Some(6000.0 * AUTO_HEADROOM / 2.0); 16];
    let buf = render(16, &style, &temps, &fans);
    assert_eq!(buf.len(), 16 * 16 * 4);
    assert_eq!(px(&buf, 16, 0, 0), Rgba(0, 0, 0, 255), "black above the traces");
    let under = px(&buf, 16, 8, 14);
    assert!(under.0 > 60 && under.1 < 40, "translucent red fill under the temperature: {under:?}");
    let fan_row = y_for(3300.0, 0.0, 6600.0, 16) as u32;
    let fan_px = px(&buf, 16, 8, fan_row);
    assert!(fan_px.2 > 150, "cyan fan trace: {fan_px:?}");
}

#[test]
fn newest_sample_is_at_the_right_and_gaps_stay_blank() {
    let mut s = GraphIconSettings::cpu_default();
    s.show_fan = false;
    let style = style_from(&s, None);
    let buf = render(16, &style, &[Some(95.0)], &[]);
    assert!(px(&buf, 16, 15, 14).0 > 60, "single sample drawn in the last column");
    assert_eq!(px(&buf, 16, 0, 14), Rgba(0, 0, 0, 255), "older columns empty");
    let gaps = render(16, &style, &[Some(90.0), None, None, Some(90.0)], &[]);
    assert_eq!(px(&gaps, 16, 13, 15), Rgba(0, 0, 0, 255), "no interpolation across a gap");
}

#[test]
fn scales_to_high_dpi_and_minimum_size() {
    let style = style_from(&GraphIconSettings::gpu_default(), None);
    assert_eq!(render(32, &style, &[Some(70.0); 40], &[Some(4000.0); 40]).len(), 32 * 32 * 4);
    assert_eq!(render(2, &style, &[], &[]).len(), 8 * 8 * 4);
}

#[test]
fn tooltips() {
    assert_eq!(tooltip("CPU", Some(85.4), TemperatureUnit::Celsius, Some(7317)), "CPU 85°C · Fan 7317 RPM");
    assert_eq!(tooltip("GPU", Some(100.0), TemperatureUnit::Fahrenheit, None), "GPU 212°F · Fan --");
    assert_eq!(tooltip("GPU", None, TemperatureUnit::Celsius, None), "GPU --°C · Fan --");
    assert!(tooltip(&"x".repeat(300), None, TemperatureUnit::Celsius, None).len() <= 127);
}

#[test]
fn trace_width_scales_with_icon_size() {
    let mut s = GraphIconSettings::cpu_default();
    s.show_temperature = false;
    let style = style_from(&s, Some(6000.0));
    let lit = |size: u32| {
        let buf = render(size, &style, &[], &vec![Some(3000.0); size as usize]);
        (0..size).filter(|&y| px(&buf, size, size / 2, y).2 > 40).count()
    };
    assert!(lit(16) <= 2, "one-pixel trace at 100 %");
    assert!(lit(48) >= 3, "three-pixel trace at 300 %: {}", lit(48));
}
