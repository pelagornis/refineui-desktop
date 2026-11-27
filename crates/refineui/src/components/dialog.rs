//! Dialog and Drawer — web `Dialog` (`sm | lg`) / `Drawer` (`sm | md | lg`).
//!
//! Both are *controlled*. Keep rendering them and pass the state through
//! [`Dialog::open`] / [`Drawer::open`] so they can play their exit animation;
//! close them in `on_close` (scrim click, × button, or Escape).
//!
//! Motion (web `Dialog.tsx` / `Drawer.tsx`): the scrim fades over
//! `duration-overlay`; the dialog panel fades and rises `sizeSmall` (8px) over
//! `duration-panel` with the panel curve; the drawer slides in from the
//! trailing edge over `duration-panel`.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, anchored, div,
    point, prelude::*, px,
};
use refineui_foundation::{radius, size::component, spacing, typography::role};
use smallvec::SmallVec;

use crate::motion::{Motion, Presence, child_id, presence};
use crate::{
    ActiveTheme, Button, ButtonVariant, ControlSize, Elevation, IconName, IconSize, StyledExt,
};

/// Close callback.
pub type CloseHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Dialog width (web `size`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DialogSize {
    /// `dialogWidthSm` — 300px.
    Small,
    /// `dialogMaxWidth` — up to 600px (default).
    #[default]
    Large,
}

impl DialogSize {
    fn px(self) -> Pixels {
        match self {
            Self::Small => px(component::DIALOG_WIDTH_SM),
            Self::Large => px(component::DIALOG_MAX_WIDTH),
        }
    }
}

/// A centered modal dialog.
///
/// Panel: `backgroundPrimary`, `roundedXXLarge`, `sizeXXLarge` (24px) padding,
/// `shadow-8`; title `subtitleLg`, description `bodyXs` in `foregroundSecondary`;
/// ghost `sm` close button.
#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    size: DialogSize,
    children: SmallVec<[AnyElement; 2]>,
    footer: Option<AnyElement>,
    on_close: Option<CloseHandler>,
    inline: bool,
    open: bool,
    bare: bool,
    material: bool,
    initial_focus: Option<gpui::FocusHandle>,
    on_open: Option<CloseHandler>,
}

impl Dialog {
    /// New dialog.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            size: DialogSize::Large,
            children: SmallVec::new(),
            footer: None,
            on_close: None,
            inline: false,
            open: true,
            bare: false,
            material: false,
            initial_focus: None,
            on_open: None,
        }
    }

    /// What takes focus when the dialog opens (web `DialogContent` focuses
    /// the first focusable element in the panel). Default: the panel itself.
    /// Either way, closing gives focus back to whatever had it before.
    pub fn initial_focus(mut self, handle: gpui::FocusHandle) -> Self {
        self.initial_focus = Some(handle);
        self
    }

    /// Runs each time the dialog opens, before its content renders. The web
    /// `DialogContent` unmounts its children while closed, so their state
    /// starts fresh on every opening; entity content resets itself here.
    pub fn on_open(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(f));
        self
    }

    /// Draw the panel as the Sheet material (thick blur, glass rim, modal
    /// shadow) when [`crate::materials_active`]; otherwise unchanged.
    /// Default `false` (web parity: an opaque `backgroundPrimary` panel).
    pub fn material(mut self, material: bool) -> Self {
        self.material = material;
        self
    }

    /// Transparent panel without title, padding or chrome: children float on
    /// the scrim (web `DialogContent` with `commandStyles.dialogContentPanel`,
    /// used by [`crate::CommandDialog`]).
    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }

    /// Supporting text under the title.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Width.
    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Footer (usually buttons, right-aligned).
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Close handler (× button, scrim click, Escape).
    pub fn on_close(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    /// Open state (default `true`). Keep rendering with `open(false)` to play
    /// the exit animation; the dialog unmounts itself when it finishes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Render only the panel, in flow (docs / previews).
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

/// Web `DialogClose` / drawer close: ghost `sm` icon button, 20px dismiss icon.
fn close_button(id: ElementId, on_close: Option<CloseHandler>) -> AnyElement {
    Button::icon_only(id, IconName::Dismiss)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Small)
        .icon_size(IconSize::Small)
        .on_click(move |_, w, cx| {
            if let Some(h) = &on_close {
                h(w, cx)
            }
        })
        .into_any_element()
}

