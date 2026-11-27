//! Floating Panel (brief §6 Composition) — a panel torn off into its own
//! always-on-top window.
//!
//! [`open_floating_panel`] opens a `WindowKind::Floating` window at screen
//! bounds with a compact titlebar (title, a **Dock** button, close only).
//! On macOS the window background is `WindowBackgroundAppearance::Blurred`
//! and the panel fill is the translucent `surfaceFrosted`, so the desktop
//! shows through blurred; elsewhere it is opaque `backgroundPrimary`
//! (Windows apps can add Mica / Acrylic with
//! `crate::platform::windows::set_backdrop`). Moves and resizes are
//! reported as screen bounds for [`crate::workspace::WorkspaceLayout`]
//! (`set_floating_bounds`), Dock / close as [`FloatingPanelEvent`]s.
//!
//! [`FloatingPanelFrame`] is the window's content as a plain element
//! (previews, docs) and the root of the window view [`FloatingPanel`].

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Context, ElementId, IntoElement, ParentElement, Render, RenderOnce,
    SharedString, Styled, Subscription, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div, point, prelude::*, px, size,
};
use refineui_foundation::{radius, stroke};

use super::{ControlButtons, Titlebar, WindowState};
use crate::motion::child_id;
use crate::{ActiveTheme, Button, ButtonVariant, ControlSize, Elevation, IconName};

/// Titlebar height of a floating panel, px.
pub const FLOATING_TITLEBAR: f32 = 32.;
/// Room on each side of the floating panel's centred title: the trailing
/// Dock + close buttons (two small icon buttons and gaps).
const FLOATING_TITLE_INSET: f32 = 72.;

/// What happened to a floating panel window.
#[derive(Clone, Debug, PartialEq)]
pub enum FloatingPanelEvent {
    /// Moved or resized: `[x, y, width, height]`, screen px.
    Bounds([f32; 4]),
    /// The Dock button: put the panel back (the window closes itself).
    Dock,
    /// The window was closed by the user.
    Closed,
}

/// How to open a floating panel.
#[derive(Clone, Debug, PartialEq)]
pub struct FloatingPanelOptions {
    /// Panel id (passed back in events).
    pub panel: String,
    /// Window title.
    pub title: SharedString,
    /// `[x, y, width, height]`, screen px.
    pub bounds: [f32; 4],
    /// Blur the window background where supported (macOS).
    pub blur: bool,
    /// Minimum size, px.
    pub min_size: (f32, f32),
}

impl FloatingPanelOptions {
    /// Panel `panel` titled `title` at `bounds`, blurred where supported.
    pub fn new(panel: impl Into<String>, title: impl Into<SharedString>, bounds: [f32; 4]) -> Self {
        Self {
            panel: panel.into(),
            title: title.into(),
            bounds,
            blur: true,
            min_size: (200., 120.),
        }
    }

    /// Whether this platform will blur (macOS with `blur`).
    pub fn blurred(&self) -> bool {
        self.blur && cfg!(target_os = "macos")
    }

    /// GPUI window options.
    pub fn window_options(&self) -> WindowOptions {
        let [x, y, w, h] = self.bounds;
        let (mw, mh) = self.min_size;
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(x), px(y)),
                size(px(w.max(mw)), px(h.max(mh))),
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some(self.title.clone()),
                appears_transparent: true,
                traffic_light_position: crate::platform::current()
                    .traffic_light_position(FLOATING_TITLEBAR)
                    .map(|(x, y)| point(px(x), px(y))),
            }),
            focus: true,
            show: true,
            kind: WindowKind::Floating,
            is_movable: true,
            is_resizable: true,
            is_minimizable: false,
            window_background: if self.blurred() {
                WindowBackgroundAppearance::Blurred
            } else {
                WindowBackgroundAppearance::Opaque
            },
            window_min_size: Some(size(px(mw), px(mh))),
            window_decorations: Some(if crate::platform::current().client_decorations() {
                gpui::WindowDecorations::Client
            } else {
                gpui::WindowDecorations::Server
            }),
            ..Default::default()
        }
    }
}

/// Screen bounds of `window` as `[x, y, width, height]`.
pub fn window_screen_bounds(window: &Window) -> [f32; 4] {
    let s = WindowState::capture(window);
    [s.x, s.y, s.width, s.height]
}

type Content = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type EventHandler = Rc<dyn Fn(&str, FloatingPanelEvent, &mut App)>;
type DockHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// The floating panel's content as an element. See the module docs.
#[derive(IntoElement)]
pub struct FloatingPanelFrame {
    id: ElementId,
    title: SharedString,
    blurred: bool,
    interactive: bool,
    on_dock: Option<DockHandler>,
    content: Vec<AnyElement>,
}

