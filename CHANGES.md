# Changes

Change log entries, newest last.

```yaml
id: CU-20260930-001
type: feature
title: "Project skeleton"
description: "Cargo crate with NitroTray and NitroTrayService binaries, identity constants, build/test scripts, governance docs."
impact: none
affected_modules: ["src/meta.rs", "src/main.rs", "src/bin/nitrotray_service.rs"]
related_tests: ["tests/unit/meta_test.rs"]
commit_ref: "feature/skeleton"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T07:40:00Z"
```

```yaml
id: CU-20260930-002
type: fix
title: "IPC rejects unknown fields on field-less ops"
description: "serde ignores extra keys on unit variants of an internally tagged enum, so {\"op\":\"get_sensors\",\"x\":1} was accepted; parse_request now rejects any key absent from the request's canonical form."
impact: medium
affected_modules: ["src/ipc/protocol.rs"]
related_tests: ["tests/regression/cu_20260930_002.rs", "tests/unit/protocol_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T08:05:00Z"
```

```yaml
id: CU-20260930-003
type: fix
title: "Provider error log de-duplicates recurring errors"
description: "Dedupe compared only the last entry, so alternating recurring errors flooded the log; entries are now unique per (source, message) and refreshed to the latest occurrence."
impact: low
affected_modules: ["src/telemetry/collector.rs"]
related_tests: ["tests/regression/cu_20260930_003.rs", "tests/unit/collector_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T08:06:00Z"
```

```yaml
id: CU-20260930-004
type: fix
title: "Pipe server: owned Server handle with guaranteed stop; clients retry busy pipes to a deadline"
description: "A server thread could stay blocked in ConnectNamedPipe when its stop wake-up raced, stalling process exit (observed as a hung test process); concurrent clients could exhaust 3 busy retries. Server::spawn now creates the first instance synchronously, always keeps a listening instance, stops via repeated wake until the thread is finished, and stops on drop; clients retry within a 2 s budget."
impact: high
affected_modules: ["src/ipc/pipe.rs", "src/service/host.rs"]
related_tests: ["tests/regression/cu_20260930_004.rs", "tests/unit/pipe_test.rs", "tests/unit/host_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T08:30:00Z"
```

```yaml
id: CU-20260930-005
type: feature
title: "Hardware backend, helper service and telemetry core"
description: "AcerGamingFunction codec (verified on AN515-58), WMI transport, capability discovery with model gating, typed IPC protocol + ACL'd named-pipe transport + client, helper service core with read-back verification and fail-closed custom-curve controller, simulated dev backend, PDH/NVML providers, telemetry collector, scheduler, histories, thresholds, fan-ring honesty rules."
impact: high
affected_modules: ["src/acer", "src/ipc", "src/service", "src/telemetry", "src/controls", "src/platform"]
related_tests: ["tests/unit/*_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T08:31:00Z"
```

```yaml
id: CU-20260930-006
type: fix
title: "Production pipe name restored to \\\\.\\pipe\\NitroTray"
description: "A shell heredoc collapsed the leading double backslash of PIPE_NAME (and of its test, which therefore agreed); the tray and service would have used an invalid pipe path. Constant corrected; the test now asserts the exact bytes independently of the literal."
impact: high
affected_modules: ["src/meta.rs"]
related_tests: ["tests/regression/cu_20260930_006.rs", "tests/unit/meta_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T08:45:00Z"
```

```yaml
id: CU-20260930-007
type: feature
title: "Tray host, popup matched to the reference, full window, System Informer-style graph icons"
description: "tao/wry tray host with instantly re-shown popup (DWM rounded, no notch), tray positioning for all taskbar edges, focus-loss dismissal, Win+Alt+N hotkey, second-instance show / --exit signals; popup UI measured against the 2x reference (170/172 ink boxes within 1.5 DIP); monitoring charts, fan-curve editor mirroring the Rust safety rules, settings incl. per-icon graph customisation; CPU/GPU tray graph icons with temperature fill and fan trace; diagnostics export; helper install / icon pin actions."
impact: high
affected_modules: ["src/app", "src/tray", "src/ui", "src/config", "src/diagnostics.rs", "src/platform"]
related_tests: ["tests/unit/model_test.rs", "tests/unit/graph_icon_test.rs", "tests/unit/bridge_test.rs", "tests/unit/assets_test.rs", "tests/ui/*.test.mjs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T09:55:00Z"
```

```yaml
id: CU-20260930-008
type: refactor
title: "Split runtime; remove import cycles"
description: "app::runtime split into events/bootstrap/surfaces/runtime; AcerError moved to acer::error (backend<->discovery cycle); webview builder made generic over the event type (runtime<->webview cycle); TelemetryConfig::from_settings (bootstrap<->runtime cycle). Module structure fitness 85.7 -> 100.0. No behaviour change."
impact: none
affected_modules: ["src/app", "src/acer/error.rs", "src/ui/webview.rs"]
related_tests: ["tests/unit/*_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T10:05:00Z"
```

