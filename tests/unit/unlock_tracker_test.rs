//! @module unlock_tracker_test
//! @description The app's memory of the tensor check: when another check is due, what a failed or running
//! one leaves in the view, and how the boot report and the session start reach the wording.
use nitrotray::unlock::boot_log::{BootReport, BootStatus};
use nitrotray::unlock::state::TensorRates;
use nitrotray::unlock::tracker::{UnlockTracker, STALE_MS};
use nitrotray::unlock::view::ViewState;

const LOCKED: TensorRates = TensorRates { acc32_tflops: 27.6, acc16_tflops: 55.1 };
const UNLOCKED: TensorRates = TensorRates { acc32_tflops: 54.0, acc16_tflops: 55.1 };
const SESSION: u64 = 1_793_000_000;

fn boot(status: BootStatus, written: u64) -> BootReport {
    BootReport { run: "RUN0028".into(), status, build: "auto".into(), written_unix: Some(written) }
}

#[test]
fn a_new_tracker_has_checked_nothing() {
    let t = UnlockTracker::default();
    assert!(t.due(0, false));
    let v = t.view(true, 0);
    assert_eq!(v.state, ViewState::Unknown);
    assert!(v.detail.contains("Not checked"));
}

#[test]
fn a_running_check_is_never_started_twice() {
    let mut t = UnlockTracker::default();
    t.begin();
    assert!(!t.due(1_000_000, true), "not even when forced");
    assert_eq!(t.view(true, 0).state, ViewState::Checking);
}

#[test]
fn a_finished_check_shows_its_verdict_and_ages() {
    let mut t = UnlockTracker::default();
    t.begin();
    t.finish(Ok(LOCKED), Ok(None), 1_000);
    let v = t.view(true, 13_000);
    assert_eq!(v.state, ViewState::Locked);
    assert_eq!(v.age_s, Some(12));
    t.begin();
    t.finish(Ok(UNLOCKED), Ok(None), 20_000);
    assert_eq!(t.view(true, 20_000).state, ViewState::Unlocked);
}

#[test]
fn another_check_is_due_after_the_stale_time_or_when_asked() {
    let mut t = UnlockTracker::default();
    t.begin();
    t.finish(Ok(LOCKED), Ok(None), 1_000);
    assert!(!t.due(1_000 + STALE_MS - 1, false));
    assert!(t.due(1_000 + STALE_MS, false));
    assert!(t.due(1_001, true), "an explicit re-check does not wait");
}

#[test]
fn a_failed_check_keeps_the_last_reading_and_says_why() {
    let mut t = UnlockTracker::default();
    t.begin();
    t.finish(Ok(UNLOCKED), Ok(None), 1_000);
    t.begin();
    t.finish(Err("CUDA error 719".into()), Ok(None), 70_000);
    let v = t.view(true, 71_000);
    assert_eq!(v.state, ViewState::Unlocked, "the last good reading stands, with its age");
    assert_eq!(v.age_s, Some(70));
    assert!(v.detail.contains("CUDA error 719"), "{}", v.detail);
    t.begin();
    t.finish(Ok(LOCKED), Ok(None), 140_000);
    assert!(!t.view(true, 140_000).detail.contains("CUDA error"), "a good check clears the error");
}

#[test]
fn a_failed_check_is_not_retried_every_time_a_window_opens() {
    let mut t = UnlockTracker::default();
    t.begin();
    t.finish(Err("nvcuda.dll not found (no NVIDIA driver)".into()), Ok(None), 5_000);
    assert!(!t.due(5_000 + STALE_MS - 1, false));
    assert!(t.due(5_000 + STALE_MS, false));
}

#[test]
fn a_gpu_that_reports_no_throughput_is_a_failed_check_not_a_verdict() {
    let mut t = UnlockTracker::default();
    t.begin();
    t.finish(Ok(TensorRates { acc32_tflops: 0.0, acc16_tflops: 0.0 }), Ok(None), 1_000);
    let v = t.view(true, 1_000);
    assert_eq!(v.state, ViewState::Unknown);
    assert!(v.detail.contains("no throughput"), "{}", v.detail);
}

#[test]
fn the_boot_report_and_the_session_start_decide_whether_an_unlock_was_lost() {
    let mut t = UnlockTracker::default();
    t.set_session_start(Some(SESSION));
    t.begin();
    t.finish(Ok(LOCKED), Ok(Some(boot(BootStatus::Unlocked, SESSION - 2))), 1_000);
    assert_eq!(t.view(true, 1_000).state, ViewState::Lost);
    t.begin();
    t.finish(Ok(LOCKED), Ok(Some(boot(BootStatus::Unlocked, SESSION - 86_400))), 2_000);
    assert_eq!(t.view(true, 2_000).state, ViewState::Locked, "yesterday's unlock is not this session's");
}

#[test]
fn a_helper_that_cannot_answer_keeps_the_last_boot_report() {
    let mut t = UnlockTracker::default();
    t.set_session_start(Some(SESSION));
    t.begin();
    t.finish(Ok(LOCKED), Ok(Some(boot(BootStatus::Unlocked, SESSION - 2))), 1_000);
    t.begin();
    t.finish(Ok(LOCKED), Err("helper not running".into()), 2_000);
    assert!(t.view(true, 2_000).boot.is_some(), "an unreachable helper is not 'no log'");
    t.begin();
    t.finish(Ok(LOCKED), Ok(None), 3_000);
    assert!(t.view(true, 3_000).boot.is_none(), "but a helper that says there is no log means there is none");
}

#[test]
fn nothing_is_claimed_while_driver_queries_are_off() {
    let mut t = UnlockTracker::default();
    t.begin();
    t.finish(Ok(UNLOCKED), Ok(None), 1_000);
    assert_eq!(t.view(false, 1_000).state, ViewState::Unknown);
}
