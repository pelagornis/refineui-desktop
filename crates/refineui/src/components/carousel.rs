//! Carousel — web `Carousel` (`components/Carousel`): a stage plus footer
//! controls, looping infinitely.
//!
//! Stage: `roundedXXLarge`, thin `borderDefault`, `backgroundPrimary`,
//! `shadow-2`, clipped; slides glide over `duration-panel` (emphasized).
//! Controls (12px under the stage, centered, 10px apart): outline md icon
//! buttons around a sliding-pill meter — 10px tall, ≤ 192px, `circle`,
//! `backgroundBrandSubtle`, 2px inset — whose `backgroundBrand` thumb is one
//! segment wide and slides over `duration-medium` (emphasized). Clicking a
//! segment jumps to that slide. Going past the last slide continues to the
//! first in the same direction (the web clones slides for this; here slides
//! are built on demand by index).
//!
//! Drag the stage to swipe (web pointer drag): the track follows the pointer
//! 1:1; releasing past 18% of the stage width (`DRAG_COMMIT_RATIO`) commits
//! to the neighbouring slide, otherwise it springs back — both with the
//! same panel glide, starting from where the track was let go. (The web has
//! no velocity-based fling either.) ← / → work while focused.

use std::rc::Rc;
use std::time::Instant;

use gpui::{
    AnyElement, App, ElementId, IntoElement, KeyDownEvent, ParentElement, RenderOnce, Styled,
    Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke};

use crate::motion::{Motion, animate, child_id, probe, use_layout_probe};
use crate::{ActiveTheme, Button, ButtonVariant, ControlSize, Elevation, IconName};

type SlideFn = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement>;

/// `max-w-[12rem]` of the meter.
pub const CAROUSEL_METER_MAX_WIDTH: f32 = 192.;

/// Looping slide carousel.
#[derive(IntoElement)]
pub struct Carousel {
    id: ElementId,
    count: usize,
    slide: SlideFn,
    height: f32,
}

/// Web `DRAG_COMMIT_RATIO`: a drag past 18% of the stage width commits to the
/// neighbouring slide; shorter drags spring back.
pub const CAROUSEL_DRAG_COMMIT_RATIO: f32 = 0.18;

/// Slide glide (web `trackAnimated`: `duration-panel`, emphasized).
const GLIDE: Motion = Motion::new(
    crate::motion::duration::PANEL,
    crate::motion::easing::EMPHASIZED,
);

struct CarouselDrag(ElementId);
struct EmptyDrag;
impl gpui::Render for EmptyDrag {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Track position: the logical slide, the glide in flight and the drag.
#[derive(Clone, Copy, Debug, Default)]
pub struct Track {
    /// Logical (unbounded) slide position.
    pub pos: i64,
    from: f32,
    start: Option<Instant>,
    /// `(pointer start x, offset in slides)` while dragging.
    drag: Option<(f32, f32)>,
}

impl Track {
    /// Displayed position at `now`, and whether it has settled.
    pub fn visual(&self, now: Instant) -> (f32, bool) {
        if let Some((_, off)) = self.drag {
            return (self.pos as f32 - off, true);
        }
        let Some(start) = self.start else {
            return (self.pos as f32, true);
        };
        let ms = GLIDE.duration.as_secs_f32();
        let t = (now.duration_since(start).as_secs_f32() / ms.max(1e-3)).min(1.);
        let e = GLIDE.ease(t);
        (self.from + (self.pos as f32 - self.from) * e, t >= 1.)
    }

    /// Move by `delta` slides, gliding from wherever the track is now.
    pub fn go(&mut self, delta: i64) {
        self.from = self.visual(Instant::now()).0;
        self.drag = None;
        self.pos += delta;
        self.start = Some(Instant::now());
    }