```yaml
id: CU-20260930-009
type: fix
title: "Graph icons read CPU | GPU | app, fan trace never pinned to the top edge"
description: "Observed live: the shell inserts newer icons to the left, so icons are now created app, GPU, CPU; auto fan scale gains 10 % headroom above the session peak."
impact: low
affected_modules: ["src/app/tray_host.rs", "src/tray/graph_icon.rs", "src/ui/web/js/graph-preview.js"]
related_tests: ["tests/regression/cu_20260930_009.rs", "tests/unit/graph_icon_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T09:20:00Z"
```

```yaml
id: CU-20260930-010
type: fix
title: "Security: helper service registered only from Program Files"
description: "Found while designing the installer: 'Install helper' registered a LocalSystem service pointing at NitroTrayService.exe beside the per-user tray (user-writable), allowing any user process to replace it and run as SYSTEM. install now copies the binary to %ProgramFiles%\\NitroTray, stops/updates an existing registration, and registers that copy; uninstall removes it."
impact: high
affected_modules: ["src/service/host.rs", "src/service/install_path.rs"]
related_tests: ["tests/regression/cu_20260930_010.rs", "tests/unit/install_path_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T10:50:00Z"
```

```yaml
id: CU-20260930-011
type: feature
title: "Live firmware verification tests"
description: "hardware_test (read-only, elevated) and live_control_test (opt-in via NITROTRAY_LIVE_PIPE: idempotent writes, Auto->Max->Auto, performance away and back, custom curve, guaranteed restore) exercised the production helper against the AN515-58."
impact: none
affected_modules: ["tests/unit/hardware_test.rs", "tests/unit/live_control_test.rs"]
related_tests: ["tests/unit/hardware_test.rs", "tests/unit/live_control_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T10:40:00Z"
```

```yaml
id: CU-20260930-012
type: fix
title: "Instance-scoped session names; tests can no longer exit the user's tray; foreground hand-off"
description: "binaries_test ran NitroTray.exe --exit, signalling the session-global exit event, so every mutation-testing iteration shut down the running tray (observed: live instances vanished with no crash record). Mutex and show/exit events are now scoped by NITROTRAY_INSTANCE (tests use a unique tag, dev-run uses 'dev'). A second instance also calls AllowSetForegroundWindow(ASFW_ANY) before signalling show, so the popup can take focus and dismiss on click-away."
impact: medium
affected_modules: ["src/meta.rs", "src/app/bootstrap.rs", "bin/dev-run.ps1", "tests/unit/binaries_test.rs"]
related_tests: ["tests/regression/cu_20260930_012.rs", "tests/unit/meta_test.rs", "tests/unit/binaries_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T10:35:00Z"
```

```yaml
id: CU-20260930-013
type: fix
title: "Idle CPU: pause WebView2 while hidden (2.318 % -> 0.036 % of the machine)"
description: "Measured on the AN515-58: with the popup hidden, NitroTray + WebView2 used 2.3 % of the machine because the WebView2 controller stayed visible behind the hidden window and kept compositing (fan-icon animation). Webviews are now created hidden and toggled with their windows, and hidden surfaces request WebView2's low memory target. After: 0.036 % (target < 0.2 %). Memory-target effect not measured live: Defender ASR blocked the rebuilt unsigned exe."
impact: medium
affected_modules: ["src/ui/webview.rs", "src/app/surfaces.rs"]
related_tests: ["tests/regression/cu_20260930_013.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T11:05:00Z"
```

```yaml
id: CU-20260930-014
type: fix
title: "Mutation-testing findings: dead code removed, assertions strengthened"
description: "cargo-mutants survivors triaged: Capabilities::any_fan_control/any_performance_control were used only by tests (deleted); added assertions for autostart default, reapply curve carriage, app-icon rule with one graph icon, exact response size limit, HelperClient mode readings, per-message error-log dedupe. Equivalent mutants excluded with reasons in .cargo/mutants.toml."
impact: low
affected_modules: ["src/acer/discovery.rs", "tests/unit"]
related_tests: ["tests/unit/model_test.rs", "tests/unit/settings_test.rs", "tests/unit/protocol_test.rs", "tests/unit/collector_test.rs", "tests/unit/app_worker_test.rs", "tests/unit/discovery_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T11:10:00Z"
```

```yaml
id: CU-20260930-015
type: fix
title: "Pipe client reports NotRunning as soon as the server is gone"
description: "The client treated 'not found after busy' as a transient gap and retried to its 2 s deadline, returning Busy after a clean helper stop (flaky under load). The Server always keeps a listening instance while running, so 'not found' now always means NotRunning."
impact: low
affected_modules: ["src/ipc/pipe.rs"]
related_tests: ["tests/unit/pipe_test.rs", "tests/regression/cu_20260930_004.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T11:20:00Z"
```

