//! @module tray::clicks
//! @description Left-button gestures on the tray icons. Hovering is left to Windows' own tooltip; a single
//! click opens the graph icon's popup (the app icon's opens the main popup), a double click opens the main
//! popup.
//!
//! @input  Left-button press, release and double-click notifications with the icon's rectangle.
//! @output `ClickAction`s for the runtime to carry out.
//! @dependencies tray::positioning (Rect), ui::flyout (FlyoutKind)
use crate::tray::positioning::Rect;
use crate::ui::flyout::FlyoutKind;

/// The icon a notification came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    App,
    Graph(FlyoutKind),
}

impl Target {
    /// Maps the id `app::tray_host` gives each icon ("app", "cpu", "gpu").
    pub fn from_icon_id(id: &str) -> Option<Self> {
        match id {
            "app" => Some(Self::App),
            other => FlyoutKind::from_icon_id(other).map(Self::Graph),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction {
    /// A single click on a graph icon: open that icon's popup, or close it if it is the one showing.
    ToggleDetails(FlyoutKind, Rect),
    /// A single click on the app icon: open or close the main popup.
    ToggleMain(Rect),
    /// A double click on any icon: show the main popup (never close it).
    OpenMain(Rect),
}

#[derive(Debug, Clone, Default)]
pub struct Clicks {
    /// A double click was just reported; the release that ends it is not a click of its own.
    swallow_release: bool,
}

impl Clicks {
    pub fn new() -> Self {
        Self::default()
    }

    /// The left button went down. Every genuine click starts here, so a release that was expected but
    /// never came (the button was let go away from the icon) cannot swallow a later click.
    pub fn down(&mut self) {
        self.swallow_release = false;
    }

    /// The left button was released over an icon.
    pub fn up(&mut self, target: Target, rect: Rect) -> Option<ClickAction> {
        if std::mem::take(&mut self.swallow_release) {
            return None;
        }
        Some(match target {
            Target::App => ClickAction::ToggleMain(rect),
            Target::Graph(kind) => ClickAction::ToggleDetails(kind, rect),
        })
    }

    /// Windows reports a double click as press, release, double-click, release: the first release has
    /// already acted as a single click, so this swaps whatever it opened for the main popup.
    pub fn double(&mut self, rect: Rect) -> ClickAction {
        self.swallow_release = true;
        ClickAction::OpenMain(rect)
    }
}
