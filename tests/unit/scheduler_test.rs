//! @module scheduler_test
//! @description Poll cadences for open/closed UI states and the due/next-wake logic.
use nitrotray::telemetry::scheduler::{cadence, Cadence, DueSet, PollScheduler, RefreshRate};

#[test]
fn open_popup_uses_the_specified_cadences() {
    let c = cadence(RefreshRate::Normal, 2000, true, false);
    assert_eq!(c.utilisation_ms, Some(500));
    assert_eq!(c.clocks_ms, Some(1000));
    assert_eq!(c.temperatures_ms, Some(1000));
    assert_eq!(c.fans_ms, Some(1000));
    assert_eq!(c.modes_ms, Some(3000));
    let fast = cadence(RefreshRate::Fast, 2000, true, true);
    assert_eq!(
        (fast.clocks_ms, fast.temperatures_ms, fast.fans_ms, fast.modes_ms),
        (Some(500), Some(500), Some(500), Some(2000))
    );
    let relaxed = cadence(RefreshRate::Relaxed, 2000, true, true);
    assert_eq!(relaxed.utilisation_ms, Some(1000));
    assert_eq!(relaxed.clocks_ms, Some(2000));
    assert_eq!(relaxed.temperatures_ms, Some(2000));
    assert_eq!(relaxed.fans_ms, Some(2000));
    assert_eq!(relaxed.modes_ms, Some(5000));
    assert_eq!(fast.utilisation_ms, Some(500));
}

#[test]
fn closed_popup_suspends_nonessential_polling() {
    let with_icons = cadence(RefreshRate::Normal, 2000, false, true);
    assert_eq!(with_icons.utilisation_ms, None);
    assert_eq!(with_icons.clocks_ms, None);
    assert_eq!(with_icons.temperatures_ms, Some(2000));
    assert_eq!(with_icons.fans_ms, Some(2000));
    assert_eq!(with_icons.modes_ms, Some(5000));
    let no_icons = cadence(RefreshRate::Normal, 2000, false, false);
    assert_eq!((no_icons.temperatures_ms, no_icons.fans_ms), (None, None));
    assert_eq!(no_icons.modes_ms, Some(5000));
}

#[test]
fn background_interval_is_clamped() {
    assert_eq!(cadence(RefreshRate::Normal, 10, false, true).temperatures_ms, Some(1000));
    assert_eq!(cadence(RefreshRate::Normal, 60_000, false, true).fans_ms, Some(5000));
    assert_eq!(cadence(RefreshRate::Normal, 3000, false, true).fans_ms, Some(3000));
}

#[test]
fn everything_is_due_first_then_by_interval() {
    let c = cadence(RefreshRate::Normal, 2000, true, true);
    let mut s = PollScheduler::new();
    assert_eq!(s.due(0, &c), DueSet::all());
    let d = s.due(499, &c);
    assert!(!d.any());
    let d = s.due(500, &c);
    assert!(d.utilisation && !d.clocks && !d.temperatures && !d.fans && !d.modes);
    let d = s.due(1000, &c);
    assert!(d.utilisation && d.clocks && d.temperatures && d.fans && !d.modes);
    let d = s.due(3000, &c);
    assert!(d.modes);
    s.reset();
    assert_eq!(s.due(3001, &c), DueSet::all());
}

#[test]
fn suspended_groups_are_never_due() {
    let c = cadence(RefreshRate::Normal, 2000, false, false);
    let mut s = PollScheduler::new();
    let d = s.due(0, &c);
    assert!(!d.utilisation && !d.clocks && !d.temperatures && !d.fans && d.modes);
    let none = Cadence { utilisation_ms: None, clocks_ms: None, temperatures_ms: None, fans_ms: None, modes_ms: None };
    assert!(!s.due(10_000, &none).any());
    assert_eq!(s.next_wake_ms(10_000, &none), None);
}

#[test]
fn next_wake_reports_the_soonest_group() {
    let c = cadence(RefreshRate::Normal, 2000, true, true);
    let mut s = PollScheduler::new();
    assert_eq!(s.next_wake_ms(0, &c), Some(0));
    s.due(0, &c);
    assert_eq!(s.next_wake_ms(0, &c), Some(500));
    assert_eq!(s.next_wake_ms(300, &c), Some(200));
    assert_eq!(s.next_wake_ms(900, &c), Some(0));
}

#[test]
fn refresh_rate_serialises_snake_case_and_defaults_normal() {
    assert_eq!(serde_json::to_string(&RefreshRate::Relaxed).unwrap(), "\"relaxed\"");
    assert_eq!(RefreshRate::default(), RefreshRate::Normal);
}

#[test]
fn any_is_true_for_each_single_flag() {
    let one = [
        DueSet { utilisation: true, ..DueSet::default() },
        DueSet { clocks: true, ..DueSet::default() },
        DueSet { temperatures: true, ..DueSet::default() },
        DueSet { fans: true, ..DueSet::default() },
        DueSet { modes: true, ..DueSet::default() },
    ];
    for d in one {
        assert!(d.any(), "{d:?}");
    }
    assert!(!DueSet::default().any());
}