```yaml
id: CU-20260930-016
type: test
title: "Mutation-driven tests: scheduler due flags, collector poll gating, trace width"
description: "Second cargo-mutants round: DueSet::any is checked one flag at a time; the collector's poll is checked per due flag (utilisation/clocks/temperatures/fans) and with the GPU driver disabled; graph-icon trace width is checked to scale with icon size (1 px at 16, 3 px at 48); the graph-icon temperature span is checked at exactly 10 °C; a request of exactly MAX_MESSAGE_BYTES is accepted. Response::from_error drops a redundant `ok: false` (the default), which removes an equivalent mutant instead of excluding it. No behaviour change."
impact: none
affected_modules: ["tests/unit", "src/ipc/protocol.rs", ".cargo/mutants.toml"]
related_tests: ["tests/unit/scheduler_test.rs", "tests/unit/collector_test.rs", "tests/unit/graph_icon_test.rs", "tests/unit/settings_test.rs", "tests/unit/protocol_test.rs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T12:40:00Z"
```

```yaml
id: CU-20260930-017
type: test
title: "Web UI mutation pass: golden snapshots, window/curve/popup behaviour tests, dead code removed"
description: "Stryker run 1 scored 69.7 % (1259/1807) with weak spots in window.js (53 %), icons.js (47 %) and monitor.js (60 %). Added golden snapshots (tests/ui/fixtures/snapshots, rewritten only with NITROTRAY_WRITE_FIXTURES=1) for the icon family, settings form, monitoring charts (markup + canvas draw log at 1x/2x, °C/°F), curve plot and tray-icon preview, each hand-checked against computed coordinates; plus behaviour tests for window.js (initial page, per-frame rendering, units, notices, curve errors, previews, autostart), popup-view (connected-and-supported gating, pending, idle fans), segmented keyboard/ARIA, curve editor pointer/keyboard/table and bridge fallbacks. Source, behaviour unchanged: removed a no-op hydrate(form), a redundant slice, a render guard the callee already applies, a history guard setHistory already handles and an unreachable page fallback in window.js; dropped settings.js's colour lower-casing (colour inputs sanitise to lower case per the HTML spec), an always-true canvas guard and a dead default parameter; folded popup-view's disabled-choice branch into one send; segmented.js sets or removes aria-busy in one branch (a toggle was followed by a redundant set). Run 2 after the tests: 97.9 % (timeouts counted as detected)."
impact: none
affected_modules: ["src/ui/web/js/window.js", "src/ui/web/js/popup-view.js", "src/ui/web/js/settings.js", "src/ui/web/js/monitor.js", "src/ui/web/js/segmented.js", "tests/ui"]
related_tests: ["tests/ui/snapshots.test.mjs", "tests/ui/window.test.mjs", "tests/ui/popup-view.test.mjs", "tests/ui/curve.test.mjs", "tests/ui/segmented.test.mjs", "tests/ui/entries.test.mjs", "tests/ui/bridge.test.mjs", "tests/ui/monitor.test.mjs", "tests/ui/settings.test.mjs"]
commit_ref: "feature/core"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T13:30:00Z"
```

```yaml
id: CU-20260930-018
type: feature
title: "NSIS installer replaces the PowerShell install scripts"
description: "src/installer/nitrotray.nsi builds NitroTray-<version>-setup.exe (bin/package.ps1: release build, makensis with warnings as errors, SHA256SUMS.txt). Per-machine install into the fixed %ProgramFiles%\\NitroTray (no directory page, /D= ignored) with components app / helper service / start with Windows; upgrades stop the tray (--exit) and the service and wait for both binaries to be released, keep the installed components, and remove unticked ones; uninstall removes the service, files, shortcut, Apps entry and this install's Run value, and asks before deleting settings (silent keeps them). /S, /NOHELPER, /NOAUTOSTART for unattended use; WebView2 missing only warns. The setup icon is generated from the tray app icon and checked against it. bin/install.ps1 and bin/uninstall.ps1 (per-user layout) are retired to the archive."
impact: medium
affected_modules: ["src/installer/nitrotray.nsi", "src/installer/nitrotray.ico", "bin/package.ps1", "bin/install.ps1", "bin/uninstall.ps1", "package.json"]
related_tests: ["tests/regression/cu_20260930_018.rs", "tests/unit/installer_test.rs", "tests/installer/roundtrip.ps1"]
commit_ref: "feature/release"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T22:30:00Z"
```

```yaml
id: CU-20260930-019
type: security
title: "Helper's custom-curve marker moves out of %ProgramData%"
description: "The LocalSystem helper wrote its crash marker to %ProgramData%\\NitroTray\\curve-active. Standard users may create folders in %ProgramData%, so one could create NitroTray first and redirect the SYSTEM write with a junction/link (arbitrary file write as SYSTEM). The production marker now lives in the helper's administrators-only folder, %ProgramFiles%\\NitroTray\\curve-active; marker_path returns None (no crash marker) if Program Files is unknown. Development helpers keep per-pipe markers under %ProgramData%. Found while designing the uninstaller; the installer's uninstall removes the new marker."
impact: medium
affected_modules: ["src/service/host.rs", "src/installer/nitrotray.nsi", "docs/architecture.md"]
related_tests: ["tests/regression/cu_20260930_019.rs", "tests/unit/host_test.rs"]
commit_ref: "feature/release"
author: "Brandon Stonebridge"
timestamp: "2026-09-30T22:30:00Z"
```

