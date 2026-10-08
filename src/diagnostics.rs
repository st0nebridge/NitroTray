//! @module diagnostics
//! @description Support export: versions, machine model, Acer services, provider,
//! capability matrix and recent provider errors — and nothing else (no user or host names).
//!
//! @input  Provider state gathered by the app; machine info and service states (Windows).
//! @output A serialisable report; `export()` writes it as JSON and returns the path.
//! @dependencies serde, serde_json, acer, ipc::protocol, platform::windows (Windows), telemetry
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::acer::services::ServiceReport;
use crate::acer::Capabilities;
use crate::ipc::protocol::ProviderStatus;
use crate::telemetry::ProviderErrorEntry;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ErrorLine {
    pub at_ms: u64,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Machine {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub bios_version: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_build: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiagnosticsReport {
    pub app: &'static str,
    pub app_version: &'static str,
    pub generated_utc: String,
    pub machine: Machine,
    pub services: Vec<ServiceReport>,
    pub provider: String,
    pub provider_status: ProviderStatus,
    pub capabilities: Capabilities,
    pub recent_errors: Vec<ErrorLine>,
}

pub struct ReportInputs<'a> {
    pub unix_secs: u64,
    pub machine: Machine,
    pub services: Vec<ServiceReport>,
    pub provider: &'a str,
    pub status: ProviderStatus,
    pub capabilities: Capabilities,
    pub errors: &'a [ProviderErrorEntry],
}

/// `YYYY-MM-DDTHH:MM:SSZ` from Unix seconds (civil-from-days, proleptic Gregorian).
pub fn iso8601_utc(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let rem = unix_secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

pub fn build(i: ReportInputs<'_>) -> DiagnosticsReport {
    DiagnosticsReport {
        app: crate::meta::APP_NAME,
        app_version: crate::meta::version(),
        generated_utc: iso8601_utc(i.unix_secs),
        machine: i.machine,
        services: i.services,
        provider: i.provider.to_string(),
        provider_status: i.status,
        capabilities: i.capabilities,
        recent_errors: i
            .errors
            .iter()
            .map(|e| ErrorLine { at_ms: e.timestamp_ms, source: e.source.to_string(), message: e.message.clone() })
            .collect(),
    }
}

/// Writes `diagnostics-<utc>.json` into `dir` and returns its path.
pub fn write(dir: &Path, report: &DiagnosticsReport) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = report.generated_utc.replace([':', '-'], "");
    let path = dir.join(format!("diagnostics-{stamp}.json"));
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(windows)]
pub fn machine() -> Machine {
    let m = crate::platform::windows::machine_info();
    Machine {
        manufacturer: m.manufacturer,
        model: m.model,
        bios_version: m.bios_version,
        os_name: m.os_name,
        os_version: m.os_version,
        os_build: m.os_build,
    }
}
