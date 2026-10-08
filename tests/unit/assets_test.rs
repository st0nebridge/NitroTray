//! @module assets_test
//! @description Embedded web assets: every page resource is present, typed, and path traversal is refused.
use nitrotray::ui::assets::{lookup, ASSETS, CSP, SCHEME};

#[test]
fn every_asset_is_embedded_with_a_type() {
    assert!(ASSETS.len() >= 16);
    for a in ASSETS {
        assert!(!a.bytes.is_empty(), "{}", a.path);
        let ext = a.path.rsplit('.').next().unwrap();
        let expected = match ext {
            "html" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            other => panic!("unexpected asset type {other}"),
        };
        assert!(a.mime.starts_with(expected), "{}", a.path);
    }
}

#[test]
fn pages_reference_only_embedded_files() {
    for page in ["popup.html", "flyout.html", "window.html"] {
        let html = std::str::from_utf8(lookup(page).unwrap().bytes).unwrap();
        for attr in ["href=\"", "src=\""] {
            for part in html.split(attr).skip(1) {
                let target = part.split('"').next().unwrap();
                assert!(lookup(target).is_some(), "{page} references missing {target}");
            }
        }
    }
}

#[test]
fn js_imports_resolve_to_embedded_modules() {
    for a in ASSETS.iter().filter(|a| a.path.ends_with(".js")) {
        let src = std::str::from_utf8(a.bytes).unwrap();
        for line in src.lines().filter(|l| l.starts_with("import ")) {
            let spec = line.split('\'').nth(1).expect("quoted specifier");
            let resolved = format!("js/{}", spec.trim_start_matches("./"));
            assert!(lookup(&resolved).is_some(), "{} imports missing {spec}", a.path);
        }
    }
}

#[test]
fn lookup_normalises_and_refuses_traversal() {
    assert_eq!(lookup("/").unwrap().path, "popup.html");
    assert_eq!(lookup("/js/popup.js?v=2#x").unwrap().path, "js/popup.js");
    for bad in ["/../Cargo.toml", "..\\secrets", "/js/../../x", "/nope.html", "/js\\popup.js"] {
        assert!(lookup(bad).is_none(), "{bad}");
    }
    assert_eq!(SCHEME, "nitro");
}

#[test]
fn csp_blocks_remote_content() {
    assert!(CSP.contains("default-src 'self'"));
    assert!(CSP.contains("script-src 'self'"));
    assert!(CSP.contains("connect-src 'none'"));
    assert!(!CSP.contains("unsafe-eval"));
}
