//! Resizable — web `ResizablePanelGroup` / `ResizablePanel` / `ResizableHandle`.
//!
//! Panels are flex items whose `flex-grow` is their size percentage
//! (`flex-basis: 0`, `min-width: minSize%`), separated by 12px handles
//! (`foundation-size-120`). Dragging a handle moves the boundary between
//! its two neighbours, clamped by both panels' min / max:
//! `minLeft = max(left.min, pair − right.max)`,
//! `maxLeft = min(left.max, pair − right.min)`.
//! `withHandle` draws only a centered 4×32 pill (`borderDefault`, hover /
//! focus / drag `borderStrong`, `duration-fast`); there is no full-length rule.
//! Keyboard: arrows move 2% (Shift 10%), Home / End jump to min / max.

use std::rc::Rc;

use gpui::{
    AnyElement, App, CursorStyle, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, RenderOnce, Styled, Window,
    div, prelude::*, px, relative,
};

use crate::ActiveTheme;
use crate::motion::{
    Motion, TrackInteraction, animate, child_id, probe, use_focus_handle, use_interaction,
    use_layout_probe,
};

/// Web `KEYBOARD_STEP`.
const KEYBOARD_STEP: f32 = 2.;
/// Web `KEYBOARD_STEP_LARGE`.
const KEYBOARD_STEP_LARGE: f32 = 10.;
/// Handle hit area (`foundation-size-120`).
const HANDLE: f32 = 12.;
/// Grip pill (`foundation-size-40` × `foundation-size-320`).
const GRIP_THICKNESS: f32 = 4.;
const GRIP_LENGTH: f32 = 32.;

/// Group direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResizableOrientation {
    /// Panels side by side (vertical handles, `col-resize`).
    #[default]
    Horizontal,
    /// Panels stacked (horizontal handles, `row-resize`).
    Vertical,
}

/// One panel in a [`ResizablePanelGroup`].
pub struct ResizablePanel {
    default_size: f32,
    min_size: f32,
    max_size: f32,
    size: Option<f32>,
    children: Vec<AnyElement>,
}

impl Default for ResizablePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ResizablePanel {
    /// New panel: `defaultSize` 50, `minSize` 10, `maxSize` 100.
    pub fn new() -> Self {
        Self {
            default_size: 50.,
            min_size: 10.,
            max_size: 100.,
            size: None,
            children: Vec::new(),
        }
    }

    /// Initial size (% of the group).
    pub fn default_size(mut self, pct: f32) -> Self {
        self.default_size = pct;
        self
    }

    /// Minimum size (%).
    pub fn min_size(mut self, pct: f32) -> Self {
        self.min_size = pct;
        self
    }

    /// Maximum size (%).
    pub fn max_size(mut self, pct: f32) -> Self {
        self.max_size = pct;
        self
    }

    /// Controlled size (%). Applied whenever the value changes (drags in
    /// between move freely; report them back through `on_layout`).
    pub fn size(mut self, pct: f32) -> Self {
        self.size = Some(pct);
        self
    }

    /// Panel content.
    pub fn child(mut self, el: impl IntoElement) -> Self {
        self.children.push(el.into_any_element());
        self
    }
}

/// Separator between two panels.
#[derive(Clone, Default)]
pub struct ResizableHandle {
    with_handle: bool,
    disabled: bool,
}

impl ResizableHandle {
    /// Invisible hit area.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw the center pill.
    pub fn with_handle(mut self, on: bool) -> Self {
        self.with_handle = on;
        self
    }

    /// Disable dragging (`opacity: 0.5`, no pointer).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

enum Item {
    Panel(ResizablePanel),
    Handle(ResizableHandle),
}

#[derive(Default)]
struct GroupState {
    /// Size per panel (%), normalized from the defaults on first render.
    sizes: Vec<f32>,
    /// Active drag: (handle index, pointer start, left size at start).
    drag: Option<(usize, Pixels, f32)>,
    /// Controlled sizes applied last frame — a controlled value is applied
    /// only when it changes, so dragging between `on_layout` updates works.
    applied: Vec<Option<f32>>,
}

type LayoutHandler = Rc<dyn Fn(&[f32], &mut Window, &mut App)>;

/// Panel group (web `ResizablePanelGroup`).
#[derive(IntoElement)]
pub struct ResizablePanelGroup {
    id: ElementId,
    orientation: ResizableOrientation,
    items: Vec<Item>,
    on_layout: Option<LayoutHandler>,
}

impl ResizablePanelGroup {
    /// New horizontal group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            orientation: ResizableOrientation::Horizontal,
            items: Vec::new(),
            on_layout: None,
        }
    }

    /// Direction.
    pub fn orientation(mut self, o: ResizableOrientation) -> Self {
        self.orientation = o;
        self
    }

    /// Stack panels vertically.
    pub fn vertical(self) -> Self {
        self.orientation(ResizableOrientation::Vertical)
    }

    /// Append a panel.
    pub fn panel(mut self, p: ResizablePanel) -> Self {
        self.items.push(Item::Panel(p));
        self
    }

    /// Append a handle (between two panels).
    pub fn handle(mut self, h: ResizableHandle) -> Self {
        self.items.push(Item::Handle(h));
        self
    }

    /// Called after a drag / key resize settles with every panel's size (%).
    pub fn on_layout(mut self, f: impl Fn(&[f32], &mut Window, &mut App) + 'static) -> Self {
        self.on_layout = Some(Rc::new(f));
        self
    }
}