    /// Pointer at `x` on a stage `width` px wide (starts a drag on first call).
    pub fn drag_to(&mut self, x: f32, width: f32) {
        match &mut self.drag {
            Some((start, off)) => *off = (x - *start) / width,
            None => {
                // Grab mid-glide: rebase so the slide stays under the pointer.
                let v = self.visual(Instant::now()).0;
                self.pos = v.round() as i64;
                self.drag = Some((x - (self.pos as f32 - v) * width, (self.pos as f32 - v)));
            }
        }
    }

    /// Pointer released: commit past [`CAROUSEL_DRAG_COMMIT_RATIO`], else spring back.
    pub fn release(&mut self) {
        let Some((_, off)) = self.drag else { return };
        let from = self.pos as f32 - off;
        let delta = if off <= -CAROUSEL_DRAG_COMMIT_RATIO {
            1
        } else if off >= CAROUSEL_DRAG_COMMIT_RATIO {
            -1
        } else {
            0
        };
        self.drag = None;
        self.from = from;
        self.pos += delta;
        self.start = Some(Instant::now());
    }
}

/// Wrap a signed position into `0..count`.
pub fn wrap_index(pos: i64, count: usize) -> usize {
    pos.rem_euclid(count.max(1) as i64) as usize
}

impl Carousel {
    /// `count` slides, each built by `slide(index)`.
    pub fn new(
        id: impl Into<ElementId>,
        count: usize,
        slide: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            count,
            slide: Rc::new(slide),
            height: 240.,
        }
    }

    /// Stage height (default 240).
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
}