fn focus_on_mount(id: &ElementId, window: &mut Window, cx: &mut App) -> gpui::FocusHandle {
    let focus = window.use_keyed_state(child_id(id, "focus"), cx, |window, cx| {
        let h = cx.focus_handle();
        window.focus(&h);
        h
    });
    focus.read(cx).clone()
}

/// Web `useFocusTrap` on open/close: when `open` turns on, remember the
/// focused element, run `on_open` and focus `target`; when it turns off,
/// give focus back.
fn track_open(
    id: &ElementId,
    open: bool,
    target: &gpui::FocusHandle,
    on_open: Option<&CloseHandler>,
    window: &mut Window,
    cx: &mut App,
) {
    // (was open last frame, focus to give back on close)
    let state = window.use_keyed_state(child_id(id, "open-focus"), cx, |_, _| {
        (false, None::<gpui::FocusHandle>)
    });
    let was_open = state.read(cx).0;
    if open && !was_open {
        let restore = window.focused(cx);
        if let Some(h) = on_open {
            h(window, cx);
        }
        window.focus(target);
        state.update(cx, |s, _| *s = (true, restore));
    } else if !open && was_open {
        let restore = state.update(cx, |s, _| {
            s.0 = false;
            s.1.take()
        });
        if let Some(h) = restore {
            window.focus(&h);
        }
    }
}

impl RenderOnce for Dialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let p = if self.inline {
            Presence {
                mounted: true,
                progress: 1.0,
                open: true,
            }
        } else {
            presence(
                child_id(&self.id, "presence"),
                self.open,
                Motion::PANEL,
                Motion::PANEL,
                window,
                cx,
            )
        };
        if !p.mounted {
            return div().into_any_element();
        }
        let scrim_p = if self.inline {
            1.0
        } else {
            crate::motion::animate_from(
                child_id(&self.id, "scrim"),
                0.0,
                if self.open { 1.0 } else { 0.0 },
                Motion::SCRIM,
                window,
                cx,
            )
        };
        let max_h = window.viewport_size().height * 0.9;
        let panel_focus = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let target = self.initial_focus.clone().unwrap_or(panel_focus.clone());
        track_open(
            &self.id,
            self.open,
            &target,
            self.on_open.as_ref(),
            window,
            cx,
        );
        if self.bare {
            // Bare panels have no chrome to focus: `initial_focus` (e.g. a
            // search field) takes it; Escape still bubbles to the layer.
            let panel = div()
                .id("panel")
                .track_focus(&panel_focus)
                .occlude()
                .w(self.size.px())
                .max_w_full()
                .max_h(max_h)
                .flex()
                .flex_col()
                .font_family(theme.font_family.clone())
                .text_color(c.foreground_primary)
                .opacity(p.progress)
                .mt(px((1. - p.progress) * spacing::SMALL * 2.))
                .on_click(|_, _, cx| cx.stop_propagation())
                .children(self.children);
            if self.inline {
                return panel.into_any_element();
            }
            return modal_layer(
                self.id,
                panel.into_any_element(),
                if p.open { self.on_close } else { None },
                false,
                scrim_p,
                window,
                cx,
            );
        }
        let focus = panel_focus;
        let plate = if self.material {
            crate::material::backing(
                crate::SurfaceKind::Sheet,
                px(radius::XX_LARGE),
                0.,
                c.background_primary,
                None,
                theme.shadow(Elevation::FLOATING),
                cx,
            )
        } else {
            crate::material::Backing::opaque(
                c.background_primary,
                theme.shadow(Elevation::FLOATING),
            )
        };
        let panel = div()
            .id("panel")
            .track_focus(&focus)
            .occlude()
            .w(self.size.px())
            .max_w_full()
            .max_h(max_h)
            .flex()
            .flex_col()
            .rounded(px(radius::XX_LARGE))
            .bg(plate.fill)
            .shadow(plate.shadows)
            .children(plate.layer)
            .font_family(theme.font_family.clone())
            .text_color(c.foreground_primary)
            .text_style_token(role::BODY_MD)
            .opacity(p.progress)
            .mt(px((1. - p.progress) * spacing::SMALL * 2.))
            // ^ margin shifts both edges of a centered box, so 2× moves it `sizeSmall` (8px).
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .p(px(spacing::XX_LARGE))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::LARGE))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap(px(spacing::X_SMALL))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(spacing::X_SMALL))
                                    .child(
                                        div()
                                            .text_style_token(role::SUBTITLE_LG)
                                            .child(self.title),
                                    )
                                    .when_some(self.description, |d, t| {
                                        d.child(
                                            div()
                                                .text_style_token(role::BODY_XS)
                                                .text_color(c.foreground_secondary)
                                                .child(t),
                                        )
                                    }),
                            )
                            .child(close_button(child_id(&self.id, "close"), self.on_close.clone())),
                    )
                    .when(!self.children.is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::LARGE))
                                .children(self.children),
                        )
                    })
                    .when_some(self.footer, |d, f| {
                        d.child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(spacing::X_SMALL))
                                .child(f),
                        )
                    }),
            );

        if self.inline {
            return panel.into_any_element();
        }
        modal_layer(
            self.id,
            panel.into_any_element(),
            if p.open { self.on_close } else { None },
            false,
            scrim_p,
            window,
            cx,
        )
    }
}

