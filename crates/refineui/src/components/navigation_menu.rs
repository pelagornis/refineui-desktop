//! NavigationMenu — web `NavigationMenu` (`components/NavigationMenu`),
//! compact dropdown form: a row of top-level links / triggers (`captionLg`,
//! 36px tall, min 40px wide, `px-medium` / `py-x-small`, `roundedXLarge`,
//! 2px apart) whose hover / open plate is `backgroundPrimaryHover`. A trigger
//! opens a dropdown panel anchored under its item: `roundedXXLarge`,
//! hairline `borderDefault`, `p-xx-small`, rows 1px apart, `shadow-2`,
//! min width 200 (`dropdownMenuWidth`) on the frosted surface. Rows match a
//! Menu item (`bodyMd`, `p-x-small`, `roundedLarge`, gap 6) with an optional
//! description. The panel fades + slides in over `duration-normal`
//! (emphasized) and the chevron turns 180° over `duration-fast`. While a
//! panel is open, hovering another trigger switches to it (web behavior).
//!
//! The web "floating" mega panel and full-width "overlay" variants are not
//! ported: they are site-header layouts, not desktop app chrome.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Transformation, Window, anchored, div, prelude::*, px,
    radians,
};
use refineui_tokens::{radius, spacing, stroke, typography::role};

use crate::motion::{Motion, animate, child_id, presence, probe, use_layout_probe};
use crate::{ActiveTheme, Elevation, Icon, IconName, IconSize, StyledExt};

type Click = Rc<dyn Fn(&mut Window, &mut App)>;

/// A row inside a navigation dropdown.
pub struct NavigationLink {
    label: SharedString,
    description: Option<SharedString>,
    icon: Option<IconName>,
    on_click: Option<Click>,
}

impl NavigationLink {
    /// Row labelled `label`.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            icon: None,
            on_click: None,
        }
    }

    /// Secondary line under the label.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Activation handler (the panel closes afterwards).
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

enum Entry {
    Link {
        label: SharedString,
        active: bool,
        disabled: bool,
        on_click: Option<Click>,
    },
    Menu {
        label: SharedString,
        disabled: bool,
        links: Vec<NavigationLink>,
        layout: NavigationLayout,
        custom: Option<PanelFn>,
    },
}

type PanelFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Panel treatment (web `NavigationMenuContent layout`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationLayout {
    /// Compact frosted panel under the trigger (default).
    #[default]
    Dropdown,
    /// Wide "mega" panel centred in the window under the bar, ≤ 1000px
    /// (`navigationMenuMegaWidth`), on `backgroundPrimary`; fades and rises
    /// 6px over `duration-normal` (emphasized).
    Floating,
    /// Full-width sheet from the bar's bottom edge over a scrim; its height
    /// grows over `duration-panel` (panel curve). Hovering or clicking the
    /// scrim closes it.
    Overlay,
}

/// `navigationMenuMegaWidth` (foundationSize10000).
pub const NAVIGATION_MENU_MEGA_WIDTH: f32 = 1000.;

/// Top navigation bar.
#[derive(IntoElement)]
pub struct NavigationMenu {
    id: ElementId,
    entries: Vec<Entry>,
    default_open: Option<usize>,
}

impl NavigationMenu {
    /// Empty bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            default_open: None,
        }
    }

    /// Open the menu entry at `index` (0-based over all entries) on first
    /// render — handy for previews and screenshots.
    pub fn default_open(mut self, index: usize) -> Self {
        self.default_open = Some(index);
        self
    }

    /// Plain top-level link; `active` marks the current page.
    pub fn link(
        mut self,
        label: impl Into<SharedString>,
        active: bool,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(Entry::Link {
            label: label.into(),
            active,
            disabled: false,
            on_click: Some(Rc::new(on_click)),
        });
        self
    }

    /// Disabled top-level link.
    pub fn disabled_link(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Link {
            label: label.into(),
            active: false,
            disabled: true,
            on_click: None,
        });
        self
    }

    /// Trigger with a dropdown of `links`.
    pub fn menu(
        mut self,
        label: impl Into<SharedString>,
        links: impl IntoIterator<Item = NavigationLink>,
    ) -> Self {
        self.entries.push(Entry::Menu {
            label: label.into(),
            disabled: false,
            links: links.into_iter().collect(),
            layout: NavigationLayout::Dropdown,
            custom: None,
        });
        self
    }

    /// Trigger with a custom panel (web `NavigationMenuContent` children) in
    /// the given `layout` — e.g. a [`NavigationLayout::Floating`] mega menu.
    pub fn panel(
        mut self,
        label: impl Into<SharedString>,
        layout: NavigationLayout,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.entries.push(Entry::Menu {
            label: label.into(),
            disabled: false,
            links: Vec::new(),
            layout,
            custom: Some(Rc::new(content)),
        });
        self
    }
}

