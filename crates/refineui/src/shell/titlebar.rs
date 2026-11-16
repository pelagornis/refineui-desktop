//! Titlebar, Window Controls and the Linux client-side Window Frame
//! (brief §5 Application).
//!
//! **Titlebar** — the window's drag area, drawn by RefineUI on every
//! platform (the window is opened with a transparent / hidden system
//! titlebar by [`crate::shell::window_options`]):
//!
//! * Height from the platform and density
//!   ([`crate::platform::PlatformAdapter::titlebar_height`]; unified with a
//!   toolbar by default: 52 macOS, 48 Windows, 46 Linux at Comfortable).
//! * Drag to move (`WindowControlArea::Drag` for the Windows hit test plus
//!   `start_window_move` on macOS / Linux); double-click zooms (macOS: the
//!   system "double-click a window's title bar" preference via
//!   `titlebar_double_click`); right-click opens the window menu on Linux.
//! * Slots: `leading` (after the controls on macOS), centred `title` /
//!   `subtitle`, `trailing` (before the controls on Windows / Linux), or a
//!   full-width `content` (a unified [`crate::Toolbar`]).
//! * macOS keeps [`crate::platform::PlatformAdapter::controls_reserved`]
//!   free for the traffic lights (re-centred by the window options) and
//!   drops the reservation in fullscreen, where AppKit hides them.
//!
//! **WindowControls** — minimize / maximize-restore / close:
//! * macOS — nothing (AppKit draws the traffic lights); under a
//!   [`crate::platform::ChromePreview`] on another host, static lights.
//! * Windows — 46 × titlebar-height caption buttons flush with the
//!   trailing edge, Segoe-style glyph icons, `backgroundPrimaryHover`
//!   hover and the system red close hover. Each button is also a
//!   `WindowControlArea` so Windows' own hit test drives them (snap
//!   layouts on hover over maximize).
//! * Linux — 24px round header-bar buttons, 6px apart, on a
//!   `backgroundSurface` disc that darkens on hover.
//!
//! Buttons the platform does not support (`window.window_controls()`) or
//! the window preset disables are omitted.
//!
//! **WindowFrame** — wrap the root on client-side-decorated windows
//! (Linux): rounded 12px corners when floating, a hairline border, the
//! shadow inset and 6px / 12px resize edges and corners
//! (`start_window_resize`). Server-decorated windows render the child as is.

use std::rc::Rc;

use gpui::{
    AnyElement, App, CursorStyle, Decorations, ElementId, Hsla, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, ResizeEdge, SharedString, StatefulInteractiveElement,
    Styled, Window, WindowControlArea, div, prelude::*, px, rgb,
};
use refineui_tokens::{spacing, stroke, typography::role, window as wm};

use crate::motion::{TrackInteraction, child_id, use_interaction};
use crate::platform::{self, Os};
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

fn hex(c: u32) -> Hsla {
    rgb(c).into()
}

/// Which window buttons to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlButtons {
    /// Minimize.
    pub minimize: bool,
    /// Maximize / restore (zoom).
    pub maximize: bool,
    /// Close.
    pub close: bool,
}

impl Default for ControlButtons {
    fn default() -> Self {
        Self {
            minimize: true,
            maximize: true,
            close: true,
        }
    }
}

impl From<super::WindowPreset> for ControlButtons {
    fn from(p: super::WindowPreset) -> Self {
        Self {
            minimize: p.minimizable(),
            maximize: p.maximizable(),
            close: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

/// Minimize / maximize / close. See the module docs.
#[derive(IntoElement)]
pub struct WindowControls {
    id: ElementId,
    buttons: ControlButtons,
    height: Option<f32>,
    os: Option<Os>,
    interactive: bool,
}

impl WindowControls {
    /// Controls with every button.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            buttons: ControlButtons::default(),
            height: None,
            os: None,
            interactive: true,
        }
    }

    /// Draw for this OS instead of [`crate::platform::chrome_os`] (docs,
    /// previews).
    pub fn os(mut self, os: Os) -> Self {
        self.os = Some(os);
        self
    }

    /// `false`: draw only (no window actions) — for previews embedded in
    /// another window.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Show only some buttons (or `ControlButtons::from(preset)`).
    pub fn buttons(mut self, buttons: impl Into<ControlButtons>) -> Self {
        self.buttons = buttons.into();
        self
    }

    /// Titlebar height (Windows caption buttons fill it).
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }
}

