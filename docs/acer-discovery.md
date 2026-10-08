# NitroSense integration discovery

Machine: **Acer Nitro AN515-58**, BIOS V2.21, i7-12650H, RTX 4060 Laptop GPU, Windows 11 Pro 26200.
Everything below was observed on this machine on 2026-09-30 unless marked otherwise.

## What is installed

| Component | Where | Notes |
|---|---|---|
| NitroSense 3.1 (UI) | AppX `AcerIncorporated.NitroSenseV31_3.1.3056.0`, AUMID `AcerIncorporated.NitroSenseV31_48frkmn4z8aw4!App` | Runs as the user; launched by NitroTray's "Acer Quick Settings" |
| Predator/NitroSense service | `PSSvc` → `C:\Program Files\Acer\NitroSense Service\PSSvc.exe`, LocalSystem, manual start | Native (MSVC) binary, not .NET |
| Companion binaries | `PSAgent.exe`, `PSAdminAgent.exe`, `PSLauncher.exe`, `SdkWrapper.dll`, `IntelOverclockingSDK.dll` | Same folder |
| Named pipes | `\\.\pipe\predatorsense_service_namedpipe`, `\\.\pipe\PredatorSense_admin_agent_1` | Undocumented wire format — **not** reverse engineered (DECISIONS D-002) |
| NitroSense state | `HKLM\SOFTWARE\OEM\NitroSense\{FanControl,Overclock,…}` | e.g. `CurrentOperationMode=4`, `CPUFanPercentage=50`; NitroTray reads it only for discovery and never writes it (D-011) |

## The firmware interface

`root\WMI` class **`AcerGamingFunction`**, instance `ACPI\PNP0C14\APGe_0` — the gaming-function
interface that upstream Linux `acer-wmi` drives (method ids 5/14–17/22–23 in its `WMID_GUID4` block).
NitroSense uses the same class: `PSSvc.exe` and `PSAdminAgent.exe` both embed the strings
`AcerGamingFunction`, `GetGamingSysInfo`, `SetGamingFanBehavior` (PSSvc also `SetGamingMiscSetting`)
alongside `predatorsense_service_namedpipe` (string scan, 2026-09-30).
The AN515-58 is in the Linux quirk table as `quirk_acer_nitro_an515_58 { predator_v4 = 1, pwm = 1 }`.

Access: **Access denied** to a normal user (enumeration fails) → NitroTray's privileged helper (D-003).

Method shapes (from the class definition): every `Get*` takes `gmInput: uint32` and returns
`gmOutput: uint64`; every `Set*` takes `gmInput: uint64` and returns `gmOutput: uint32`. In every output,
bits 7:0 are a status byte, 0 = success.

### Commands NitroTray uses

| Purpose | Method | Input | Output decoding |
|---|---|---|---|
| Supported sensors | `GetGamingSysInfo` | `0x0000` | bits 39:24 bitmap, bit (id − 1) |
| Sensor reading | `GetGamingSysInfo` | `0x0001 \| id << 8` | bits 23:8 (°C or RPM) |
| Fan behaviour (read) | `GetGamingFanBehavior` | fan bitmap (`CPU 0x01 \| GPU 0x08`) | CPU mode bits 9:8, GPU 15:14 (1 auto, 2 turbo/max, 3 custom) |
| Fan behaviour (write) | `SetGamingFanBehavior` | bitmap \| cpu_mode << 16 \| gpu_mode << 22 | status |
| Fan duty (read) | `GetGamingFanSpeed` | fan id (CPU 0x01, GPU 0x04) | bits 15:8 = **stored custom set-point**, not live duty (D-006) |
| Fan duty (write) | `SetGamingFanSpeed` | id \| pct << 8 (pct ≤ 100) | status (1 = no such fan, 2 = invalid) |
| Supported profiles | `GetGamingMiscSetting` | `0x0A` | bits 15:8 bitmap, bit N = profile N |
| Platform profile | `Get/SetGamingMiscSetting` | `0x0B` (\| value << 8 to set) | bits 15:8 |

Sensor ids: CPU temperature 0x01, CPU fan 0x02, external temperature 0x03, GPU fan 0x06, GPU temperature 0x0A.
Profiles: Quiet 0x00, Balanced ("Default") 0x01, Performance 0x04, Turbo 0x05, Eco 0x06.

### Read-only probe results (elevated via gsudo)

| Query | Raw | Meaning |
|---|---|---|
| `GetGamingSysInfo(0x0000)` | `0x227000000` | sensors {1, 2, 3, 6, 10} all present |
| `GetGamingSysInfo(0x0101)` | `0x4100` | CPU 65 °C |
| `GetGamingSysInfo(0x0201)` | `0x168900` | CPU fan 5769 RPM |
| `GetGamingSysInfo(0x0601)` | `0x17EA00` | GPU fan 6122 RPM |
| `GetGamingSysInfo(0x0A01)` | `0x3100` | GPU 49 °C |
| `GetGamingFanBehavior(0x09)` | `0x4100` | CPU Auto, GPU Auto |
| `GetGamingFanSpeed(0x01/0x04)` | `0x3200` | 50 % (equals NitroSense `CPUFanPercentage`) |
| `GetGamingMiscSetting(0x0A)` | `0x3300` | profiles {0, 1, 4, 5} |
| `GetGamingMiscSetting(0x0B)` | `0x0400` | Performance (NitroSense `CurrentOperationMode=4`) |

### Live control verification (production helper, `tests/unit/live_control_test.rs`)

Start fan Auto / performance Performance → idempotent writes accepted → Max: CPU fan 7894 RPM,
GPU fan 8000 RPM → Performance→Default→Performance with read-back → Custom curve active (controller
applied CPU 100 % under a ≥ 90 °C load via the safety override, GPU 46 %) → restored Auto /
Performance and confirmed by read-back.

## Model differences and gating

- Fan-mode control requires `GetGamingFanBehavior` to read back values 1–3 for both fans.
- Direct duty control (Custom) is **model-gated** to `CUSTOM_FAN_MODELS` (currently `Nitro AN515-58`,
  matching Linux `pwm = 1`), and additionally requires `GetGamingFanSpeed` to succeed.
- Performance modes come from the supported-profiles bitmap; the popup never invents a fourth mode —
  Turbo/Eco are reported as "other" if firmware is in them (D-005).
- Models without `AcerGamingFunction` get `NotPresent`: telemetry falls back to PDH/NVML and every
  control is shown disabled with the reason (read-only degradation).

## Known interactions with NitroSense

- NitroSense's UI may show a stale mode after NitroTray changes it (NitroTray does not write NitroSense's
  registry); NitroTray itself always reads firmware state back.
- PSSvc may re-apply its stored mode on power events; NitroTray's popup reflects whatever the firmware
  reports on its next poll.
