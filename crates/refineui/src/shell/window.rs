//! Window: presets, minimum sizes, `WindowOptions` with a RefineUI titlebar
//! and size / position restore (brief §5 Application: Window).
//!
//! ```ignore
//! let saved = WindowState::load(&path);          // None on first run
//! let opts = window_options(&WindowSpec::new(WindowPreset::Main, "Notes").restore(saved), cx);
//! cx.open_window(opts, |window, cx| { … track_window_state(window, cx, path) … });
//! ```
//!
//! * **Presets** ([`WindowPreset`]) fix the default size, minimum size and
//!   which window buttons apply (a Settings window is not minimizable on
//!   macOS-style platforms but is resizable; a Utility panel is small).
//! * **Titlebar** — always `appears_transparent` so the RefineUI
//!   [`crate::shell::Titlebar`] draws it; on macOS the traffic lights are
//!   re-centred in our titlebar height
//!   ([`crate::platform::PlatformAdapter::traffic_light_position`]); on
//!   Linux the window asks for client-side decorations
//!   ([`crate::shell::WindowFrame`] draws the frame and resize edges).
//! * **Restore** ([`WindowState`]) — bounds plus maximized / fullscreen.
//!   [`WindowState::sanitized`] rejects a saved frame that is no longer on
//!   any display (monitor unplugged) and clamps it to the minimum size and
//!   the display, so a restored window is always reachable.

use gpui::{
    App, Bounds, Pixels, Point, Size, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, point, px, size,
};

use crate::platform::{self, PlatformAdapter};

/// Window kinds with their default and minimum sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowPreset {
    /// The main application window (full shell).
    #[default]
    Main,
    /// A document / secondary editor window.
    Document,
    /// Preferences / settings.
    Settings,
    /// A small tool / inspector panel.
    Utility,
}

impl WindowPreset {
    /// Every preset.
    pub const ALL: [WindowPreset; 4] = [Self::Main, Self::Document, Self::Settings, Self::Utility];

    /// Default (first-run) size.
    pub const fn default_size(self) -> (f32, f32) {
        match self {
            Self::Main => (1280., 800.),
            Self::Document => (960., 720.),
            Self::Settings => (720., 560.),
            Self::Utility => (360., 480.),
        }
    }

    /// Minimum size.
    pub const fn min_size(self) -> (f32, f32) {
        match self {
            Self::Main => refineui_foundation::window::MIN_WINDOW,
            Self::Document => (480., 360.),
            Self::Settings => (560., 420.),
            Self::Utility => (280., 240.),
        }
    }

    /// Shows a minimize button.
    pub const fn minimizable(self) -> bool {
        !matches!(self, Self::Settings | Self::Utility)
    }

    /// Shows a maximize / zoom button.
    pub const fn maximizable(self) -> bool {
        matches!(self, Self::Main | Self::Document)
    }
}

/// Saved window placement (logical pixels).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowState {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width (the *restored* width when maximized / fullscreen).
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Maximized (zoomed).
    pub maximized: bool,
    /// Fullscreen.
    pub fullscreen: bool,
}

/// A display's frame `(x, y, width, height)`.
pub type DisplayFrame = (f32, f32, f32, f32);

/// How much of the titlebar must stay on a display (px) for a saved frame
/// to be accepted.
const VISIBLE_GRIP: (f32, f32) = (120., 24.);

impl WindowState {
    /// The current placement of `window`.
    pub fn capture(window: &Window) -> Self {
        let (b, maximized, fullscreen) = match window.window_bounds() {
            WindowBounds::Windowed(b) => (b, false, false),
            WindowBounds::Maximized(b) => (b, true, false),
            WindowBounds::Fullscreen(b) => (b, false, true),
        };
        Self {
            x: b.origin.x.into(),
            y: b.origin.y.into(),
            width: b.size.width.into(),
            height: b.size.height.into(),
            maximized,
            fullscreen,
        }
    }

