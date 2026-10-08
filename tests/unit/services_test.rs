//! @module services_test
//! @description Service-state queries against the real SCM on this machine.
use nitrotray::acer::services::{query, report, ServiceState, KNOWN_SERVICES};

#[test]
fn acer_service_is_installed_here() {
    assert_ne!(query("PSSvc"), ServiceState::NotInstalled, "PSSvc exists on this AN515-58");
}

#[test]
fn unknown_service_is_not_installed() {
    assert_eq!(query("NitroTrayDefinitelyMissing"), ServiceState::NotInstalled);
}

#[test]
fn report_covers_known_services() {
    let r = report();
    assert_eq!(r.len(), KNOWN_SERVICES.len());
    assert_eq!(r[0].name, "PSSvc");
    assert!(r.iter().any(|s| s.name == nitrotray::meta::SERVICE_NAME));
    assert_eq!(serde_json::to_value(ServiceState::NotInstalled).unwrap(), "not_installed");
}