fn top_item(id: ElementId, active: bool, disabled: bool, cx: &App) -> gpui::Stateful<gpui::Div> {
    let c = &cx.theme().colors;
    let (hover, press) = (c.background_primary_hover, c.background_primary_active);
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(spacing::X_SMALL))
        .h(px(36.))
        .min_w(px(40.))
        .px(px(spacing::MEDIUM))
        .py(px(spacing::X_SMALL))
        .rounded(px(radius::X_LARGE))
        .text_style_token(role::CAPTION_LG)
        .when(active, |d| d.bg(hover))
        .when(disabled, |d| {
            d.text_color(c.foreground_disabled).cursor_not_allowed()
        })
        .when(!disabled, |d| {
            d.text_color(c.foreground_primary)
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .active(move |s| s.bg(press))
        })
}

impl RenderOnce for NavigationMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let initial = self.default_open;
        let open = window.use_keyed_state(child_id(&self.id, "open"), cx, move |_, _| initial);
        let current = *open.read(cx);
        let mut list = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(spacing::XXX_SMALL));
        for (ix, entry) in self.entries.into_iter().enumerate() {
            let item_id =
                ElementId::NamedChild(Box::new(self.id.clone()), format!("item-{ix}").into());
            match entry {
                Entry::Link {
                    label,
                    active,
                    disabled,
                    on_click,
                } => {
                    let close = open.clone();
                    list = list.child(
                        top_item(item_id, active, disabled, cx)
                            .when(!disabled, |d| {
                                d.on_click(move |_, window, cx| {
                                    close.update(cx, |o, cx| {
                                        *o = None;
                                        cx.notify();
                                    });
                                    if let Some(f) = &on_click {
                                        f(window, cx);
                                    }
                                })
                            })
                            .child(label),
                    );
                }
                Entry::Menu {
                    label,
                    disabled,
                    links,
                    layout,
                    custom,
                } => {
                    let is_open = current == Some(ix);
                    let chevron = animate(
                        child_id(&item_id, "chev"),
                        if is_open { 1.0f32 } else { 0. },
                        Motion::FAST,
                        window,
                        cx,
                    );
                    let enter = match layout {
                        NavigationLayout::Overlay => Motion::PANEL,
                        _ => Motion::new(
                            crate::motion::duration::NORMAL,
                            crate::motion::easing::EMPHASIZED,
                        ),
                    };
                    let exit = match layout {
                        NavigationLayout::Overlay => Motion::PANEL,
                        _ => Motion::FAST,
                    };
                    let p = presence(
                        child_id(&item_id, "panel"),
                        is_open,
                        enter,
                        exit,
                        window,
                        cx,
                    );
                    let item_probe = use_layout_probe(&child_id(&item_id, "probe"), window, cx);
                    let toggle = open.clone();
                    let switch = open.clone();
                    let trigger = top_item(item_id.clone(), is_open, disabled, cx)
                        .when(!disabled, |d| {
                            d.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                toggle.update(cx, |o, cx| {
                                    *o = if *o == Some(ix) { None } else { Some(ix) };
                                    cx.notify();
                                });
                            })
                            .on_hover(move |inside, _, cx| {
                                if *inside {
                                    switch.update(cx, |o, cx| {
                                        if o.is_some() && *o != Some(ix) {
                                            *o = Some(ix);
                                            cx.notify();
                                        }
                                    });
                                }
                            })
                        })
                        .child(label)
                        .child(
                            Icon::new(IconName::ChevronDown)
                                .size(IconSize::Small)
                                .color(if disabled {
                                    c.foreground_disabled
                                } else {
                                    c.foreground_secondary
                                })
                                .transform(Transformation::rotate(radians(
                                    chevron * std::f32::consts::PI,
                                ))),
                        );
                    let panel: Option<AnyElement> = p.mounted.then(|| {
                        let close_out = open.clone();
                        let rows = links.into_iter().enumerate().map(|(r, link)| {
                            let close = open.clone();
                            let on_click = link.on_click.clone();
                            div()
                                .id(r)
                                .flex()
                                .w_full()
                                .items_start()
                                .gap(px(spacing::X_SMALL))
                                .p(px(spacing::X_SMALL))
                                .rounded(px(radius::LARGE))
                                .cursor_pointer()
                                .text_color(c.foreground_primary)
                                .hover(|s| s.bg(c.background_primary_hover))
                                .active(|s| s.bg(c.background_primary_active))
                                .on_click(move |_, window, cx| {
                                    close.update(cx, |o, cx| {
                                        *o = None;
                                        cx.notify();
                                    });
                                    if let Some(f) = &on_click {
                                        f(window, cx);
                                    }
                                })
                                .when_some(link.icon, |d, ic| {
                                    d.child(
                                        Icon::new(ic)
                                            .size(IconSize::Medium)
                                            .color(c.foreground_secondary),
                                    )
                                })
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .min_w_0()
                                        .child(
                                            div().text_style_token(role::BODY_MD).child(link.label),
                                        )
                                        .when_some(link.description, |d, t| {
                                            d.child(
                                                div()
                                                    .text_style_token(role::CAPTION_MD)
                                                    .text_color(c.foreground_tertiary)
                                                    .child(t),
                                            )
                                        }),
                                )
                        });
                        let body: AnyElement = match &custom {
                            Some(f) => f(window, cx),
                            None => div()
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .children(rows)
                                .into_any_element(),
                        };
                        let trigger_bounds = item_probe.read(cx).container;
                        let panel_size = item_probe.read(cx).items.first().copied().flatten();
                        match layout {
                            NavigationLayout::Dropdown => crate::layer(
                                anchored().snap_to_window_with_margin(px(8.)).child(
                                    div()
                                        .id("nav-panel")
                                        .occlude()
                                        .mt(px(spacing::XX_SMALL
                                            - spacing::X_SMALL * (1. - p.progress)))
                                        .opacity(p.progress)
                                        .when(p.open, |d| {
                                            d.on_mouse_down_out(move |_, _, cx| {
                                                close_out.update(cx, |o, cx| {
                                                    *o = None;
                                                    cx.notify();
                                                })
                                            })
                                        })
                                        .min_w(px(200.))
                                        .max_w(px(360.))
                                        .p(px(spacing::XX_SMALL))
                                        .rounded(px(radius::XX_LARGE))
                                        .border(px(stroke::HAIRLINE))
                                        .border_color(c.border_default)
                                        .bg(theme.floating_fill())
                                        .shadow(theme.shadow(Elevation::RAISED))
                                        .font_family(theme.font_family.clone())
                                        .child(body),
                                ),
                            )
                            .into_any_element(),
                            NavigationLayout::Floating => {
                                let vw = f32::from(window.viewport_size().width);
                                let (left, top, measured) = match (trigger_bounds, panel_size) {
                                    (Some(t), Some(pb)) => {
                                        let w = f32::from(pb.size.width);
                                        let edge = spacing::MEDIUM;
                                        let left = ((vw - w) / 2.).min(vw - w - edge).max(edge);
                                        (left, f32::from(t.bottom()) + spacing::XX_SMALL, true)
                                    }
                                    (Some(t), None) => (0., f32::from(t.bottom()), false),
                                    _ => (0., 0., false),
                                };
                                crate::layer(
                                    anchored()
                                        .position(gpui::point(px(left), px(top)))
                                        .child(
                                            div()
                                                .id("nav-mega")
                                                .relative()
                                                .occlude()
                                                .mt(px(spacing::X_SMALL * (1. - p.progress)))
                                                .opacity(if measured { p.progress } else { 0. })
                                                .when(p.open, |d| {
                                                    d.on_mouse_down_out(move |_, _, cx| {
                                                        close_out.update(cx, |o, cx| {
                                                            *o = None;
                                                            cx.notify();
                                                        })
                                                    })
                                                })
                                                .max_w(px(NAVIGATION_MENU_MEGA_WIDTH.min(vw - 2. * spacing::MEDIUM)))
                                                .p(px(spacing::XX_SMALL))
                                                .rounded(px(radius::XX_LARGE))
                                                .border(px(stroke::HAIRLINE))
                                                .border_color(c.border_default)
                                                .bg(c.background_primary)
                                                .shadow(theme.shadow(Elevation::RAISED))
                                                .font_family(theme.font_family.clone())
                                                .child(probe(&item_probe, Some(0)))
                                                .child(body),
                                        ),
                                )
                                .into_any_element()
                            }
                            NavigationLayout::Overlay => {
                                let vp = window.viewport_size();
                                let top = trigger_bounds.map(|t| f32::from(t.bottom())).unwrap_or(0.);
                                let inner_h = panel_size.map(|b| f32::from(b.size.height));
                                let close_scrim = open.clone();
                                let close_hover = open.clone();
                                crate::layer(
                                    anchored().position(gpui::point(px(0.), px(top))).child(
                                        div()
                                            .relative()
                                            .w(vp.width)
                                            .h(vp.height - px(top))
                                            .font_family(theme.font_family.clone())
                                            .child(
                                                div()
                                                    .id("nav-scrim")
                                                    .absolute()
                                                    .inset_0()
                                                    .bg(c.surface_overlay)
                                                    .opacity(p.progress)
                                                    .when(p.open, |d| {
                                                        d.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                                            close_scrim.update(cx, |o, cx| {
                                                                *o = None;
                                                                cx.notify();
                                                            })
                                                        })
                                                        // Web: pointer entering the scrim closes.
                                                        .on_hover(move |inside, _, cx| {
                                                            if *inside {
                                                                close_hover.update(cx, |o, cx| {
                                                                    *o = None;
                                                                    cx.notify();
                                                                })
                                                            }
                                                        })
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .id("nav-overlay")
                                                    .relative()
                                                    .occlude()
                                                    .w_full()
                                                    .overflow_hidden()
                                                    .when_some(inner_h, |d, h| d.h(px(h * p.progress)))
                                                    .when(inner_h.is_none(), |d| d.opacity(0.))
                                                    .border_b(px(stroke::HAIRLINE))
                                                    .border_color(c.border_default)
                                                    .bg(c.background_primary)
                                                    .shadow(theme.shadow(Elevation::RAISED))
                                                    .child(
                                                        div()
                                                            .relative()
                                                            .flex()
                                                            .w_full()
                                                            .justify_center()
                                                            .child(probe(&item_probe, Some(0)))
                                                            .child(body),
                                                    ),
                                            ),
                                    ),
                                )
                                .into_any_element()
                            }
                        }
                    });
                    list = list.child(
                        div()
                            .relative()
                            .flex_none()
                            .child(probe(&item_probe, None))
                            .child(trigger)
                            .children(panel),
                    );
                }
            }
        }
        div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .child(list)
    }
}
