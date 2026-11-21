//! SelectionArea — a canvas of items with a rubber-band **selection box**
//! (marquee), click / ⌘-click / Shift-click selection and select-all
//! (brief §5 Interaction: Selection Box).
//!
//! * Press on empty space and drag: a marquee (`backgroundBrandSubtle` at
//!   60% with a hairline `borderFocus` outline) selects every item it
//!   touches, live. Holding Shift or ⌘/Ctrl adds to the selection made
//!   before the drag. A press without a drag (under 4px) clears it.
//! * Items: click selects, ⌘/Ctrl-click toggles, Shift-click extends from
//!   the anchor ([`crate::interaction::selection::SelectionModel`]).
//!   Selected items sit on a `backgroundBrandSubtle` plate with a
//!   `borderFocus` hairline.
//! * Keyboard (when focused): ⌘/Ctrl-A selects all, Escape clears,
//!   ←/→/↑/↓ move the cursor (Shift extends).
//!
//! The hit test is [`crate::interaction::selection::marquee_hits`] over the
//! items' laid-out bounds, so it works for any layout
//! ([`SelectionLayout::Wrap`] grid of tiles or a [`SelectionLayout::Column`]
//! list).

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, Point, RenderOnce,
    SharedString, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke};

use crate::ActiveTheme;
use crate::interaction::selection::{
    Rect, SelectGesture, SelectionMode, SelectionModel, marquee_hits,
};
use crate::motion::{LayoutProbe, child_id, focus_visible, probe, use_layout_probe};

/// Drag distance (px) below which a press is a click.
pub const MARQUEE_THRESHOLD: f32 = 4.;

/// How items are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionLayout {
    /// Wrapping rows of tiles (icon view).
    #[default]
    Wrap,
    /// One item per row (list view).
    Column,
}

type ChangeHandler = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

struct MarqueeState {
    model: SelectionModel<SharedString>,
    start: Option<Point<Pixels>>,
    current: Point<Pixels>,
    base: Vec<SharedString>,
    additive: bool,
    initialized: bool,
}

/// Selectable item canvas with a marquee. See the module docs.
#[derive(IntoElement)]
pub struct SelectionArea {
    id: ElementId,
    items: Vec<(SharedString, AnyElement)>,
    layout: SelectionLayout,
    initial: Vec<SharedString>,
    on_change: Option<ChangeHandler>,
}

