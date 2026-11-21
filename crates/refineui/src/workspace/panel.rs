//! Panel and Panel Group (brief §6 Composition).
//!
//! * [`Panel`] — a titled region: a `panel_header_height` header
//!   (Caption 2 semibold, `foregroundSecondary`, trailing actions) over its
//!   content. Collapsible panels get a chevron; the header is then one tab
//!   stop (Enter / Space toggle, ← collapse, → expand).
//! * [`PanelGroup`] — the panels of one dock. `Tabs` mode shows a compact
//!   tab strip (active = `foregroundPrimary` + 2px brand underline) and the
//!   active panel; `Stack` mode stacks collapsible Panels (Explorer
//!   style). Panel tabs / headers are draggable ([`PanelDrag`], `Grab`
//!   cursor, [`DragPreview`] chip); dropping on a strip shows a
//!   [`DropIndicator`] and reports the insertion slot. The strip is one
//!   tab stop: ← / → activate the neighbour.

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, FontWeight, IntoElement, KeyDownEvent, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{spacing, stroke, typography::role};

use super::layout::DockSide;
use super::{insertion_slot, is_dragging, mark_drag};
use crate::foundation::CursorRole;
use crate::motion::{
    TrackInteraction, child_id, focus_visible, probe, use_focus_handle, use_interaction,
    use_layout_probe,
};
use crate::{ActiveTheme, DragPreview, DropIndicator, Icon, IconName, IconSize, StyledExt};

/// Drag payload of a panel tab / header.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelDrag {
    /// Panel id.
    pub panel: String,
    /// Title (preview chip).
    pub title: SharedString,
    /// Icon (preview chip).
    pub icon: Option<IconName>,
    /// Dock it came from (`None` = floating / elsewhere).
    pub from: Option<DockSide>,
}

impl PanelDrag {
    fn preview(&self) -> DragPreview {
        let p = DragPreview::new(self.title.clone());
        match self.icon {
            Some(i) => p.icon(i),
            None => p,
        }
    }
}

/// Start a [`PanelDrag`] from `el` (marks it for edge drop zones).
pub(crate) fn panel_draggable(
    el: gpui::Stateful<gpui::Div>,
    drag: PanelDrag,
) -> gpui::Stateful<gpui::Div> {
    el.on_drag(drag, |d, _, _, cx| {
        let v = cx.new(|_| d.preview());
        mark_drag::<PanelDrag, _>(v, cx)
    })
}

type ToggleHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A titled region. See the module docs.
#[derive(IntoElement)]
pub struct Panel {
    id: ElementId,
    title: SharedString,
    icon: Option<IconName>,
    actions: Vec<AnyElement>,
    collapsible: bool,
    collapsed: Option<bool>,
    on_toggle: Option<ToggleHandler>,
    content: Vec<AnyElement>,
    drag: Option<PanelDrag>,
    fill: bool,
    default_collapsed: bool,
}

impl Panel {
    /// Panel titled `title`.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            actions: Vec::new(),
            collapsible: false,
            collapsed: None,
            on_toggle: None,
            content: Vec::new(),
            drag: None,
            fill: true,
            default_collapsed: false,
        }
    }

    /// Header icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Trailing header action (icon buttons).
    pub fn action(mut self, e: impl IntoElement) -> Self {
        self.actions.push(e.into_any_element());
        self
    }

    /// Chevron + click-to-collapse header.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Controlled collapsed state (uncontrolled when unset).
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = Some(collapsed);
        self
    }

    /// Initial state when uncontrolled.
    pub fn default_collapsed(mut self, collapsed: bool) -> Self {
        self.default_collapsed = collapsed;
        self
    }

    /// Collapse toggled (`true` = now collapsed).
    pub fn on_toggle(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }

    /// Content.
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content.push(e.into_any_element());
        self
    }

    /// Make the header draggable with this payload.
    pub fn draggable(mut self, drag: PanelDrag) -> Self {
        self.drag = Some(drag);
        self
    }

    /// Grow to fill the parent when expanded (default true).
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }
}

