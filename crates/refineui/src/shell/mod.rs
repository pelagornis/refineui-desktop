//! **Application layer** — application shell anatomy (brief §3, §5
//! Application). Phase 1 defines the *contract*: which regions exist, in
//! what order, how big they are per platform and density. Phase 2–3 build
//! the GPUI views (Window, Titlebar, Window Controls, Menu Bar, Toolbar,
//! Activity Bar, Navigation Rail, Status Bar, Application Shell).
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Titlebar  [controls] [menu bar*] [toolbar (unified)] [title]  │
//! ├────┬──────────┬───────────────────────────────┬──────────────┤
//! │ A  │ Left     │ Workspace (splits / tab groups)│ Right dock  │
//! │ c  │ dock     │                               │ (inspector)  │
//! │ t  │          ├───────────────────────────────┤              │
//! │ .  │          │ Bottom dock                    │              │
//! ├────┴──────────┴───────────────────────────────┴──────────────┤
//! │ Status bar                                                    │
//! └──────────────────────────────────────────────────────────────┘
//! * in-window menu bar on Windows / Linux; macOS uses the global bar.
//! ```

mod activity_bar;
mod app_shell;
mod menu_bar;
mod status_bar;
mod titlebar;
mod window;

pub use activity_bar::*;
pub use app_shell::*;
pub use menu_bar::*;
pub use status_bar::*;
pub use titlebar::*;
pub use window::*;

use refineui_tokens::size::Density;

use crate::platform::{MenuBarPlacement, PlatformAdapter};

/// One region of the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShellRegion {
    /// Window drag area with window controls and title.
    Titlebar,
    /// In-window menu bar (Windows / Linux).
    MenuBar,
    /// Toolbar row (when not unified into the titlebar).
    Toolbar,
    /// Activity bar / navigation rail.
    ActivityBar,
    /// Leading dock.
    LeftDock,
    /// Center workspace.
    Workspace,
    /// Bottom dock.
    BottomDock,
    /// Trailing dock.
    RightDock,
    /// Status bar.
    StatusBar,
}

/// Where the toolbar goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolbarPlacement {
    /// Merged into the titlebar (macOS unified style; default).
    #[default]
    Unified,
    /// Its own row under the titlebar / menu bar.
    Separate,
    /// No toolbar.
    None,
}

/// Which edge the activity bar sits on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RailEdge {
    /// Leading edge (default).
    #[default]
    Leading,
    /// Trailing edge.
    Trailing,
}

/// Shell configuration (application choice; persisted with the layout).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellSpec {
    /// Toolbar placement.
    pub toolbar: ToolbarPlacement,
    /// Activity bar edge, `None` = no activity bar.
    pub activity_bar: Option<RailEdge>,
    /// Show the status bar.
    pub status_bar: bool,
    /// Ask for an in-window menu bar even where the platform has a global
    /// one (rare; e.g. a web-like app on macOS). Ignored on Windows/Linux,
    /// where the menu bar is always in-window.
    pub force_in_window_menu: bool,
}

impl Default for ShellSpec {
    fn default() -> Self {
        Self {
            toolbar: ToolbarPlacement::Unified,
            activity_bar: Some(RailEdge::Leading),
            status_bar: true,
            force_in_window_menu: false,
        }
    }
}

/// Resolved vertical chrome sizes (px) for a platform + density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellMetrics {
    /// Titlebar (includes a unified toolbar).
    pub titlebar: f32,
    /// In-window menu bar row (0 when global / merged).
    pub menu_bar: f32,
    /// Separate toolbar row (0 when unified / none).
    pub toolbar: f32,
    /// Activity bar width (0 when hidden).
    pub activity_bar: f32,
    /// Status bar (0 when hidden).
    pub status_bar: f32,
}

impl ShellSpec {
    /// Is the menu bar drawn inside the window on `platform`?
    pub fn in_window_menu(&self, platform: &dyn PlatformAdapter) -> bool {
        platform.menu_bar() == MenuBarPlacement::InWindow || self.force_in_window_menu
    }

    /// Regions top-to-bottom / leading-to-trailing.
    pub fn regions(&self, platform: &dyn PlatformAdapter) -> Vec<ShellRegion> {
        let mut v = vec![ShellRegion::Titlebar];
        if self.in_window_menu(platform) {
            v.push(ShellRegion::MenuBar);
        }
        if self.toolbar == ToolbarPlacement::Separate {
            v.push(ShellRegion::Toolbar);
        }
        if self.activity_bar == Some(RailEdge::Leading) {
            v.push(ShellRegion::ActivityBar);
        }
        v.extend([
            ShellRegion::LeftDock,
            ShellRegion::Workspace,
            ShellRegion::BottomDock,
            ShellRegion::RightDock,
        ]);
        if self.activity_bar == Some(RailEdge::Trailing) {
            v.push(ShellRegion::ActivityBar);
        }
        if self.status_bar {
            v.push(ShellRegion::StatusBar);
        }
        v
    }

    /// Chrome sizes for `platform` at `density`.
    pub fn metrics(&self, platform: &dyn PlatformAdapter, density: Density) -> ShellMetrics {
        let d = density.desktop();
        let unified = self.toolbar == ToolbarPlacement::Unified;
        ShellMetrics {
            titlebar: platform.titlebar_height(unified, density),
            // Windows / Linux put the menu bar in its own row under a
            // unified titlebar; a plain titlebar hosts it inline.
            menu_bar: if self.in_window_menu(platform) && unified {
                d.tab_height
            } else {
                0.0
            },
            toolbar: if self.toolbar == ToolbarPlacement::Separate {
                d.toolbar_height
            } else {
                0.0
            },
            activity_bar: if self.activity_bar.is_some() {
                d.activity_bar_width
            } else {
                0.0
            },
            status_bar: if self.status_bar {
                d.status_bar_height
            } else {
                0.0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Os, for_os};

    #[test]
    fn regions_follow_platform() {
        let spec = ShellSpec::default();
        let mac = spec.regions(for_os(Os::MacOs));
        let win = spec.regions(for_os(Os::Windows));
        assert!(!mac.contains(&ShellRegion::MenuBar));
        assert!(win.contains(&ShellRegion::MenuBar));
        assert_eq!(mac.first(), Some(&ShellRegion::Titlebar));
        assert_eq!(mac.last(), Some(&ShellRegion::StatusBar));
        let m = spec.metrics(for_os(Os::MacOs), Density::Comfortable);
        assert_eq!((m.titlebar, m.menu_bar, m.status_bar), (52.0, 0.0, 24.0));
        let w = spec.metrics(for_os(Os::Windows), Density::Compact);
        assert_eq!((w.titlebar, w.menu_bar), (40.0, 28.0));
    }
}