impl SelectionArea {
    /// Empty area.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            layout: SelectionLayout::Wrap,
            initial: Vec::new(),
            on_change: None,
        }
    }

    /// Add an item with a unique key.
    pub fn item(mut self, key: impl Into<SharedString>, element: impl IntoElement) -> Self {
        self.items.push((key.into(), element.into_any_element()));
        self
    }

    /// Item layout.
    pub fn layout(mut self, layout: SelectionLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Selection on first render.
    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.initial = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Called whenever the selection changes (live while dragging).
    pub fn on_change(
        mut self,
        f: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

fn rect_of(b: Bounds<Pixels>) -> Rect {
    Rect::new(
        f32::from(b.left()),
        f32::from(b.top()),
        f32::from(b.size.width),
        f32::from(b.size.height),
    )
}

fn gesture(m: &gpui::Modifiers) -> SelectGesture {
    SelectGesture {
        toggle: m.secondary(),
        extend: m.shift,
    }
}

fn hits(probe: &LayoutProbe, a: Point<Pixels>, b: Point<Pixels>, n: usize) -> Vec<usize> {
    let rects: Vec<Rect> = (0..n)
        .map(|i| {
            probe
                .items
                .get(i)
                .copied()
                .flatten()
                .map(rect_of)
                .unwrap_or_default()
        })
        .collect();
    marquee_hits(
        Rect::from_points((a.x.into(), a.y.into()), (b.x.into(), b.y.into())),
        &rects,
        MARQUEE_THRESHOLD,
    )
}

impl RenderOnce for SelectionArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let initial = self.initial.clone();
        let state =
            window.use_keyed_state(child_id(&self.id, "marquee"), cx, |_, _| MarqueeState {
                model: SelectionModel::new(SelectionMode::Multiple),
                start: None,
                current: Point::default(),
                base: Vec::new(),
                additive: false,
                initialized: false,
            });
        if !state.read(cx).initialized {
            state.update(cx, |s, _| {
                s.model.set(initial);
                s.initialized = true;
            });
        }
        let focus: FocusHandle = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focused = focus.is_focused(window);
        let host = use_layout_probe(&self.id, window, cx);
        let keys: Rc<Vec<SharedString>> =
            Rc::new(self.items.iter().map(|(k, _)| k.clone()).collect());
        let n = keys.len();
        let snapshot = state.read(cx);
        let selected: Vec<SharedString> = snapshot.model.selected().to_vec();
        let cursor = snapshot.model.cursor().cloned();
        let marquee = snapshot.start.map(|s| (s, snapshot.current));
        let container = host.read(cx).container;
        let on_change = self.on_change.clone();
        let notify = {
            let on_change = on_change.clone();
            move |st: &gpui::Entity<MarqueeState>, window: &mut Window, cx: &mut App| {
                let sel = st.read(cx).model.selected().to_vec();
                if let Some(h) = &on_change {
                    h(&sel, window, cx);
                }
            }
        };
        let notify = Rc::new(notify);

        let items = self.items.into_iter().enumerate().map(|(ix, (key, el))| {
            let is_sel = selected.contains(&key);
            let is_cursor = focused && focus_visible(cx) && cursor.as_ref() == Some(&key);
            let st = state.clone();
            let order = keys.clone();
            let notify = notify.clone();
            let focus = focus.clone();
            div()
                .id(ElementId::NamedChild(
                    Box::new(self.id.clone()),
                    format!("item-{key}").into(),
                ))
                .relative()
                .when(self.layout == SelectionLayout::Column, |d| d.w_full())
                .rounded(px(radius::LARGE))
                .border(px(stroke::HAIRLINE))
                .border_color(if is_sel {
                    c.border_focus
                } else {
                    gpui::transparent_black()
                })
                .when(is_sel, |d| d.bg(c.background_brand_subtle))
                .when(is_cursor, |d| d.shadow(theme.focus_ring()))
                .child(probe(&host, Some(ix)))
                .child(el)
                // Presses on an item select it; they never start a marquee.
                .on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    window.focus(&focus);
                    let g = gesture(&e.modifiers);
                    st.update(cx, |s, cx| {
                        s.model.click(key.clone(), g, &order);
                        cx.notify();
                    });
                    notify(&st, window, cx);
                })
        });

        let marquee_el = marquee.zip(container).map(|((a, b), origin)| {
            let r = Rect::from_points(
                (
                    f32::from(a.x - origin.left()),
                    f32::from(a.y - origin.top()),
                ),
                (
                    f32::from(b.x - origin.left()),
                    f32::from(b.y - origin.top()),
                ),
            );
            div()
                .absolute()
                .left(px(r.x))
                .top(px(r.y))
                .w(px(r.w))
                .h(px(r.h))
                .bg(c.background_brand_subtle.opacity(0.6))
                .border(px(stroke::HAIRLINE))
                .border_color(c.border_focus)
        });

        let down = state.clone();
        let mv = state.clone();
        let mv_host = host.clone();
        let up = state.clone();
        let up_out = state.clone();
        let keys_kb = keys.clone();
        let kb = state.clone();
        let notify_move = notify.clone();
        let notify_up = notify.clone();
        let notify_kb = notify.clone();
        let focus_down = focus.clone();
        div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .track_focus(&focus)
            .tab_index(0)
            .child(probe(&host, None))
            .p(px(spacing::X_SMALL))
            .map(|d| match self.layout {
                SelectionLayout::Wrap => d
                    .flex()
                    .flex_wrap()
                    .content_start()
                    .gap(px(spacing::X_SMALL)),
                SelectionLayout::Column => d.flex().flex_col().gap(px(spacing::XXX_SMALL)),
            })
            .children(items)
            .children(marquee_el)
            .on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, window, cx| {
                window.focus(&focus_down);
                let additive = e.modifiers.shift || e.modifiers.secondary();
                down.update(cx, |s, cx| {
                    s.start = Some(e.position);
                    s.current = e.position;
                    s.additive = additive;
                    s.base = if additive {
                        s.model.selected().to_vec()
                    } else {
                        Vec::new()
                    };
                    cx.notify();
                });
            })
            .on_mouse_move(move |e: &MouseMoveEvent, window, cx| {
                let Some(start) = mv.read(cx).start else {
                    return;
                };
                if e.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let hit = hits(mv_host.read(cx), start, e.position, n);
                let order = keys.clone();
                mv.update(cx, |s, cx| {
                    s.current = e.position;
                    let mut sel = s.base.clone();
                    for i in hit {
                        if !sel.contains(&order[i]) {
                            sel.push(order[i].clone());
                        }
                    }
                    s.model.set(sel);
                    cx.notify();
                });
                notify_move(&mv, window, cx);
            })
            .on_mouse_up(MouseButton::Left, move |e, window, cx| {
                let Some(start) = up.read(cx).start else {
                    return;
                };
                let dragged = (f32::from(e.position.x - start.x)).abs() >= MARQUEE_THRESHOLD
                    || (f32::from(e.position.y - start.y)).abs() >= MARQUEE_THRESHOLD;
                up.update(cx, |s, cx| {
                    s.start = None;
                    if !dragged && !s.additive {
                        s.model.clear();
                    }
                    cx.notify();
                });
                if !dragged {
                    notify_up(&up, window, cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                up_out.update(cx, |s, cx| {
                    if s.start.take().is_some() {
                        cx.notify();
                    }
                });
            })
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                let k = e.keystroke.key.as_str();
                let m = &e.keystroke.modifiers;
                let delta: isize = match k {
                    "right" | "down" => 1,
                    "left" | "up" => -1,
                    _ => 0,
                };
                let handled = if k == "a" && m.secondary() {
                    kb.update(cx, |s, _| s.model.select_all(&keys_kb));
                    true
                } else if k == "escape" {
                    kb.update(cx, |s, _| s.model.clear());
                    true
                } else if delta != 0 {
                    let extend = m.shift;
                    kb.update(cx, |s, _| s.model.move_cursor(delta, extend, &keys_kb));
                    true
                } else {
                    false
                };
                if handled {
                    cx.stop_propagation();
                    kb.update(cx, |_, cx| cx.notify());
                    notify_kb(&kb, window, cx);
                }
            })
    }
}
