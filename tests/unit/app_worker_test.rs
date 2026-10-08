//! @module app_worker_test
//! @description Telemetry worker loop (cadence, visibility, refresh messages) and the serial control executor.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use nitrotray::app::model::ControlRequest;
use nitrotray::app::worker::*;
use nitrotray::controls::{FanCurveProfile, FanMode, PerformanceMode};
use nitrotray::ipc::client::{HelperClient, Transport, TransportError};
use nitrotray::ipc::protocol::ProviderStatus;
use nitrotray::service::core::ServiceCore;
use nitrotray::telemetry::scheduler::RefreshRate;
use nitrotray::telemetry::TelemetryService;

use crate::support::FakeBackend;

struct CoreTransport(Mutex<ServiceCore<FakeBackend>>);
impl Transport for CoreTransport {
    fn roundtrip(&self, line: &str) -> Result<String, TransportError> {
        Ok(self.0.lock().unwrap().handle_line(line))
    }
}

fn client() -> HelperClient<CoreTransport> {
    HelperClient::new(CoreTransport(Mutex::new(ServiceCore::new(
        FakeBackend::default(),
        "m",
        ProviderStatus::Connected,
        None,
    ))))
}

fn cfg() -> TelemetryConfig {
    TelemetryConfig { rate: RefreshRate::Fast, background_ms: 1000, graph_icons: true, gpu_driver: true, fans: true }
}

#[test]
fn telemetry_loop_delivers_and_reacts_to_messages() {
    let (tx, rx) = mpsc::channel();
    let deliveries = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&deliveries);
    let clock = Arc::new(AtomicU64::new(0));
    let c = Arc::clone(&clock);
    let handle = std::thread::spawn(move || {
        let svc = TelemetryService::new(None, None, Box::new(client()));
        run_telemetry(svc, &rx, cfg(), move || c.load(Ordering::SeqCst), move |d| sink.lock().unwrap().push(d));
    });
    std::thread::sleep(Duration::from_millis(60));
    {
        let d = deliveries.lock().unwrap();
        let first = d.first().expect("initial poll delivered");
        assert_eq!(first.update.status, ProviderStatus::Connected);
        assert_eq!(first.update.provider, "fake");
        assert_eq!(first.update.snapshot.fan_mode, Some(FanMode::Auto), "modes read through HelperClient");
        assert_eq!(first.update.snapshot.performance_mode, Some(PerformanceMode::Default));
        assert!(first.due.temperatures && first.due.modes, "closed popup still polls temps for the icons");
        assert!(!first.due.utilisation, "utilisation suspended while hidden");
    }
    tx.send(WorkerMsg::Visibility(true)).unwrap();
    std::thread::sleep(Duration::from_millis(60));
    assert!(deliveries.lock().unwrap().iter().any(|d| d.due.utilisation), "opening the popup polls everything");
    let before = deliveries.lock().unwrap().len();
    tx.send(WorkerMsg::RefreshModes).unwrap();
    std::thread::sleep(Duration::from_millis(60));
    assert!(deliveries.lock().unwrap()[before..].iter().any(|d| d.due.modes && !d.due.temperatures));
    tx.send(WorkerMsg::RefreshCapabilities).unwrap();
    tx.send(WorkerMsg::Config(TelemetryConfig { rate: RefreshRate::Relaxed, ..cfg() })).unwrap();
    clock.store(10_000, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(60));
    tx.send(WorkerMsg::Stop).unwrap();
    handle.join().unwrap();
    assert!(deliveries.lock().unwrap().last().unwrap().errors.is_empty());
}

#[test]
fn telemetry_loop_ends_when_the_channel_closes() {
    let (tx, rx) = mpsc::channel::<WorkerMsg>();
    drop(tx);
    let svc = TelemetryService::new(None, None, Box::new(client()));
    run_telemetry(svc, &rx, cfg(), || 0, |_| {});
}

#[test]
fn control_executor_runs_requests_in_order() {
    let (tx, rx) = mpsc::channel();
    tx.send(ControlRequest::Fan(FanMode::Max, None)).unwrap();
    tx.send(ControlRequest::Performance(PerformanceMode::Quiet)).unwrap();
    tx.send(ControlRequest::Fan(FanMode::Custom, None)).unwrap();
    tx.send(ControlRequest::Fan(FanMode::Custom, Some(FanCurveProfile::default_profile()))).unwrap();
    drop(tx);
    let results = Mutex::new(Vec::new());
    run_control(&client(), &rx, |req, r| results.lock().unwrap().push((req, r)));
    let r = results.into_inner().unwrap();
    assert_eq!(r[0].1, Ok("max".into()));
    assert_eq!(r[1].1, Ok("quiet".into()));
    assert!(r[2].1.as_ref().unwrap_err().contains("requires a curve"), "{:?}", r[2]);
    assert_eq!(r[3].1, Ok("custom".into()));
    assert_eq!(r.len(), 4);
}
