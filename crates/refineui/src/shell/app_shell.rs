//! Application Shell — composes the chrome around a workspace (brief §5
//! Application, §6 layout rules). Layout follows
//! [`crate::shell::ShellSpec`] for the chrome platform:
//!
//! ```text
//! Titlebar (unified toolbar inside, or title + inline menu bar)
//! Menu bar row (Windows / Linux, under a unified titlebar)
//! Toolbar row (ToolbarPlacement::Separate)
//! Activity bar │ Left dock │ Workspace          │ Right dock
//!              │           ├────────────────────┤
//!              │           │ Bottom dock         │
//! Status bar
//! ```
//!
//! * macOS shows no in-window menu bar — install the same
//!   [`crate::shell::AppMenu`]s with [`crate::shell::set_system_menus`].
//! * Docks are fixed-size panes separated by hairline `borderSubtle`
//!   (resizable docks and tab groups arrive with the Phase 3 workspace).
//! * **F6 / Shift-F6** cycle keyboard focus through the regions in order
//!   (menu bar, toolbar, activity bar, left dock, workspace, bottom dock,
//!   right dock, status bar), wrapping; a region that receives focus this
//!   way shows an inset focus outline until focus moves inside it.
//! * On client-decorated windows (Linux) the whole shell sits in a
//!   [`crate::shell::WindowFrame`].

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::stroke;

use super::{
    AppMenu, ControlButtons, MenuBar, ShellRegion, ShellSpec, StatusBar, Titlebar,
    ToolbarPlacement, WindowFrame,
};
use crate::interaction::focus::cycle_region;
use crate::motion::{child_id, focus_visible};
use crate::{ActiveTheme, platform};

#[allow(missing_docs)]
mod actions {
    gpui::actions!(shell, [FocusNextRegion, FocusPreviousRegion]);
}
pub use actions::{FocusNextRegion, FocusPreviousRegion};

/// A docked pane and its size (width for side docks, height for the
/// bottom dock).
pub struct ShellDock {
    element: AnyElement,
    size: f32,
}

impl ShellDock {
    /// Dock `element` at `size` px.
    pub fn new(element: impl IntoElement, size: f32) -> Self {
        Self {
            element: element.into_any_element(),
            size,
        }
    }
}

/// The application shell. See the module docs.
#[derive(IntoElement)]
pub struct ApplicationShell {
    id: ElementId,
    spec: ShellSpec,
    title: Option<SharedString>,
    subtitle: Option<SharedString>,
    buttons: ControlButtons,
    titlebar_leading: Vec<AnyElement>,
    titlebar_trailing: Vec<AnyElement>,
    menus: Vec<AppMenu>,
    toolbar: Option<AnyElement>,
    activity_bar: Option<AnyElement>,
    left: Option<ShellDock>,
    right: Option<ShellDock>,
    bottom: Option<ShellDock>,
    center: Option<AnyElement>,
    area: Option<crate::workspace::DockArea>,
    status_bar: Option<StatusBar>,
    overlays: Vec<AnyElement>,
    os: Option<platform::Os>,
    embedded: bool,
    vibrant: bool,
}

