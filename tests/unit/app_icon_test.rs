//! @module app_icon_test
//! @description The Nitro shield icon: red outline, transparent inner ring, red chevron, transparent corners.
use nitrotray::tray::app_icon::{render, NITRO_RED};

fn alpha(buf: &[u8], size: u32, x: u32, y: u32) -> u8 {
    buf[((y * size + x) * 4 + 3) as usize]
}

#[test]
fn draws_the_shield_design() {
    let size = 32;
    let buf = render(size);
    assert_eq!(buf.len(), 32 * 32 * 4);
    assert_eq!(alpha(&buf, size, 0, 31), 0, "transparent corner");
    assert_eq!(alpha(&buf, size, 16, 6), 255, "outer outline");
    let i = ((6 * size + 16) * 4) as usize;
    assert_eq!((buf[i], buf[i + 1], buf[i + 2]), (NITRO_RED.0, NITRO_RED.1, NITRO_RED.2));
    assert_eq!(alpha(&buf, size, 11, 9), 0, "hole between outline and chevron");
    assert!(alpha(&buf, size, 16, 17) > 200, "chevron body");
}

#[test]
fn renders_at_tray_sizes() {
    for s in [16, 20, 24, 32, 48] {
        assert_eq!(render(s).len() as u32, s * s * 4);
    }
    assert_eq!(render(1).len(), 8 * 8 * 4);
}
