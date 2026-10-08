//! @module tray::positioning
//! @description Places the popup beside the notification area on any taskbar edge.
//!
//! @input  Popup size (physical px), tray-icon anchor rect, monitor work area, taskbar edge.
//! @output Top-left popup position fully inside the work area; Win32 queries for edge and monitor.
//! @dependencies windows (Shell, Gdi, HiDpi) for the queries; the maths is pure.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn center(&self) -> (i32, i32) {
        ((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarEdge {
    Bottom,
    Top,
    Left,
    Right,
}

/// Gap between the popup and the taskbar / screen edge, in physical pixels at 100 %.
pub const MARGIN_DIP: f64 = 12.0;

/// Which side the taskbar occupies, from how the work area is inset inside the monitor.
pub fn infer_edge(monitor: Rect, work: Rect) -> TaskbarEdge {
    let insets = [
        (work.top - monitor.top, TaskbarEdge::Top),
        (monitor.bottom - work.bottom, TaskbarEdge::Bottom),
        (work.left - monitor.left, TaskbarEdge::Left),
        (monitor.right - work.right, TaskbarEdge::Right),
    ];
    insets.iter().filter(|(d, _)| *d > 0).max_by_key(|(d, _)| *d).map_or(TaskbarEdge::Bottom, |(_, e)| *e)
}

fn clamp_axis(start: i32, size: i32, lo: i32, hi: i32) -> i32 {
    if size >= hi - lo {
        lo
    } else {
        start.clamp(lo, hi - size)
    }
}

/// Popup top-left. With no anchor (hotkey), the popup goes to the tray corner of the edge.
pub fn popup_position(
    size: (i32, i32),
    anchor: Option<Rect>,
    work: Rect,
    edge: TaskbarEdge,
    margin: i32,
) -> (i32, i32) {
    let (w, h) = size;
    let (lo_x, hi_x) = (work.left + margin, work.right - margin);
    let (lo_y, hi_y) = (work.top + margin, work.bottom - margin);
    let (ax, ay) = anchor.map_or((work.right, work.bottom), |a| a.center());
    let (x, y) = match edge {
        TaskbarEdge::Bottom => (ax - w / 2, work.bottom - margin - h),
        TaskbarEdge::Top => (ax - w / 2, work.top + margin),
        TaskbarEdge::Left => (work.left + margin, ay - h / 2),
        TaskbarEdge::Right => (work.right - margin - w, ay - h / 2),
    };
    (clamp_axis(x, w, lo_x, hi_x), clamp_axis(y, h, lo_y, hi_y))
}

/// Monitor geometry around a point, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorGeometry {
    pub monitor: Rect,
    pub work: Rect,
    pub scale: f64,
}

#[cfg(windows)]
pub fn monitor_at(x: i32, y: i32) -> Option<MonitorGeometry> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    // SAFETY: plain queries writing into locals.
    unsafe {
        let hmon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(hmon, &mut info).as_bool() {
            return None;
        }
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let r = |r: windows::Win32::Foundation::RECT| Rect::new(r.left, r.top, r.right, r.bottom);
        Some(MonitorGeometry { monitor: r(info.rcMonitor), work: r(info.rcWork), scale: f64::from(dx) / 96.0 })
    }
}

/// Taskbar edge as reported by the shell (falls back to inference from the work area).
#[cfg(windows)]
pub fn taskbar_edge(geometry: &MonitorGeometry) -> TaskbarEdge {
    use windows::Win32::UI::Shell::{
        SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETTASKBARPOS, APPBARDATA,
    };
    let mut data = APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() };
    // SAFETY: the shell fills a local struct.
    let ok = unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) } != 0;
    let bar = Rect::new(data.rc.left, data.rc.top, data.rc.right, data.rc.bottom);
    let (cx, cy) = bar.center();
    let m = geometry.monitor;
    let on_this_monitor = cx >= m.left && cx < m.right && cy >= m.top && cy < m.bottom;
    if ok && on_this_monitor {
        match data.uEdge {
            ABE_TOP => return TaskbarEdge::Top,
            ABE_LEFT => return TaskbarEdge::Left,
            ABE_RIGHT => return TaskbarEdge::Right,
            ABE_BOTTOM => return TaskbarEdge::Bottom,
            _ => {}
        }
    }
    infer_edge(geometry.monitor, geometry.work)
}
