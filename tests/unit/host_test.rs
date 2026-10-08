//! @module host_test
//! @description End-to-end in-process: Runner (simulated backend) ↔ real pipe ↔ typed client.
use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::client::{HelperClient, PipeTransport};
use nitrotray::ipc::pipe::ServerPolicy;
use nitrotray::ipc::protocol::ProviderStatus;
use nitrotray::service::host::{marker_path, Runner};

use crate::support::unique_pipe;

#[test]
fn simulated_helper_serves_the_full_command_set() {
    let name = unique_pipe("host");
    let runner = Runner::start(&name, true).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(150));
    let client = HelperClient::new(PipeTransport { name: name.clone(), policy: ServerPolicy::AnySession });
    let caps = client.capabilities().unwrap();
    assert_eq!(caps.status, ProviderStatus::Simulated);
    assert_eq!(caps.provider, "simulated");
    assert!(client.sensors().unwrap().cpu_fan_rpm.is_some());
    assert_eq!(client.set_fan_mode(FanMode::Max, None).unwrap(), "max");
    assert_eq!(client.fan_state().unwrap().cpu_duty_pct, Some(100));
    assert_eq!(client.set_performance_mode(PerformanceMode::Quiet).unwrap(), "quiet");
    assert_eq!(client.performance().unwrap().mode, Some(PerformanceMode::Quiet));
    assert_eq!(client.set_fan_mode(FanMode::Custom, Some(FanCurveProfile::default_profile())).unwrap(), "custom");
    assert!(client.fan_state().unwrap().curve_active);
    runner.stop();
    assert!(!marker_path(&name, true).unwrap().exists(), "clean stop removes the curve marker");
}

#[test]
fn production_policy_refuses_the_console_helper() {
    let name = unique_pipe("prodpolicy");
    let runner = Runner::start(&name, true).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(150));
    let client = HelperClient::new(PipeTransport { name: name.clone(), policy: ServerPolicy::ServiceOnly });
    let err = client.capabilities().unwrap_err();
    assert_eq!(err.status(), ProviderStatus::HelperUnavailable);
    runner.stop();
}

#[test]
fn from_env_defaults_to_service_policy() {
    std::env::remove_var("NITROTRAY_DEV_HELPER");
    std::env::remove_var("NITROTRAY_PIPE");
    let t = PipeTransport::from_env();
    assert_eq!(t.name, nitrotray::meta::PIPE_NAME);
    assert_eq!(t.policy, ServerPolicy::ServiceOnly);
    assert!(marker_path(nitrotray::meta::PIPE_NAME, false).unwrap().ends_with(r"NitroTray\curve-active"));
    assert!(marker_path(nitrotray::meta::PIPE_NAME, true)
        .unwrap()
        .to_string_lossy()
        .contains("curve-active-dev-pipeNitroTray"));
}
