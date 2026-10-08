//! @module positioning_test
//! @description Popup placement for all four taskbar edges, clamping, and live monitor queries.
use nitrotray::tray::positioning::{infer_edge, popup_position, Rect, TaskbarEdge};

const MON: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };

#[test]
fn bottom_taskbar_puts_popup_above_the_icon() {
    let work = Rect::new(0, 0, 1920, 1032);
    let anchor = Rect::new(1500, 1040, 1524, 1064);
    assert_eq!(popup_position((572, 440), Some(anchor), work, TaskbarEdge::Bottom, 12), (1226, 580));
}

#[test]
fn top_left_and_right_taskbars() {
    let a = Rect::new(1500, 8, 1524, 32);
    assert_eq!(popup_position((572, 440), Some(a), Rect::new(0, 48, 1920, 1080), TaskbarEdge::Top, 12), (1226, 60));
    let l = Rect::new(8, 900, 32, 924);
    assert_eq!(popup_position((572, 440), Some(l), Rect::new(48, 0, 1920, 1080), TaskbarEdge::Left, 12), (60, 628));
    let r = Rect::new(1888, 900, 1912, 924);
    assert_eq!(popup_position((572, 440), Some(r), Rect::new(0, 0, 1872, 1080), TaskbarEdge::Right, 12), (1288, 628));
}

#[test]
fn clamps_inside_the_work_area() {
    let work = Rect::new(0, 0, 1920, 1032);
    let corner = Rect::new(1900, 1040, 1920, 1064);
    assert_eq!(popup_position((572, 440), Some(corner), work, TaskbarEdge::Bottom, 12).0, 1920 - 12 - 572);
    let left = Rect::new(0, 1040, 20, 1064);
    assert_eq!(popup_position((572, 440), Some(left), work, TaskbarEdge::Bottom, 12).0, 12);
    let low = Rect::new(8, 1070, 32, 1080);
    assert_eq!(
        popup_position((572, 440), Some(low), Rect::new(48, 0, 1920, 1080), TaskbarEdge::Left, 12).1,
        1080 - 12 - 440
    );
}

#[test]
fn no_anchor_uses_the_tray_corner() {
    let work = Rect::new(0, 0, 1920, 1032);
    assert_eq!(popup_position((572, 440), None, work, TaskbarEdge::Bottom, 12), (1920 - 12 - 572, 1032 - 12 - 440));
}

#[test]
fn oversized_popup_is_pinned_to_the_origin() {
    let work = Rect::new(100, 100, 400, 300);
    assert_eq!(popup_position((572, 440), None, work, TaskbarEdge::Bottom, 12), (112, 112));
}

#[test]
fn infers_edge_from_work_area() {
    assert_eq!(infer_edge(MON, Rect::new(0, 0, 1920, 1032)), TaskbarEdge::Bottom);
    assert_eq!(infer_edge(MON, Rect::new(0, 48, 1920, 1080)), TaskbarEdge::Top);
    assert_eq!(infer_edge(MON, Rect::new(62, 0, 1920, 1080)), TaskbarEdge::Left);
    assert_eq!(infer_edge(MON, Rect::new(0, 0, 1858, 1080)), TaskbarEdge::Right);
    assert_eq!(infer_edge(MON, MON), TaskbarEdge::Bottom, "auto-hide taskbar");
}

#[test]
fn rect_helpers() {
    let r = Rect::new(10, 20, 30, 60);
    assert_eq!((r.width(), r.height(), r.center()), (20, 40, (20, 40)));
}

#[cfg(windows)]
#[test]
fn live_monitor_and_taskbar_queries() {
    let geo = nitrotray::tray::positioning::monitor_at(10, 10).expect("primary monitor");
    assert!(geo.scale >= 1.0);
    assert!(geo.work.width() > 0 && geo.work.height() <= geo.monitor.height());
    let _ = nitrotray::tray::positioning::taskbar_edge(&geo);
}
