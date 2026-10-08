# Coverage waiver: event-loop and window glue

- **Scope:** `src/app/runtime.rs`, `src/app/surfaces.rs`, `src/app/effects.rs`, `src/app/tray_host.rs`,
  `src/app/bootstrap.rs`, `src/ui/webview.rs`, `src/main.rs`, `src/bin/nitrotray_service.rs`.
- **Why:** these files only wire OS objects together (tao event loop, WebView2 windows, notification-area
  icons, global hotkeys, SCM). They cannot run headlessly in a unit-test process without putting real
  windows and tray icons on the user's desktop.
- **What they call is tested:** every decision they execute lives in tested modules — `app::model`
  (reducer), `app::worker`, `tray::{positioning, graph_icon, app_icon, menu}`, `ui::{bridge, assets}`,
  `config`, `ipc`, `service`, `telemetry`, `platform`.
- **Compensating evidence:** live end-to-end runs (tray + helper over a real pipe; `bin/ui-snap.ps1`
  renders the screenshots into `reports/ui/`), headless-Edge renders of the popup compared against the reference
  (170/172 ink boxes within ±1.5 DIP), binary-level tests of both executables, JS view tests (jsdom) with
  golden snapshots of the rendered markup and canvas drawing.
- **Gate:** the coverage gate is evaluated on the logic scope (`--ignore-filename-regex` of the files above);
  the whole-crate figure is always reported alongside it.
- **Expiry / review:** revisit if a headless tao/WebView2 test harness becomes practical.
