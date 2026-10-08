//! @module ui::webview
//! @description Builds the popup, hover flyout and full windows with their WebView2 pages, and applies
//! Windows 11 chrome (DWM rounded corners, border colour, dark title bar, backdrop).
//!
//! @input  The tao event-loop target, a page-message callback, a shared WebContext.
//! @output `Surface`s (window + webview); page messages are handed to the callback.
//! @dependencies tao, wry, webview2-com, windows (Dwm, WindowsAndMessaging, HiDpi), ui::{assets, flyout}, tray::app_icon
use std::borrow::Cow;

use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopWindowTarget;
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Window, WindowBuilder};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_BORDER_COLOR, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWM_SYSTEMBACKDROP_TYPE,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use wry::http::{header, Request, Response, StatusCode};
use wry::{WebContext, WebView, WebViewBuilder};

use crate::ui::assets;

/// Popup size in DIP (reference: 572 × 439.5 at 2x; target range 520–580 × 410–470).
pub const POPUP_SIZE: (f64, f64) = (572.0, 440.0);
pub const FULL_SIZE: (f64, f64) = (1040.0, 720.0);

pub struct Surface {
    pub window: Window,
    pub webview: WebView,
}

impl Surface {
    pub fn hwnd(&self) -> HWND {
        HWND(self.window.hwnd() as *mut _)
    }

    pub fn eval(&self, js: &str) {
        let _ = self.webview.evaluate_script(js);
    }

    /// Hidden surfaces stop rendering and ask WebView2 to trim memory; shown
    /// surfaces resume at normal priority.
    pub fn set_hidden(&self, hidden: bool) {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        };
        use windows::core::Interface;
        use wry::WebViewExtWindows;
        let _ = self.webview.set_visible(!hidden);
        let level = if hidden {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
        } else {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
        };
        // SAFETY: COM calls on the controller wry owns; ICoreWebView2_19 is optional (older runtimes).
        unsafe {
            if let Ok(core) = self.webview.controller().CoreWebView2() {
                if let Ok(v19) = core.cast::<ICoreWebView2_19>() {
                    let _ = v19.SetMemoryUsageTargetLevel(level);
                }
            }
        }
    }
}

fn serve(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    match assets::lookup(request.uri().path()) {
        Some(a) => Response::builder()
            .header(header::CONTENT_TYPE, a.mime)
            .header("Content-Security-Policy", assets::CSP)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Cow::Borrowed(a.bytes))
            .unwrap_or_else(|_| Response::new(Cow::Borrowed(&[]))),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(&b"not found"[..]))
            .unwrap_or_else(|_| Response::new(Cow::Borrowed(&[]))),
    }
}

fn build_webview(
    window: &Window,
    context: &mut WebContext,
    entry: &str,
    on_message: impl Fn(String) + 'static,
    transparent: bool,
) -> wry::Result<WebView> {
    WebViewBuilder::new_with_web_context(context)
        .with_custom_protocol(assets::SCHEME.to_string(), move |_id, req| serve(req))
        .with_url(format!("{}://localhost/{entry}", assets::SCHEME))
        .with_ipc_handler(move |req: Request<String>| on_message(req.into_body()))
        .with_background_color((13, 18, 21, if transparent { 0 } else { 255 }))
        .with_transparent(transparent)
        .with_devtools(cfg!(debug_assertions))
        .with_hotkeys_zoom(false)
        .with_accept_first_mouse(true)
        // Created hidden: a visible controller keeps Chromium compositing (CSS animations included)
        // behind a hidden window. Surfaces toggle it together with their window.
        .with_visible(false)
        .build(window)
}

/// Makes Windows apply tao's borderless frame now, while the window is still hidden. Until the first
/// WM_NCCALCSIZE, the client area still excludes a caption, so `set_inner_size` measures a ~30 px
/// "frame" that is not there once shown and the first open comes out that much taller than its
/// computed placement (overlapping the taskbar).
pub fn settle_frame(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };
    // SAFETY: a style-preserving frame refresh of a window this process owns.
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );
    }
}

fn dwm<T>(hwnd: HWND, attr: windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE, value: &T) {
    // SAFETY: passes a pointer/size pair describing `value`.
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, attr, (value as *const T).cast(), std::mem::size_of::<T>() as u32);
    }
}

/// Rounded corners, border colour and dark/light chrome (rectangular, no notch).
pub fn apply_chrome(hwnd: HWND, dark: bool, rounded_border: bool, backdrop: bool) {
    let dark_flag: i32 = dark.into();
    dwm(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark_flag);
    if rounded_border {
        dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWM_WINDOW_CORNER_PREFERENCE(DWMWCP_ROUND.0));
        // COLORREF 0x00BBGGRR — a light slate border like the reference frame.
        let border: u32 = if dark { 0x0074_6B64 } else { 0x00C8_C2BC };
        dwm(hwnd, DWMWA_BORDER_COLOR, &border);
    }
    let kind = if backdrop { DWMSBT_TRANSIENTWINDOW } else { DWMSBT_NONE };
    dwm(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &DWM_SYSTEMBACKDROP_TYPE(kind.0));
}