impl ApplicationShell {
    /// Empty shell with the default spec.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            spec: ShellSpec::default(),
            title: None,
            subtitle: None,
            buttons: ControlButtons::default(),
            titlebar_leading: Vec::new(),
            titlebar_trailing: Vec::new(),
            menus: Vec::new(),
            toolbar: None,
            activity_bar: None,
            left: None,
            right: None,
            bottom: None,
            center: None,
            area: None,
            status_bar: None,
            overlays: Vec::new(),
            os: None,
            embedded: false,
            vibrant: false,
        }
    }

    /// Lay out and draw chrome for this OS (previews).
    pub fn os(mut self, os: platform::Os) -> Self {
        self.os = Some(os);
        self
    }

    /// A static preview inside another window: the titlebar does not move
    /// or close the window and no client-side frame is drawn.
    pub fn embedded(mut self, embedded: bool) -> Self {
        self.embedded = embedded;
        self
    }

    /// Let the chrome (titlebar, menu strip, activity bar, side docks,
    /// status bar) show what is behind the window: with the `material`
    /// feature active the shell root is left unpainted, so each chrome
    /// material blurs the window backdrop (a transparent / blurred OS
    /// window, or the gallery wallpaper). The workspace keeps its opaque
    /// `backgroundPrimary`. No effect without materials.
    pub fn vibrant(mut self, vibrant: bool) -> Self {
        self.vibrant = vibrant;
        self
    }

    /// Bind F6 / Shift-F6 to region cycling (also done by
    /// [`crate::init`]).
    pub fn bind_keys(cx: &mut App) {
        cx.bind_keys([
            gpui::KeyBinding::new("f6", FocusNextRegion, None),
            gpui::KeyBinding::new("shift-f6", FocusPreviousRegion, None),
        ]);
    }

    /// Shell configuration.
    pub fn spec(mut self, spec: ShellSpec) -> Self {
        self.spec = spec;
        self
    }

    /// Window title (titlebar).
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Titlebar subtitle.
    pub fn subtitle(mut self, t: impl Into<SharedString>) -> Self {
        self.subtitle = Some(t.into());
        self
    }

    /// Window buttons (`ControlButtons::from(preset)`).
    pub fn buttons(mut self, b: impl Into<ControlButtons>) -> Self {
        self.buttons = b.into();
        self
    }

    /// Extra element at the titlebar's leading edge.
    pub fn titlebar_leading(mut self, e: impl IntoElement) -> Self {
        self.titlebar_leading.push(e.into_any_element());
        self
    }

    /// Extra element at the titlebar's trailing edge.
    pub fn titlebar_trailing(mut self, e: impl IntoElement) -> Self {
        self.titlebar_trailing.push(e.into_any_element());
        self
    }

    /// Application menus (in-window bar on Windows / Linux).
    pub fn menus(mut self, menus: Vec<AppMenu>) -> Self {
        self.menus = menus;
        self
    }

    /// Toolbar (a `Toolbar::new(..).fill(true)`), placed per
    /// [`ShellSpec::toolbar`].
    pub fn toolbar(mut self, e: impl IntoElement) -> Self {
        self.toolbar = Some(e.into_any_element());
        self
    }

    /// Activity bar / navigation rail.
    pub fn activity_bar(mut self, e: impl IntoElement) -> Self {
        self.activity_bar = Some(e.into_any_element());
        self
    }

    /// Leading dock (sidebar).
    pub fn left_dock(mut self, dock: ShellDock) -> Self {
        self.left = Some(dock);
        self
    }

    /// Trailing dock (inspector).
    pub fn right_dock(mut self, dock: ShellDock) -> Self {
        self.right = Some(dock);
        self
    }

    /// Bottom dock (terminal, problems).
    pub fn bottom_dock(mut self, dock: ShellDock) -> Self {
        self.bottom = Some(dock);
        self
    }

    /// Center workspace.
    pub fn center(mut self, e: impl IntoElement) -> Self {
        self.center = Some(e.into_any_element());
        self
    }

    /// A [`crate::workspace::DockArea`]: its center becomes the workspace
    /// region and its visible docks the left / right / bottom dock regions
    /// (F6 stops), honouring zoom and the panel-drag edge zones. Replaces
    /// `left_dock` / `right_dock` / `bottom_dock` / `center`.
    pub fn workspace(mut self, area: crate::workspace::DockArea) -> Self {
        self.area = Some(area);
        self
    }

    /// Status bar.
    pub fn status_bar(mut self, bar: StatusBar) -> Self {
        self.status_bar = Some(bar);
        self
    }

    /// Element painted over the shell (a command dialog, toasts).
    pub fn overlay(mut self, e: impl IntoElement) -> Self {
        self.overlays.push(e.into_any_element());
        self
    }
}

fn region_id(shell: &ElementId, r: ShellRegion) -> ElementId {
    ElementId::NamedChild(Box::new(shell.clone()), format!("region-{r:?}").into())
}

impl RenderOnce for ApplicationShell {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(area) = self.area.take() {
            let parts = area.into_parts(cx);
            let dock =
                |p: Option<(AnyElement, f32)>| p.map(|(element, size)| ShellDock { element, size });
            self.left = dock(parts.left);
            self.right = dock(parts.right);
            self.bottom = dock(parts.bottom);
            self.center = Some(parts.center);
        }
        let theme = cx.theme().clone();
        let c = theme.colors;
        let os = self.os.unwrap_or_else(|| platform::chrome_os(cx));
        let adapter = platform::for_os(os);
        let metrics = self.spec.metrics(adapter, theme.density);
        let in_window_menu = self.spec.in_window_menu(adapter) && !self.menus.is_empty();
        let unified = self.spec.toolbar == ToolbarPlacement::Unified;

