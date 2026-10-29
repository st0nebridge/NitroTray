//! @module unlock_view_test
//! @description What the popup and the details popup say about the tensor unlock: the verdict, whether the
//! unlocker's boot log belongs to this session, and the wording.
use nitrotray::unlock::boot_log::{BootReport, BootStatus};
use nitrotray::unlock::state::{TensorRates, TensorState};
use nitrotray::unlock::view::{view, LiveCheck, UnlockInputs, ViewState, SESSION_GRACE_S};

const SESSION: u64 = 1_793_000_000;

fn boot(status: BootStatus, written: Option<u64>) -> BootReport {
    BootReport { run: "RUN0028".into(), status, build: "auto".into(), written_unix: written }
}

fn live(state: TensorState, acc32: f32, acc16: f32, age_s: u64) -> LiveCheck {
    LiveCheck { rates: TensorRates { acc32_tflops: acc32, acc16_tflops: acc16 }, state, age_s }
}

fn inputs() -> UnlockInputs {
    UnlockInputs { enabled: true, session_start_unix: Some(SESSION), ..UnlockInputs::default() }
}

#[test]
fn a_full_rate_reading_is_unlocked() {
    let v = view(&UnlockInputs { live: Some(live(TensorState::Unlocked, 45.5, 45.7, 12)), ..inputs() });
    assert_eq!(v.state, ViewState::Unlocked);
    assert_eq!(v.badge, "Unlocked");
    assert!(
        v.detail.contains("full rate") && v.detail.contains("1.00") && v.detail.contains("12 s ago"),
        "{}",
        v.detail
    );
    assert_eq!((v.ratio.map(|r| (r * 100.0).round()), v.age_s), (Some(100.0), Some(12)));
}

#[test]
fn a_half_rate_reading_is_locked() {
    let v = view(&UnlockInputs { live: Some(live(TensorState::Locked, 27.6, 55.1, 90)), ..inputs() });
    assert_eq!(v.state, ViewState::Locked);
    assert_eq!(v.badge, "Locked");
    assert!(
        v.detail.contains("half rate") && v.detail.contains("0.50") && v.detail.contains("1 min ago"),
        "{}",
        v.detail
    );
}

#[test]
fn locked_after_an_unlocked_boot_of_this_session_is_a_lost_unlock() {
    let v = view(&UnlockInputs {
        live: Some(live(TensorState::Locked, 27.6, 55.1, 5)),
        boot: Some(boot(BootStatus::Unlocked, Some(SESSION - 60))),
        ..inputs()
    });
    assert_eq!(v.state, ViewState::Lost);
    assert_eq!(v.badge, "Unlock lost");
    assert!(v.detail.contains("RUN0028") && v.detail.contains("reset"), "{}", v.detail);
}

#[test]
fn a_log_from_before_this_session_does_not_explain_a_locked_gpu() {
    let old = SESSION - SESSION_GRACE_S - 1;
    let v = view(&UnlockInputs {
        live: Some(live(TensorState::Locked, 27.6, 55.1, 5)),
        boot: Some(boot(BootStatus::Unlocked, Some(old))),
        ..inputs()
    });
    assert_eq!(v.state, ViewState::Locked);
    assert!(v.boot.as_deref().is_some_and(|b| b.contains("before this session")), "{:?}", v.boot);
}

#[test]
fn the_session_grace_window_includes_a_log_written_just_before_the_start() {
    let v = view(&UnlockInputs {
        live: Some(live(TensorState::Locked, 27.6, 55.1, 5)),
        boot: Some(boot(BootStatus::Unlocked, Some(SESSION - SESSION_GRACE_S))),
        ..inputs()
    });
    assert_eq!(v.state, ViewState::Lost, "the unlocker runs before Windows starts, so its log predates the session");
}

#[test]
fn a_log_without_a_time_or_a_session_start_is_given_the_benefit_of_the_doubt() {
    for (written, start) in [(None, Some(SESSION)), (Some(SESSION), None), (None, None)] {
        let v = view(&UnlockInputs {
            live: Some(live(TensorState::Locked, 27.6, 55.1, 5)),
            boot: Some(boot(BootStatus::Unlocked, written)),
            session_start_unix: start,
            ..inputs()
        });
        assert_eq!(v.state, ViewState::Lost, "{written:?} {start:?}");
    }
}

#[test]
fn a_boot_that_did_not_unlock_does_not_make_it_lost() {
    for status in [BootStatus::DryRun, BootStatus::Skipped, BootStatus::Stopped, BootStatus::NotUnlocked] {
        let v = view(&UnlockInputs {
            live: Some(live(TensorState::Locked, 27.6, 55.1, 5)),
            boot: Some(boot(status, Some(SESSION))),
            ..inputs()
        });
        assert_eq!(v.state, ViewState::Locked, "{status:?}");
    }
}

#[test]
fn an_unlocked_gpu_stays_unlocked_whatever_the_log_says() {
    let v = view(&UnlockInputs {
        live: Some(live(TensorState::Unlocked, 45.5, 45.7, 5)),
        boot: Some(boot(BootStatus::Skipped, Some(SESSION))),
        ..inputs()
    });
    assert_eq!(v.state, ViewState::Unlocked);
}

