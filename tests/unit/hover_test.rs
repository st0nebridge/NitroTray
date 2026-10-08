//! @module hover_test
//! @description Tray flyout hover timing: rest delay, leave/click hide, instant switch and re-show.
use nitrotray::tray::hover::{Hover, HoverAction, RESHOW_WINDOW_MS, SHOW_DELAY_MS};
use nitrotray::tray::positioning::Rect;
use nitrotray::ui::flyout::FlyoutKind::{Cpu, Gpu};

const CPU_ICON: Rect = Rect { left: 1500, top: 1040, right: 1524, bottom: 1064 };
const GPU_ICON: Rect = Rect { left: 1476, top: 1040, right: 1500, bottom: 1064 };

#[test]
fn shows_only_after_the_pointer_rests() {
    let mut h = Hover::new();
    assert_eq!(h.enter(Cpu, CPU_ICON, 1000), None);
    assert_eq!(h.deadline(), Some(1000 + SHOW_DELAY_MS));
    assert_eq!(h.tick(1000 + SHOW_DELAY_MS - 1), None);
    assert_eq!(h.shown(), None);
    assert_eq!(h.tick(1000 + SHOW_DELAY_MS), Some(HoverAction::Show(Cpu, CPU_ICON)));
    assert_eq!((h.shown(), h.deadline()), (Some(Cpu), None));
    assert_eq!(h.enter(Cpu, CPU_ICON, 1500), None, "already showing that flyout");
}

#[test]
fn passing_over_an_icon_never_shows_it() {
    let mut h = Hover::new();
    h.enter(Gpu, GPU_ICON, 0);
    assert_eq!(h.leave(Gpu, 100), None);
    assert_eq!(h.deadline(), None);
    assert_eq!(h.tick(10_000), None);
}

#[test]
fn leaving_hides_and_arms_an_instant_reshow() {
    let mut h = Hover::new();
    h.enter(Cpu, CPU_ICON, 0);
    h.tick(SHOW_DELAY_MS);
    assert_eq!(h.leave(Gpu, 400), None, "leaving another icon changes nothing");
    assert_eq!(h.leave(Cpu, 1000), Some(HoverAction::Hide));
    assert_eq!(h.shown(), None);
    assert_eq!(h.enter(Gpu, GPU_ICON, 1000 + RESHOW_WINDOW_MS - 1), Some(HoverAction::Show(Gpu, GPU_ICON)));
    h.leave(Gpu, 5000);
    assert_eq!(h.enter(Cpu, CPU_ICON, 5000 + RESHOW_WINDOW_MS), None, "the re-show window has passed");
}

#[test]
fn entering_a_neighbour_while_shown_switches_at_once() {
    let mut h = Hover::new();
    h.enter(Cpu, CPU_ICON, 0);
    h.tick(SHOW_DELAY_MS);
    assert_eq!(h.enter(Gpu, GPU_ICON, 600), Some(HoverAction::Show(Gpu, GPU_ICON)));
    assert_eq!(h.shown(), Some(Gpu));
    assert_eq!(h.leave(Cpu, 610), None, "the late leave of the old icon is ignored");
    assert_eq!(h.shown(), Some(Gpu));
}

#[test]
fn a_click_hides_and_cancels_without_arming_reshow() {
    let mut h = Hover::new();
    h.enter(Cpu, CPU_ICON, 0);
    assert_eq!(h.click(), None, "nothing shown yet");
    assert_eq!(h.tick(SHOW_DELAY_MS), None, "the pending show was dropped");
    h.enter(Cpu, CPU_ICON, 1000);
    h.tick(1000 + SHOW_DELAY_MS);
    assert_eq!(h.click(), Some(HoverAction::Hide));
    assert_eq!(h.enter(Gpu, GPU_ICON, 1400), None, "after a click the next flyout waits again");
}

#[test]
fn dismissal_from_elsewhere_resets_state() {
    let mut h = Hover::new();
    h.enter(Cpu, CPU_ICON, 0);
    h.tick(SHOW_DELAY_MS);
    h.enter(Gpu, GPU_ICON, 400);
    h.dismissed();
    assert_eq!((h.shown(), h.deadline()), (None, None));
    assert_eq!(h.leave(Gpu, 500), None);
}
