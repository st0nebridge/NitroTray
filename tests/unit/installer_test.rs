//! @module installer_test
//! @description The NSIS installer's inputs: `src/installer/nitrotray.ico` is the tray app icon (rewritten
//! with NITROTRAY_WRITE_FIXTURES=1), and the script uses the names the binaries use.
use nitrotray::meta;
use nitrotray::tray::app_icon;

const ICO: &str = "src/installer/nitrotray.ico";
const SCRIPT: &str = include_str!("../../src/installer/nitrotray.nsi");
const SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// One 32-bit BMP icon frame: BITMAPINFOHEADER (height doubled), bottom-up BGRA, 1-bit AND mask.
fn bmp_frame(size: u32, rgba: &[u8]) -> Vec<u8> {
    let mask_stride = size.div_ceil(32) * 4;
    let mut f = Vec::new();
    for v in [40, size, size * 2] {
        f.extend_from_slice(&v.to_le_bytes());
    }
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&32u16.to_le_bytes());
    for v in [0, size * size * 4 + mask_stride * size, 0, 0, 0, 0] {
        f.extend_from_slice(&v.to_le_bytes());
    }
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            f.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    for y in (0..size).rev() {
        let mut row = vec![0u8; mask_stride as usize];
        for x in 0..size {
            if rgba[((y * size + x) * 4 + 3) as usize] == 0 {
                row[(x / 8) as usize] |= 0x80 >> (x % 8);
            }
        }
        f.extend_from_slice(&row);
    }
    f
}

fn encode_ico(sizes: &[u32]) -> Vec<u8> {
    let frames: Vec<Vec<u8>> = sizes.iter().map(|&s| bmp_frame(s, &app_icon::render(s))).collect();
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len();
    for (&s, frame) in sizes.iter().zip(&frames) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += frame.len();
    }
    frames.iter().for_each(|f| out.extend_from_slice(f));
    out
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

#[test]
fn installer_icon_is_the_app_icon() {
    let expected = encode_ico(&SIZES);
    if std::env::var("NITROTRAY_WRITE_FIXTURES").is_ok() {
        std::fs::write(ICO, &expected).unwrap();
        return;
    }
    let actual = std::fs::read(ICO).expect("icon exists; regenerate with NITROTRAY_WRITE_FIXTURES=1");
    assert!(actual == expected, "{ICO} is out of date — rerun with NITROTRAY_WRITE_FIXTURES=1");
}

#[test]
fn icon_directory_is_well_formed() {
    let ico = std::fs::read(ICO).unwrap();
    assert_eq!((u16_at(&ico, 0), u16_at(&ico, 2), u16_at(&ico, 4)), (0, 1, SIZES.len() as u16));
    for (n, &size) in SIZES.iter().enumerate() {
        let e = 6 + 16 * n;
        assert_eq!(ico[e] as u32, size % 256, "width byte (0 means 256)");
        assert_eq!((u16_at(&ico, e + 4), u16_at(&ico, e + 6)), (1, 32), "one plane, 32 bpp");
        let (len, at) = (u32_at(&ico, e + 8) as usize, u32_at(&ico, e + 12) as usize);
        assert!(at + len <= ico.len());
        assert_eq!((u32_at(&ico, at), u32_at(&ico, at + 4), u32_at(&ico, at + 8)), (40, size, size * 2));
        let mask = size.div_ceil(32) as usize * 4 * size as usize;
        assert_eq!(len, 40 + (size * size * 4) as usize + mask);
    }
    // 32 px frame (index 3): the chevron at (16, 17) is Nitro red, stored as BGRA bottom-up.
    let at = u32_at(&ico, 6 + 16 * 3 + 12) as usize + 40;
    let px = at + ((31 - 17) * 32 + 16) * 4;
    assert_eq!(&ico[px..px + 3], &[app_icon::NITRO_RED.2, app_icon::NITRO_RED.1, app_icon::NITRO_RED.0]);
    assert!(ico[px + 3] > 200);
}

#[cfg(windows)]
#[test]
fn windows_loads_the_icon() {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, LoadImageW, HICON, IMAGE_ICON, LR_LOADFROMFILE};
    let path = HSTRING::from(std::fs::canonicalize(ICO).unwrap().as_os_str());
    for size in [16, 32, 48, 256] {
        // SAFETY: the path outlives the call; the handle is destroyed below.
        let handle = unsafe { LoadImageW(None, &path, IMAGE_ICON, size, size, LR_LOADFROMFILE) }.expect("valid .ico");
        // SAFETY: an icon handle this test owns.
        unsafe { DestroyIcon(HICON(handle.0)) }.unwrap();
    }
}

#[test]
fn script_uses_the_binaries_names() {
    for line in [
        format!("!define APP_NAME \"{}\"", meta::APP_NAME),
        format!("!define SERVICE_NAME \"{}\"", meta::SERVICE_NAME),
        "!define TRAY_EXE \"NitroTray.exe\"".to_string(),
        format!("!define HELPER_EXE \"{}\"", nitrotray::service::install_path::SERVICE_EXE),
    ] {
        assert!(SCRIPT.contains(&line), "missing: {line}");
    }
    assert!(SCRIPT.contains("'\"$INSTDIR\\${HELPER_EXE}\" install'"), "the helper registers itself");
    assert!(SCRIPT.contains("'\"$INSTDIR\\${HELPER_EXE}\" uninstall'"), "and removes itself");
    assert!(SCRIPT.contains("'\"$INSTDIR\\${TRAY_EXE}\" --exit'"), "the running tray is asked to exit");
}

#[cfg(windows)]
#[test]
fn autostart_entry_matches_the_app() {
    let written = nitrotray::platform::windows::autostart_command("$INSTDIR\\${TRAY_EXE}");
    let pattern = format!("WriteRegStr HKCU \"${{RUN_KEY}}\" \"${{APP_NAME}}\" '{written}'");
    assert!(SCRIPT.contains(&pattern), "the installer writes the Run value the app reads: {pattern}");
    assert!(SCRIPT.contains(&format!("${{If}} $0 == '{written}'")), "uninstall removes only its own entry");
    assert!(SCRIPT.contains(&format!("!define RUN_KEY \"{}\"", nitrotray::platform::windows::RUN_KEY)));
}
