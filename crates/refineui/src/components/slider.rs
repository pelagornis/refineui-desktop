//! Slider — web `Slider` (`sm | md`). Click or drag the track; arrow keys step.

use std::rc::Rc;

use gpui::{
    App, Bounds, Context, CursorStyle, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, ParentElement, Pixels, Render, RenderOnce, StatefulInteractiveElement, Styled,
    Window, canvas, div, prelude::*, px, relative,
};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, Elevation};

/// Value-change callback for continuous controls.
pub type ValueHandler = Rc<dyn Fn(&f32, &mut Window, &mut App) + 'static>;

#[derive(Clone)]
struct SliderDrag(ElementId);

struct EmptyDragView;

impl Render for EmptyDragView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Horizontal range input.
#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    small: bool,
    on_change: Option<ValueHandler>,
}

impl Slider {
    /// Slider over `0.0..=1.0`.
    pub fn new(id: impl Into<ElementId>, value: f32) -> Self {
        Self {
            id: id.into(),
            value,
            min: 0.,
            max: 1.,
            step: 0.01,
            disabled: false,
            small: false,
            on_change: None,
        }
    }

    /// Range.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max.max(min + f32::EPSILON);
        self
    }

    /// Step for keyboard and snapping.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step.abs();
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 2px track (web `size="sm"`).
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(&f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

fn snap(v: f32, min: f32, max: f32, step: f32) -> f32 {
    let v = v.clamp(min, max);
    if step > 0. {
        (min + ((v - min) / step).round() * step).clamp(min, max)
    } else {
        v
    }
}

fn value_at(x: Pixels, bounds: Bounds<Pixels>, min: f32, max: f32, step: f32) -> f32 {
    let w = f32::from(bounds.size.width).max(1.);
    let t = (f32::from(x - bounds.left()) / w).clamp(0., 1.);
    snap(min + t * (max - min), min, max, step)
}

/// Fill pill width for progress `t` on a rail `track_w` wide and `track_h`
/// tall: `t × width`, but at least one cap (`track_h`) and at most the rail.
/// `None` until the rail has been measured.
fn fill_width(t: f32, track_w: f32, track_h: f32) -> Option<f32> {
    (track_w > 0.).then(|| (t.clamp(0., 1.) * track_w).clamp(track_h.min(track_w), track_w))
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use refineui_foundation::size::component::{
            SLIDER_INTERACTION, SLIDER_THUMB, SLIDER_TRACK, SLIDER_TRACK_SM,
        };
        let theme = cx.theme().clone();
        let c = theme.colors;
        let bounds = window.use_keyed_state(child_id(&self.id, "bounds"), cx, |_, _| {
            Bounds::<Pixels>::default()
        });
        let (min, max, step) = (self.min, self.max, self.step);
        let track_w = f32::from(bounds.read(cx).size.width);
        let t = ((self.value - min) / (max - min)).clamp(0., 1.);
        let track_h = if self.small {
            SLIDER_TRACK_SM
        } else {
            SLIDER_TRACK
        };
        let (rail, fill) = if self.disabled {
            (c.background_surface_disabled, c.foreground_disabled)
        } else {
            (c.background_brand_subtle, c.background_brand)
        };
        let (root_state, root_i) = use_interaction(&self.id, window, cx);
        let thumb_id = child_id(&self.id, "thumb");
        let (thumb_state, thumb_i) = use_interaction(&thumb_id, window, cx);
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus_visible(cx) && focus.is_focused(window);
        let pressed = !self.disabled && root_i.pressed;
        let hovered = !self.disabled && thumb_i.hovered;
        // Thumb: 20px `backgroundPrimary` circle, thin `borderDefault`,
        // shadow-2 (lighter). Hover → `borderStrong`, shadow-2, scale 1.15;
        // pressed → `borderStrong`, scale 0.98; focus-visible → `borderFocus`
        // + 2px ring. Scale is emulated by resizing the absolutely positioned
        // thumb (GPUI cannot transform elements).
        let (border, scale) = if self.disabled {
            (c.border_disabled, 1.0)
        } else if pressed {
            (c.border_strong, refineui_foundation::motion::scale::PRESS)
        } else if hovered {
            (
                c.border_strong,
                refineui_foundation::motion::scale::HOVER_GROW,
            )
        } else if focused {
            (c.border_focus, 1.0)
        } else {
            (c.border_default, 1.0)
        };
        let border = animate(child_id(&self.id, "bd"), border, Motion::FAST, window, cx);
        let scale = animate(child_id(&self.id, "sc"), scale, Motion::FAST, window, cx);
        let thumb = SLIDER_THUMB * scale;
        let mut shadow = theme.shadow(Elevation::SURFACE);
        if !hovered {
            for s in &mut shadow {
                s.color.a *= 0.5;
            }
        }
        if self.disabled {
            shadow.clear();
        }
        if focused {
            shadow.extend(theme.ring(c.border_focus, refineui_foundation::stroke::THICK));
        }
        let value = self.value;
        let id = self.id.clone();
        let on_change = self.on_change.clone();
        let on_change_drag = self.on_change.clone();
        let on_change_key = self.on_change.clone();
        let bounds_for_down = bounds.clone();
        let drag_id = id.clone();

        div()
            .id(self.id)
            .relative()
            .w_full()
            .h(px(SLIDER_INTERACTION))
            .flex()
            .items_center()
            .when(self.disabled, |d| {
                d.cursor(CursorStyle::OperationNotAllowed)
            })
            .when(!self.disabled, |d| {
                d.cursor(if pressed {
                    CursorStyle::ClosedHand
                } else {
                    CursorStyle::OpenHand
                })
                .track_focus(&focus)
                .track_interaction(&root_state)
                .on_mouse_down(MouseButton::Left, move |e, w, cx| {
                    let b = *bounds_for_down.read(cx);
                    if let Some(h) = &on_change {
                        h(&value_at(e.position.x, b, min, max, step), w, cx);
                    }
                })
                .on_drag(SliderDrag(drag_id.clone()), |_, _, _, cx| {
                    cx.new(|_| EmptyDragView)
                })
                .on_drag_move::<SliderDrag>(move |e, w, cx| {
                    if e.drag(cx).0 != drag_id {
                        return;
                    }
                    if let Some(h) = &on_change_drag {
                        h(
                            &value_at(e.event.position.x, e.bounds, min, max, step),
                            w,
                            cx,
                        );
                    }
                })
                .on_key_down(move |e: &KeyDownEvent, w, cx| {
                    let delta = match e.keystroke.key.as_str() {
                        "left" | "down" => -step.max((max - min) / 100.),
                        "right" | "up" => step.max((max - min) / 100.),
                        "home" => min - max,
                        "end" => max - min,
                        _ => return,
                    };
                    cx.stop_propagation();
                    if let Some(h) = &on_change_key {
                        h(&snap(value + delta, min, max, step), w, cx);
                    }
                })
            })
            .child(
                canvas(
                    move |b, _, cx| bounds.update(cx, |s, _| *s = b),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                // Web `::-webkit-slider-runnable-track`: `rounded-circle` rail
                // (4px md / 2px sm) with the fill inside it. GPUI clips
                // `overflow_hidden` to a rectangle, so the fill is its own pill
                // sharing the rail's left cap; it is never narrower than the
                // rail is tall (the extra sits under the thumb), and absent at 0.
                div()
                    .relative()
                    .w_full()
                    .h(px(track_h))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(rail)
                    .when(t > 0., |d| {
                        d.child(div().h_full().rounded_full().bg(fill).map(|f| {
                            match fill_width(t, track_w, track_h) {
                                Some(w) => f.w(px(w)),
                                None => f.w(relative(t)).min_w(px(track_h)),
                            }
                        }))
                    }),
            )
            .child(
                div()
                    .id(thumb_id)
                    .absolute()
                    .top(px((SLIDER_INTERACTION - thumb) / 2.))
                    .left(relative(t))
                    .ml(px(-thumb / 2.))
                    .size(px(thumb))
                    .rounded_full()
                    .bg(if self.disabled {
                        c.background_surface_disabled
                    } else {
                        c.background_primary
                    })
                    .border_1()
                    .border_color(border)
                    .shadow(shadow)
                    .when(!self.disabled, |d| d.track_interaction(&thumb_state)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{fill_width, snap};

    #[test]
    fn fill_stays_a_pill_inside_the_rail() {
        assert_eq!(fill_width(0.5, 200., 4.), Some(100.));
        assert_eq!(fill_width(0.001, 200., 4.), Some(4.));
        assert_eq!(fill_width(1.0, 200., 4.), Some(200.));
        assert_eq!(fill_width(0.5, 0., 4.), None);
    }

    #[test]
    fn snapping() {
        assert_eq!(snap(0.234, 0., 1., 0.1), 0.2);
        assert_eq!(snap(5.0, 0., 1., 0.1), 1.0);
        assert_eq!(snap(-1.0, 0., 10., 1.), 0.0);
    }
}