impl RenderOnce for Carousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let n = self.count.max(1);
        let pos_state =
            window.use_keyed_state(child_id(&self.id, "pos"), cx, |_, _| Track::default());
        let pos = pos_state.read(cx).pos;
        let stage = use_layout_probe(&child_id(&self.id, "stage"), window, cx);
        let meter = use_layout_probe(&child_id(&self.id, "meter"), window, cx);
        let width = stage
            .read(cx)
            .container
            .map(|b| f32::from(b.size.width))
            .unwrap_or(0.);
        let meter_w = meter
            .read(cx)
            .container
            .map(|b| f32::from(b.size.width))
            .unwrap_or(0.);
        let (fpos, settled) = pos_state.read(cx).visual(Instant::now());
        if !settled {
            window.request_animation_frame();
        }
        let current = wrap_index(pos, n);
        let seg = (meter_w - 4.).max(0.) / n as f32;
        let thumb_x = animate(
            child_id(&self.id, "thumb"),
            current as f32 * seg,
            Motion::INDICATOR,
            window,
            cx,
        );
        let lo = fpos.floor() as i64;
        let hi = fpos.ceil() as i64;
        let slides: Vec<AnyElement> = (lo..=hi)
            .map(|k| {
                let body = (self.slide)(wrap_index(k, n), window, cx);
                div()
                    .absolute()
                    .top_0()
                    .left(px((k as f32 - fpos) * width))
                    .w(px(width))
                    .h_full()
                    .child(body)
                    .into_any_element()
            })
            .collect();
        let step = |delta: i64| {
            let s = pos_state.clone();
            move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                s.update(cx, |t, cx| {
                    t.go(delta);
                    cx.notify();
                })
            }
        };
        let keys = pos_state.clone();
        let dragging = pos_state.read(cx).drag.is_some();
        let drag_id = self.id.clone();
        let drag_id_move = self.id.clone();
        let drag_state = pos_state.clone();
        let release = pos_state.clone();
        let release_out = pos_state.clone();
        let focus = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focus_click = focus.clone();
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .w_full()
            .gap(px(spacing::LARGE))
            .track_focus(&focus)
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, _| {
                window.focus(&focus_click)
            })
            .on_key_down(move |e: &KeyDownEvent, _, cx| {
                let d = match e.keystroke.key.as_str() {
                    "left" => -1,
                    "right" => 1,
                    _ => return,
                };
                keys.update(cx, |t, cx| {
                    t.go(d);
                    cx.notify();
                });
            })
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(self.height))
                    .overflow_hidden()
                    .rounded(px(radius::XX_LARGE))
                    .border(px(stroke::THIN))
                    .border_color(c.border_default)
                    .bg(c.background_primary)
                    .shadow(theme.shadow(Elevation::RAISED))
                    .id(child_id(&self.id, "stage-drag"))
                    .cursor(if dragging {
                        gpui::CursorStyle::ClosedHand
                    } else {
                        gpui::CursorStyle::OpenHand
                    })
                    .on_drag(CarouselDrag(drag_id.clone()), |_, _, _, cx| {
                        cx.new(|_| EmptyDrag)
                    })
                    .on_drag_move::<CarouselDrag>(move |e, _, cx| {
                        if e.drag(cx).0 != drag_id_move {
                            return;
                        }
                        let w = f32::from(e.bounds.size.width).max(1.);
                        let x = f32::from(e.event.position.x);
                        drag_state.update(cx, |t, cx| {
                            t.drag_to(x, w);
                            cx.notify();
                        });
                    })
                    .on_mouse_up(gpui::MouseButton::Left, move |_, _, cx| {
                        release.update(cx, |t, cx| {
                            t.release();
                            cx.notify();
                        })
                    })
                    .on_mouse_up_out(gpui::MouseButton::Left, move |_, _, cx| {
                        release_out.update(cx, |t, cx| {
                            t.release();
                            cx.notify();
                        })
                    })
                    .child(probe(&stage, None))
                    .children(slides),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .gap(px(spacing::MEDIUM))
                    .child(
                        Button::icon_only(child_id(&self.id, "prev"), IconName::ChevronLeft)
                            .variant(ButtonVariant::Outline)
                            .size(ControlSize::Medium)
                            .on_click(step(-1)),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .max_w(px(CAROUSEL_METER_MAX_WIDTH))
                            .h(px(spacing::MEDIUM))
                            .p(px(spacing::XXX_SMALL))
                            .rounded_full()
                            .bg(c.background_brand_subtle)
                            .overflow_hidden()
                            .child(probe(&meter, None))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(spacing::XXX_SMALL))
                                    .bottom(px(spacing::XXX_SMALL))
                                    .left(px(spacing::XXX_SMALL + thumb_x))
                                    .w(px(seg))
                                    .rounded_full()
                                    .bg(c.background_brand)
                                    .shadow(theme.shadow(Elevation::RAISED)),
                            )
                            .child(
                                div()
                                    .relative()
                                    .flex()
                                    .size_full()
                                    .children((0..n).map(|i| {
                                        let s = pos_state.clone();
                                        div()
                                            .id(("carousel-hit", i))
                                            .flex_1()
                                            .h_full()
                                            .cursor_pointer()
                                            .on_click(move |_, _, cx| {
                                                s.update(cx, |t, cx| {
                                                    let cur = wrap_index(t.pos, n) as i64;
                                                    t.go(i as i64 - cur);
                                                    cx.notify();
                                                })
                                            })
                                    })),
                            ),
                    )
                    .child(
                        Button::icon_only(child_id(&self.id, "next"), IconName::ChevronRight)
                            .variant(ButtonVariant::Outline)
                            .size(ControlSize::Medium)
                            .on_click(step(1)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::wrap_index;

    #[test]
    fn drags_commit_past_18_percent() {
        let mut t = super::Track::default();
        t.drag_to(100., 500.);
        t.drag_to(20., 500.); // -16%
        t.release();
        assert_eq!(t.pos, 0);
        t.drag_to(100., 500.);
        t.drag_to(0., 500.); // -20%
        t.release();
        assert_eq!(t.pos, 1);
        t.drag_to(0., 500.);
        t.drag_to(100., 500.); // +20%
        t.release();
        assert_eq!(t.pos, 0);
    }

    #[test]
    fn positions_wrap_both_ways() {
        assert_eq!(wrap_index(-1, 4), 3);
        assert_eq!(wrap_index(5, 4), 1);
        assert_eq!(wrap_index(0, 0), 0);
    }
}