fn run(control: Control, window: &mut Window) {
    match control {
        Control::Minimize => window.minimize_window(),
        Control::Maximize => window.zoom_window(),
        Control::Close => window.remove_window(),
    }
}

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let os = self.os.unwrap_or_else(|| platform::chrome_os(cx));
        let host = Os::current();
        let interactive = self.interactive;
        let supported = window.window_controls();
        let maximized = window.is_maximized();
        let mut list: Vec<Control> = Vec::new();
        // On the real host, ask the platform what it supports; previews
        // show the full set.
        let real = os == host && interactive;
        if self.buttons.minimize && (!real || supported.minimize) {
            list.push(Control::Minimize);
        }
        if self.buttons.maximize && (!real || supported.maximize) {
            list.push(Control::Maximize);
        }
        if self.buttons.close {
            list.push(Control::Close);
        }
        let m = platform::for_os(os).controls();
        let c = cx.theme().colors;
        match os {
            Os::MacOs => {
                if real {
                    // AppKit draws the traffic lights.
                    return div().into_any_element();
                }
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(m.gap))
                    .children(
                        wm::MACOS_TRAFFIC_LIGHTS
                            .iter()
                            .enumerate()
                            .map(|(ix, &fill)| {
                                // AppKit greys out the buttons a window does not
                                // support (e.g. a Settings window's minimize / zoom).
                                let on = match ix {
                                    0 => self.buttons.close,
                                    1 => self.buttons.minimize,
                                    _ => self.buttons.maximize,
                                };
                                div()
                                    .size(px(m.button_width))
                                    .rounded_full()
                                    .bg(if on {
                                        hex(fill)
                                    } else {
                                        cx.theme().colors.background_surface_disabled
                                    })
                                    .border(px(stroke::HAIRLINE))
                                    .border_color(gpui::black().opacity(0.12))
                            }),
                    )
                    .into_any_element()
            }
            Os::Windows => {
                let h = self.height.unwrap_or(m.button_height);
                div()
                    .flex()
                    .flex_none()
                    .h(px(h))
                    .children(list.into_iter().map(|ctl| {
                        let id = child_id(
                            &self.id,
                            match ctl {
                                Control::Minimize => "min",
                                Control::Maximize => "max",
                                Control::Close => "close",
                            },
                        );
                        let (state, i) = use_interaction(&id, window, cx);
                        let (bg, fg) = match (ctl, i.pressed, i.hovered) {
                            (Control::Close, true, _) => {
                                (hex(wm::WINDOWS_CLOSE_PRESSED), gpui::white())
                            }
                            (Control::Close, _, true) => {
                                (hex(wm::WINDOWS_CLOSE_HOVER), gpui::white())
                            }
                            (_, true, _) => (c.background_primary_active, c.foreground_primary),
                            (_, _, true) => (c.background_primary_hover, c.foreground_primary),
                            _ => (gpui::transparent_black(), c.foreground_secondary),
                        };
                        let icon = match ctl {
                            Control::Minimize => IconName::Subtract,
                            Control::Maximize if maximized => IconName::SquareMultiple,
                            Control::Maximize => IconName::Square,
                            Control::Close => IconName::Dismiss,
                        };
                        let area = match ctl {
                            Control::Minimize => WindowControlArea::Min,
                            Control::Maximize => WindowControlArea::Max,
                            Control::Close => WindowControlArea::Close,
                        };
                        div()
                            .id(id)
                            .w(px(m.button_width))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(bg)
                            .when(interactive, |d| d.window_control_area(area))
                            .track_interaction(&state)
                            // Windows routes clicks through the hit test
                            // (`WindowControlArea`); other hosts click directly.
                            .when(interactive && host != Os::Windows, |d| {
                                d.on_click(move |_, window, cx| {
                                    cx.stop_propagation();
                                    run(ctl, window);
                                })
                            })
                            .child(Icon::new(icon).size(IconSize::XSmall).color(fg))
                    }))
                    .into_any_element()
            }
            Os::Linux => div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(m.gap))
                .children(list.into_iter().map(|ctl| {
                    let id = child_id(
                        &self.id,
                        match ctl {
                            Control::Minimize => "min",
                            Control::Maximize => "max",
                            Control::Close => "close",
                        },
                    );
                    let (state, i) = use_interaction(&id, window, cx);
                    let bg = if i.pressed {
                        c.background_surface_active
                    } else if i.hovered {
                        c.background_surface_hover
                    } else {
                        c.background_surface
                    };
                    let icon = match ctl {
                        Control::Minimize => IconName::Subtract,
                        Control::Maximize if maximized => IconName::SquareMultiple,
                        Control::Maximize => IconName::Square,
                        Control::Close => IconName::Dismiss,
                    };
                    div()
                        .id(id)
                        .size(px(m.button_width))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(bg)
                        .track_interaction(&state)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .when(interactive, |d| {
                            d.on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                run(ctl, window);
                            })
                        })
                        .child(
                            Icon::new(icon)
                                .size(IconSize::XSmall)
                                .color(c.foreground_primary),
                        )
                }))
                .into_any_element(),
        }
    }
}

