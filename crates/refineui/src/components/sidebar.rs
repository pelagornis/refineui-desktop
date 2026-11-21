//! Sidebar — web `Sidebar` family: `Sidebar`, `SidebarHeader` / `SidebarBrand`,
//! `SidebarContent` (built-in ScrollArea with edge blur), `SidebarNav` /
//! `SidebarGroup` / `SidebarGroupLabel` / `SidebarLink`, `SidebarFooter`,
//! `SidebarTrigger` and `SidebarPeek`.
//!
//! Tokens (`style.ts` + `refineui.css`):
//! - root: `backgroundPrimary`, hairline `borderDefault` end edge;
//! - header: hairline bottom rule, `sizeLarge` padding, brand `bodyMd` primary;
//! - content: ScrollArea, viewport `sizeMedium` padding, `sizeXLarge` between
//!   groups, `sizeXxSmall` between rows;
//! - group label: `captionLg` tertiary, `px sizeMedium`, `py sizeXxxSmall`;
//! - link: `bodyMd`, `gap sizeXSmall`, `px sizeMedium`, `py sizeXSmall`,
//!   `radius large`, tertiary text; hover → `backgroundPrimaryHover` +
//!   `foregroundPrimaryHover`; active (current page) → `backgroundPrimaryHover`
//!   \+ primary text + `font-medium`; pressed → `backgroundPrimaryActive`;
//!   colors transition over `duration-fast`;
//! - footer: hairline top rule, `sizeMedium` padding, holds link rows.

use std::rc::Rc;
use std::time::Instant;

use gpui::{
    AnyElement, App, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{
    motion::duration, radius, shadow::Elevation, spacing, stroke, typography::role,
};

use crate::components::plate::row_plate;
use crate::motion::{
    LayoutProbe, Motion, TrackInteraction, animate, child_id, focus_visible, probe,
    use_focus_handle, use_interaction, use_layout_probe,
};
use crate::{
    ActiveTheme, Badge, BadgeVariant, Button, ButtonVariant, ControlSize, Icon, IconName, IconSize,
    ScrollArea, StyledExt, scroll_edge_item,
};

/// One navigation item (web `SidebarLink`).
#[derive(Clone)]
pub struct SidebarItem {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    badge: Option<SharedString>,
    disabled: bool,
}

impl SidebarItem {
    /// New item.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            badge: None,
            disabled: false,
        }
    }

    /// Leading glyph (`iconSizes.small`).
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Trailing badge.
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Disabled link (`aria-disabled`).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

struct Group {
    label: Option<SharedString>,
    items: Vec<SidebarItem>,
}

type SelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// App navigation landmark.
#[derive(IntoElement)]
pub struct Sidebar {
    id: ElementId,
    header: Option<AnyElement>,
    brand: Option<SharedString>,
    footer: Option<AnyElement>,
    footer_items: Vec<SidebarItem>,
    groups: Vec<Group>,
    selected: Option<SharedString>,
    width: Option<Pixels>,
    collapsed: bool,
    embedded: bool,
    on_select: Option<SelectHandler>,
}