pub(crate) fn modal_layer(
    id: ElementId,
    panel: AnyElement,
    on_close: Option<CloseHandler>,
    drawer: bool,
    scrim_progress: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // `surfaceOverlay` scrim. With materials active it is the Scrim material
    // (the web's 2px backdrop blur + dim), faded with the scrim progress.
    let backdrop = crate::material::backing(
        crate::SurfaceKind::Scrim,
        px(0.),
        0.,
        cx.theme().colors.surface_overlay,
        None,
        Vec::new(),
        cx,
    );
    let mut scrim = backdrop.fill;
    scrim.a *= scrim_progress;
    let scrim_layer = backdrop.layer.map(|layer| {
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .opacity(scrim_progress)
            .child(layer)
    });
    let viewport = window.viewport_size();
    let close_click = on_close.clone();
    let close_key = on_close;
    crate::layer(
        anchored().position(point(px(0.), px(0.))).child(
            div()
                .id(id)
                .occlude()
                .w(viewport.width)
                .h(viewport.height)
                .flex()
                .when(drawer, |d| d.justify_end())
                .when(!drawer, |d| {
                    d.items_center().justify_center().p(px(spacing::LARGE))
                })
                .bg(scrim)
                .on_click(move |_, w, cx| {
                    if let Some(h) = &close_click {
                        h(w, cx)
                    }
                })
                .on_key_down(move |e: &KeyDownEvent, w, cx| {
                    if e.keystroke.key == "escape"
                        && let Some(h) = &close_key
                    {
                        cx.stop_propagation();
                        h(w, cx)
                    }
                })
                .children(scrim_layer)
                .child(panel),
        ),
    )
    .priority(2)
    .into_any_element()
}

/// Drawer width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawerSize {
    /// `drawerWidthSm` — 320px (default)
    #[default]
    Small,
    /// `drawerWidthMd` — 575px
    Medium,
    /// `drawerWidthLg` — 850px
    Large,
}

/// Side sheet sliding in from the trailing edge.
///
/// `backgroundPrimary` panel with `shadow-16`; header `24 / 24 / 10` padding with
/// a `subtitleLg` title and `bodyXs` description; body `bodyMd`; footer
/// `sizeXXLarge × sizeXLarge` padding. Like the web there is no grab handle
/// and no close button by default (scrim click / Escape close it); add one
/// with [`Drawer::close_button`]. [`Drawer::footer_actions`] follows web
/// `DrawerFooter` states: one action → full width, two → equal split, more
/// → right-aligned.
#[derive(IntoElement)]
pub struct Drawer {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    size: DrawerSize,
    children: SmallVec<[AnyElement; 2]>,
    footer: Option<AnyElement>,
    footer_actions: Vec<AnyElement>,
    close_button: bool,
    on_close: Option<CloseHandler>,
    inline: bool,
    open: bool,
}