/// Borderless, always-on-top window with a native shadow and no taskbar button, `size` DIP inside, its
/// frame settled so the first `set_inner_size` is exact (see `settle_frame`).
pub fn tool_window<T: 'static>(
    target: &EventLoopWindowTarget<T>,
    size: (f64, f64),
    transparent: bool,
    focused: bool,
) -> Result<Window, String> {
    let window = WindowBuilder::new()
        .with_title(crate::meta::APP_NAME)
        .with_decorations(false)
        .with_resizable(false)
        .with_visible(false)
        .with_focused(focused)
        .with_always_on_top(true)
        .with_transparent(transparent)
        .with_inner_size(LogicalSize::new(size.0, size.1))
        .with_skip_taskbar(true)
        .with_undecorated_shadow(true)
        .build(target)
        .map_err(|e| format!("window: {e}"))?;
    settle_frame(HWND(window.hwnd() as *mut _));
    window.set_inner_size(LogicalSize::new(size.0, size.1));
    Ok(window)
}

pub fn build_popup<T: 'static>(
    target: &EventLoopWindowTarget<T>,
    context: &mut WebContext,
    on_message: impl Fn(String) + 'static,
    transparent: bool,
) -> Result<Surface, String> {
    let window = tool_window(target, POPUP_SIZE, transparent, true).map_err(|e| format!("popup {e}"))?;
    let webview = build_webview(&window, context, "popup.html", on_message, transparent)
        .map_err(|e| format!("popup webview: {e}"))?;
    Ok(Surface { window, webview })
}

/// The hover flyout: never takes focus, even when clicked (WS_EX_NOACTIVATE).
pub fn build_flyout<T: 'static>(
    target: &EventLoopWindowTarget<T>,
    context: &mut WebContext,
    on_message: impl Fn(String) + 'static,
) -> Result<Surface, String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    let window =
        tool_window(target, crate::ui::flyout::FLYOUT_SIZE, false, false).map_err(|e| format!("flyout {e}"))?;
    let hwnd = HWND(window.hwnd() as *mut _);
    // SAFETY: adds one extended style bit to a window this process owns.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
    }
    let webview = build_webview(&window, context, "flyout.html", on_message, false)
        .map_err(|e| format!("flyout webview: {e}"))?;
    Ok(Surface { window, webview })
}

/// Shows a window on top without activating it (the foreground app keeps focus).
pub fn show_inactive(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE,
    };
    // SAFETY: visibility and z-order of a window this process owns.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Hides a window shown with `show_inactive` (tao's own visibility flag was never set for it).
pub fn hide_window(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    // SAFETY: hides a window this process owns.
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// The NitroTray app icon at a system icon metric (`SM_CXSMICON` title bar, `SM_CXICON` taskbar/Alt-Tab)
/// for the system DPI, so the window shows it crisply instead of the generic application icon.
fn app_icon(metric: windows::Win32::UI::WindowsAndMessaging::SYSTEM_METRICS_INDEX) -> Option<tao::window::Icon> {
    use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
    // SAFETY: plain metric queries.
    let size = unsafe { GetSystemMetricsForDpi(metric, GetDpiForSystem()) }.clamp(16, 256) as u32;
    tao::window::Icon::from_rgba(crate::tray::app_icon::render(size), size, size).ok()
}

pub fn build_full<T: 'static>(
    target: &EventLoopWindowTarget<T>,
    context: &mut WebContext,
    on_message: impl Fn(String) + 'static,
) -> Result<Surface, String> {
    use windows::Win32::UI::WindowsAndMessaging::{SM_CXICON, SM_CXSMICON};
    let window = WindowBuilder::new()
        .with_title(format!("{} — NitroSense companion", crate::meta::APP_NAME))
        .with_window_icon(app_icon(SM_CXSMICON))
        .with_taskbar_icon(app_icon(SM_CXICON))
        .with_visible(false)
        .with_inner_size(LogicalSize::new(FULL_SIZE.0, FULL_SIZE.1))
        .with_min_inner_size(LogicalSize::new(760.0, 520.0))
        .build(target)
        .map_err(|e| format!("window: {e}"))?;
    let webview = build_webview(&window, context, "window.html", on_message, false)
        .map_err(|e| format!("window webview: {e}"))?;
    Ok(Surface { window, webview })
}