impl Sidebar {
    /// New sidebar, 256px wide (`sidebarWidth`). Use [`Self::fill`] inside a
    /// resizable panel, like the web demo.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            header: None,
            brand: None,
            footer: None,
            footer_items: Vec::new(),
            groups: Vec::new(),
            selected: None,
            width: Some(px(refineui_foundation::size::component::SIDEBAR_WIDTH)),
            collapsed: false,
            embedded: false,
            on_select: None,
        }
    }

    /// Custom header content (inside `SidebarHeader`).
    pub fn header(mut self, el: impl IntoElement) -> Self {
        self.header = Some(el.into_any_element());
        self
    }

    /// `SidebarBrand` text in the header.
    pub fn brand(mut self, text: impl Into<SharedString>) -> Self {
        self.brand = Some(text.into());
        self
    }

    /// Custom footer content (inside `SidebarFooter`).
    pub fn footer(mut self, el: impl IntoElement) -> Self {
        self.footer = Some(el.into_any_element());
        self
    }

    /// A link row in the footer (web demo: the account row).
    pub fn footer_item(mut self, item: SidebarItem) -> Self {
        self.footer_items.push(item);
        self
    }

    /// Add a group (`SidebarGroup` with an optional `SidebarGroupLabel`).
    pub fn group(
        mut self,
        label: Option<&str>,
        items: impl IntoIterator<Item = SidebarItem>,
    ) -> Self {
        self.groups.push(Group {
            label: label.map(|l| SharedString::from(l.to_string())),
            items: items.into_iter().collect(),
        });
        self
    }

    /// Current page (`active` link).
    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    /// Fixed width.
    pub fn width(mut self, w: Pixels) -> Self {
        self.width = Some(w);
        self
    }

    /// Fill the parent (web `w-full flex-1`), e.g. inside a [`crate::ResizablePanel`].
    pub fn fill(mut self) -> Self {
        self.width = None;
        self
    }

    /// Collapse a fixed-width sidebar to 0 (`duration-panel`, panel easing).
    /// The rail keeps its width while the edge closes over it, like the web
    /// demo's frozen rail.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Inside a [`SidebarPeek`] panel: transparent, no end border.
    pub fn embedded(mut self, embedded: bool) -> Self {
        self.embedded = embedded;
        self
    }

    /// Link click.
    pub fn on_select(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

fn link_row(
    parent: &ElementId,
    item: SidebarItem,
    active: bool,
    on_select: Option<SelectHandler>,
    host: Option<(&Entity<LayoutProbe>, usize)>,
    window: &mut Window,
    cx: &mut App,
) -> (AnyElement, bool) {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let id = ElementId::NamedChild(Box::new(parent.clone()), item.id.clone());
    let (state, mut i) = use_interaction(&id, window, cx);
    let hovered = i.hovered && !item.disabled;
    // With a sliding plate the row drops its own hover fill (web
    // `[data-sliding-highlight]`), keeping active / pressed fills.
    if host.is_some() && !i.pressed {
        i.hovered = false;
    }
    let focus = use_focus_handle(&id, 0, window, cx);
    let (bg, fg) = row_plate(i, active, item.disabled, c.foreground_tertiary, cx);
    let bg = animate(child_id(&id, "bg"), bg, Motion::FAST, window, cx);
    let fg = animate(child_id(&id, "fg"), fg, Motion::FAST, window, cx);
    let focused = focus_visible(cx) && focus.is_focused(window);
    let key_handler = on_select.clone();
    let item_id = item.id.clone();
    let key_id = item.id.clone();
    let el = div()
        .id(id)
        .relative()
        .flex()
        .w_full()
        .min_w_0()
        .items_center()
        .gap(px(spacing::X_SMALL))
        .px(px(spacing::MEDIUM))
        .py(px(spacing::X_SMALL + theme.row_inset()))
        .rounded(px(radius::LARGE))
        .bg(bg)
        .text_style_token(role::BODY_MD)
        .text_color(fg)
        .when(active, |d| d.font_weight(gpui::FontWeight::MEDIUM))
        .when(focused, |d| d.shadow(theme.focus_ring()))
        .when(item.disabled, |d| {
            d.cursor(gpui::CursorStyle::OperationNotAllowed)
        })
        .when(!item.disabled, |d| {
            d.cursor_action()
                .track_focus(&focus)
                .track_interaction(&state)
                .when_some(on_select, |d, h| {
                    d.on_click(move |_, w, cx| h(&item_id, w, cx))
                })
                .when_some(key_handler, |d, h| {
                    d.on_key_down(move |e: &gpui::KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                            h(&key_id, w, cx)
                        }
                    })
                })
        })
        .when_some(item.icon, |d, ic| {
            d.child(Icon::new(ic).size(IconSize::Small).color(fg))
        })
        .child(div().flex_1().min_w_0().truncate().child(item.label))
        .when_some(item.badge, |d, b| {
            d.child(Badge::new(b).variant(BadgeVariant::Neutral))
        })
        .when_some(host, |d, (h, ix)| d.child(probe(h, Some(ix))))
        .into_any_element();
    (el, hovered)
}