/// The window's titlebar. See the module docs.
#[derive(IntoElement)]
pub struct Titlebar {
    id: ElementId,
    title: Option<SharedString>,
    subtitle: Option<SharedString>,
    leading: Vec<AnyElement>,
    trailing: Vec<AnyElement>,
    content: Option<AnyElement>,
    height: Option<f32>,
    unified: bool,
    buttons: ControlButtons,
    on_double_click: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    os: Option<Os>,
    interactive: bool,
}

impl Titlebar {
    /// Empty titlebar (unified height).
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: None,
            subtitle: None,
            leading: Vec::new(),
            trailing: Vec::new(),
            content: None,
            height: None,
            unified: true,
            buttons: ControlButtons::default(),
            on_double_click: None,
            os: None,
            interactive: true,
        }
    }

    /// Draw for this OS instead of [`crate::platform::chrome_os`].
    pub fn os(mut self, os: Os) -> Self {
        self.os = Some(os);
        self
    }

    /// `false`: a static preview (no window drag, zoom or controls) — for
    /// titlebars drawn inside another window (docs, galleries).
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Centred window title (`bodySm` semibold).
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Second line / trailing detail under the title (`captionMd`
    /// tertiary), e.g. the document path.
    pub fn subtitle(mut self, t: impl Into<SharedString>) -> Self {
        self.subtitle = Some(t.into());
        self
    }

    /// Element at the leading edge (after macOS traffic lights).
    pub fn leading(mut self, e: impl IntoElement) -> Self {
        self.leading.push(e.into_any_element());
        self
    }

    /// Element at the trailing edge (before Windows / Linux controls).
    pub fn trailing(mut self, e: impl IntoElement) -> Self {
        self.trailing.push(e.into_any_element());
        self
    }

    /// Full-width content between the controls (a unified toolbar);
    /// replaces the title.
    pub fn content(mut self, e: impl IntoElement) -> Self {
        self.content = Some(e.into_any_element());
        self
    }

    /// `false`: plain (title-only) height instead of unified.
    pub fn unified(mut self, unified: bool) -> Self {
        self.unified = unified;
        self
    }

    /// Explicit height.
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }

    /// Window buttons to show.
    pub fn buttons(mut self, b: impl Into<ControlButtons>) -> Self {
        self.buttons = b.into();
        self
    }

    /// Replace the default double-click (zoom) behaviour.
    pub fn on_double_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_double_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Titlebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let os = self.os.unwrap_or_else(|| platform::chrome_os(cx));
        let interactive = self.interactive;
        let real = interactive && os == Os::current();
        let adapter = platform::for_os(os);
        let h = self
            .height
            .unwrap_or_else(|| adapter.titlebar_height(self.unified, theme.density));
        let leading_controls = adapter.controls().leading;
        let fullscreen = window.is_fullscreen();
        let reserve = if leading_controls && !(fullscreen && real) {
            adapter.controls_reserved(h)
        } else {
            0.
        };
        let should_move = window.use_keyed_state(child_id(&self.id, "move"), cx, |_, _| false);
        let down = should_move.clone();
        let up = should_move.clone();
        let mv = should_move.clone();
        let dbl = self.on_double_click.clone();

        let title = (self.content.is_none())
            .then(|| {
                self.title.clone().map(|t| {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_style_token(role::BODY_SM)
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(c.foreground_primary)
                                .child(t),
                        )
                        .when_some(self.subtitle.clone(), |d, s| {
                            d.child(
                                div()
                                    .truncate()
                                    .text_style_token(role::CAPTION_MD)
                                    .text_color(c.foreground_tertiary)
                                    .child(s),
                            )
                        })
                })
            })
            .flatten();

        let controls = WindowControls::new(child_id(&self.id, "controls"))
            .buttons(self.buttons)
            .height(h)
            .os(os)
            .interactive(interactive);

        div()
            .id(self.id.clone())
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(px(h))
            .bg(c.surface_sunken)
            .border_b(px(stroke::HAIRLINE))
            .border_color(c.border_subtle)
            .font_family(theme.font_family.clone())
            .when(interactive, |d| d.window_control_area(WindowControlArea::Drag))
            .on_mouse_down(MouseButton::Left, move |e, window, cx| {
                if !interactive {
                    return;
                }
                if e.click_count >= 2 {
                    match &dbl {
                        Some(f) => f(window, cx),
                        None if Os::current() == Os::MacOs => window.titlebar_double_click(),
                        None => window.zoom_window(),
                    }
                    return;
                }
                down.update(cx, |m, _| *m = true);
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                up.update(cx, |m, _| *m = false);
            })
            .on_mouse_move(move |_, window, cx| {
                if *mv.read(cx) {
                    mv.update(cx, |m, _| *m = false);
                    window.start_window_move();
                }
            })
            .on_mouse_down(MouseButton::Right, move |e, window, _| {
                if interactive && Os::current() == Os::Linux {
                    window.show_window_menu(e.position);
                }
            })
            .when(reserve > 0., |d| d.child(div().flex_none().w(px(reserve)).h_full().flex().items_center().pl(px(adapter.traffic_light_position(h).map(|p| p.0).unwrap_or(spacing::MEDIUM))).when(!real, |d| {
                d.child(
                    WindowControls::new(child_id(&self.id, "preview-lights"))
                        .os(os)
                        .interactive(false),
                )
            })))
            .when(reserve == 0., |d| d.pl(px(spacing::MEDIUM)))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(spacing::XX_SMALL))
                    .children(self.leading.into_iter().map(no_drag)),
            )
            .map(|d| match self.content {
                Some(content) => d.child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .items_center()
                        .px(px(spacing::X_SMALL))
                        .child(content),
                ),
                None => d.child(div().flex_1()),
            })
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(spacing::XX_SMALL))
                    .pr(px(if leading_controls { spacing::MEDIUM } else { spacing::X_SMALL }))
                    .children(self.trailing.into_iter().map(no_drag)),
            )
            .when(!leading_controls, |d| {
                d.child(
                    div()
                        .flex_none()
                        .h_full()
                        .flex()
                        .items_center()
                        .when(os == Os::Linux, |d| d.pr(px(spacing::X_SMALL)))
                        .child(controls),
                )
            })
            // The title is centred on the window, not on the free space.
            .children(title.map(|t| {
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(px(240.))
                    .child(t)
            }))
    }
}

