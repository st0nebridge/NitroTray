//! @module worker_test
//! @description Hardware-thread loop: serialised requests, periodic ticks, guaranteed shutdown.
use std::sync::mpsc;
use std::time::Duration;

use nitrotray::controls::FanCurveProfile;
use nitrotray::ipc::protocol::{encode_line, parse_response, ProviderStatus, Request};
use nitrotray::service::core::ServiceCore;
use nitrotray::service::worker::{run, submit, CoreMsg};

use crate::support::FakeBackend;

#[test]
fn serves_requests_and_stops_cleanly() {
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let mut core = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::Connected, None);
        let custom = Request::SetFanMode {
            mode: nitrotray::controls::FanMode::Custom,
            curve: Some(FanCurveProfile::default_profile()),
        };
        core.handle(custom);
        run(&mut core, &rx, Duration::from_millis(20));
        core.curve_active()
    });
    let reply = submit(&tx, &encode_line(&Request::GetSensors), Duration::from_secs(2));
    assert!(parse_response(&reply).unwrap().sensors.is_some());
    std::thread::sleep(Duration::from_millis(60));
    tx.send(CoreMsg::Stop).unwrap();
    assert!(!handle.join().unwrap(), "shutdown stops the curve");
}

#[test]
fn submit_reports_a_stopped_core() {
    let (tx, rx) = mpsc::channel::<CoreMsg>();
    drop(rx);
    let reply = submit(&tx, "{}", Duration::from_millis(50));
    assert!(parse_response(&reply).unwrap().error.unwrap().message.contains("shutting down"));
}

#[test]
fn submit_times_out_when_the_core_is_stuck() {
    let (tx, rx) = mpsc::channel::<CoreMsg>();
    let reply = submit(&tx, "{}", Duration::from_millis(30));
    assert!(parse_response(&reply).unwrap().error.unwrap().message.contains("did not answer"));
    drop(rx);
}

#[test]
fn loop_ends_when_all_senders_drop() {
    let (tx, rx) = mpsc::channel::<CoreMsg>();
    drop(tx);
    let mut core = ServiceCore::new(FakeBackend::default(), "m", ProviderStatus::Connected, None);
    run(&mut core, &rx, Duration::from_millis(5));
}