impl FloatingPanelFrame {
    /// Frame titled `title`.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            blurred: false,
            interactive: false,
            on_dock: None,
            content: Vec::new(),
        }
    }

    /// Translucent fill (the window background is blurred).
    pub fn blurred(mut self, blurred: bool) -> Self {
        self.blurred = blurred;
        self
    }

    /// A live window (titlebar drags / closes it); `false` for previews,
    /// which also get a border and shadow.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// The Dock button.
    pub fn on_dock(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dock = Some(Rc::new(f));
        self
    }

    /// Content.
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content.push(e.into_any_element());
        self
    }
}

impl RenderOnce for FloatingPanelFrame {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let fill = if self.blurred {
            c.surface_frosted
        } else {
            c.background_primary
        };
        let dock = self.on_dock.clone();
        let titlebar = Titlebar::new(child_id(&self.id, "titlebar"))
            .title(self.title.clone())
            .height(FLOATING_TITLEBAR)
            // Clear the Dock and close buttons on the trailing side.
            .title_inset(FLOATING_TITLE_INSET)
            .interactive(self.interactive)
            .buttons(ControlButtons {
                minimize: false,
                maximize: false,
                close: true,
            })
            .trailing(
                Button::icon_only(child_id(&self.id, "dock"), IconName::PanelLeft)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Small)
                    .tooltip("Dock panel")
                    .on_click(move |_, w, cx| {
                        if let Some(f) = &dock {
                            f(w, cx);
                        }
                    }),
            );
        div()
            .id(self.id)
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(fill)
            .font_family(theme.font_family.clone())
            .text_color(c.foreground_primary)
            .when(!self.interactive, |d| {
                d.border(px(stroke::HAIRLINE))
                    .border_color(c.border_default)
                    .rounded(px(radius::LARGE))
                    .shadow(theme.shadow(Elevation::FLOATING))
            })
            .child(div().flex_none().child(titlebar))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .children(self.content),
            )
    }
}

/// Root view of a floating panel window.
pub struct FloatingPanel {
    panel: String,
    title: SharedString,
    blurred: bool,
    content: Content,
    on_event: EventHandler,
    _subscriptions: Vec<Subscription>,
}

impl FloatingPanel {
    /// The panel id.
    pub fn panel(&self) -> &str {
        &self.panel
    }

    fn new(
        opts: &FloatingPanelOptions,
        content: Content,
        on_event: EventHandler,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let bounds_event = on_event.clone();
        let close_event = on_event.clone();
        let panel = opts.panel.clone();
        let close_panel = opts.panel.clone();
        let moved = cx.observe_window_bounds(window, move |_, window, cx| {
            bounds_event(
                &panel,
                FloatingPanelEvent::Bounds(window_screen_bounds(window)),
                cx,
            );
        });
        window.on_window_should_close(cx, move |_, cx| {
            close_event(&close_panel, FloatingPanelEvent::Closed, cx);
            true
        });
        Self {
            panel: opts.panel.clone(),
            title: opts.title.clone(),
            blurred: opts.blurred(),
            content,
            on_event,
            _subscriptions: vec![moved],
        }
    }
}

impl Render for FloatingPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = (self.content)(window, cx);
        let on_event = self.on_event.clone();
        let panel = self.panel.clone();
        let mut root = crate::root(cx);
        if self.blurred {
            root = root.bg(gpui::transparent_black());
        }
        root.child(
            FloatingPanelFrame::new("floating-panel", self.title.clone())
                .blurred(self.blurred)
                .interactive(true)
                .on_dock(move |window, cx| {
                    on_event(&panel, FloatingPanelEvent::Dock, cx);
                    window.remove_window();
                })
                .child(content),
        )
    }
}

/// Open `opts.panel` in a floating window. `content` draws the panel
/// body every frame; `on_event(panel, event, cx)` reports moves, Dock and
/// close.
pub fn open_floating_panel(
    opts: FloatingPanelOptions,
    cx: &mut App,
    content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    on_event: impl Fn(&str, FloatingPanelEvent, &mut App) + 'static,
) -> Option<WindowHandle<FloatingPanel>> {
    let content: Content = Rc::new(content);
    let on_event: EventHandler = Rc::new(on_event);
    cx.open_window(opts.window_options(), move |window, cx| {
        cx.new(|cx| FloatingPanel::new(&opts, content, on_event, window, cx))
    })
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_map_to_a_floating_window() {
        let o = FloatingPanelOptions::new("outline", "Outline", [100., 80., 50., 40.]);
        let w = o.window_options();
        assert_eq!(w.kind, WindowKind::Floating);
        assert!(!w.is_minimizable);
        // Bounds honour the minimum size.
        match w.window_bounds {
            Some(WindowBounds::Windowed(b)) => {
                assert_eq!(f32::from(b.size.width), 200.);
                assert_eq!(f32::from(b.size.height), 120.);
                assert_eq!(f32::from(b.origin.x), 100.);
            }
            other => panic!("{other:?}"),
        }
        let expect = if cfg!(target_os = "macos") {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };
        assert_eq!(w.window_background, expect);
        let mut o = o;
        o.blur = false;
        assert_eq!(
            o.window_options().window_background,
            WindowBackgroundAppearance::Opaque
        );
    }
}