```yaml
id: CU-20261001-001
type: fix
title: "Popup opened too tall (over the taskbar) the first time"
description: "tao's set_inner_size adds the measured window/client difference for borderless windows with a shadow. Until the first WM_NCCALCSIZE the client area still excluded a caption, so the first show was 30 px taller (572x470 instead of 572x440 at 96 DPI) than the size its placement was computed for and its bottom ran 20 px into the taskbar; later opens were correct. Measured on the AN515-58 before and after. ui::webview::tool_window now forces the frame recalculation (SWP_FRAMECHANGED) while the window is hidden and re-applies the size, for the popup and the new hover flyout."
impact: low
affected_modules: ["src/ui/webview.rs", "src/app/surfaces.rs"]
related_tests: ["tests/regression/cu_20261001_001.rs"]
commit_ref: "feature/tray-hover-flyout"
author: "Brandon Stonebridge"
timestamp: "2026-10-01T09:00:00Z"
```

```yaml
id: CU-20261001-002
type: feature
title: "Hover flyout on the CPU and GPU tray icons"
description: "Resting the pointer on the CPU or GPU graph icon for 350 ms shows a 340x214 DIP flyout above it: that processor's temperature (filled, the icon's colour) and fan-speed (line) history from the 300-sample monitoring history, the current temperature in its threshold colour, fan RPM, min/avg/max and the time span. It hides as soon as the pointer leaves the icon or any icon is clicked, switches instantly between neighbouring icons, never takes focus (WS_EX_NOACTIVATE, SW_SHOWNOACTIVATE), is display-only (its page may only send ready), and is suppressed while the main popup is open. Single-click behaviour is unchanged. Only embedded-controller readings are shown, at the graph icons' existing cadence, so hovering never starts NVML. Settings > Tray icons > Details on hover (default on) turns it off; while on, the graph icons have no plain tooltip, which would cover the flyout."
impact: medium
affected_modules: ["src/ui/flyout.rs", "src/tray/hover.rs", "src/app/runtime.rs", "src/app/surfaces.rs", "src/app/effects.rs", "src/app/tray_host.rs", "src/app/model.rs", "src/app/events.rs", "src/ui/webview.rs", "src/ui/assets.rs", "src/config/settings.rs", "src/ui/web/flyout.html", "src/ui/web/css/flyout.css", "src/ui/web/js/flyout.js", "src/ui/web/js/flyout-view.js", "src/ui/web/js/settings.js"]
related_tests: ["tests/regression/cu_20261001_002.rs", "tests/unit/flyout_test.rs", "tests/unit/hover_test.rs", "tests/ui/flyout-view.test.mjs", "tests/ui/entries.test.mjs"]
commit_ref: "feature/tray-hover-flyout"
author: "Brandon Stonebridge"
timestamp: "2026-10-01T09:00:00Z"
```

```yaml
id: CU-20261001-003
type: fix
title: "Tray app hung at start-up with a blank popup once the hover flyout existed"
description: "CU-20261001-002 created the flyout's webview one loop turn after start-up, when the popup had already been shown. Creating a WebView2 controller while another of ours was on screen never completed: the event loop stayed inside wry's controller wait, the popup stayed blank at about:blank and never closed, the tray menu and hover did nothing and --exit was ignored. Reproduced in 3 of 3 isolated runs on an idle machine (main and the fix loaded in about 1 s in 3 of 3); traced to build_flyout never returning. The flyout is now created during start-up before the popup is first shown, and any later webview (re)creation (flyout after Details on hover is turned on, popup after a transparency change) waits until no popup, flyout or window of ours is on screen. Found by the user on the first run of the branch build; never released."
impact: medium
affected_modules: ["src/app/runtime.rs"]
related_tests: ["tests/regression/cu_20261001_003.rs", "tests/live/webviews.ps1"]
commit_ref: "feature/tray-hover-flyout"
author: "Brandon Stonebridge"
timestamp: "2026-10-01T08:30:00Z"
```

```yaml
id: CU-20261001-004
type: ui
title: "Popup on one alignment grid; new settings gear; flyout edges aligned"
description: "The popup reproduced the reference image's own horizontal offsets: card edges at 15/17 and 555.5-558 DIP, chevron centres at 255.5/259/256.75 (left column) and 526/534/541 (right), the GPU half's content 20 DIP further in than the CPU half's, and the footer divider at 327 instead of the column split. It now uses one grid: 16 DIP margins, columns 16-280 and 292-556 (split at 286), chevrons 20.5 DIP wide on two shared lines (centres 255.75 and 531.75, with the close button over the right one), card and footer icons at column + 11, titles at column + 54, fan rings and CPU/GPU icons at the same inset, temperatures under the fan readings. Vertically, the header gains room under the CONTROL > COOL > PLAY strapline (baseline to rule ~5 -> ~10 DIP: rule 4 DIP lower, title 2 DIP higher, gear and close centred on the wordmark's cap line like the acer logo); the fan card, divider and CPU/GPU row move down 4 DIP, taken from the gap above the control cards, which now equals the gap below them (14.5 DIP). Footer labels were 1 DIP and the Quick Settings icon 0.9 DIP above their buttons' centre; labels (cap height), icons (ink) and chevrons now centre on 412.5. Typography is unchanged. The settings gear is redrawn: eight evenly spaced rounded teeth, open hub, 1.8 stroke (was a hand-plotted polygon at stroke 2, heavier than the close glyph); it also appears in the window's navigation. The hover flyout's graph now shares the header and footer edges (14 DIP) with equal 8.5 DIP gaps above and below. Measured in the rendered page (getBoundingClientRect) before and after."
impact: low
affected_modules: ["src/ui/web/css/popup.css", "src/ui/web/js/icons.js", "src/ui/web/css/flyout.css", "src/ui/web/js/flyout-view.js"]
related_tests: ["tests/regression/cu_20261001_004.rs", "tests/ui/snapshots.test.mjs", "tests/ui/flyout-view.test.mjs"]
commit_ref: "feature/ui-polish"
author: "Brandon Stonebridge"
timestamp: "2026-10-01T10:00:00Z"
```