    /// A placement that is safe to restore on `displays`, or `None` when
    /// the saved titlebar is not on any of them. Size is clamped to
    /// `min` and to the display the window lands on; the frame is nudged
    /// fully onto that display.
    pub fn sanitized(self, displays: &[DisplayFrame], min: (f32, f32)) -> Option<Self> {
        if !(self.width.is_finite() && self.height.is_finite() && self.x.is_finite()) {
            return None;
        }
        let grip = |d: &DisplayFrame| {
            let (dx, dy, dw, dh) = *d;
            let ox = (self.x + self.width).min(dx + dw) - self.x.max(dx);
            let oy = (self.y + VISIBLE_GRIP.1).min(dy + dh) - self.y.max(dy);
            ox >= VISIBLE_GRIP.0.min(self.width) && oy >= VISIBLE_GRIP.1.min(self.height)
        };
        let d = displays.iter().find(|d| grip(d))?;
        let (dx, dy, dw, dh) = *d;
        let width = self.width.max(min.0).min(dw);
        let height = self.height.max(min.1).min(dh);
        let x = self.x.clamp(dx, dx + dw - width);
        let y = self.y.clamp(dy, dy + dh - height);
        Some(Self {
            x,
            y,
            width,
            height,
            ..self
        })
    }

    /// GPUI bounds for `WindowOptions::window_bounds`.
    pub fn to_window_bounds(self) -> WindowBounds {
        let b = Bounds::new(
            point(px(self.x), px(self.y)),
            size(px(self.width), px(self.height)),
        );
        if self.fullscreen {
            WindowBounds::Fullscreen(b)
        } else if self.maximized {
            WindowBounds::Maximized(b)
        } else {
            WindowBounds::Windowed(b)
        }
    }

    /// One-line text form (`x y width height maximized fullscreen`) — no
    /// serde needed. With the `serde` feature the type is also
    /// `Serialize` / `Deserialize`.
    pub fn to_line(self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.x, self.y, self.width, self.height, self.maximized as u8, self.fullscreen as u8
        )
    }

    /// Parse [`Self::to_line`].
    pub fn from_line(line: &str) -> Option<Self> {
        let v: Vec<&str> = line.split_whitespace().collect();
        if v.len() != 6 {
            return None;
        }
        Some(Self {
            x: v[0].parse().ok()?,
            y: v[1].parse().ok()?,
            width: v[2].parse().ok()?,
            height: v[3].parse().ok()?,
            maximized: v[4] == "1",
            fullscreen: v[5] == "1",
        })
    }

    /// Read a state file written by [`Self::save`] (`None` if missing or
    /// unreadable).
    pub fn load(path: &std::path::Path) -> Option<Self> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| Self::from_line(s.trim()))
    }

    /// Write the state file (creating parent directories).
    pub fn save(self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_line() + "\n")
    }
}

/// What to open.
#[derive(Clone, Debug)]
pub struct WindowSpec {
    /// Preset.
    pub preset: WindowPreset,
    /// Window title (also the OS window-list name).
    pub title: gpui::SharedString,
    /// Saved placement to restore.
    pub restore: Option<WindowState>,
    /// Linux / Wayland application id (desktop-file name).
    pub app_id: Option<String>,
    /// Titlebar height the traffic lights are centred in; defaults to the
    /// platform's unified-titlebar height at the current density.
    pub titlebar_height: Option<f32>,
}

impl WindowSpec {
    /// Spec for `preset` titled `title`.
    pub fn new(preset: WindowPreset, title: impl Into<gpui::SharedString>) -> Self {
        Self {
            preset,
            title: title.into(),
            restore: None,
            app_id: None,
            titlebar_height: None,
        }
    }

    /// Restore a saved placement (sanitized against the current displays).
    pub fn restore(mut self, state: Option<WindowState>) -> Self {
        self.restore = state;
        self
    }

    /// Linux application id.
    pub fn app_id(mut self, id: impl Into<String>) -> Self {
        self.app_id = Some(id.into());
        self
    }

    /// Titlebar height for the traffic-light position.
    pub fn titlebar_height(mut self, h: f32) -> Self {
        self.titlebar_height = Some(h);
        self
    }
}

/// Frames of every connected display.
pub fn display_frames(cx: &App) -> Vec<DisplayFrame> {
    cx.displays()
        .iter()
        .map(|d| {
            let b = d.bounds();
            (
                b.origin.x.into(),
                b.origin.y.into(),
                b.size.width.into(),
                b.size.height.into(),
            )
        })
        .collect()
}