#[test]
fn the_boot_line_says_what_the_unlocker_did() {
    let line = |status, written| {
        view(&UnlockInputs { boot: Some(boot(status, written)), ..inputs() }).boot.unwrap_or_default()
    };
    assert_eq!(line(BootStatus::Unlocked, Some(SESSION)), "Unlocker run RUN0028: unlocked at boot");
    assert_eq!(line(BootStatus::DryRun, Some(SESSION)), "Unlocker run RUN0028: dry run, nothing written");
    assert_eq!(line(BootStatus::Skipped, Some(SESSION)), "Unlocker run RUN0028: skipped, nothing written");
    assert_eq!(line(BootStatus::Stopped, Some(SESSION)), "Unlocker run RUN0028: stopped");
    assert_eq!(line(BootStatus::NotUnlocked, Some(SESSION)), "Unlocker run RUN0028: did not unlock");
    assert_eq!(line(BootStatus::Unlocked, Some(1)), "Unlocker run RUN0028: unlocked at boot (before this session)");
}

#[test]
fn nothing_measured_yet_is_unknown_not_locked() {
    let v = view(&inputs());
    assert_eq!(v.state, ViewState::Unknown);
    assert_eq!(v.badge, "");
    assert!(v.detail.contains("Not checked"), "{}", v.detail);
    assert_eq!((v.ratio, v.age_s, v.boot), (None, None, None));
}

#[test]
fn the_first_check_in_flight_says_checking() {
    let v = view(&UnlockInputs { checking: true, ..inputs() });
    assert_eq!((v.state, v.badge.as_str()), (ViewState::Checking, "Checking"));
}

#[test]
fn a_recheck_keeps_the_last_reading() {
    let v = view(&UnlockInputs { checking: true, live: Some(live(TensorState::Locked, 27.6, 55.1, 90)), ..inputs() });
    assert_eq!(v.state, ViewState::Locked);
    assert!(v.detail.ends_with("Checking again."), "{}", v.detail);
}

#[test]
fn a_failed_check_is_unknown_and_says_why() {
    let v = view(&UnlockInputs { probe_error: Some("nvcuda.dll not found".into()), ..inputs() });
    assert_eq!(v.state, ViewState::Unknown);
    assert!(v.detail.contains("nvcuda.dll not found"), "{}", v.detail);
}

#[test]
fn a_failed_recheck_keeps_the_last_reading_and_notes_the_failure() {
    let v = view(&UnlockInputs {
        probe_error: Some("CUDA error 2".into()),
        live: Some(live(TensorState::Unlocked, 45.5, 45.7, 300)),
        ..inputs()
    });
    assert_eq!(v.state, ViewState::Unlocked);
    assert!(v.detail.contains("5 min ago") && v.detail.contains("CUDA error 2"), "{}", v.detail);
}

#[test]
fn with_driver_queries_off_nothing_is_claimed() {
    let v = view(&UnlockInputs { enabled: false, live: Some(live(TensorState::Unlocked, 45.5, 45.7, 5)), ..inputs() });
    assert_eq!(v.state, ViewState::Unknown);
    assert!(v.detail.contains("GPU driver telemetry"), "{}", v.detail);
}

#[test]
fn ages_are_written_in_the_largest_whole_unit() {
    let age = |s| view(&UnlockInputs { live: Some(live(TensorState::Locked, 1.0, 2.0, s)), ..inputs() }).detail;
    assert!(age(0).contains("just now"), "{}", age(0));
    assert!(age(59).contains("59 s ago"), "{}", age(59));
    assert!(age(60).contains("1 min ago"), "{}", age(60));
    assert!(age(3599).contains("59 min ago"), "{}", age(3599));
    assert!(age(3600).contains("1 h ago"), "{}", age(3600));
    assert!(age(7300).contains("2 h ago"), "{}", age(7300));
}

#[test]
fn the_view_serialises_for_the_pages() {
    let v = view(&UnlockInputs { live: Some(live(TensorState::Locked, 27.6, 55.1, 5)), ..inputs() });
    let json = serde_json::to_value(&v).unwrap();
    assert_eq!(json["state"], "locked");
    for key in ["badge", "detail", "summary", "ratio", "age_s", "boot"] {
        assert!(json.get(key).is_some(), "{key}");
    }
}

#[test]
fn the_summary_is_the_short_measurement_and_empty_without_one() {
    let with = |state, age| view(&UnlockInputs { live: Some(live(state, 27.6, 55.1, age)), ..inputs() }).summary;
    assert_eq!(with(TensorState::Locked, 0), "ratio 0.50, checked just now");
    assert_eq!(with(TensorState::Unlocked, 12), "ratio 0.50, checked 12 s ago", "the ratio is the one measured");
    assert_eq!(view(&inputs()).summary, "");
    assert_eq!(view(&UnlockInputs { checking: true, ..inputs() }).summary, "");
    let off = view(&UnlockInputs { enabled: false, live: Some(live(TensorState::Locked, 1.0, 2.0, 5)), ..inputs() });
    assert_eq!(off.summary, "");
}

#[test]
fn the_session_window_is_fifteen_minutes() {
    assert_eq!(SESSION_GRACE_S, 900);
    let lost = |before_start: u64| {
        view(&UnlockInputs {
            live: Some(live(TensorState::Locked, 27.6, 55.1, 5)),
            boot: Some(boot(BootStatus::Unlocked, Some(SESSION - before_start))),
            ..inputs()
        })
        .state
    };
    assert_eq!(lost(600), ViewState::Lost, "ten minutes before the session start: the unlocker's own boot");
    assert_eq!(lost(1_200), ViewState::Locked, "twenty minutes before: an earlier boot");
}