impl RenderOnce for Panel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let m = theme.desktop();
        let initial = self.default_collapsed;
        let own: Entity<bool> =
            window.use_keyed_state(child_id(&self.id, "collapsed"), cx, move |_, _| initial);
        let collapsed = self.collapsible && self.collapsed.unwrap_or(*own.read(cx));
        let header_id = child_id(&self.id, "header");
        let (hover, i) = use_interaction(&header_id, window, cx);
        let focus = use_focus_handle(&header_id, 0, window, cx);
        let ring = self.collapsible && focus.is_focused(window) && focus_visible(cx);
        let toggle = {
            let own = own.clone();
            let user = self.on_toggle.clone();
            Rc::new(move |next: bool, w: &mut Window, cx: &mut App| {
                own.update(cx, |s, cx| {
                    *s = next;
                    cx.notify();
                });
                if let Some(f) = &user {
                    f(next, w, cx);
                }
            })
        };

        let mut header = div()
            .id(header_id)
            .flex_none()
            .h(px(m.panel_header_height))
            .px(px(m.panel_padding))
            .flex()
            .items_center()
            .gap(px(spacing::X_SMALL))
            .text_style_token(role::CAPTION_MD)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c.foreground_secondary)
            .when(i.hovered && self.collapsible, |d| {
                d.bg(c.background_primary_hover)
            })
            .when(ring, |d| d.shadow(theme.focus_ring()))
            .track_interaction(&hover)
            .when(self.collapsible, |d| {
                d.child(
                    Icon::new(if collapsed {
                        IconName::ChevronRight
                    } else {
                        IconName::ChevronDown
                    })
                    .size(IconSize::XSmall)
                    .color(c.foreground_tertiary),
                )
            })
            .when_some(self.icon, |d, icon| {
                d.child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .color(c.foreground_secondary),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(spacing::XX_SMALL))
                    .children(self.actions),
            );
        if self.collapsible {
            let t1 = toggle.clone();
            let t2 = toggle.clone();
            header = header
                .track_focus(&focus)
                .cursor_action()
                .on_click(move |_, w, cx| t1(!collapsed, w, cx))
                .on_key_down(move |e: &KeyDownEvent, w, cx| {
                    let next = match e.keystroke.key.as_str() {
                        "enter" | "space" => !collapsed,
                        "left" => true,
                        "right" => false,
                        _ => return,
                    };
                    cx.stop_propagation();
                    if next != collapsed {
                        t2(next, w, cx);
                    }
                });
        }
        if let Some(drag) = self.drag {
            header = panel_draggable(header.cursor_role(CursorRole::Grab), drag);
        }

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .min_h_0()
            .when(self.fill && !collapsed, |d| d.flex_1())
            .when(!self.fill || collapsed, |d| d.flex_none())
            .child(header)
            .when(!collapsed, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .children(self.content),
                )
            })
    }
}

/// How a [`PanelGroup`] shows its panels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PanelGroupMode {
    /// Tab strip + active panel (bottom dock, inspector).
    #[default]
    Tabs,
    /// Collapsible panels stacked vertically (Explorer).
    Stack,
}

/// One panel of a [`PanelGroup`].
pub struct PanelEntry {
    id: String,
    title: SharedString,
    icon: Option<IconName>,
    actions: Vec<AnyElement>,
    content: Option<AnyElement>,
}

impl PanelEntry {
    /// Panel `id` titled `title`.
    pub fn new(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            actions: Vec::new(),
            content: None,
        }
    }

    /// Tab / header icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Header action (shown while the panel is active / expanded).
    pub fn action(mut self, e: impl IntoElement) -> Self {
        self.actions.push(e.into_any_element());
        self
    }

    /// Content.
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content = Some(e.into_any_element());
        self
    }
}

type IdHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type DropHandler = Rc<dyn Fn(&PanelDrag, usize, &mut Window, &mut App)>;

/// A dock's panels. See the module docs.
#[derive(IntoElement)]
pub struct PanelGroup {
    id: ElementId,
    mode: PanelGroupMode,
    side: Option<DockSide>,
    entries: Vec<PanelEntry>,
    active: Option<usize>,
    actions: Vec<AnyElement>,
    on_activate: Option<IdHandler>,
    on_drop: Option<DropHandler>,
}

impl PanelGroup {
    /// Empty group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            mode: PanelGroupMode::Tabs,
            side: None,
            entries: Vec::new(),
            active: None,
            actions: Vec::new(),
            on_activate: None,
            on_drop: None,
        }
    }

    /// Tabs or Stack.
    pub fn mode(mut self, mode: PanelGroupMode) -> Self {
        self.mode = mode;
        self
    }

    /// The dock it sits in (recorded in drags).
    pub fn side(mut self, side: DockSide) -> Self {
        self.side = Some(side);
        self
    }

    /// Add a panel.
    pub fn entry(mut self, e: PanelEntry) -> Self {
        self.entries.push(e);
        self
    }

    /// Active panel index (Tabs mode).
    pub fn active(mut self, ix: Option<usize>) -> Self {
        self.active = ix;
        self
    }

    /// Group-level trailing action (zoom, hide — added by [`super::Dock`]).
    pub fn action(mut self, e: impl IntoElement) -> Self {
        self.actions.push(e.into_any_element());
        self
    }

    /// A panel tab was activated.
    pub fn on_activate(mut self, f: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_activate = Some(Rc::new(f));
        self
    }

    /// A panel was dropped at insertion slot `usize` (as shown).
    pub fn on_drop(
        mut self,
        f: impl Fn(&PanelDrag, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drop = Some(Rc::new(f));
        self
    }

    /// Panel count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// No panels.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl RenderOnce for PanelGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let m = theme.desktop();
        let n = self.entries.len();
        let active = self.active.filter(|a| *a < n).or((n > 0).then_some(0));
        let measure = use_layout_probe(&self.id, window, cx);
        let slot: Entity<Option<usize>> =
            window.use_keyed_state(child_id(&self.id, "drop-slot"), cx, |_, _| None);
        let dragging = is_dragging::<PanelDrag>(cx);
        let shown_slot = if dragging { *slot.read(cx) } else { None };
        let vertical = self.mode == PanelGroupMode::Stack;
        let side = self.side;

        // Pointer → insertion slot (strip items measured by the probe).
        let track = {
            let measure = measure.clone();
            let slot = slot.clone();
            move |e: &gpui::DragMoveEvent<PanelDrag>, _: &mut Window, cx: &mut App| {
                let inside = e.bounds.contains(&e.event.position);
                let next = inside.then(|| {
                    let p = measure.read(cx);
                    let items: Vec<(f32, f32)> = p
                        .items
                        .iter()
                        .flatten()
                        .map(|b| {
                            if vertical {
                                (f32::from(b.top()), f32::from(b.size.height))
                            } else {
                                (f32::from(b.left()), f32::from(b.size.width))
                            }
                        })
                        .collect();
                    let pos = if vertical {
                        e.event.position.y
                    } else {
                        e.event.position.x
                    };
                    insertion_slot(f32::from(pos), &items)
                });
                slot.update(cx, |s, cx| {
                    if *s != next {
                        *s = next;
                        cx.notify();
                    }
                });
            }
        };
        let drop = {
            let slot = slot.clone();
            let on_drop = self.on_drop.clone();
            move |d: &PanelDrag, w: &mut Window, cx: &mut App| {
                let at = slot.update(cx, |s, _| s.take()).unwrap_or(n);
                if let Some(f) = &on_drop {
                    f(d, at, w, cx);
                }
            }
        };
        let indicator_at = |ix: usize, cx: &App| -> Option<f32> {
            let p = measure.read(cx);
            let container = p.container?;
            let item = |i: usize| p.items.get(i).copied().flatten();
            if ix < n {
                item(ix).map(|b| {
                    if vertical {
                        f32::from(b.top() - container.top())
                    } else {
                        f32::from(b.left() - container.left())
                    }
                })
            } else {
                item(n.checked_sub(1)?).map(|b| {
                    if vertical {
                        f32::from(b.bottom() - container.top())
                    } else {
                        f32::from(b.right() - container.left())
                    }
                })
            }
        };
        let indicator = shown_slot.and_then(|s| indicator_at(s, cx)).map(|x| {
            if vertical {
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(x - 3.))
                    .child(DropIndicator::between_rows())
                    .into_any_element()
            } else {
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(x - 3.))
                    .child(DropIndicator::between_tabs())
                    .into_any_element()
            }
        });

        if n == 0 {
            return div()
                .id(self.id)
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .child(probe(&measure, None))
                .on_drag_move::<PanelDrag>(track)
                .on_drop::<PanelDrag>(drop)
                .child(
                    div()
                        .flex_none()
                        .h(px(m.panel_header_height))
                        .flex()
                        .justify_end()
                        .items_center()
                        .px(px(spacing::XX_SMALL))
                        .children(self.actions),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .p(px(spacing::MEDIUM))
                        .text_style_token(role::BODY_SM)
                        .text_color(c.foreground_tertiary)
                        .child("Drag a panel here"),
                )
                .into_any_element();
        }

        match self.mode {
            PanelGroupMode::Tabs => {
                let focus = use_focus_handle(&child_id(&self.id, "strip"), 0, window, cx);
                let ring = focus.is_focused(window) && focus_visible(cx);
                let ids: Vec<String> = self.entries.iter().map(|e| e.id.clone()).collect();
                let on_key = self.on_activate.clone();
                let mut tabs: Vec<AnyElement> = Vec::new();
                let mut body = None;
                let mut panel_actions = Vec::new();
                for (ix, e) in self.entries.into_iter().enumerate() {
                    let is_active = Some(ix) == active;
                    let tab_id = ElementId::NamedChild(
                        Box::new(self.id.clone()),
                        format!("tab:{}", e.id).into(),
                    );
                    let (st, hi) = use_interaction(&tab_id, window, cx);
                    let on_act = self.on_activate.clone();
                    let pid = e.id.clone();
                    let fg = if is_active || hi.hovered {
                        c.foreground_primary
                    } else {
                        c.foreground_secondary
                    };
                    let tab = div()
                        .id(tab_id)
                        .relative()
                        .flex_none()
                        .h_full()
                        .px(px(spacing::SMALL))
                        .flex()
                        .items_center()
                        .gap(px(spacing::XX_SMALL))
                        .text_style_token(role::CAPTION_MD)
                        .font_weight(if is_active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(fg)
                        .cursor_role(CursorRole::Action)
                        .track_interaction(&st)
                        .child(probe(&measure, Some(ix)))
                        .when_some(e.icon, |d, icon| {
                            d.child(Icon::new(icon).size(IconSize::XSmall).color(fg))
                        })
                        .child(e.title.clone())
                        .when(is_active, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .left(px(spacing::SMALL))
                                    .right(px(spacing::SMALL))
                                    .bottom_0()
                                    .h(px(stroke::THICK))
                                    .rounded_full()
                                    .bg(c.background_brand),
                            )
                        })
                        .on_click(move |_, w, cx| {
                            if let Some(f) = &on_act {
                                f(&pid, w, cx);
                            }
                        });
                    let tab = panel_draggable(
                        tab,
                        PanelDrag {
                            panel: e.id.clone(),
                            title: e.title.clone(),
                            icon: e.icon,
                            from: side,
                        },
                    );
                    tabs.push(tab.into_any_element());
                    if is_active {
                        body = e.content;
                        panel_actions = e.actions;
                    }
                }
                div()
                    .id(self.id.clone())
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id(child_id(&self.id, "strip"))
                            .relative()
                            .flex_none()
                            .h(px(m.panel_header_height))
                            .flex()
                            .items_center()
                            .px(px(spacing::XX_SMALL))
                            .border_b(px(stroke::HAIRLINE))
                            .border_color(c.border_subtle)
                            .track_focus(&focus)
                            .when(ring, |d| d.shadow(theme.focus_ring()))
                            .on_key_down(move |ev: &KeyDownEvent, w, cx| {
                                let Some(a) = active else { return };
                                let next = match ev.keystroke.key.as_str() {
                                    "left" => a.checked_sub(1),
                                    "right" => (a + 1 < ids.len()).then_some(a + 1),
                                    "home" => Some(0),
                                    "end" => ids.len().checked_sub(1),
                                    _ => return,
                                };
                                cx.stop_propagation();
                                if let (Some(nx), Some(f)) = (next, &on_key) {
                                    f(&ids[nx], w, cx);
                                }
                            })
                            .on_drag_move::<PanelDrag>(track)
                            .on_drop::<PanelDrag>(drop)
                            .child(probe(&measure, None))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .flex()
                                    .overflow_hidden()
                                    .children(tabs),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::XX_SMALL))
                                    .children(panel_actions)
                                    .children(self.actions),
                            )
                            .children(indicator),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .children(body),
                    )
                    .into_any_element()
            }
            PanelGroupMode::Stack => {
                let collapsed: Entity<HashSet<String>> =
                    window.use_keyed_state(child_id(&self.id, "collapsed"), cx, |_, _| {
                        HashSet::new()
                    });
                let set = collapsed.read(cx).clone();
                let mut group_actions = Some(self.actions);
                let mut first = true;
                let panels: Vec<AnyElement> = self
                    .entries
                    .into_iter()
                    .enumerate()
                    .map(|(ix, e)| {
                        let pid = e.id.clone();
                        let state = collapsed.clone();
                        let mut p = Panel::new(
                            ElementId::NamedChild(
                                Box::new(self.id.clone()),
                                format!("panel:{}", e.id).into(),
                            ),
                            e.title.clone(),
                        )
                        .collapsible(true)
                        .collapsed(set.contains(&e.id))
                        .on_toggle(move |now, _, cx| {
                            state.update(cx, |s, cx| {
                                if now {
                                    s.insert(pid.clone());
                                } else {
                                    s.remove(&pid);
                                }
                                cx.notify();
                            })
                        })
                        .draggable(PanelDrag {
                            panel: e.id.clone(),
                            title: e.title.clone(),
                            icon: e.icon,
                            from: side,
                        });
                        for a in e.actions {
                            p = p.action(a);
                        }
                        // Dock actions (zoom, hide) sit in the first header.
                        for a in group_actions.take().unwrap_or_default() {
                            p = p.action(a);
                        }
                        if let Some(content) = e.content {
                            p = p.child(content);
                        }
                        let wrap = div()
                            .relative()
                            .flex()
                            .flex_col()
                            .min_h_0()
                            .when(!set.contains(&e.id), |d| d.flex_1())
                            .when(!first, |d| {
                                d.border_t(px(stroke::HAIRLINE))
                                    .border_color(c.border_subtle)
                            })
                            .child(probe(&measure, Some(ix)))
                            .child(p);
                        first = false;
                        wrap.into_any_element()
                    })
                    .collect();
                div()
                    .id(self.id.clone())
                    .relative()
                    .size_full()
                    .flex()
                    .flex_col()
                    .on_drag_move::<PanelDrag>(track)
                    .on_drop::<PanelDrag>(drop)
                    .child(probe(&measure, None))
                    .children(panels)
                    .children(indicator)
                    .into_any_element()
            }
        }
    }
}