```yaml
id: CU-20261001-005
type: fix
title: "Full window shows the NitroTray icon; save-bar status level with its button; screenshots refreshed"
description: "No NitroTray window set an icon and the exe has no icon resource, so the full monitoring window's title bar, taskbar button and Alt-Tab entry showed the generic application icon. build_full now sets the app icon at SM_CXSMICON (title bar) and SM_CXICON (taskbar/Alt-Tab) for the system DPI; verified with WM_GETICON on the live window (16 and 32 px NitroTray icons). In the settings save bar, 'Unsaved changes' used the paragraph style .muted, whose 16 DIP bottom margin made the centred flex row lift it 8 DIP above the button (cap centres 872.7 vs 881.6); the bar now aligns on the text baseline with no margin (both baselines 887). README screenshots refreshed: popup and both hover flyouts rendered at 2x by bin/ui-snap.ps1 (new -View flyout-cpu|flyout-gpu), and tray-icons@6x.png now rendered by the icon code (docs_images_test) instead of a pre-CU-20260930-009 live capture."
impact: low
affected_modules: ["src/ui/webview.rs", "src/ui/web/css/window.css", "bin/ui-snap.ps1", "tests/ui/preview/flyout.html", "docs/images", "README.md"]
related_tests: ["tests/regression/cu_20261001_005.rs", "tests/unit/docs_images_test.rs"]
commit_ref: "feature/ui-polish"
author: "Brandon Stonebridge"
timestamp: "2026-10-01T11:00:00Z"
```

```yaml
id: CU-20261001-006
type: ui
title: "Header: logo and gear/close centred on the title block"
description: "After CU-20261001-004 the acer logo and the gear/close buttons were centred on the NITROSENSE wordmark only (ink centres 31.25 and 26 DIP), which read as too high beside the two-line title. They now centre on the whole title block, wordmark plus strapline (ink 17-54 DIP, centre 35.5): logo 35.25, gear 35.5, close 35.75, measured by ink rows in the 2x render. README popup screenshot refreshed."
impact: low
affected_modules: ["src/ui/web/css/popup.css", "docs/images/popup@2x.png"]
related_tests: ["tests/regression/cu_20261001_006.rs"]
commit_ref: "feature/header-centring"
author: "Brandon Stonebridge"
timestamp: "2026-10-01T12:00:00Z"
```

```yaml
id: CU-20261001-007
type: fix
title: "Mutation runs end themselves instead of leaking test processes"
description: "Stryker runs the UI tests through its command runner. On Windows, its kill of a timed-out run is taskkill /T /F via exec, and when that fails under memory pressure, Stryker starts a new runner and abandons the old cmd -> node --test tree. Each abandoned tree adds pressure, so the leaks snowball. On 2026-10-01 the same setup in another project reached 229 node processes holding 25 GB and stalled the machine. The command is now node --test --test-timeout=10000 --test-force-exit, so a hung test is cancelled and the process exits past open handles. timeoutMS goes 60000 -> 20000, still above the tests' own 10 s, so a run ends itself before Stryker has to kill it. Concurrency stays at 4. Measured on Node 22.23.2: three hangs (await forever with a server and child open, a server left open, a sync infinite loop) run forever under plain node --test; with the flags they end in 6 s, 2 s and 6 s with no processes left, and the open-handle test still passes. UI suite with the flags on 0.1.1: 86 tests, 0 failed, 0 cancelled, 4.8 s. Written on a branch cut before 0.1.1 as CU-20261001-004, an ID main had since used for the popup grid; renumbered on merge. Test tooling only: the app and installer are unchanged."
impact: low
affected_modules: ["stryker.config.json"]
related_tests: ["tests/regression/cu_20261001_007.rs"]
commit_ref: "feature/mutation-self-ending"
author: "Brandon Stonebridge"
timestamp: "2026-10-08T00:00:00Z"
```

