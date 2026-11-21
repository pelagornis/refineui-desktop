//! SortableList — complete drag and drop for lists (brief §4 "Drag and
//! Drop"): reorder, drag between lists, drop indicators, a keyboard
//! alternative, Escape to cancel and auto-scroll.
//!
//! * **Pointer** — drag a row; a [`DropIndicator`] shows the insertion slot
//!   (half-row hit zones, [`crate::interaction::dnd::drop_edge`]). Dropping
//!   on another list of the same `group` moves the item there. A drag
//!   preview ([`crate::DragPreview`]) follows the pointer.
//! * **Escape** cancels a pointer drag (the list takes focus on press).
//! * **Auto-scroll** — while dragging within [`AUTO_SCROLL_EDGE`] px of the
//!   list's top / bottom edge the list scrolls, faster closer to the edge.
//! * **Keyboard** — with the list focused: ↑ / ↓ move the cursor; Space
//!   lifts the item; while lifted ↑ / ↓ move the drop slot, ← / → move it to
//!   the previous / next list ([`SortableList::neighbors`]), Enter / Space
//!   drop and Escape cancels. Alt+↑ / Alt+↓ move the item immediately.
//!
//! The list is controlled: it reports a [`SortableMove`] and the owner
//! updates its data ([`SortableMove::apply`] does it for `Vec`s).

use std::rc::Rc;

use gpui::{
    App, Context, DragMoveEvent, ElementId, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Pixels, Render, RenderOnce, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, point, prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::interaction::dnd::{DropEdge, drop_edge};
use crate::motion::{child_id, focus_visible, probe, use_layout_probe};
use crate::{ActiveTheme, DragPreview, DropIndicator, Icon, IconName, IconSize, StyledExt};

/// Distance from an edge where dragging auto-scrolls.
pub const AUTO_SCROLL_EDGE: f32 = 32.;
/// Maximum auto-scroll step per pointer move.
pub const AUTO_SCROLL_STEP: f32 = 14.;

/// Drag payload shared by every [`SortableList`].
#[derive(Clone, Debug)]
pub struct SortableDrag {
    /// Lists only accept drags from their own group.
    pub group: SharedString,
    /// Source list key.
    pub list: SharedString,
    /// Source index.
    pub index: usize,
    /// Label for the preview.
    pub label: SharedString,
    /// Icon for the preview.
    pub icon: Option<IconName>,
}

impl Render for SortableDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut p = DragPreview::new(self.label.clone());
        if let Some(i) = self.icon {
            p = p.icon(i);
        }
        p.element(cx)
    }
}

/// A move reported by a [`SortableList`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortableMove {
    /// Within one list; `to` is the final index after removal.
    Reorder {
        /// List key.
        list: SharedString,
        /// Old index.
        from: usize,
        /// New index.
        to: usize,
    },
    /// From one list to another; `to` is the insertion index in `to_list`.
    Transfer {
        /// Source list.
        from_list: SharedString,
        /// Source index.
        from: usize,
        /// Target list.
        to_list: SharedString,
        /// Insertion index.
        to: usize,
    },
}

impl SortableMove {
    /// Applies the move to `lists` (looked up by key).
    pub fn apply<T>(&self, lists: &mut [(SharedString, Vec<T>)]) {
        match self {
            SortableMove::Reorder { list, from, to } => {
                if let Some((_, v)) = lists.iter_mut().find(|(k, _)| k == list)
                    && *from < v.len()
                {
                    let it = v.remove(*from);
                    let to = (*to).min(v.len());
                    v.insert(to, it);
                }
            }
            SortableMove::Transfer {
                from_list,
                from,
                to_list,
                to,
            } => {
                let item = lists
                    .iter_mut()
                    .find(|(k, _)| k == from_list)
                    .filter(|(_, v)| *from < v.len())
                    .map(|(_, v)| v.remove(*from));
                if let Some(item) = item
                    && let Some((_, v)) = lists.iter_mut().find(|(k, _)| k == to_list)
                {
                    let to = (*to).min(v.len());
                    v.insert(to, item);
                }
            }
        }
    }
}

