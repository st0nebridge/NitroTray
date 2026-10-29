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
├── app::tensor      tensor-check thread                ├── service::{esp, unlock_log}  unlocker boot log
├── app::tray_host   app icon + CPU/GPU graph icons     └── acer::{gaming, wmi, acpi, discovery}
├── telemetry        PDH (CPU), NVML (GPU, lazy),                         │
│                    helper readings, scheduler, history                  ▼
├── unlock           tensor probe (PTX + CUDA driver),        root\WMI  AcerGamingFunction
│                    boot-log parser, verdict, view
├── ui               bridge (contract), assets (CSP),
│                    webview (WebView2 surfaces)
├── tray             positioning, clicks, raster, graph_icon, app_icon, menu
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
- Operations: `get_capabilities`, `get_sensors`, `get_fan_state`, `get_performance_mode`, `get_unlock_boot`,
  `set_fan_mode {mode, curve?}`, `set_performance_mode {mode}`. Unknown ops/fields are rejected; there is
  no generic read/write, IOCTL or registry operation. `get_unlock_boot` takes no argument: it reads the newest
  `RUN<n>.TXT` of the UEFI tensor unlocker from the one fixed directory of the EFI system partition
  (D-20261029-002) and returns a typed summary, never the file.
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

## Tray icon gestures and the details popup

Hovering a graph icon shows Windows' plain tooltip (`app::tray_host` always sets it). The left button is
interpreted by `tray::clicks` (D-20261029-001): a single click on the CPU or GPU icon toggles that icon's
details popup, a single click on the app icon toggles the main popup, and a double click opens the main
popup from any icon. Windows reports a double click as press, release, double-click, release, so the first
release has already opened the details popup (instantly, with no wait for a possible second click) and the
double click swaps it for the main popup; the release that ends the double click is ignored, and every
press clears that flag so a release that never arrived cannot swallow a later click. With "Open popup on
tray click" off, the left button opens the tray menu and none of this runs.

The details popup is `flyout.html` (340 × 214 DIP), built once at start-up and kept hidden like the popup.
`ui::flyout` turns the 300-sample monitoring history into that icon's series in the user's unit, with the
icon's colours, range and fan scale. It is a focused window like the main popup, so Escape (the page sends
`close_popup`; it may send nothing else besides `ready`) and, with "close on focus loss", clicking elsewhere
close it; a click on the icon that closed it by taking focus counts as the close, not a reopen
(`TOGGLE_GRACE`). Only one of the two popups is shown at a time. It reads only the embedded-controller
values the graph icons already poll and does not count as a visible surface for the telemetry worker, so
it never wakes the discrete GPU (D-20261001-002, D-20260930-010).

Webviews are created only while none of ours is on screen (D-20261001-003): the popup and details popup
during start-up before the popup is first shown; a later details popup (a graph icon turned on) or popup
rebuild (transparency changed) waits until popup, details popup and window are all hidden. Creating a
controller while another was visible never completed and left the event loop inside wry's wait.
`tests/live/webviews.ps1` checks both paths and the click gestures against a real build.

## Tensor-core lock state

GeForce parts run FP16 multiply-accumulate with FP32 accumulate at half rate unless the throttle has been lifted
(D-20261029-003). Two sources feed one verdict (`unlock::view`):

- **Live probe** (`unlock::{ptx, cuda}`, run by `app::tensor` on its own thread). `unlock::ptx` generates a loop
  of eight independent `mma.sync.m16n8k16` chains in PTX, once with FP32 and once with FP16 accumulate; the
  driver JIT-compiles it for the installed GPU through `nvcuda.dll`, so no toolkit is needed. `unlock::cuda`
  creates a context, times each kernel (best of three, CUDA events), and destroys everything before returning.
  The ratio of the two rates is about 0.50 throttled and about 1.0 lifted; above 0.75 is unlocked
  (`unlock::state`). It runs when the popup opens, when the GPU icon is clicked, and on a click on the badge,
  never twice at once (`unlock::tracker`: a reading is reused for 60 s, a failure is not retried sooner), and
  never with *GPU driver telemetry* off.
- **Boot log** (`service::{esp, unlock_log}`, the helper's `get_unlock_boot`). The helper finds the FAT volume that
  holds the unlocker's log directory by enumerating volume GUID paths, so the EFI system partition is read where
  it is, unmounted and read-only; the highest-numbered `RUN<n>.TXT` is parsed (`unlock::boot_log`) down to its
  status, build and write time. A log is "from this session" when it was written no more than 15 minutes before
  the session started (`platform::windows::session_start_unix`: the unlocker runs before Windows starts, and a
  Fast Startup or hibernate boot writes its log after the original boot).

`unlock::view` turns both into a badge for the popup (`#tensor`, in the GPU block's free strip above the
temperature, clickable) and a row for the GPU details popup. With a half-rate reading and a log of this session
that says "unlocked" it says *Unlock lost*. With no reading it says nothing: it never claims "locked" without a
measurement.