/// GPUI `WindowOptions` for `spec` on the host platform.
pub fn window_options(spec: &WindowSpec, cx: &App) -> WindowOptions {
    let adapter: &dyn PlatformAdapter = platform::current();
    let density = crate::ActiveTheme::theme(cx).density;
    let h = spec
        .titlebar_height
        .unwrap_or_else(|| adapter.titlebar_height(true, density));
    let min = spec.preset.min_size();
    let restored = spec
        .restore
        .and_then(|s| s.sanitized(&display_frames(cx), min))
        .map(WindowState::to_window_bounds);
    let (w, hh) = spec.preset.default_size();
    let bounds = restored
        .unwrap_or_else(|| WindowBounds::Windowed(Bounds::centered(None, size(px(w), px(hh)), cx)));
    WindowOptions {
        window_bounds: Some(bounds),
        titlebar: Some(TitlebarOptions {
            title: Some(spec.title.clone()),
            appears_transparent: true,
            traffic_light_position: adapter
                .traffic_light_position(h)
                .map(|(x, y)| point(px(x), px(y))),
        }),
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: true,
        is_minimizable: spec.preset.minimizable(),
        window_background: WindowBackgroundAppearance::Opaque,
        app_id: spec.app_id.clone(),
        window_min_size: Some(size(px(min.0), px(min.1))),
        window_decorations: Some(if adapter.client_decorations() {
            WindowDecorations::Client
        } else {
            WindowDecorations::Server
        }),
        ..Default::default()
    }
}

/// Save the window's placement to `path` whenever it moves or resizes, and
/// once more when it closes. Call from the root view's constructor.
pub fn track_window_state<V: 'static>(
    window: &mut Window,
    cx: &mut gpui::Context<V>,
    path: std::path::PathBuf,
) -> gpui::Subscription {
    let p = path.clone();
    window.on_window_should_close(cx, move |window, _| {
        let _ = WindowState::capture(window).save(&p);
        true
    });
    cx.observe_window_bounds(window, move |_, window, _| {
        let _ = WindowState::capture(window).save(&path);
    })
}

/// The traffic-light origin as a GPUI point (for apps building their own
/// `TitlebarOptions`).
pub fn traffic_light_point(titlebar_h: f32) -> Option<Point<Pixels>> {
    platform::current()
        .traffic_light_position(titlebar_h)
        .map(|(x, y)| point(px(x), px(y)))
}

/// Minimum size of `preset` as a GPUI size.
pub fn min_size(preset: WindowPreset) -> Size<Pixels> {
    let (w, h) = preset.min_size();
    size(px(w), px(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: DisplayFrame = (0., 0., 1440., 900.);
    const RIGHT: DisplayFrame = (1440., 0., 1920., 1080.);

    fn st(x: f32, y: f32, w: f32, h: f32) -> WindowState {
        WindowState {
            x,
            y,
            width: w,
            height: h,
            maximized: false,
            fullscreen: false,
        }
    }

    #[test]
    fn restore_keeps_a_visible_frame() {
        let s = st(100., 80., 1000., 700.);
        assert_eq!(s.sanitized(&[MAIN], (640., 400.)), Some(s));
    }

    #[test]
    fn restore_rejects_a_frame_on_an_unplugged_display() {
        let s = st(2000., 100., 1000., 700.);
        assert_eq!(s.sanitized(&[MAIN], (640., 400.)), None);
        // Still there: accepted on the second display.
        assert_eq!(s.sanitized(&[MAIN, RIGHT], (640., 400.)), Some(s));
    }

    #[test]
    fn restore_clamps_size_and_position() {
        // Too small, partially off the bottom-right edge.
        let s = st(1300., 850., 200., 100.);
        let r = s.sanitized(&[MAIN], (640., 400.)).unwrap();
        assert_eq!((r.width, r.height), (640., 400.));
        assert_eq!((r.x, r.y), (800., 500.));
        // Larger than the display.
        let big = st(0., 0., 3000., 2000.)
            .sanitized(&[MAIN], (640., 400.))
            .unwrap();
        assert_eq!((big.width, big.height), (1440., 900.));
        // Titlebar above the display top is rejected (cannot be grabbed).
        assert_eq!(
            st(100., -500., 800., 600.).sanitized(&[MAIN], (640., 400.)),
            None
        );
    }

    #[test]
    fn state_round_trips_as_a_line() {
        let s = WindowState {
            maximized: true,
            ..st(10.5, 20., 800., 600.)
        };
        assert_eq!(WindowState::from_line(&s.to_line()), Some(s));
        assert_eq!(WindowState::from_line("garbage"), None);
        assert!(matches!(s.to_window_bounds(), WindowBounds::Maximized(_)));
    }

    #[test]
    fn presets_have_sane_sizes() {
        for p in WindowPreset::ALL {
            let (dw, dh) = p.default_size();
            let (mw, mh) = p.min_size();
            assert!(dw >= mw && dh >= mh, "{p:?}");
        }
        assert!(!WindowPreset::Settings.minimizable());
        assert!(WindowPreset::Main.maximizable());
    }
}