```yaml
id: CU-20261008-001
type: fix
title: "Release binaries no longer embed build-machine paths"
description: "The 0.1.1 executables carried 120 absolute paths into the Cargo home inside the builder's user profile (104 in NitroTray.exe, 16 in NitroTrayService.exe): dependencies' panic locations. A release scan of the binaries' strings failed on them. bin/build.ps1 now passes --remap-path-prefix for the Cargo home (CARGO_HOME, else .cargo in the user profile) and for the checkout, through CARGO_ENCODED_RUSTFLAGS so paths with spaces survive and any existing flags are kept. Debug builds are unchanged."
impact: low
affected_modules: ["bin/build.ps1"]
related_tests: ["tests/regression/cu_20261008_001.rs"]
commit_ref: "release/0.1.2"
author: "Brandon Stonebridge"
timestamp: "2026-10-08T00:00:00Z"
```

```yaml
id: CU-20261008-002
type: docs
title: "Public release preparation for 0.1.2"
description: "Prepares the first public release under D-20261008-001. References to the private build blueprint (51 lines: section numbers in code comments, docs, test names and DECISIONS.md) become plain words; regression tests are described as regression tests rather than anchors; CHANGES.md's author fields name the owner; a test fixture's real checkout path is replaced by an invented one; the coverage waiver no longer names a local archive; the README no longer links to the development-only quality report and release runbook, and says the NitroSense reference screenshot is not distributed. Public metadata: LICENSE, Cargo.toml, package.json and the installer's publisher name Brandon Stonebridge, with the repository URL. Version 0.1.2. .release-scan holds the release identity, the private-words list location, the excluded paths and the reasoned allowances; bin/release-public.sh builds the public commit from it. No runtime behaviour changes."
impact: none
affected_modules: ["README.md", "DECISIONS.md", "CHANGES.md", "docs", "src (comments)", "tests (names and comments)", "LICENSE", "Cargo.toml", "package.json", "src/installer/nitrotray.nsi", ".release-scan", "bin/release-public.sh"]
related_tests: []
commit_ref: "release/0.1.2"
author: "Brandon Stonebridge"
timestamp: "2026-10-08T00:00:00Z"
```

```yaml
id: CU-20261008-003
type: chore
title: "Release 0.1.2 prepared: public commit b64bba6 from development commit 17cf85e"
description: "First public release under D-20261008-001, prepared and not yet pushed. Public commit b64bba682c60899ee5148d736f2c171ce9f1f4b3 (new root, tree 5a345ad, 222 files) is development commit 17cf85e minus the 7 paths .release-scan excludes; every other blob and mode is identical. Release scan, strict, exit 0 with nothing to fix or review on the tree and commit; it fails on the development commit (12 findings: excluded paths, agent notes, scan tooling, a private word) and on a canary private word (44 findings), so it can fail. Release notes scan clean. From a git archive export: npm ci, bin/test.ps1 (286 unit, 34 regression, 86 web UI tests, all passing), bin/package.ps1 -> NitroTray-0.1.2-setup.exe, 1,099,676 bytes, sha256 85dfef52b9ea88ebbb76eef4ba6770f707b6c5890d22c585b4cae1a6943ba699. The unpacked installer scans clean with --binaries: no build-machine paths (0.1.1 had 120), and its 10 distinct GUIDs are Microsoft's WebView2 loader ids and NSIS's supportedOS manifest ids, allowed with reasons. The elevated round trip of that installer passed 62 of 62 checks; 0.1.2 was then installed with the helper and Start with Windows, and the tray started unelevated. Firmware modes were not changed (Auto / Quiet before and after). The images carry no metadata chunks."
impact: none
affected_modules: [".release-scan"]
related_tests: []
commit_ref: "release/0.1.2"
author: "Brandon Stonebridge"
timestamp: "2026-10-08T21:20:00Z"
```

```yaml
id: CU-20261008-004
type: chore
title: "Release 0.1.2 published on st0nebridge/NitroTray"
description: "Pushed on the owner's request. Backup first: a bundle of every local ref, the remote's refs and the v0.1.0/v0.1.1 release assets and notes, kept in the git directory (backup/2026-10-08-before-0.1.2). One atomic push, leased on the remote's old values: main bc9a00e -> b64bba6 (forced, new root), tag v0.1.2 added, tags v0.1.0 and v0.1.1 deleted. The GitHub releases v0.1.0 and v0.1.1 were deleted. v0.1.2 was created as a draft from the export's dist/ (NitroTray-0.1.2-setup.exe and SHA256SUMS.txt), downloaded back and checked against SHA256SUMS.txt, then published as Latest; GitHub reports the installer's digest as sha256 85dfef52b9ea88ebbb76eef4ba6770f707b6c5890d22c585b4cae1a6943ba699. git ls-remote origin now lists only main and v0.1.2 at b64bba6. The repository stays private. Local branch public tracks origin/main; the development main is not pushed and keeps the old v0.1.0/v0.1.1 tags locally."
impact: none
affected_modules: []
related_tests: []
commit_ref: "v0.1.2"
author: "Brandon Stonebridge"
timestamp: "2026-10-08T21:30:00Z"
```