impl RenderOnce for Sidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let parent = self.id.clone();
        let selected = self.selected.clone();
        let hairline = px(stroke::HAIRLINE);
        let mut groups = Vec::new();
        let host = use_layout_probe(&child_id(&self.id, "highlight"), window, cx);
        let mut row_ix = 0usize;
        let mut plate_target = None;
        for (gi, g) in self.groups.into_iter().enumerate() {
            // Web: links and group labels inside the scroll are
            // `data-scroll-edge-item` rows.
            let rows: Vec<AnyElement> = g
                .items
                .into_iter()
                .map(|item| {
                    let active = selected.as_ref() == Some(&item.id);
                    let key = ElementId::Name(format!("edge-{}", item.id).into());
                    let ix = row_ix;
                    row_ix += 1;
                    let (row, hovered) = link_row(
                        &parent,
                        item,
                        active,
                        self.on_select.clone(),
                        Some((&host, ix)),
                        window,
                        cx,
                    );
                    if hovered {
                        plate_target = Some(ix);
                    }
                    scroll_edge_item(key, row).into_any_element()
                })
                .collect();
            groups.push(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .gap(px(spacing::XX_SMALL))
                    .when_some(g.label, |d, l| {
                        d.child(scroll_edge_item(
                            ElementId::Name(format!("edge-label-{gi}").into()),
                            div()
                                .px(px(spacing::MEDIUM))
                                .py(px(spacing::XXX_SMALL))
                                .text_style_token(role::CAPTION_LG)
                                .text_color(c.foreground_tertiary)
                                .child(l),
                        ))
                    })
                    .children(rows),
            );
        }
        let footer_rows: Vec<AnyElement> = self
            .footer_items
            .into_iter()
            .map(|item| {
                let active = selected.as_ref() == Some(&item.id);
                link_row(
                    &parent,
                    item,
                    active,
                    self.on_select.clone(),
                    None,
                    window,
                    cx,
                )
                .0
            })
            .collect();
        let slide_plate = crate::components::sliding_highlight::sliding_plate(
            &child_id(&self.id, "highlight"),
            &host,
            plate_target,
            c.background_primary_hover,
            radius::LARGE,
            window,
            cx,
        );
        let has_header = self.header.is_some() || self.brand.is_some();
        let has_footer = self.footer.is_some() || !footer_rows.is_empty();
        let rail = div()
            .flex()
            .flex_col()
            .h_full()
            .min_h_0()
            .min_w_0()
            .when_some(self.width, |d, w| d.w(w).flex_none())
            .when(self.width.is_none(), |d| d.w_full().flex_1())
            .when(!self.embedded, |d| {
                let (fill, layer) =
                    crate::material::chrome(crate::SurfaceKind::Sidebar, c.background_primary, cx);
                d.relative()
                    .bg(fill)
                    .children(layer)
                    .border_r(hairline)
                    .border_color(c.border_default)
            })
            .when(has_header, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_none()
                        .border_b(hairline)
                        .border_color(c.border_default)
                        .p(px(spacing::LARGE))
                        .when_some(self.brand, |d, b| {
                            d.child(
                                div()
                                    .flex()
                                    .min_w_0()
                                    .items_center()
                                    .gap(px(spacing::SMALL))
                                    .text_style_token(role::BODY_MD)
                                    .text_color(c.foreground_primary)
                                    .child(b),
                            )
                        })
                        .when_some(self.header, |d, h| d.child(h)),
                )
            })
            .child(
                div().flex().flex_col().flex_1().min_h_0().child(
                    ScrollArea::new(child_id(&self.id, "content")).child(
                        div()
                            .relative()
                            .p(px(spacing::MEDIUM))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::X_LARGE))
                            .child(probe(&host, None))
                            .child(slide_plate)
                            .children(groups),
                    ),
                ),
            )
            .when(has_footer, |d| {
                d.child(
                    div()
                        .mt_auto()
                        .flex()
                        .flex_col()
                        .flex_none()
                        .border_t(hairline)
                        .border_color(c.border_default)
                        .p(px(spacing::MEDIUM))
                        .children(footer_rows)
                        .when_some(self.footer, |d, f| d.child(f)),
                )
            });
        match self.width {
            Some(w) => {
                let outer = animate(
                    child_id(&self.id, "collapse"),
                    if self.collapsed { px(0.) } else { w },
                    Motion::PANEL,
                    window,
                    cx,
                );
                div()
                    .id(self.id)
                    .flex_none()
                    .h_full()
                    .w(outer)
                    .overflow_hidden()
                    .when(outer > px(0.5), |d| d.child(rail))
            }
            None => div()
                .id(self.id)
                .flex()
                .flex_1()
                .h_full()
                .min_w_0()
                .child(rail),
        }
    }
}

