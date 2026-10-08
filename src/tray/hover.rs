//! @module tray::hover
//! @description Hover timing for the CPU/GPU tray flyouts: show after the pointer rests on an icon,
//! hide the moment it leaves or the icon is clicked, and switch instantly between neighbouring icons.
//!
//! @input  Enter/leave/click notifications for a flyout kind with the icon rectangle, and a ms clock.
//! @output `HoverAction`s (show at an anchor / hide) and the next deadline for the event loop.
//! @dependencies tray::positioning (Rect), ui::flyout (FlyoutKind)
use crate::tray::positioning::Rect;
use crate::ui::flyout::FlyoutKind;

/// Rest time before a flyout appears (close to Windows' default 400 ms hover time).
pub const SHOW_DELAY_MS: u64 = 350;
/// After a flyout closes, entering another graph icon within this window shows it at once.
pub const RESHOW_WINDOW_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverAction {
    Show(FlyoutKind, Rect),
    Hide,
}

#[derive(Debug, Clone, Default)]
pub struct Hover {
    pending: Option<(FlyoutKind, Rect, u64)>,
    shown: Option<FlyoutKind>,
    hidden_at: Option<u64>,
}

impl Hover {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shown(&self) -> Option<FlyoutKind> {
        self.shown
    }

    /// When `tick` next has something to do.
    pub fn deadline(&self) -> Option<u64> {
        self.pending.map(|(_, _, at)| at)
    }

    pub fn enter(&mut self, kind: FlyoutKind, rect: Rect, now: u64) -> Option<HoverAction> {
        if self.shown == Some(kind) {
            return None;
        }
        let recent = self.hidden_at.is_some_and(|t| now.saturating_sub(t) < RESHOW_WINDOW_MS);
        if self.shown.is_some() || recent {
            self.pending = None;
            self.shown = Some(kind);
            return Some(HoverAction::Show(kind, rect));
        }
        self.pending = Some((kind, rect, now + SHOW_DELAY_MS));
        None
    }

    pub fn leave(&mut self, kind: FlyoutKind, now: u64) -> Option<HoverAction> {
        if self.pending.is_some_and(|(k, _, _)| k == kind) {
            self.pending = None;
        }
        if self.shown == Some(kind) {
            return self.close(now);
        }
        None
    }

    /// A click on any tray icon: whatever it opens replaces the flyout, and a pending one is dropped.
    /// It does not arm the instant re-show: the click opened something else.
    pub fn click(&mut self) -> Option<HoverAction> {
        self.pending = None;
        self.hidden_at = None;
        self.shown.take().map(|_| HoverAction::Hide)
    }

    pub fn tick(&mut self, now: u64) -> Option<HoverAction> {
        match self.pending {
            Some((kind, rect, at)) if now >= at => {
                self.pending = None;
                self.shown = Some(kind);
                Some(HoverAction::Show(kind, rect))
            }
            _ => None,
        }
    }

    /// The flyout was closed from elsewhere (e.g. the main popup opened).
    pub fn dismissed(&mut self) {
        self.pending = None;
        self.shown = None;
    }

    fn close(&mut self, now: u64) -> Option<HoverAction> {
        self.shown = None;
        self.hidden_at = Some(now);
        Some(HoverAction::Hide)
    }
}
