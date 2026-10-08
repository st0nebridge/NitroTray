//! @module live_control_test
//! @description LIVE firmware control through the production helper (opt-in: NITROTRAY_LIVE_PIPE names a
//! running elevated console helper). Records the starting modes, performs idempotent writes, a short
//! round trip (Auto→Max→Auto, performance away and back, a few seconds of the default custom curve),
//! and ALWAYS restores and re-reads the original state. Skips silently when the variable is unset.
use std::time::Duration;

use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::client::{HelperClient, PipeTransport};
use nitrotray::ipc::pipe::ServerPolicy;
use nitrotray::ipc::protocol::ProviderStatus;

#[test]
fn live_round_trip_restores_original_state() {
    let Ok(pipe) = std::env::var("NITROTRAY_LIVE_PIPE") else {
        eprintln!("NITROTRAY_LIVE_PIPE unset: live control test skipped");
        return;
    };
    let c = HelperClient::new(PipeTransport { name: pipe, policy: ServerPolicy::AnySession });
    let caps = c.capabilities().expect("helper reachable");
    assert_eq!(caps.status, ProviderStatus::Connected, "must be the real firmware, not simulated");
    let fan0 = c.fan_state().unwrap().mode.expect("known fan mode");
    let perf0 = c.performance().unwrap().mode.expect("known performance mode");
    eprintln!("start: fan={fan0:?} performance={perf0:?}");

    let outcome = std::panic::catch_unwind(|| {
        // Idempotent writes: nothing changes, the write path and read-back are exercised.
        assert_eq!(c.set_performance_mode(perf0).unwrap(), perf0.as_str());
        if fan0 != FanMode::Custom {
            assert_eq!(c.set_fan_mode(fan0, None).unwrap(), fan0.as_str());
        }
        // Round trip: fans.
        assert_eq!(c.set_fan_mode(FanMode::Max, None).unwrap(), "max");
        std::thread::sleep(Duration::from_secs(4));
        let s = c.sensors().unwrap();
        eprintln!("max: cpu fan {:?} rpm, gpu fan {:?} rpm", s.cpu_fan_rpm, s.gpu_fan_rpm);
        assert_eq!(c.fan_state().unwrap().cpu_duty_pct, Some(100));
        assert_eq!(c.set_fan_mode(FanMode::Auto, None).unwrap(), "auto");
        // Round trip: performance mode away and back.
        let other = if perf0 == PerformanceMode::Default { PerformanceMode::Quiet } else { PerformanceMode::Default };
        assert_eq!(c.set_performance_mode(other).unwrap(), other.as_str());
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(c.performance().unwrap().mode, Some(other));
        // Custom curve for a few ticks, driven by the helper's controller.
        assert_eq!(c.set_fan_mode(FanMode::Custom, Some(FanCurveProfile::default_profile())).unwrap(), "custom");
        std::thread::sleep(Duration::from_secs(4));
        let fs = c.fan_state().unwrap();
        eprintln!("custom: {fs:?}");
        assert!(fs.curve_active && fs.cpu_duty_pct.is_some());
    });

    // Restore, whatever happened above.
    let restore_fan = if fan0 == FanMode::Custom { FanMode::Auto } else { fan0 };
    c.set_fan_mode(restore_fan, None).expect("restore fan mode");
    c.set_performance_mode(perf0).expect("restore performance mode");
    let fan1 = c.fan_state().unwrap();
    let perf1 = c.performance().unwrap().mode;
    eprintln!("restored: fan={:?} curve_active={} performance={perf1:?}", fan1.mode, fan1.curve_active);
    assert_eq!(fan1.mode, Some(restore_fan));
    assert!(!fan1.curve_active);
    assert_eq!(perf1, Some(perf0));
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