        // Regions present, in F6 order.
        let mut present: Vec<ShellRegion> = Vec::new();
        if in_window_menu {
            present.push(ShellRegion::MenuBar);
        }
        if self.toolbar.is_some() && self.spec.toolbar != ToolbarPlacement::None {
            present.push(if unified {
                ShellRegion::Titlebar
            } else {
                ShellRegion::Toolbar
            });
        }
        if self.activity_bar.is_some() && self.spec.activity_bar.is_some() {
            present.push(ShellRegion::ActivityBar);
        }
        if self.left.is_some() {
            present.push(ShellRegion::LeftDock);
        }
        present.push(ShellRegion::Workspace);
        if self.bottom.is_some() {
            present.push(ShellRegion::BottomDock);
        }
        if self.right.is_some() {
            present.push(ShellRegion::RightDock);
        }
        if self.status_bar.is_some() && self.spec.status_bar {
            present.push(ShellRegion::StatusBar);
        }
        let menubar_id = child_id(&self.id, "menubar");
        let menu_focus =
            in_window_menu.then(|| super::menu_bar::bar_handles(&menubar_id, window, cx));
        let handles: Vec<(ShellRegion, FocusHandle)> = present
            .iter()
            .map(|r| {
                if *r == ShellRegion::MenuBar
                    && let Some((f, _)) = &menu_focus
                {
                    // The bar owns its focus handle (arrow keys).
                    return (*r, f.clone());
                }
                let h = window
                    .use_keyed_state(region_id(&self.id, *r), cx, |_, cx| cx.focus_handle())
                    .read(cx)
                    .clone();
                (*r, h)
            })
            .collect();
        let ring = focus_visible(cx);
        let region = |r: ShellRegion, el: gpui::Div, window: &Window| -> gpui::Div {
            match handles
                .iter()
                .find(|(rr, _)| *rr == r && r != ShellRegion::MenuBar)
            {
                Some((_, h)) => {
                    let lit = ring && h.is_focused(window);
                    el.track_focus(h).relative().when(lit, |d| {
                        d.child(
                            div()
                                .absolute()
                                .inset_0()
                                .border(px(stroke::THICK))
                                .border_color(c.border_focus),
                        )
                    })
                }
                None => el,
            }
        };

        let menu_bar = in_window_menu.then(|| MenuBar::new(menubar_id.clone(), self.menus.clone()));
        let mut titlebar = Titlebar::new(child_id(&self.id, "titlebar"))
            .unified(unified && self.toolbar.is_some())
            .buttons(self.buttons)
            .os(os)
            .interactive(!self.embedded);
        if let Some(t) = &self.title {
            titlebar = titlebar.title(t.clone());
        }
        if let Some(t) = &self.subtitle {
            titlebar = titlebar.subtitle(t.clone());
        }
        let mut menu_row = None;
        let mut toolbar_row = None;
        let mut menu_bar = menu_bar;
        match self.toolbar {
            Some(tb) if unified => {
                titlebar = titlebar.content(tb);
                menu_row = menu_bar.take();
            }
            toolbar => {
                if let Some(m) = menu_bar.take() {
                    titlebar = titlebar.leading(m);
                }
                if self.spec.toolbar == ToolbarPlacement::Separate {
                    toolbar_row = toolbar;
                }
            }
        }
        for e in self.titlebar_leading {
            titlebar = titlebar.leading(e);
        }
        for e in self.titlebar_trailing {
            titlebar = titlebar.trailing(e);
        }