/// Resolves a drop at insertion slot `slot` (0..=len of the target list).
pub fn resolve_drop(drag: &SortableDrag, list: &SharedString, slot: usize) -> Option<SortableMove> {
    if &drag.list == list {
        let to = if slot > drag.index { slot - 1 } else { slot };
        (to != drag.index).then(|| SortableMove::Reorder {
            list: list.clone(),
            from: drag.index,
            to,
        })
    } else {
        Some(SortableMove::Transfer {
            from_list: drag.list.clone(),
            from: drag.index,
            to_list: list.clone(),
            to: slot,
        })
    }
}

/// Auto-scroll delta for a pointer at `y` in a viewport `[top, bottom]`
/// (negative scrolls up).
pub fn auto_scroll_delta(y: f32, top: f32, bottom: f32) -> f32 {
    if y < top + AUTO_SCROLL_EDGE {
        -AUTO_SCROLL_STEP * (1. - ((y - top).max(0.) / AUTO_SCROLL_EDGE))
    } else if y > bottom - AUTO_SCROLL_EDGE {
        AUTO_SCROLL_STEP * (1. - ((bottom - y).max(0.) / AUTO_SCROLL_EDGE))
    } else {
        0.
    }
}

/// One row.
#[derive(Clone)]
pub struct SortableItem {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    detail: Option<SharedString>,
}

impl SortableItem {
    /// Row `id` showing `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            detail: None,
        }
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Trailing detail text.
    pub fn detail(mut self, d: impl Into<SharedString>) -> Self {
        self.detail = Some(d.into());
        self
    }
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
struct ListState {
    /// Pointer drop slot while a drag hovers this list.
    slot: Option<usize>,
    /// Keyboard cursor.
    cursor: usize,
    /// Keyboard-lifted item and its target slot.
    lifted: Option<(usize, usize)>,
}

type MoveHandler = Rc<dyn Fn(&SortableMove, &mut Window, &mut App)>;

/// A reorderable list. See the module docs.
#[derive(IntoElement)]
pub struct SortableList {
    id: ElementId,
    list: SharedString,
    group: SharedString,
    items: Vec<SortableItem>,
    on_move: Option<MoveHandler>,
    max_height: Option<Pixels>,
    neighbors: (Option<SharedString>, Option<SharedString>),
}

impl SortableList {
    /// List `list` (its key in [`SortableMove`]s), accepting drags from
    /// lists of the same `group`.
    pub fn new(
        id: impl Into<ElementId>,
        list: impl Into<SharedString>,
        group: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            list: list.into(),
            group: group.into(),
            items: Vec::new(),
            on_move: None,
            max_height: None,
            neighbors: (None, None),
        }
    }

    /// Rows.
    pub fn items(mut self, items: impl IntoIterator<Item = SortableItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Move handler.
    pub fn on_move(mut self, f: impl Fn(&SortableMove, &mut Window, &mut App) + 'static) -> Self {
        self.on_move = Some(Rc::new(f));
        self
    }

    /// Scroll beyond this height (auto-scroll applies).
    pub fn max_height(mut self, h: Pixels) -> Self {
        self.max_height = Some(h);
        self
    }

    /// Previous / next list keys for keyboard transfers (← / →).
    pub fn neighbors(mut self, prev: Option<SharedString>, next: Option<SharedString>) -> Self {
        self.neighbors = (prev, next);
        self
    }
}

