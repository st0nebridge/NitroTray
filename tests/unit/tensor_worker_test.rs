//! @module tensor_worker_test
//! @description The check worker: one measurement and one boot-report fetch per request, results delivered
//! together, a panicking probe isolated, and a clean stop.
use std::cell::RefCell;
use std::sync::mpsc;

use nitrotray::app::tensor::{run_tensor, TensorMsg, TensorResult};
use nitrotray::unlock::boot_log::{BootReport, BootStatus};
use nitrotray::unlock::state::TensorRates;

const RATES: TensorRates = TensorRates { acc32_tflops: 27.6, acc16_tflops: 55.1 };

fn report() -> BootReport {
    BootReport { run: "RUN0028".into(), status: BootStatus::Unlocked, build: "auto".into(), written_unix: Some(1) }
}

#[test]
fn each_request_measures_once_and_delivers_both_answers() {
    let (tx, rx) = mpsc::channel();
    tx.send(TensorMsg::Check).unwrap();
    tx.send(TensorMsg::Check).unwrap();
    tx.send(TensorMsg::Stop).unwrap();
    let delivered = RefCell::new(Vec::<TensorResult>::new());
    let (mut measured, mut fetched) = (0, 0);
    run_tensor(
        &rx,
        || {
            measured += 1;
            Ok(RATES)
        },
        || {
            fetched += 1;
            Ok(Some(report()))
        },
        |r| delivered.borrow_mut().push(r),
    );
    assert_eq!((measured, fetched), (2, 2));
    let delivered = delivered.into_inner();
    assert_eq!(delivered.len(), 2);
    assert_eq!(delivered[0].rates, Ok(RATES));
    assert_eq!(delivered[0].boot, Ok(Some(report())));
}

#[test]
fn failures_are_delivered_not_swallowed() {
    let (tx, rx) = mpsc::channel();
    tx.send(TensorMsg::Check).unwrap();
    tx.send(TensorMsg::Stop).unwrap();
    let delivered = RefCell::new(Vec::<TensorResult>::new());
    run_tensor(
        &rx,
        || Err("nvcuda.dll not found (no NVIDIA driver)".into()),
        || Err("helper not running".into()),
        |r| delivered.borrow_mut().push(r),
    );
    let r = &delivered.borrow()[0];
    assert_eq!(r.rates, Err("nvcuda.dll not found (no NVIDIA driver)".to_string()));
    assert_eq!(r.boot, Err("helper not running".to_string()));
}

#[test]
fn a_panicking_probe_becomes_an_error_and_the_worker_lives_on() {
    let (tx, rx) = mpsc::channel();
    tx.send(TensorMsg::Check).unwrap();
    tx.send(TensorMsg::Check).unwrap();
    tx.send(TensorMsg::Stop).unwrap();
    let delivered = RefCell::new(Vec::<TensorResult>::new());
    let mut calls = 0;
    run_tensor(
        &rx,
        || {
            calls += 1;
            if calls == 1 {
                panic!("driver blew up");
            }
            Ok(RATES)
        },
        || Ok(None),
        |r| delivered.borrow_mut().push(r),
    );
    let delivered = delivered.into_inner();
    assert_eq!(delivered.len(), 2);
    assert!(delivered[0].rates.as_ref().unwrap_err().contains("panicked"));
    assert_eq!(delivered[1].rates, Ok(RATES));
}

#[test]
fn a_closed_channel_ends_the_worker() {
    let (tx, rx) = mpsc::channel::<TensorMsg>();
    drop(tx);
    run_tensor(&rx, || Ok(RATES), || Ok(None), |_| panic!("nothing was asked"));
}
