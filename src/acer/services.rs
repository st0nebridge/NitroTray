//! @module acer::services
//! @description Queries the state of Windows services relevant to NitroTray (Acer PSSvc, the NitroTray helper).
//!
//! @input  Service names.
//! @output `ServiceState` per service, for diagnostics and the settings integration page.
//! @dependencies serde, windows-service (Windows)
use serde::Serialize;

/// Services worth reporting in diagnostics: (service name, description).
pub const KNOWN_SERVICES: &[(&str, &str)] =
    &[("PSSvc", "Acer NitroSense / Predator service"), (crate::meta::SERVICE_NAME, "NitroTray privileged helper")];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Running,
    Stopped,
    Pending,
    NotInstalled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceReport {
    pub name: String,
    pub description: String,
    pub state: ServiceState,
}

#[cfg(windows)]
pub fn query(name: &str) -> ServiceState {
    use windows_service::service::{ServiceAccess, ServiceState as S};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    let Ok(manager) = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT) else {
        return ServiceState::Unknown;
    };
    let service = match manager.open_service(name, ServiceAccess::QUERY_STATUS) {
        Ok(s) => s,
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1060) => return ServiceState::NotInstalled,
        Err(_) => return ServiceState::Unknown,
    };
    match service.query_status().map(|s| s.current_state) {
        Ok(S::Running) => ServiceState::Running,
        Ok(S::Stopped) => ServiceState::Stopped,
        Ok(_) => ServiceState::Pending,
        Err(_) => ServiceState::Unknown,
    }
}

#[cfg(not(windows))]
pub fn query(_name: &str) -> ServiceState {
    ServiceState::Unknown
}

/// State of every known service.
pub fn report() -> Vec<ServiceReport> {
    KNOWN_SERVICES
        .iter()
        .map(|(name, description)| ServiceReport {
            name: (*name).to_string(),
            description: (*description).to_string(),
            state: query(name),
        })
        .collect()
}