impl Drawer {
    /// New drawer.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            size: DrawerSize::Small,
            children: SmallVec::new(),
            footer: None,
            footer_actions: Vec::new(),
            close_button: false,
            on_close: None,
            inline: false,
            open: true,
        }
    }

    /// Footer actions laid out like web `DrawerFooter` (`single` → full
    /// width, `split` → equal halves, `icons` → trailing row). Pass buttons
    /// built with `.full_width(true)` for the single / split states.
    pub fn footer_actions(mut self, actions: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.footer_actions = actions.into_iter().map(|a| a.into_any_element()).collect();
        self
    }

    /// Show a ghost dismiss button in the header (web `DrawerHeader actions`).
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }

    /// Supporting text under the title.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Width.
    pub fn size(mut self, size: DrawerSize) -> Self {
        self.size = size;
        self
    }

    /// Footer.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Close handler.
    pub fn on_close(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    /// Open state (default `true`); see [`Dialog::open`].
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Render only the panel, in flow.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

impl ParentElement for Drawer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Drawer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let width = match self.size {
            DrawerSize::Small => component::DRAWER_WIDTH_SM,
            DrawerSize::Medium => component::DRAWER_WIDTH_MD,
            DrawerSize::Large => component::DRAWER_WIDTH_LG,
        };
        let p = if self.inline {
            Presence {
                mounted: true,
                progress: 1.0,
                open: true,
            }
        } else {
            presence(
                child_id(&self.id, "presence"),
                self.open,
                Motion::PANEL,
                Motion::PANEL,
                window,
                cx,
            )
        };
        if !p.mounted {
            return div().into_any_element();
        }
        let scrim_p = if self.inline {
            1.0
        } else {
            crate::motion::animate_from(
                child_id(&self.id, "scrim"),
                0.0,
                if self.open { 1.0 } else { 0.0 },
                Motion::SCRIM,
                window,
                cx,
            )
        };
        let focus = focus_on_mount(&self.id, window, cx);
        let panel = div()
            .id("panel")
            .track_focus(&focus)
            .occlude()
            .relative()
            .w(px(width))
            .h_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(c.background_primary)
            .when(!self.inline, |d| d.shadow(theme.shadow(Elevation::OVERLAY)))
            .font_family(theme.font_family.clone())
            .text_color(c.foreground_primary)
            .when(!self.inline, |d| d.mr(px(-(1. - p.progress) * width)))
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_start()
                    .gap(px(spacing::X_SMALL))
                    .px(px(spacing::XX_LARGE))
                    .pt(px(spacing::XX_LARGE))
                    .pb(px(spacing::MEDIUM))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(spacing::XXX_SMALL))
                            .child(div().text_style_token(role::SUBTITLE_LG).child(self.title))
                            .when_some(self.description, |d, t| {
                                d.child(
                                    div()
                                        .text_style_token(role::BODY_XS)
                                        .text_color(c.foreground_secondary)
                                        .child(t),
                                )
                            }),
                    )
                    .when(self.close_button, |d| {
                        d.child(close_button(
                            child_id(&self.id, "close"),
                            self.on_close.clone(),
                        ))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::MEDIUM))
                    .px(px(spacing::XX_LARGE))
                    .text_style_token(role::BODY_MD)
                    .children(self.children),
            )
            .when(!self.footer_actions.is_empty(), |d| {
                let n = self.footer_actions.len();
                d.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_end()
                        .gap(px(spacing::X_SMALL))
                        .px(px(spacing::XX_LARGE))
                        .py(px(spacing::X_LARGE))
                        .children(self.footer_actions.into_iter().map(move |a| {
                            if n <= 2 {
                                div().flex().flex_1().min_w_0().child(a)
                            } else {
                                div().flex().flex_none().child(a)
                            }
                        })),
                )
            })
            .when_some(self.footer, |d, f| {
                d.child(
                    div()
                        .flex()
                        .flex_none()
                        .justify_end()
                        .gap(px(spacing::X_SMALL))
                        .px(px(spacing::XX_LARGE))
                        .py(px(spacing::X_LARGE))
                        .child(f),
                )
            });
        if self.inline {
            return panel.into_any_element();
        }
        modal_layer(
            self.id,
            panel.into_any_element(),
            if p.open { self.on_close } else { None },
            true,
            scrim_p,
            window,
            cx,
        )
    }
}
