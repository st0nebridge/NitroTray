//! @module cu_20260930_009
//! @description Regression test: the automatic fan scale leaves headroom above the session peak, so a fan at its
//! observed maximum is drawn below the icon's top edge rather than pinned to it.
use nitrotray::config::settings::GraphIconSettings;
use nitrotray::tray::graph_icon::{fan_scale, render, style_from, y_for};

#[test]
fn peak_fan_speed_is_drawn_below_the_top_edge() {
    let scale = fan_scale(0, Some(7700.0));
    assert!(scale > 7700.0);
    assert!(y_for(7700.0, 0.0, scale, 16) > 1.5, "not on the top row");
    let mut s = GraphIconSettings::gpu_default();
    s.show_temperature = false;
    let style = style_from(&s, Some(7700.0));
    let buf = render(16, &style, &[], &[Some(7700.0); 16]);
    // Background is black; the GPU fan trace is green, so any trace pixel on row 0 raises its green channel.
    let top_row_green: u32 = (0..16).map(|x| u32::from(buf[(x * 4 + 1) as usize])).sum();
    assert_eq!(top_row_green, 0, "top row stays background");
}
