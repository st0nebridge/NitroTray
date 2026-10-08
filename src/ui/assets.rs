//! @module ui::assets
//! @description Web UI files embedded in the executable and served to WebView2 via a custom protocol.
//!
//! @input  A request path such as "/popup.html" or "js/popup.js".
//! @output The file bytes and MIME type, or `None` (unknown path or traversal attempt).
//! @dependencies none (files are compiled in with `include_bytes!`)

pub struct Asset {
    pub path: &'static str,
    pub bytes: &'static [u8],
    pub mime: &'static str,
}

macro_rules! assets {
    ($($path:literal => $mime:literal),* $(,)?) => {
        pub const ASSETS: &[Asset] = &[
            $(Asset { path: $path, bytes: include_bytes!(concat!("web/", $path)), mime: $mime },)*
        ];
    };
}

assets! {
    "popup.html" => "text/html; charset=utf-8",
    "flyout.html" => "text/html; charset=utf-8",
    "window.html" => "text/html; charset=utf-8",
    "css/theme.css" => "text/css; charset=utf-8",
    "css/popup.css" => "text/css; charset=utf-8",
    "css/flyout.css" => "text/css; charset=utf-8",
    "css/window.css" => "text/css; charset=utf-8",
    "js/bridge.js" => "text/javascript; charset=utf-8",
    "js/curve.js" => "text/javascript; charset=utf-8",
    "js/flyout.js" => "text/javascript; charset=utf-8",
    "js/flyout-view.js" => "text/javascript; charset=utf-8",
    "js/format.js" => "text/javascript; charset=utf-8",
    "js/graph-preview.js" => "text/javascript; charset=utf-8",
    "js/icons.js" => "text/javascript; charset=utf-8",
    "js/monitor.js" => "text/javascript; charset=utf-8",
    "js/popup-view.js" => "text/javascript; charset=utf-8",
    "js/popup.js" => "text/javascript; charset=utf-8",
    "js/segmented.js" => "text/javascript; charset=utf-8",
    "js/settings.js" => "text/javascript; charset=utf-8",
    "js/window.js" => "text/javascript; charset=utf-8",
}

/// Scheme registered with wry; WebView2 exposes it as `http://nitro.localhost/`.
pub const SCHEME: &str = "nitro";

/// Normalises a request path and looks it up. Anything outside the table is refused.
pub fn lookup(path: &str) -> Option<&'static Asset> {
    let p = path.split(['?', '#']).next().unwrap_or("").trim_start_matches('/');
    let p = if p.is_empty() { "popup.html" } else { p };
    if p.contains("..") || p.contains('\\') {
        return None;
    }
    ASSETS.iter().find(|a| a.path == p)
}

/// Content-Security-Policy sent with every document: only our own scheme, no remote code.
pub const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'none'; object-src 'none'; base-uri 'none'";
