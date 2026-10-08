//! @module hardware_test
//! @description Real AN515-58 firmware, READ-ONLY: discovery, sensors, fan behaviour and platform profile
//! through the production WMI transport. Elevated runs (gsudo) exercise the firmware; normal runs assert
//! the documented AccessDenied refusal. No Set* method is ever called here.
use nitrotray::acer::backend::{AcerError, HardwareBackend};
use nitrotray::acer::gaming::AcerGaming;
use nitrotray::acer::wmi::WmiGamingExecutor;

#[test]
fn read_only_firmware_round_trip() {
    let exec = match WmiGamingExecutor::connect() {
        Ok(e) => e,
        Err(e) => {
            assert_eq!(e, AcerError::AccessDenied, "non-elevated callers are refused, nothing else");
            return;
        }
    };
    let model = nitrotray::platform::windows::machine_info().model.unwrap_or_default();
    let mut acer = AcerGaming::new(exec, model.clone());
    let caps = acer.discover();
    assert!(caps.cpu_temperature && caps.cpu_fan, "{caps:?}");
    let r = acer.sensors().unwrap();
    let cpu = r.cpu_temp_c.unwrap();
    assert!((20.0..=105.0).contains(&cpu), "CPU {cpu} °C");
    assert!(r.cpu_fan_rpm.unwrap() < 10_000);
    if caps.gpu_temperature {
        assert!((15.0..=105.0).contains(&r.gpu_temp_c.unwrap()));
    }
    assert!(acer.fan_mode().is_ok());
    let perf = acer.performance().unwrap();
    assert!(perf.raw.is_some());
    if model == "Nitro AN515-58" {
        assert!(caps.custom_fan_mode && caps.quiet_mode && caps.default_mode && caps.performance_mode);
    }
}