```yaml
id: CU-20261029-001
type: feature
title: "Tray icon gestures: tooltip on hover, details popup on click, main popup on double click"
description: "Owner's request (D-20261029-001, which supersedes D-20261001-002). Hovering the CPU or GPU graph icon now shows Windows' plain tooltip, always; the 350 ms hover flyout and src/tray/hover.rs are gone, and so is the hover_details setting (an old config that still has it loads unchanged; the Settings row is replaced by a hint). A single left click on a graph icon opens that icon's details popup (the former flyout page, 340x214 DIP): it is now a focused window like the main popup, closed by Escape (its page may send close_popup besides ready), a second click on the icon, or clicking elsewhere when close on focus loss is on; a click on the icon that the focus loss had just closed counts as the close, not a reopen. A double left click on any icon opens the main popup and leaves it open. Windows delivers a double click as press, release, double-click, release, so the first release has already opened the details popup and the double click swaps it for the main popup; the release that ends the double click is ignored, and every press clears that flag. New module tray::clicks holds the gesture logic; the runtime and app::surfaces carry it out (toggle_details, show_flyout takes focus, check_deactivation covers both popups, only one popup is shown at a time). The app icon's single click still toggles the main popup, where double-clicking it used to open and immediately close it (the second release toggled it back). Unchanged: the details popup reads only embedded-controller values and does not count as a visible surface for NVML. Replaced tests: tests/unit/hover_test.rs by clicks_test.rs; the hover cases of cu_20261001_002 by cu_20261029_001 (its driver-metrics case stays); cu_20261001_001 keeps its assertions and now owns the show-without-activating helpers that the app no longer uses. The settings-form golden changed by the removed row and the new hint only. tests/live/webviews.ps1 gained the gesture checks: the notification area's own mouse messages are posted to a graph icon's window, and a second scenario checks that focus loss closes the details popup. Verified on a release build, 3 runs of 35 checks, all passing (one earlier run failed its double-click check because the desktop app hosting the test took the foreground and the popup's close-on-focus-loss rule correctly closed it; the gesture scenario now turns that setting off). Not exercised: a physical click on the real notification area, the tooltip text itself, and focus hand-off on a real double click (the live checks post the shell's messages and cannot give the app the focus right a real click carries)."
impact: medium
affected_modules: ["src/tray/clicks.rs", "src/tray/mod.rs", "src/app/runtime.rs", "src/app/surfaces.rs", "src/app/tray_host.rs", "src/config/settings.rs", "src/ui/webview.rs", "src/ui/flyout.rs", "src/ui/web/js/flyout.js", "src/ui/web/js/settings.js", "README.md", "docs/architecture.md", "DECISIONS.md"]
related_tests: ["tests/regression/cu_20261029_001.rs", "tests/unit/clicks_test.rs", "tests/ui/entries.test.mjs", "tests/live/webviews.ps1"]
commit_ref: "feature/tray-click-gestures"
author: "Brandon Stonebridge"
timestamp: "2026-10-29T06:00:00Z"
```