#[derive(Clone, Copy)]
struct Limits {
    min: f32,
    max: f32,
}

/// Web `resizeBetween`: move `delta` % from right to left within both limits.
fn resize_between(sizes: &mut [f32], limits: &[Limits], left: usize, delta_from: f32, delta: f32) {
    let right = left + 1;
    if right >= sizes.len() {
        return;
    }
    let pair = sizes[left] + sizes[right];
    let min_left = limits[left].min.max(pair - limits[right].max);
    let max_left = limits[left].max.min(pair - limits[right].min);
    let next = (delta_from + delta).clamp(min_left, max_left.max(min_left));
    sizes[left] = next;
    sizes[right] = pair - next;
}

impl RenderOnce for ResizablePanelGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let horizontal = self.orientation == ResizableOrientation::Horizontal;
        let limits: Vec<Limits> = self
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Panel(p) => Some(Limits {
                    min: p.min_size,
                    max: p.max_size,
                }),
                Item::Handle(_) => None,
            })
            .collect();
        let defaults: Vec<f32> = self
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Panel(p) => Some(p.default_size),
                Item::Handle(_) => None,
            })
            .collect();
        let state = window.use_keyed_state(child_id(&self.id, "resizable"), cx, |_, _| {
            GroupState::default()
        });
        if state.read(cx).sizes.len() != defaults.len() {
            let sum: f32 = defaults.iter().sum::<f32>().max(f32::EPSILON);
            let norm = defaults.iter().map(|d| d / sum * 100.).collect();
            state.update(cx, |s, _| s.sizes = norm);
        }
        // Controlled sizes win.
        let mut panel_ix = 0;
        let mut controlled = Vec::new();
        for it in &self.items {
            if let Item::Panel(p) = it {
                controlled.push((panel_ix, p.size));
                panel_ix += 1;
            }
        }
        state.update(cx, |s, _| {
            if s.applied.len() != controlled.len() {
                s.applied = vec![None; controlled.len()];
            }
            for (ix, size) in &controlled {
                if let Some(size) = size
                    && s.applied[*ix] != Some(*size)
                {
                    s.sizes[*ix] = *size;
                    s.applied[*ix] = Some(*size);
                }
            }
        });
        let sizes = state.read(cx).sizes.clone();
        let dragging = state.read(cx).drag;
        let group_probe = use_layout_probe(&self.id, window, cx);
        let handle_count = self
            .items
            .iter()
            .filter(|i| matches!(i, Item::Handle(_)))
            .count();
        let limits = Rc::new(limits);

        let mut children: Vec<AnyElement> = Vec::new();
        let mut panel_ix = 0usize;
        for item in self.items {
            match item {
                Item::Panel(p) => {
                    let grow = sizes.get(panel_ix).copied().unwrap_or(p.default_size);
                    let mut d = div()
                        .relative()
                        .flex()
                        .min_w_0()
                        .min_h_0()
                        .overflow_hidden()
                        .flex_basis(px(0.))
                        .flex_shrink();
                    d.style().flex_grow = Some(grow.max(0.));
                    let d = if horizontal {
                        d.h_full().min_w(relative(p.min_size / 100.))
                    } else {
                        d.w_full().min_h(relative(p.min_size / 100.))
                    };
                    children.push(d.children(p.children).into_any_element());
                    panel_ix += 1;
                }
                Item::Handle(h) => {
                    // The handle resizes the panels on either side.
                    let left = panel_ix.saturating_sub(1);
                    let hid = ElementId::NamedChild(
                        Box::new(self.id.clone()),
                        format!("handle-{left}").into(),
                    );
                    let (istate, i) = use_interaction(&hid, window, cx);
                    let focus = use_focus_handle(&hid, 0, window, cx);
                    let active = dragging.is_some_and(|(ix, _, _)| ix == left);
                    let lit = !h.disabled && (i.hovered || active || focus.is_focused(window));
                    let grip = animate(
                        child_id(&hid, "grip"),
                        if lit {
                            c.border_strong
                        } else {
                            c.border_default
                        },
                        Motion::FAST,
                        window,
                        cx,
                    );
                    let down_state = state.clone();
                    let key_state = state.clone();
                    let key_limits = limits.clone();
                    let key_layout = self.on_layout.clone();
                    let mut el = div()
                        .id(hid)
                        .relative()
                        .flex_none()
                        .when(horizontal, |d| d.w(px(HANDLE)).h_full())
                        .when(!horizontal, |d| d.h(px(HANDLE)).w_full())
                        .when(h.disabled, |d| d.opacity(0.5))
                        .when(h.with_handle, |d| {
                            d.flex().items_center().justify_center().child(
                                div()
                                    .rounded_full()
                                    .bg(grip)
                                    .when(horizontal, |d| {
                                        d.w(px(GRIP_THICKNESS)).h(px(GRIP_LENGTH))
                                    })
                                    .when(!horizontal, |d| {
                                        d.h(px(GRIP_THICKNESS)).w(px(GRIP_LENGTH))
                                    }),
                            )
                        });
                    if !h.disabled {
                        el = el
                            .cursor(if horizontal {
                                CursorStyle::ResizeLeftRight
                            } else {
                                CursorStyle::ResizeUpDown
                            })
                            .track_focus(&focus)
                            .track_interaction(&istate)
                            .on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, w, cx| {
                                let pos = if horizontal {
                                    e.position.x
                                } else {
                                    e.position.y
                                };
                                down_state.update(cx, |s, _| {
                                    let from = s.sizes.get(left).copied().unwrap_or(0.);
                                    s.drag = Some((left, pos, from));
                                });
                                cx.stop_propagation();
                                w.refresh();
                            })
                            .on_key_down(move |e: &KeyDownEvent, w, cx| {
                                let shift = e.keystroke.modifiers.shift;
                                let step = if shift {
                                    KEYBOARD_STEP_LARGE
                                } else {
                                    KEYBOARD_STEP
                                };
                                let key = e.keystroke.key.as_str();
                                let delta = match (key, horizontal) {
                                    ("left", true) | ("up", false) => Some(-step),
                                    ("right", true) | ("down", false) => Some(step),
                                    ("home", _) => Some(f32::NEG_INFINITY),
                                    ("end", _) => Some(f32::INFINITY),
                                    _ => None,
                                };
                                let Some(delta) = delta else { return };
                                cx.stop_propagation();
                                let sizes = key_state.update(cx, |s, _| {
                                    let from = s.sizes.get(left).copied().unwrap_or(0.);
                                    let delta = if delta.is_finite() {
                                        delta
                                    } else {
                                        delta.signum() * 1000.
                                    };
                                    resize_between(&mut s.sizes, &key_limits, left, from, delta);
                                    s.sizes.clone()
                                });
                                if let Some(f) = &key_layout {
                                    f(&sizes, w, cx);
                                }
                                w.refresh();
                            });
                    }
                    children.push(el.into_any_element());
                }
            }
        }

        let move_state = state.clone();
        let move_probe = group_probe.clone();
        let move_limits = limits.clone();
        let up_state = state.clone();
        let up_state2 = state.clone();
        let up_layout = self.on_layout.clone();
        let up_layout2 = self.on_layout.clone();
        let finish = move |s: &gpui::Entity<GroupState>,
                           layout: &Option<LayoutHandler>,
                           w: &mut Window,
                           cx: &mut App| {
            if s.read(cx).drag.is_none() {
                return;
            }
            let sizes = s.update(cx, |s, _| {
                s.drag = None;
                s.sizes.clone()
            });
            if let Some(f) = layout {
                f(&sizes, w, cx);
            }
            w.refresh();
        };
        let finish2 = finish;
        div()
            .id(self.id.clone())
            .relative()
            .flex()
            .when(horizontal, |d| d.flex_row())
            .when(!horizontal, |d| d.flex_col())
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .when(dragging.is_some(), |d| {
                d.cursor(if horizontal {
                    CursorStyle::ResizeLeftRight
                } else {
                    CursorStyle::ResizeUpDown
                })
            })
            .child(probe(&group_probe, None))
            .on_mouse_move(move |e: &MouseMoveEvent, w, cx| {
                let Some((left, start, from)) = move_state.read(cx).drag else {
                    return;
                };
                let Some(bounds) = move_probe.read(cx).container else {
                    return;
                };
                let (pos, total) = if horizontal {
                    (e.position.x, bounds.size.width)
                } else {
                    (e.position.y, bounds.size.height)
                };
                let avail = f32::from(total) - HANDLE * handle_count as f32;
                if avail <= 0. {
                    return;
                }
                let delta = f32::from(pos - start) / avail * 100.;
                move_state.update(cx, |s, _| {
                    resize_between(&mut s.sizes, &move_limits, left, from, delta)
                });
                w.refresh();
            })
            .on_mouse_up(MouseButton::Left, move |_, w, cx| {
                finish(&up_state, &up_layout, w, cx)
            })
            .on_mouse_up_out(MouseButton::Left, move |_, w, cx| {
                finish2(&up_state2, &up_layout2, w, cx)
            })
            .children(children)
    }
}

impl gpui::prelude::FluentBuilder for ResizablePanel {}
impl gpui::prelude::FluentBuilder for ResizableHandle {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_respects_both_limits() {
        let limits = [
            Limits {
                min: 20.,
                max: 100.,
            },
            Limits {
                min: 30.,
                max: 100.,
            },
        ];
        let mut sizes = [35., 65.];
        resize_between(&mut sizes, &limits, 0, 35., 50.);
        assert_eq!(sizes, [70., 30.]);
        resize_between(&mut sizes, &limits, 0, 70., -80.);
        assert_eq!(sizes, [20., 80.]);
    }
}
