//! @module diagnostics_test
//! @description Diagnostics report contents, timestamp formatting and file export.
use nitrotray::acer::services::{ServiceReport, ServiceState};
use nitrotray::diagnostics::*;
use nitrotray::ipc::protocol::ProviderStatus;
use nitrotray::telemetry::ProviderErrorEntry;

use crate::support::all_caps;

#[test]
fn formats_utc_timestamps() {
    assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00Z");
    assert_eq!(iso8601_utc(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(iso8601_utc(1_790_755_200), "2026-09-30T08:00:00Z");
    assert_eq!(iso8601_utc(4_107_542_399), "2100-02-28T23:59:59Z");
}

fn report() -> DiagnosticsReport {
    let errors = vec![ProviderErrorEntry { timestamp_ms: 5, source: "helper", message: "not running".into() }];
    build(ReportInputs {
        unix_secs: 1_790_755_200,
        machine: Machine {
            model: Some("Nitro AN515-58".into()),
            bios_version: Some("V2.21".into()),
            ..Machine::default()
        },
        services: vec![ServiceReport {
            name: "PSSvc".into(),
            description: "Acer".into(),
            state: ServiceState::Running,
        }],
        provider: "acer-wmi",
        status: ProviderStatus::Connected,
        capabilities: all_caps(),
        errors: &errors,
    })
}

#[test]
fn report_contains_the_specified_fields_only() {
    let r = report();
    assert_eq!(r.app, "NitroTray");
    assert_eq!(r.generated_utc, "2026-09-30T08:00:00Z");
    assert_eq!(r.recent_errors[0].source, "helper");
    let json = serde_json::to_value(&r).unwrap();
    for key in ["app_version", "machine", "services", "provider", "provider_status", "capabilities", "recent_errors"] {
        assert!(json.get(key).is_some(), "{key}");
    }
    let text = json.to_string().to_lowercase();
    for private in ["brandon", "username", "hostname", "computername"] {
        assert!(!text.contains(private), "report must exclude {private}");
    }
}

#[test]
fn writes_a_timestamped_file() {
    let dir = std::env::temp_dir().join(format!("nitrotray-diag-{}", std::process::id()));
    let path = write(&dir, &report()).unwrap();
    assert!(path.file_name().unwrap().to_string_lossy().starts_with("diagnostics-20260930T080000Z"));
    let back: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(back["machine"]["bios_version"], "V2.21");
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(windows)]
#[test]
fn live_machine_info_is_collected() {
    let m = machine();
    assert!(m.os_build.is_some() && m.model.is_some());
}
