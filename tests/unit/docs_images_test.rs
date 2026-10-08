//! @module docs_images_test
//! @description `docs/images/tray-icons@6x.png` is rendered by the tray's own icon code (CPU graph | GPU graph
//! | app icon at the 150 % tray size, magnified 6x without smoothing) from fixed, realistic history, so the
//! README image always matches what the notification area shows. Rewritten with NITROTRAY_WRITE_FIXTURES=1.
use nitrotray::config::settings::GraphIconSettings;
use nitrotray::tray::{app_icon, graph_icon};

const IMAGE: &str = "docs/images/tray-icons@6x.png";
/// Tray icon edge at 150 % scaling, the AN515-58's default.
const ICON: u32 = 24;
const SCALE: u32 = 6;
const PAD: u32 = 6;
const GAP: u32 = 8;
/// Windows 11 dark taskbar.
const TASKBAR: [u8; 4] = [0x1c, 0x1c, 0x1c, 0xff];

/// Temperatures (°C) and fan speeds (RPM), oldest first.
type Series = (Vec<Option<f32>>, Vec<Option<f32>>);

/// Deterministic history shaped like the measured readings: CPU package bursting 60-92 °C with the fan
/// following, GPU steady in the 50s.
fn history() -> [Series; 2] {
    let (mut ct, mut cf, mut gt, mut gf) = (vec![], vec![], vec![], vec![]);
    let mut fan = 3200.0f32;
    for i in 0..ICON {
        let burst = (i as f32 / 3.0).sin() > 0.2;
        ct.push(Some(if burst { 86.0 + (i % 3) as f32 * 3.0 } else { 61.0 + (i % 4) as f32 * 2.0 }));
        fan += ((if burst { 4400.0 } else { 3200.0 }) - fan) * 0.25;
        cf.push(Some(fan));
        gt.push(Some(52.0 + (i as f32 / 6.0).sin() * 3.0));
        gf.push(Some(3600.0 + (i as f32 / 6.0).sin() * 200.0));
    }
    [(ct, cf), (gt, gf)]
}

fn render_image() -> (u32, u32, Vec<u8>) {
    let [(ct, cf), (gt, gf)] = history();
    let peak = |f: &[Option<f32>]| f.iter().flatten().copied().reduce(f32::max);
    let icons = [
        graph_icon::render(ICON, &graph_icon::style_from(&GraphIconSettings::cpu_default(), peak(&cf)), &ct, &cf),
        graph_icon::render(ICON, &graph_icon::style_from(&GraphIconSettings::gpu_default(), peak(&gf)), &gt, &gf),
        app_icon::render(ICON),
    ];
    let w = PAD * 2 + ICON * 3 + GAP * 2;
    let h = PAD * 2 + ICON;
    let mut px = TASKBAR.repeat((w * h) as usize);
    for (n, icon) in icons.iter().enumerate() {
        let x0 = PAD + n as u32 * (ICON + GAP);
        for y in 0..ICON {
            for x in 0..ICON {
                let s = ((y * ICON + x) * 4) as usize;
                let d = (((PAD + y) * w + x0 + x) * 4) as usize;
                let a = u32::from(icon[s + 3]);
                for c in 0..3 {
                    px[d + c] = ((u32::from(icon[s + c]) * a + u32::from(px[d + c]) * (255 - a)) / 255) as u8;
                }
            }
        }
    }
    let (sw, sh) = (w * SCALE, h * SCALE);
    let mut out = Vec::with_capacity((sw * sh * 4) as usize);
    for y in 0..sh {
        for x in 0..sw {
            let s = (((y / SCALE) * w + x / SCALE) * 4) as usize;
            out.extend_from_slice(&px[s..s + 4]);
        }
    }
    (sw, sh, out)
}

fn encode(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut enc = png::Encoder::new(&mut bytes, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().unwrap();
    writer.write_image_data(rgba).unwrap();
    writer.finish().unwrap();
    bytes
}

#[test]
fn tray_icons_image_is_rendered_by_the_icon_code() {
    let (w, h, rgba) = render_image();
    assert_eq!((w, h), ((PAD * 2 + ICON * 3 + GAP * 2) * SCALE, (PAD * 2 + ICON) * SCALE));
    let png = encode(w, h, &rgba);
    if std::env::var("NITROTRAY_WRITE_FIXTURES").is_ok() {
        std::fs::write(IMAGE, &png).unwrap();
        return;
    }
    let actual = std::fs::read(IMAGE).expect("image exists; regenerate with NITROTRAY_WRITE_FIXTURES=1");
    assert!(actual == png, "{IMAGE} is out of date — rerun with NITROTRAY_WRITE_FIXTURES=1");
}