        let hair = px(stroke::HAIRLINE);
        let titlebar_el = region(
            ShellRegion::Titlebar,
            div().flex_none().child(titlebar),
            window,
        );
        let menu_el = menu_row.map(|m| {
            region(
                ShellRegion::MenuBar,
                div()
                    .flex_none()
                    .h(px(metrics.menu_bar))
                    .flex()
                    .items_center()
                    .px(px(refineui_foundation::spacing::X_SMALL))
                    .map(|el| {
                        let (fill, layer) = crate::material::chrome(
                            crate::SurfaceKind::Sidebar,
                            c.surface_sunken,
                            cx,
                        );
                        el.bg(fill).children(layer)
                    })
                    .border_b(hair)
                    .border_color(c.border_subtle)
                    .child(m),
                window,
            )
        });
        let toolbar_el = toolbar_row.map(|t| {
            region(
                ShellRegion::Toolbar,
                div()
                    .flex_none()
                    .h(px(metrics.toolbar))
                    .flex()
                    .items_center()
                    .px(px(refineui_foundation::spacing::X_SMALL))
                    .bg(c.background_primary)
                    .border_b(hair)
                    .border_color(c.border_subtle)
                    .child(t),
                window,
            )
        });
        let activity = self
            .activity_bar
            .filter(|_| self.spec.activity_bar.is_some())
            .map(|a| {
                region(
                    ShellRegion::ActivityBar,
                    div().flex_none().h_full().child(a),
                    window,
                )
            });
        let trailing_activity = self.spec.activity_bar == Some(super::RailEdge::Trailing);
        let (lead_act, trail_act) = if trailing_activity {
            (None, activity)
        } else {
            (activity, None)
        };
        let left = self.left.map(|d| {
            region(
                ShellRegion::LeftDock,
                div()
                    .flex_none()
                    .w(px(d.size))
                    .h_full()
                    .overflow_hidden()
                    .map(|el| {
                        let (fill, layer) = crate::material::chrome(
                            crate::SurfaceKind::Sidebar,
                            c.surface_sunken,
                            cx,
                        );
                        el.bg(fill).children(layer)
                    })
                    .border_r(hair)
                    .border_color(c.border_subtle)
                    .child(d.element),
                window,
            )
        });
        let right = self.right.map(|d| {
            region(
                ShellRegion::RightDock,
                div()
                    .flex_none()
                    .w(px(d.size))
                    .h_full()
                    .overflow_hidden()
                    .map(|el| {
                        let (fill, layer) = crate::material::chrome(
                            crate::SurfaceKind::Sidebar,
                            c.surface_sunken,
                            cx,
                        );
                        el.bg(fill).children(layer)
                    })
                    .border_l(hair)
                    .border_color(c.border_subtle)
                    .child(d.element),
                window,
            )
        });
        let bottom = self.bottom.map(|d| {
            region(
                ShellRegion::BottomDock,
                div()
                    .flex_none()
                    .h(px(d.size))
                    .w_full()
                    .overflow_hidden()
                    .bg(c.background_primary)
                    .border_t(hair)
                    .border_color(c.border_subtle)
                    .child(d.element),
                window,
            )
        });
        let center = region(
            ShellRegion::Workspace,
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .bg(c.background_primary)
                .children(self.center),
            window,
        );
        let status = self
            .status_bar
            .filter(|_| self.spec.status_bar)
            .map(|s| region(ShellRegion::StatusBar, div().flex_none().child(s), window));

        let order: Vec<FocusHandle> = handles.iter().map(|(_, h)| h.clone()).collect();
        let order_prev = order.clone();
        let step =
            move |forward: bool, order: &[FocusHandle], window: &mut Window, cx: &mut App| {
                let current = order.iter().position(|h| h.contains_focused(window, cx));
                if let Some(next) = cycle_region(current, order.len(), forward) {
                    crate::root::set_focus_visible(true, window, cx);
                    window.focus(&order[next]);
                }
            };

        let body = div()
            .id(self.id.clone())
            .key_context("ApplicationShell")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(if self.vibrant && crate::material::materials_active(cx) {
                gpui::transparent_black()
            } else {
                c.background_primary
            })
            .when_some(menu_focus, |d, (f, st)| {
                d.on_action(move |_: &super::FocusMenuBar, window, cx| {
                    super::menu_bar::enter_bar(&f, &st, window, cx)
                })
            })
            .on_action(move |_: &FocusNextRegion, window, cx| step(true, &order, window, cx))
            .on_action(move |_: &FocusPreviousRegion, window, cx| {
                step(false, &order_prev, window, cx)
            })
            .child(titlebar_el)
            .children(menu_el)
            .children(toolbar_el)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .children(lead_act)
                    .children(left)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(center)
                            .children(bottom),
                    )
                    .children(right)
                    .children(trail_act),
            )
            .children(status)
            .children(self.overlays);
        if self.embedded {
            body.into_any_element()
        } else {
            WindowFrame::new(body).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_ids_are_distinct() {
        let id: ElementId = "shell".into();
        assert_ne!(
            region_id(&id, ShellRegion::LeftDock),
            region_id(&id, ShellRegion::RightDock)
        );
    }
}