/// Web `SidebarTrigger`: a `secondary` icon button (`panel-left`) that toggles
/// the sidebar.
pub fn sidebar_trigger(
    id: impl Into<ElementId>,
    open: bool,
    on_toggle: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Button {
    Button::icon_only(id, IconName::PanelLeft)
        .variant(ButtonVariant::Secondary)
        .size(ControlSize::Small)
        .tooltip(if open {
            "Close sidebar"
        } else {
            "Open sidebar"
        })
        .on_click(move |_, w, cx| on_toggle(!open, w, cx))
}

#[derive(Default)]
struct PeekState {
    open: bool,
    hover_since: Option<Instant>,
    leave_since: Option<Instant>,
}

/// Web `SidebarPeek` (contained mode): while the docked sidebar is collapsed,
/// resting on the 12px left edge for `duration-medium` slides a floating
/// frosted copy in (`duration-overlay`, standard easing). Leaving both the
/// edge and the panel closes it after `duration-medium`. The panel has a pin
/// button (ghost `panel-left`) that re-docks the sidebar.
///
/// Put it in a `relative()` container over the main area; it renders the edge
/// strip and the floating panel as absolute children.
#[derive(IntoElement)]
pub struct SidebarPeek {
    id: ElementId,
    enabled: bool,
    width: Pixels,
    panel: Option<AnyElement>,
    on_pin: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl SidebarPeek {
    /// New peek layer, `width` = flyout width.
    pub fn new(id: impl Into<ElementId>, enabled: bool, width: Pixels) -> Self {
        Self {
            id: id.into(),
            enabled,
            width,
            panel: None,
            on_pin: None,
        }
    }

    /// The sidebar to show (use [`Sidebar::embedded`] + [`Sidebar::fill`]).
    pub fn panel(mut self, el: impl IntoElement) -> Self {
        self.panel = Some(el.into_any_element());
        self
    }

    /// Pin button handler (re-dock).
    pub fn on_pin(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_pin = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for SidebarPeek {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let edge_id = child_id(&self.id, "edge");
        let surface_id = child_id(&self.id, "surface");
        let (edge_state, edge_i) = use_interaction(&edge_id, window, cx);
        let (surface_state, surface_i) = use_interaction(&surface_id, window, cx);
        let state =
            window.use_keyed_state(child_id(&self.id, "peek"), cx, |_, _| PeekState::default());
        let delay = std::time::Duration::from_millis(duration::MEDIUM);
        let now = Instant::now();
        let enabled = self.enabled;
        let mut wait = false;
        state.update(cx, |s, _| {
            if !enabled {
                *s = PeekState::default();
                return;
            }
            if !s.open {
                s.leave_since = None;
                if edge_i.hovered {
                    let t = *s.hover_since.get_or_insert(now);
                    if now.duration_since(t) >= delay {
                        s.open = true;
                        s.hover_since = None;
                    } else {
                        wait = true;
                    }
                } else {
                    s.hover_since = None;
                }
            } else if edge_i.hovered || surface_i.hovered {
                s.leave_since = None;
            } else {
                let t = *s.leave_since.get_or_insert(now);
                if now.duration_since(t) >= delay {
                    s.open = false;
                    s.leave_since = None;
                } else {
                    wait = true;
                }
            }
        });
        if wait {
            window.request_animation_frame();
        }
        let open = state.read(cx).open;
        let gap = px(12.); // --refineui-sidebar-peek-float-gap
        let hidden_x = -(self.width + gap);
        let slide = Motion::new(
            duration::OVERLAY,
            refineui_foundation::motion::easing::STANDARD,
        );
        let x = animate(
            child_id(&self.id, "x"),
            if open { px(0.) } else { hidden_x },
            slide,
            window,
            cx,
        );
        // Web `box-shadow: elevation-16` → `elevation-24` while open,
        // transitioned over `duration-overlay` (standard): cross-fade the two.
        let lift = animate(
            child_id(&self.id, "lift"),
            if open { 1.0f32 } else { 0.0 },
            slide,
            window,
            cx,
        );
        let mut shadow: Vec<gpui::BoxShadow> = theme
            .shadow(Elevation::Shadow16)
            .into_iter()
            .map(|mut s| {
                s.color.a *= 1. - lift;
                s
            })
            .collect();
        shadow.extend(theme.shadow(Elevation::Shadow24).into_iter().map(|mut s| {
            s.color.a *= lift;
            s
        }));
        let peek = crate::material::backing(
            crate::SurfaceKind::Popover,
            px(radius::XX_LARGE),
            1.,
            theme.floating_fill(),
            Some(c.border_default),
            shadow,
            cx,
        );
        let visible = x > hidden_x + px(0.5);
        let pin = self.on_pin.clone();
        let close_state = state.clone();
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .when(enabled, |d| {
                d.child(
                    div()
                        .id(edge_id)
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(12.)) // foundation-size-120
                        .track_interaction(&edge_state),
                )
            })
            .when(enabled && visible, |d| {
                d.child(
                    div()
                        .id(surface_id)
                        .absolute()
                        .top(px(spacing::SMALL))
                        .bottom(px(spacing::SMALL))
                        .left(gap + x)
                        .w(self.width)
                        .flex()
                        .flex_col()
                        .rounded(px(radius::XX_LARGE))
                        .border_1()
                        .border_color(peek.border)
                        // Web: `surfaceFrosted` (55%) + `backdrop-filter:
                        // blur(surfaceFrost)`: the Popover material when
                        // materials are active, else the frost-over-page color
                        // (stock GPUI cannot blur what is behind).
                        .bg(peek.fill)
                        .shadow(peek.shadows)
                        .children(peek.layer)
                        .overflow_hidden()
                        .track_interaction(&surface_state)
                        .on_mouse_down_out(move |_, w, cx| {
                            close_state.update(cx, |s, _| s.open = false);
                            w.refresh();
                        })
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .justify_end()
                                .px(px(spacing::X_SMALL))
                                .pt(px(spacing::X_SMALL))
                                .child(
                                    Button::icon_only(
                                        child_id(&self.id, "pin"),
                                        IconName::PanelLeft,
                                    )
                                    .variant(ButtonVariant::Ghost)
                                    .size(ControlSize::Small)
                                    .tooltip("Pin sidebar open")
                                    .when_some(pin, |b, f| b.on_click(move |_, w, cx| f(w, cx))),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_h_0()
                                .w_full()
                                .children(self.panel),
                        ),
                )
            })
    }
}
