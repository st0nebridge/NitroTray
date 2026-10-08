# Architecture

The structure as built. Two executables; the tray never needs administrator rights.

```text
NitroTray.exe (user)                                    NitroTrayService.exe (LocalSystem service)
├── app::bootstrap   single instance, --exit, workers   ├── service::host     SCM entry, console mode,
├── app::runtime     event dispatch (tao)               │                     install/uninstall, Runner
├── app::surfaces    popup/window placement & focus     ├── service::worker   one hardware thread
├── app::effects     OS side effects                    ├── service::core     typed request dispatch,
├── app::model       pure state machine (tested)        │                     read-back verification
├── app::worker      telemetry poller, control queue    ├── service::curve    fail-closed fan-curve loop
├── app::tray_host   app icon + CPU/GPU graph icons     └── acer::{gaming, wmi, acpi, discovery}
├── telemetry        PDH (CPU), NVML (GPU, lazy),                         │
│                    helper readings, scheduler, history                  ▼
├── ui               bridge (contract), assets (CSP),         root\WMI  AcerGamingFunction
│                    webview (WebView2 surfaces)
├── tray             positioning, raster, graph_icon, app_icon, menu
├── config           settings (sanitised), hotkey, store
└── ipc::client ──── \\.\pipe\NitroTray (JSON lines, ACL'd, session-0 check) ────┘
```

## Data flow

1. `app::worker::run_telemetry` asks `telemetry::scheduler` which metric groups are due. Open popup:
   utilisation 0.5 s, clocks/temps/fans 1 s, modes 3 s. Closed: only temps/fans for the graph icons
   (background interval, default 2 s) and modes every 5 s; NVML is never touched while hidden.
2. `telemetry::collector` merges PDH, NVML and helper readings into one `SystemSnapshot`; each provider
   is isolated (`catch_unwind`), failures blank that reading and are logged — never synthesised.
3. The runtime feeds the snapshot to `app::model` (calibration, reapply, notices) and to `tray_host`
   (graph icons, tooltips), then pushes `UiState` JSON to visible pages via `evaluate_script`.
4. Page clicks post `UiCommand` JSON (strictly parsed) → `app::model` returns `Effect`s → the runtime
   executes them; control writes go to a single control thread → helper → firmware, then modes are
   re-read so the UI shows what the firmware actually did.

## Privilege model and IPC

- Pipe `\\.\pipe\NitroTray`, byte mode, one request line per connection, ≤ 8 KiB.
- DACL `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x0012019b;;;IU)` — interactive users may read/write
  but not create instances; `PIPE_REJECT_REMOTE_CLIENTS`; first instance is exclusive.
- Clients connect with `SECURITY_IDENTIFICATION` (a squatting server cannot impersonate them) and refuse
  servers outside session 0 unless `NITROTRAY_DEV_HELPER=1` (development console helper).
- Operations: `get_capabilities`, `get_sensors`, `get_fan_state`, `get_performance_mode`,
  `set_fan_mode {mode, curve?}`, `set_performance_mode {mode}`. Unknown ops/fields are rejected; there is
  no generic read/write, IOCTL or registry operation.
- Every write is verified by reading the firmware back; a mismatch is reported as `firmware_rejected`.

## Fan-curve safety

Validated curves only (2–8 points, strictly rising temperature, non-decreasing duty, 20–100 %), 100 % at
≥ 90 °C regardless of the curve, duty falls at most 5 %/s, three consecutive missing temperatures →
Auto. A marker file, `%ProgramFiles%\NitroTray\curve-active`, exists while a curve runs; if the service
starts and finds it, the previous instance died mid-curve and Auto is restored. A clean stop always
restores Auto. The marker sits in the service's own administrators-only folder because SYSTEM must never
write where a standard user could create the folder first and redirect the write (`%ProgramData%` allows
that); development helpers keep per-pipe markers under `%ProgramData%\NitroTray`.

## Installation layout

`src/installer/nitrotray.nsi` (built by `bin/package.ps1`) installs everything per machine into the fixed
folder `%ProgramFiles%\NitroTray`: `NitroTray.exe`, `NitroTrayService.exe`, `Uninstall.exe`, the icon and
the license. The helper is registered by running the installed copy's own `install` command, so the SCM
always points at the Program Files binary. Per-user state stays per user: settings and WebView2 data in
`%LOCALAPPDATA%\NitroTray`, autostart in `HKCU\…\Run` (the exact value the app's own toggle writes).
Before replacing or deleting binaries, setup asks the tray to exit (`--exit`), stops the service, and
waits until both files are released.

## Popup rendering

`src/ui/web` is compiled into the exe (`include_bytes!`) and served over `nitro://` with a strict CSP.
The popup is a fixed 572 × 440 DIP canvas styled from measurements of the 2× reference and laid out on
one horizontal grid (D-20261001-004: 16 DIP margins, two 264 DIP columns, shared chevron lines); WebView2
scales it per monitor DPI. DWM supplies rounded corners, border and shadow (no notch).
Borderless windows are created hidden with their frame settled (`ui::webview::tool_window`): until the
first `WM_NCCALCSIZE`, tao measures a caption that is not there, and the first open came out 30 px too tall.

## Hover flyout

Resting on the CPU or GPU graph icon (`tray::hover`: 350 ms, hide on leave/click, instant switch between
icons) shows `flyout.html` (340 × 214 DIP) above it, built once at start-up and kept hidden like the popup.
`ui::flyout` turns the 300-sample monitoring history into that icon's series in the user's unit, with the
icon's colours, range and fan scale. The window has `WS_EX_NOACTIVATE` and is shown with
`SW_SHOWNOACTIVATE`, so the foreground app keeps focus; its page may only send `ready`. It reads only the
embedded-controller values the graph icons already poll, so it never wakes the discrete GPU (D-20261001-002).

Webviews are created only while none of ours is on screen (D-20261001-003): the popup and flyout during
start-up before the popup is first shown; a later flyout (hover details turned on) or popup rebuild
(transparency changed) waits until popup, flyout and window are all hidden. Creating a controller while
another was visible never completed and left the event loop inside wry's wait. `tests/live/webviews.ps1`
checks both paths against a real build.
