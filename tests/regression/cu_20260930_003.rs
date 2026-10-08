//! @module cu_20260930_003
//! @description Regression test: recurring provider errors occupy one log entry each, refreshed to the
//! latest occurrence, instead of flooding the diagnostics log.
use nitrotray::acer::{FanState, PerformanceState, SensorReadings};
use nitrotray::ipc::client::{ClientError, TransportError};
use nitrotray::ipc::protocol::CapabilitiesReport;
use nitrotray::telemetry::scheduler::DueSet;
use nitrotray::telemetry::{CpuSample, CpuSource, GpuSample, GpuSource, HardwareSource, PollOptions, TelemetryService};

struct FailCpu;
impl CpuSource for FailCpu {
    fn sample(&mut self) -> Result<CpuSample, String> {
        Err("cpu down".into())
    }
}
struct FailGpu;
impl GpuSource for FailGpu {
    fn sample(&mut self) -> Result<GpuSample, String> {
        Err("gpu down".into())
    }
}
struct NoHelper;
impl HardwareSource for NoHelper {
    fn capabilities(&mut self) -> Result<CapabilitiesReport, ClientError> {
        Err(ClientError::Transport(TransportError::NotRunning))
    }
    fn sensors(&mut self) -> Result<SensorReadings, ClientError> {
        Err(ClientError::Transport(TransportError::NotRunning))
    }
    fn fan_state(&mut self) -> Result<FanState, ClientError> {
        Err(ClientError::Transport(TransportError::NotRunning))
    }
    fn performance(&mut self) -> Result<PerformanceState, ClientError> {
        Err(ClientError::Transport(TransportError::NotRunning))
    }
}

#[test]
fn alternating_recurring_errors_do_not_accumulate() {
    let mut t = TelemetryService::new(Some(Box::new(FailCpu)), Some(Box::new(FailGpu)), Box::new(NoHelper));
    let opts = PollOptions { gpu_driver: true, fans: true };
    for now in 0..100 {
        t.poll(DueSet::all(), opts, now * 20_000);
    }
    let errors = t.recent_errors();
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert!(errors.iter().all(|e| e.timestamp_ms >= 98 * 20_000));
}
