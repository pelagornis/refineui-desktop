//! Sliding highlight — web `useSlidingHighlight` (`components/shared/SlidingHighlight.tsx`).
//!
//! One plate per list host (Menu / Dropdown / ContextMenu panels, Select
//! popup, Sidebar, Tree, Table) that glides between the hovered (or active)
//! rows instead of every row fading its own fill. Web behaviour:
//!
//! * the plate fades in over `duration-medium` (200ms, emphasized) where it
//!   first lands — no glide from a stale position;
//! * moving to another row while shown glides `transform`, `width` and
//!   `height` with the same 200ms emphasized curve;
//! * leaving the host fades it out in place;
//! * once the plate is live, resting rows drop their own hover fill
//!   (`[data-sliding-highlight=true] … :hover { background: transparent }`);
//!   pressed rows keep their pressed fill.
//!
//! Hosts measure their rows with [`crate::motion::probe`]: the host gets slot
//! `None`, row `ix` gets `Some(ix)`.

use std::time::Instant;

use gpui::{
    AnyElement, App, Bounds, ElementId, Entity, Hsla, IntoElement, Pixels, Styled, Window, div,
    point, px, size,
};

use crate::motion::{LayoutProbe, Motion, animate, child_id, reduced_motion};

/// Web plate motion: `duration-medium` + emphasized (`slidingHighlightClass`).
pub const SLIDE: Motion = Motion::INDICATOR;

#[derive(Clone, Copy, Debug)]
struct Glide {
    from: Bounds<Pixels>,
    to: Bounds<Pixels>,
    start: Instant,
    target: Option<usize>,
    shown: bool,
}

fn lerp_bounds(a: Bounds<Pixels>, b: Bounds<Pixels>, t: f32) -> Bounds<Pixels> {
    let l = |x: Pixels, y: Pixels| x + (y - x) * t;
    Bounds::new(
        point(l(a.origin.x, b.origin.x), l(a.origin.y, b.origin.y)),
        size(
            l(a.size.width, b.size.width),
            l(a.size.height, b.size.height),
        ),
    )
}

impl Glide {
    fn sample(&self, now: Instant) -> (Bounds<Pixels>, bool) {
        let ms = SLIDE.duration.as_secs_f32() * 1000.;
        let t = (now.duration_since(self.start).as_secs_f32() * 1000. / ms.max(1.)).min(1.);
        (lerp_bounds(self.from, self.to, SLIDE.ease(t)), t >= 1.)
    }
}

/// The plate for one host. `target` is the row index the plate should rest on
/// (hovered row, else the active / selected row when `rest_on_active` hosts
/// pass it), `None` to fade out. Returns an absolutely positioned element to
/// add as the host's **first** child (so it paints under the rows).
pub fn sliding_plate(
    key: &ElementId,
    probe: &Entity<LayoutProbe>,
    target: Option<usize>,
    fill: Hsla,
    radius: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let rect = target.and_then(|ix| probe.read(cx).relative(ix));
    let now = Instant::now();
    let state = window.use_keyed_state(child_id(key, "slide"), cx, |_, _| Glide {
        from: rect.unwrap_or_default(),
        to: rect.unwrap_or_default(),
        start: now,
        target: None,
        shown: false,
    });
    let reduced = reduced_motion(cx);
    let (bounds, settled) = state.update(cx, |g, _| {
        match rect {
            Some(r) => {
                let retarget = g.target != target;
                if !g.shown || reduced {
                    // First landing: snap there and fade in (no glide).
                    g.from = r;
                    g.to = r;
                    g.start = now;
                } else if retarget || r != g.to {
                    let (current, _) = g.sample(now);
                    g.from = current;
                    g.to = r;
                    g.start = now;
                }
                g.target = target;
                g.shown = true;
            }
            None => {
                // Keep the last geometry so the fade-out happens in place.
                g.shown = false;
                g.target = None;
            }
        }
        g.sample(now)
    });
    if !settled {
        window.request_animation_frame();
    }
    let opacity = animate(
        child_id(key, "slide-opacity"),
        if rect.is_some() { 1.0f32 } else { 0.0 },
        SLIDE,
        window,
        cx,
    );
    div()
        .absolute()
        .left(bounds.origin.x)
        .top(bounds.origin.y)
        .w(bounds.size.width)
        .h(bounds.size.height)
        .rounded(px(radius))
        .bg(fill)
        .opacity(opacity)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glide_interpolates_from_current_to_target() {
        let a = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(32.)));
        let b = Bounds::new(point(px(0.), px(64.)), size(px(100.), px(40.)));
        let mid = lerp_bounds(a, b, 0.5);
        assert_eq!(mid.origin.y, px(32.));
        assert_eq!(mid.size.height, px(36.));
        assert_eq!(SLIDE.duration.as_millis(), 200);
    }
}