```yaml
id: CU-20261029-002
type: feature
title: "Tensor-core lock state in the popup and the GPU details popup"
description: "Owner's request (D-20261029-002, D-20261029-003): show whether the GPU's tensor-core throttle is lifted, from a live measurement and from the UEFI unlocker's boot log, in both the popup and the GPU details popup. New module unlock: ptx generates a loop of eight independent mma.sync.m16n8k16 chains in PTX (FP32 and FP16 accumulate), cuda binds nvcuda.dll at the moment of use, JIT-compiles it through the driver (no toolkit), times each kernel (best of three, CUDA events) and destroys the context again; state turns the ratio into a verdict (0.50 throttled, about 1.0 lifted, above 0.75 unlocked); boot_log reduces an unlocker RUN log to run, status, build and write time; view combines both into one wording (Unlocked, Locked, Unlock lost when this session's log says unlocked but the rate is half, Checking, or nothing: no measurement is never 'locked'); tracker keeps the last reading, the last failure and the next-due time (a reading is reused for 60 s, a failure is not retried sooner, one check at a time). The check runs on its own thread (app::tensor) when the popup opens, when the GPU icon is clicked and when the badge is clicked, and never with GPU driver telemetry off. The helper gets a seventh typed command, get_unlock_boot (service::{esp, unlock_log}): no argument, reads only the highest-numbered RUN<n>.TXT in the one fixed GPU4090-UNLOCK directory of the FAT volume that holds it, found by enumerating volume GUID paths so nothing is mounted or written, at most 256 KiB, returns a typed summary and never file contents, and is answerable without the Acer interface. UI: a badge in the popup's GPU block (free strip above the temperature, click to re-measure, tooltip with the full sentence and the boot line; hidden when there is nothing to claim) and a two-line row in the GPU details popup (the graph gives up 22 DIP); the popup is unchanged when the badge is hidden. The model's UI-state derivation moved to app::ui_state to keep every file under 400 lines. The installer's helper description says seven typed commands; the GPU driver telemetry hint mentions the check; docs/images popup and GPU details images were regenerated (bin/ui-snap.ps1 -Unlock). Measured on the AN515-58 (RTX 4060 Laptop, hybrid mode, throttle on): a Python spike read 27.6 / 55.1 TFLOPS (0.50); the Rust probe read 0.50 and, in a repeat test, 0.46 and 0.50, about a third of a second per probe; the EFI partition was read elevated by volume path (RUN0028, unlocked, written 2 s before this session's boot, so the FAT-to-UTC conversion and the session rule hold); an end-to-end run with the new helper in console mode and an isolated app showed 'Unlock lost, ratio 0.56, Unlocker run RUN0028: unlocked at boot' in the GPU details popup, which is what happened on this boot (unlocked, then the RTX was powered off before the keeper started); against the installed 0.1.2 helper, which does not know the command, the same run showed 'Locked, ratio 0.50' with no boot line. Tests: 370 unit, 41 regression (cu_20261029_002), 96 UI, three ignored live tests; the live webview check passed 35 of 35 on the final build; clippy and rustfmt clean; module fitness 100 of 100 with no file over 400 lines. Coverage of the logic modules is 95 to 100 %; cuda.rs and esp.rs are glue under the new waiver quality/waivers/hardware-glue.md. Mutation testing of the new pure modules: 109 mutants, 94 caught, 9 missed, 6 unviable; the missed ones got tests, dead code (UnlockTracker::checking) was removed, and one equivalent mutant (TensorRates::ratio, acc16 > 0.0 against the is_finite check beside it) is excluded in .cargo/mutants.toml with its reason; the iterate pass tested 17 mutants, 16 caught, 1 unviable, none missed. Goldens: the flyout markup gained the hidden row, the settings form's hint text changed, nothing else. Not observed: the unlocked reading (ratio about 1.0) on this machine, which needs a power-cycled unlocked boot; the 0.75 threshold sits between the measured throttled 0.50 and the unlocker's published unlocked figures."
impact: medium
affected_modules: ["src/unlock", "src/service/unlock_log.rs", "src/service/esp.rs", "src/service/core.rs", "src/service/host.rs", "src/ipc/protocol.rs", "src/ipc/client.rs", "src/app/tensor.rs", "src/app/ui_state.rs", "src/app/model.rs", "src/app/runtime.rs", "src/app/surfaces.rs", "src/app/effects.rs", "src/app/bootstrap.rs", "src/app/events.rs", "src/platform/windows.rs", "src/ui/bridge.rs", "src/ui/flyout.rs", "src/ui/web", "src/installer/nitrotray.nsi", "README.md", "docs/architecture.md", "DECISIONS.md", "quality/waivers/hardware-glue.md", "bin/ui-snap.ps1"]
related_tests: ["tests/regression/cu_20261029_002.rs", "tests/unit/unlock_state_test.rs", "tests/unit/unlock_boot_log_test.rs", "tests/unit/unlock_view_test.rs", "tests/unit/unlock_tracker_test.rs", "tests/unit/unlock_ptx_test.rs", "tests/unit/unlock_log_test.rs", "tests/unit/tensor_worker_test.rs", "tests/unit/unlock_cuda_live_test.rs", "tests/unit/esp_live_test.rs", "tests/ui/popup-view.test.mjs", "tests/ui/flyout-view.test.mjs"]
commit_ref: "feature/tensor-lock-state"
author: "Brandon Stonebridge"
timestamp: "2026-10-29T06:43:00Z"
```

```yaml
id: CU-20261029-003
type: chore
title: "Version 0.1.3 built and installed on the development machine"
description: "Owner's request: install a new release. Version 0.1.3 (Cargo.toml, Cargo.lock, package.json, package-lock.json) holds the tray icon gestures (CU-20261029-001) and the tensor-core lock state (CU-20261029-002). bin/test.ps1 passed (370 unit, 41 regression, 96 UI tests); bin/package.ps1 built dist/NitroTray-0.1.3-setup.exe, 1,128,382 bytes, sha256 9f8ab766525cd9af1b7efd0ecfaea86ba886756b72ddb6e217dd702a8ec6f770. It was run silently and elevated over the installed 0.1.2 (exit code 0): the registry says 0.1.3, NitroTray.exe and NitroTrayService.exe in the Program Files folder are byte-identical to the build, the helper service was stopped and started again (LocalSystem, automatic start kept) and the sign-in entry still points at the installed exe. The tray was then started unelevated through Explorer. End to end on the installed pair: the installed LocalSystem helper read the unlocker's newest boot log from the EFI system partition (RUN0028, unlocked) and an isolated instance of the installed app showed 'Unlock lost' in the popup badge and the GPU details popup (half rate now, ratio 0.50 and 0.45 in two checks), which matches this boot. The elevated installer round trip (tests/installer/roundtrip.ps1) needs a machine without NitroTray installed, so it is run on the release installer built from a clean export rather than on this build."
impact: none
affected_modules: ["Cargo.toml", "Cargo.lock", "package.json", "package-lock.json"]
related_tests: []
commit_ref: "release/0.1.3"
author: "Brandon Stonebridge"
timestamp: "2026-10-29T06:47:00Z"
```