impl RenderOnce for SortableList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let n = self.items.len();
        let state =
            window.use_keyed_state(child_id(&self.id, "state"), cx, |_, _| ListState::default());
        let focus: FocusHandle = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let scroll = window
            .use_keyed_state(child_id(&self.id, "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let host = use_layout_probe(&child_id(&self.id, "rows"), window, cx);
        // A finished / cancelled pointer drag leaves no stale slot behind.
        if !cx.has_active_drag() && state.read(cx).slot.is_some() {
            state.update(cx, |s, _| s.slot = None);
        }
        let st = *state.read(cx);
        let focused = focus.is_focused(window);
        let ring = focused && focus_visible(cx);
        let indicator_slot = st.slot.or(st.lifted.map(|(_, t)| t));
        let items = Rc::new(self.items);

        let mut rows = Vec::new();
        for (ix, item) in items.iter().enumerate() {
            let payload = SortableDrag {
                group: self.group.clone(),
                list: self.list.clone(),
                index: ix,
                label: item.label.clone(),
                icon: item.icon,
            };
            let lifted = st.lifted.map(|(i, _)| i) == Some(ix);
            let cursor = ring && st.cursor == ix && st.lifted.is_none();
            let f = focus.clone();
            let press_state = state.clone();
            rows.push(
                div()
                    .id(ElementId::NamedChild(
                        Box::new(child_id(&self.id, "row")),
                        item.id.clone(),
                    ))
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(spacing::X_SMALL))
                    .px(px(spacing::X_SMALL))
                    .py(px(spacing::XX_SMALL + theme.row_inset()))
                    .rounded(px(radius::LARGE))
                    .text_style_token(role::BODY_MD)
                    .text_color(c.foreground_primary)
                    .bg(if lifted {
                        c.background_brand_subtle
                    } else {
                        c.background_primary
                    })
                    .border_1()
                    .border_color(if lifted {
                        c.border_focus
                    } else if cursor {
                        c.border_focus.opacity(0.6)
                    } else {
                        c.border_subtle
                    })
                    .hover(|s| s.bg(c.background_primary_hover))
                    .cursor_grab()
                    .child(probe(&host, Some(ix)))
                    .child(
                        Icon::new(IconName::Drag)
                            .size(IconSize::XSmall)
                            .color(c.foreground_tertiary),
                    )
                    .when_some(item.icon, |d, i| {
                        d.child(
                            Icon::new(i)
                                .size(IconSize::XSmall)
                                .color(c.foreground_secondary),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(item.label.clone()),
                    )
                    .when_some(item.detail.clone(), |d, t| {
                        d.child(
                            div()
                                .text_style_token(role::CAPTION_LG)
                                .text_color(c.foreground_tertiary)
                                .child(t),
                        )
                    })
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        window.focus(&f);
                        press_state.update(cx, |s, cx| {
                            s.cursor = ix;
                            s.lifted = None;
                            cx.notify();
                        });
                    })
                    .on_drag(payload, |d: &SortableDrag, _, _, cx| cx.new(|_| d.clone())),
            );
        }

        // Insertion line at `indicator_slot`, positioned from row bounds.
        let indicator = indicator_slot.and_then(|slot| {
            let probe = host.read(cx);
            let y = if n == 0 {
                Some(0.)
            } else if slot < n {
                probe
                    .relative(slot)
                    .map(|b| f32::from(b.top()) - spacing::XXX_SMALL - 3.)
            } else {
                probe
                    .relative(n - 1)
                    .map(|b| f32::from(b.bottom()) + spacing::XXX_SMALL - 3.)
            }?;
            Some(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(y))
                    .child(DropIndicator::between_rows()),
            )
        });

        let group = self.group.clone();
        let list = self.list.clone();
        let move_state = state.clone();
        let drop_state = state.clone();
        let drop_list = self.list.clone();
        let drop_move = self.on_move.clone();
        let key_move = self.on_move.clone();
        let key_state = state.clone();
        let key_list = self.list.clone();
        let neighbors = self.neighbors.clone();
        let host_move = host.clone();
        let scroll_move = scroll.clone();
        let accept_group = self.group.clone();

        div()
            .id(self.id.clone())
            .track_focus(&focus)
            .relative()
            .w_full()
            .rounded(px(radius::X_LARGE))
            .bg(c.background_surface)
            .border_1()
            .border_color(if st.slot.is_some() {
                c.border_focus
            } else {
                c.border_subtle
            })
            .p(px(spacing::XX_SMALL))
            .child(
                div()
                    .id(child_id(&self.id, "viewport"))
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XXX_SMALL * 2.))
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .when_some(self.max_height, |d, h| d.max_h(h))
                    .min_h(px(40.))
                    .child(probe(&host, None))
                    .children(rows)
                    .when(n == 0, |d| {
                        d.child(
                            div()
                                .py(px(spacing::SMALL))
                                .text_center()
                                .text_style_token(role::CAPTION_LG)
                                .text_color(c.foreground_tertiary)
                                .child("Drop items here"),
                        )
                    })
                    .children(indicator),
            )
            .can_drop(move |v, _, _| {
                v.downcast_ref::<SortableDrag>()
                    .is_some_and(|d| d.group == accept_group)
            })
            .on_drag_move(move |e: &DragMoveEvent<SortableDrag>, window, cx| {
                let drag = e.drag(cx).clone();
                let pos = e.event.position;
                if drag.group != group || !e.bounds.contains(&pos) {
                    if move_state.read(cx).slot.is_some() && !e.bounds.contains(&pos) {
                        move_state.update(cx, |s, cx| {
                            s.slot = None;
                            cx.notify();
                        });
                    }
                    return;
                }
                // Slot from the hovered row's half (drop_edge).
                let probe = host_move.read(cx);
                let mut slot = n;
                for i in 0..n {
                    if let Some(b) = probe.items.get(i).copied().flatten()
                        && pos.y < b.bottom()
                    {
                        let edge =
                            drop_edge(f32::from(pos.y - b.top()), f32::from(b.size.height), false);
                        slot = if edge == DropEdge::Before { i } else { i + 1 };
                        break;
                    }
                }
                let _ = &list;
                // Auto-scroll near the viewport edges.
                let vb = scroll_move.bounds();
                let dy = auto_scroll_delta(
                    f32::from(pos.y),
                    f32::from(vb.top()),
                    f32::from(vb.bottom()),
                );
                if dy != 0. {
                    let o = scroll_move.offset();
                    let max = scroll_move.max_offset().height;
                    let y = (o.y - px(dy)).clamp(-max, px(0.));
                    scroll_move.set_offset(point(o.x, y));
                    window.refresh();
                }
                if move_state.read(cx).slot != Some(slot) {
                    move_state.update(cx, |s, cx| {
                        s.slot = Some(slot);
                        cx.notify();
                    });
                }
            })
            .on_drop(move |drag: &SortableDrag, window, cx| {
                let slot = drop_state.read(cx).slot.unwrap_or(n);
                drop_state.update(cx, |s, cx| {
                    s.slot = None;
                    cx.notify();
                });
                if let (Some(m), Some(h)) = (resolve_drop(drag, &drop_list, slot), &drop_move) {
                    h(&m, window, cx);
                }
            })
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                let k = e.keystroke.key.as_str();
                let alt = e.keystroke.modifiers.alt;
                let s = *key_state.read(cx);
                let emit = |m: SortableMove, window: &mut Window, cx: &mut App| {
                    if let Some(h) = &key_move {
                        h(&m, window, cx);
                    }
                };
                if k == "escape" {
                    // Cancel a pointer drag or a keyboard lift.
                    let had = cx.stop_active_drag(window) || s.lifted.is_some();
                    key_state.update(cx, |s, cx| {
                        s.slot = None;
                        s.lifted = None;
                        cx.notify();
                    });
                    if had {
                        cx.stop_propagation();
                    }
                    return;
                }
                if n == 0 {
                    return;
                }
                let mut next = s;
                match (k, s.lifted, alt) {
                    ("up", None, true) | ("down", None, true) => {
                        let to = if k == "up" {
                            s.cursor.saturating_sub(1)
                        } else {
                            (s.cursor + 1).min(n - 1)
                        };
                        if to != s.cursor {
                            emit(
                                SortableMove::Reorder {
                                    list: key_list.clone(),
                                    from: s.cursor,
                                    to,
                                },
                                window,
                                cx,
                            );
                            next.cursor = to;
                        }
                    }
                    ("up", None, false) => next.cursor = s.cursor.saturating_sub(1),
                    ("down", None, false) => next.cursor = (s.cursor + 1).min(n - 1),
                    ("space", None, _) => next.lifted = Some((s.cursor, s.cursor)),
                    ("up", Some((i, t)), _) => next.lifted = Some((i, t.saturating_sub(1))),
                    ("down", Some((i, t)), _) => next.lifted = Some((i, (t + 1).min(n))),
                    ("left", Some((i, _)), _) | ("right", Some((i, _)), _) => {
                        let target = if k == "left" {
                            neighbors.0.clone()
                        } else {
                            neighbors.1.clone()
                        };
                        if let Some(to_list) = target {
                            emit(
                                SortableMove::Transfer {
                                    from_list: key_list.clone(),
                                    from: i,
                                    to_list,
                                    to: 0,
                                },
                                window,
                                cx,
                            );
                            next.lifted = None;
                            next.cursor = i.min(n.saturating_sub(2));
                        }
                    }
                    ("enter", Some((i, t)), _) | ("space", Some((i, t)), _) => {
                        let d = SortableDrag {
                            group: SharedString::default(),
                            list: key_list.clone(),
                            index: i,
                            label: SharedString::default(),
                            icon: None,
                        };
                        if let Some(m) = resolve_drop(&d, &key_list, t) {
                            next.cursor = match &m {
                                SortableMove::Reorder { to, .. } => *to,
                                _ => s.cursor,
                            };
                            emit(m, window, cx);
                        }
                        next.lifted = None;
                    }
                    _ => return,
                }
                cx.stop_propagation();
                key_state.update(cx, |st, cx| {
                    *st = next;
                    cx.notify();
                });
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag(list: &str, index: usize) -> SortableDrag {
        SortableDrag {
            group: "g".into(),
            list: list.to_string().into(),
            index,
            label: "x".into(),
            icon: None,
        }
    }

    #[test]
    fn resolves_reorders_and_transfers() {
        let a: SharedString = "a".into();
        assert_eq!(
            resolve_drop(&drag("a", 0), &a, 3),
            Some(SortableMove::Reorder {
                list: a.clone(),
                from: 0,
                to: 2
            })
        );
        assert_eq!(resolve_drop(&drag("a", 1), &a, 1), None);
        assert_eq!(resolve_drop(&drag("a", 1), &a, 2), None);
        assert!(matches!(
            resolve_drop(&drag("b", 1), &a, 0),
            Some(SortableMove::Transfer { to: 0, .. })
        ));
    }

    #[test]
    fn applies_moves() {
        let mut lists: Vec<(SharedString, Vec<i32>)> =
            vec![("a".into(), vec![1, 2, 3]), ("b".into(), vec![9])];
        SortableMove::Reorder {
            list: "a".into(),
            from: 0,
            to: 2,
        }
        .apply(&mut lists);
        assert_eq!(lists[0].1, vec![2, 3, 1]);
        SortableMove::Transfer {
            from_list: "a".into(),
            from: 0,
            to_list: "b".into(),
            to: 1,
        }
        .apply(&mut lists);
        assert_eq!(
            (lists[0].1.clone(), lists[1].1.clone()),
            (vec![3, 1], vec![9, 2])
        );
    }

    #[test]
    fn auto_scroll_ramps_near_edges() {
        assert_eq!(auto_scroll_delta(100., 0., 200.), 0.);
        assert!(auto_scroll_delta(2., 0., 200.) < -10.);
        assert!(auto_scroll_delta(198., 0., 200.) > 10.);
    }
}