/// Wrap an interactive element placed in a titlebar so presses on it never
/// move the window (it occludes the titlebar's drag area beneath it, which
/// also keeps the Windows caption hit test from claiming it). Titlebar
/// `leading` / `trailing` slots and `Toolbar::fill` items do this already.
pub fn no_drag(e: impl IntoElement) -> gpui::Div {
    div().flex_none().occlude().child(e)
}

/// Client-side window frame (Linux CSD). See the module docs.
#[derive(IntoElement)]
pub struct WindowFrame {
    child: AnyElement,
}

impl WindowFrame {
    /// Frame `child` (the window's root content).
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }
}

/// The resize edge under `pos` in a `size` window with a `shadow` inset,
/// or `None` inside the frame.
pub fn resize_edge(pos: (f32, f32), size: (f32, f32), shadow: f32) -> Option<ResizeEdge> {
    let edge = wm::RESIZE_EDGE + shadow;
    let corner = wm::RESIZE_CORNER + shadow;
    let (x, y) = pos;
    let (w, h) = size;
    let left = x < edge;
    let right = x > w - edge;
    let top = y < edge;
    let bottom = y > h - edge;
    let near_left = x < corner;
    let near_right = x > w - corner;
    let near_top = y < corner;
    let near_bottom = y > h - corner;
    Some(match () {
        _ if (top && near_left) || (left && near_top) => ResizeEdge::TopLeft,
        _ if (top && near_right) || (right && near_top) => ResizeEdge::TopRight,
        _ if (bottom && near_left) || (left && near_bottom) => ResizeEdge::BottomLeft,
        _ if (bottom && near_right) || (right && near_bottom) => ResizeEdge::BottomRight,
        _ if top => ResizeEdge::Top,
        _ if bottom => ResizeEdge::Bottom,
        _ if left => ResizeEdge::Left,
        _ if right => ResizeEdge::Right,
        _ => return None,
    })
}

fn edge_cursor(e: ResizeEdge) -> CursorStyle {
    match e {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

impl RenderOnce for WindowFrame {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Decorations::Client { tiling } = window.window_decorations() else {
            window.set_client_inset(px(0.));
            return div().size_full().child(self.child).into_any_element();
        };
        let theme = cx.theme().clone();
        let c = theme.colors;
        let shadow = if tiling.is_tiled() {
            0.
        } else {
            wm::CSD_SHADOW
        };
        window.set_client_inset(px(shadow));
        let r = if tiling.is_tiled() {
            0.
        } else {
            wm::CSD_RADIUS
        };
        let size = window.viewport_size();
        let sz: (f32, f32) = (size.width.into(), size.height.into());
        let edge_at =
            move |p: gpui::Point<gpui::Pixels>| resize_edge((p.x.into(), p.y.into()), sz, shadow);
        div()
            .id("refineui-window-frame")
            .size_full()
            .p(px(shadow))
            .on_mouse_move(move |e, window, _| {
                if let Some(edge) = edge_at(e.position) {
                    window.set_window_cursor_style(edge_cursor(edge));
                }
            })
            .on_mouse_down(MouseButton::Left, move |e, window, cx| {
                if let Some(edge) = edge_at(e.position) {
                    cx.stop_propagation();
                    window.start_window_resize(edge);
                }
            })
            .child(
                div()
                    .size_full()
                    .overflow_hidden()
                    .rounded(px(r))
                    .border(px(stroke::HAIRLINE))
                    .border_color(c.border_default)
                    .when(shadow > 0., |d| {
                        d.shadow(theme.shadow(crate::Elevation::FLOATING))
                    })
                    .child(self.child),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_edges_and_corners() {
        let s = (800., 600.);
        assert_eq!(resize_edge((400., 300.), s, 10.), None);
        assert_eq!(resize_edge((2., 300.), s, 10.), Some(ResizeEdge::Left));
        assert_eq!(resize_edge((798., 300.), s, 10.), Some(ResizeEdge::Right));
        assert_eq!(resize_edge((400., 1.), s, 10.), Some(ResizeEdge::Top));
        assert_eq!(resize_edge((3., 3.), s, 10.), Some(ResizeEdge::TopLeft));
        assert_eq!(
            resize_edge((790., 595.), s, 10.),
            Some(ResizeEdge::BottomRight)
        );
        // Tiled windows (no shadow) keep a 6px edge.
        assert_eq!(resize_edge((5., 300.), s, 0.), Some(ResizeEdge::Left));
        assert_eq!(resize_edge((7., 300.), s, 0.), None);
    }
}
