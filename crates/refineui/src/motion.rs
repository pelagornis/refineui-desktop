//! Motion — RefineUI transitions for GPUI.
//!
//! The web kit animates with CSS `transition`s keyed on the semantic motion
//! tokens (`semantic/interaction.ts`). GPUI has no implicit transitions, so this
//! module provides the equivalent building blocks:
//!
//! * [`Motion`] — a duration + easing pair, with presets named after the web
//!   roles ([`Motion::FAST`], [`Motion::INDICATOR`], [`Motion::PANEL`], …).
//! * [`animate`] / [`animate_from`] — a tweened value that follows a target
//!   across renders (colors, pixels, floats). When the target changes, the value
//!   eases from where it currently is, exactly like a CSS transition, and the
//!   view keeps repainting with `request_animation_frame` until it settles.
//! * [`presence`] — enter / exit progress for overlays that must stay mounted
//!   while they animate out (dialog, drawer, popover, menu, select, toast).
//! * [`use_interaction`] + [`TrackInteraction`] — hover / pressed tracking so a
//!   component can compute its state colors in `render` and transition them.
//!
//! ```ignore
//! let (state, i) = use_interaction(&id, window, cx);
//! let bg = animate((id.clone(), "bg"), if i.pressed { active } else if i.hovered { hover } else { rest },
//!                  Motion::FAST, window, cx);
//! div().id(id).track_interaction(&state).bg(bg)
//! ```

use std::time::{Duration, Instant};

use gpui::{
    Animation, App, Bounds, Div, ElementId, Entity, Hsla, InteractiveElement, IntoElement,
    MouseButton, Pixels, Rgba, Stateful, StatefulInteractiveElement, Styled, Window, canvas, point,
    px,
};
pub use refineui_tokens::motion::{CubicBezier, FLOAT_DISTANCE, duration, easing, scale};

/// A transition timing: duration plus easing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// Duration.
    pub duration: Duration,
    /// Easing curve.
    pub easing: CubicBezier,
}

impl Motion {
    /// Build from milliseconds and a curve.
    pub const fn new(ms: u64, easing: CubicBezier) -> Self {
        Self {
            duration: Duration::from_millis(ms),
            easing,
        }
    }

    /// `componentTokens.motion.fast` — hover / press color feedback (150ms, ease-out).
    pub const FAST: Self = Self::new(duration::FAST, easing::EASE_OUT);
    /// `duration-fast` with the browser default `ease` (transitions that name no easing).
    pub const FAST_EASE: Self = Self::new(duration::FAST, easing::EASE);
    /// `duration-medium` + `ease` — switch track / thumb.
    pub const MEDIUM: Self = Self::new(duration::MEDIUM, easing::EASE);
    /// `componentTokens.motion.indicator` — tabs / segmented indicator slide (200ms, emphasized).
    pub const INDICATOR: Self = Self::new(duration::MEDIUM, easing::EMPHASIZED);
    /// Indicator width (`duration-slow`, emphasized) — the edges arrive apart.
    pub const INDICATOR_WIDTH: Self = Self::new(duration::SLOW, easing::EMPHASIZED);
    /// Frosted panel enter / exit: menus, select, popover, tooltip (180ms, standard).
    pub const FROSTED: Self = Self::new(duration::NORMAL, easing::STANDARD);
    /// Submenu enter (150ms, standard).
    pub const SUBMENU: Self = Self::new(duration::FAST, easing::STANDARD);
    /// `componentTokens.motion.panel` — dialog / drawer panel (320ms, panel curve).
    pub const PANEL: Self = Self::new(duration::PANEL, easing::PANEL);
    /// Scrim fade (`duration-overlay`, panel curve).
    pub const SCRIM: Self = Self::new(duration::OVERLAY, easing::PANEL);
    /// Accordion / collapsible / tree height (380ms, panel curve).
    pub const ACCORDION_PANEL: Self = Self::new(duration::ACCORDION_PANEL, easing::PANEL);
    /// Accordion content fade + lift (260ms, content curve).
    pub const ACCORDION_CONTENT: Self = Self::new(duration::ACCORDION_CONTENT, easing::CONTENT);
    /// Toast enter (280ms, emphasized).
    pub const TOAST_ENTER: Self = Self::new(duration::TOAST_ENTER, easing::EMPHASIZED);
    /// Toast leave (200ms, standard).
    pub const TOAST_LEAVE: Self = Self::new(duration::TOAST_LEAVE, easing::STANDARD);
    /// Toast stack re-layout (panel duration, emphasized).
    pub const TOAST_STACK: Self = Self::new(duration::PANEL, easing::EMPHASIZED);
    /// Progress bar width (panel duration, ease-out).
    pub const PROGRESS: Self = Self::new(duration::PANEL, easing::EASE_OUT);
    /// Chevron / tree caret rotation (medium, emphasized).
    pub const CHEVRON: Self = Self::new(duration::MEDIUM, easing::EMPHASIZED);

    /// Eased progress for a linear `t` in `0..=1`.
    pub fn ease(&self, t: f32) -> f32 {
        self.easing.ease(t)
    }

    /// A GPUI [`Animation`] with this duration and easing (one-shot).
    pub fn animation(&self) -> Animation {
        let curve = self.easing;
        Animation::new(self.duration).with_easing(move |t| curve.ease(t))
    }
}

/// Infinite skeleton pulse: opacity `1 → opacity-disabled → 1` over `duration-skeleton`, ease-in-out.
pub fn skeleton_pulse() -> Animation {
    Animation::new(Duration::from_millis(duration::SKELETON)).repeat()
}

/// Opacity for [`skeleton_pulse`] at linear progress `t`.
pub fn skeleton_pulse_opacity(t: f32) -> f32 {
    // keyframes 0%,100% → 1; 50% → 0.5, each half eased with ease-in-out.
    let (from, to, local) = if t < 0.5 {
        (1.0, refineui_tokens::opacity::DISABLED, t * 2.0)
    } else {
        (refineui_tokens::opacity::DISABLED, 1.0, (t - 0.5) * 2.0)
    };
    from + (to - from) * easing::EASE_IN_OUT.ease(local)
}

/// Infinite spinner rotation: one turn per `duration-spinner`, linear.
pub fn spinner_rotation() -> Animation {
    Animation::new(Duration::from_millis(duration::SPINNER)).repeat()
}

/// Values that can be tweened.
pub trait Lerp: Copy + PartialEq + 'static {
    /// Interpolate from `a` to `b` by `t` (`0..=1`, may overshoot for springy curves).
    fn lerp(a: Self, b: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl Lerp for Pixels {
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        px(f32::lerp(f32::from(a), f32::from(b), t))
    }
}

impl Lerp for Hsla {
    /// Interpolates in premultiplied sRGB, like CSS color transitions.
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        mix(a, b, t)
    }
}

/// Mix two colors in premultiplied sRGB (CSS transition semantics), `t = 0` → `a`.
pub fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let a: Rgba = a.into();
    let b: Rgba = b.into();
    let alpha = a.a + (b.a - a.a) * t;
    if alpha <= f32::EPSILON {
        return gpui::transparent_black();
    }
    let ch = |x: f32, xa: f32, y: f32, ya: f32| ((x * xa) + ((y * ya) - (x * xa)) * t) / alpha;
    Rgba {
        r: ch(a.r, a.a, b.r, b.a).clamp(0.0, 1.0),
        g: ch(a.g, a.a, b.g, b.a).clamp(0.0, 1.0),
        b: ch(a.b, a.a, b.b, b.a).clamp(0.0, 1.0),
        a: alpha.clamp(0.0, 1.0),
    }
    .into()
}

struct Tween<T> {
    from: T,
    to: T,
    start: Instant,
    motion: Motion,
}

impl<T: Lerp> Tween<T> {
    fn sample(&self, now: Instant) -> (T, bool) {
        let total = self.motion.duration.as_secs_f32();
        if total <= 0.0 || self.from == self.to {
            return (self.to, true);
        }
        let t = (now.saturating_duration_since(self.start).as_secs_f32() / total).min(1.0);
        if t >= 1.0 {
            (self.to, true)
        } else {
            (T::lerp(self.from, self.to, self.motion.ease(t)), false)
        }
    }
}

/// Global switch for reduced motion (`prefers-reduced-motion` on the web).
#[derive(Clone, Copy, Debug, Default)]
pub struct ReducedMotion(pub bool);
impl gpui::Global for ReducedMotion {}

/// Turn every RefineUI transition off (values jump to their targets).
pub fn set_reduced_motion(reduced: bool, cx: &mut App) {
    cx.set_global(ReducedMotion(reduced));
    cx.refresh_windows();
}

/// Whether [`ReducedMotion`] is on.
pub fn reduced_motion(cx: &App) -> bool {
    reduced(cx)
}

fn reduced(cx: &App) -> bool {
    cx.try_global::<ReducedMotion>().is_some_and(|r| r.0)
}

/// Build a child element id (`parent` + `suffix`) for keyed motion state.
pub fn child_id(parent: &ElementId, suffix: &'static str) -> ElementId {
    ElementId::NamedChild(Box::new(parent.clone()), suffix.into())
}

/// A value that transitions to `target` whenever it changes (starts settled at `target`).
pub fn animate<T: Lerp>(
    key: impl Into<ElementId>,
    target: T,
    motion: Motion,
    window: &mut Window,
    cx: &mut App,
) -> T {
    animate_from(key, target, target, motion, window, cx)
}

/// Like [`animate`], but the first render starts at `initial` (enter animations).
pub fn animate_from<T: Lerp>(
    key: impl Into<ElementId>,
    initial: T,
    target: T,
    motion: Motion,
    window: &mut Window,
    cx: &mut App,
) -> T {
    if reduced(cx) {
        return target;
    }
    let now = Instant::now();
    let state = window.use_keyed_state(key, cx, |_, _| Tween {
        from: initial,
        to: target,
        start: now,
        motion,
    });
    let (value, done) = state.update(cx, |tw, _| {
        if tw.to != target {
            let (current, _) = tw.sample(now);
            tw.from = current;
            tw.to = target;
            tw.start = now;
            tw.motion = motion;
        }
        tw.sample(now)
    });
    if !done {
        window.request_animation_frame();
    }
    value
}

/// Web `transform: scale(var(--refineui-motion-scale-press))` on `:active`
/// with `transition: transform var(--duration-fast)` (CSS default `ease`):
/// returns the current scale, easing 1 → 0.98 while `pressed` and back.
pub fn press_scale(
    key: impl Into<ElementId>,
    pressed: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    let target = if pressed {
        refineui_tokens::motion::scale::PRESS
    } else {
        1.0
    };
    animate(key, target, Motion::FAST_EASE, window, cx)
}

/// [`press_scale`] with an explicit pressed scale (Button uses
/// `scale::BUTTON_PRESS` = 0.96, web `interaction.buttonActive`).
pub fn press_scale_to(
    key: impl Into<ElementId>,
    pressed: bool,
    to: f32,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    animate(
        key,
        if pressed { to } else { 1.0 },
        Motion::FAST_EASE,
        window,
        cx,
    )
}

/// GPUI has no transforms, so a press scale is emulated: the element keeps
/// its layout box and paints its fill / stroke on this absolutely positioned
/// plate, inset by `(1 - scale) / 2` of the box on every side (a centered
/// scale). Put it as the first child of a `relative()` element.
pub fn scaled_plate(scale: f32) -> Div {
    let inset = gpui::relative(((1.0 - scale) / 2.0).max(0.0));
    gpui::div()
        .absolute()
        .top(inset)
        .bottom(inset)
        .left(inset)
        .right(inset)
}

/// `true` while a press scale is visibly away from 1.
pub fn is_scaled(scale: f32) -> bool {
    (1.0 - scale).abs() > 0.0005
}

/// Mount state of an overlay that animates in and out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Presence {
    /// Whether to render at all (open, or still animating out).
    pub mounted: bool,
    /// Eased progress, `0` = hidden, `1` = fully entered.
    pub progress: f32,
    /// `true` while entering or open; `false` while leaving.
    pub open: bool,
}

/// Enter / exit progress for `open`, using `enter` when opening and `exit` when closing.
/// The first render of an open overlay animates in from `0`.
pub fn presence(
    key: impl Into<ElementId>,
    open: bool,
    enter: Motion,
    exit: Motion,
    window: &mut Window,
    cx: &mut App,
) -> Presence {
    let target = if open { 1.0 } else { 0.0 };
    let progress = animate_from(
        key,
        0.0,
        target,
        if open { enter } else { exit },
        window,
        cx,
    );
    Presence {
        mounted: open || progress > 0.001,
        progress,
        open,
    }
}

/// Pointer state tracked across renders for one element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interaction {
    /// Pointer is over the element.
    pub hovered: bool,
    /// Primary button is held down on the element.
    pub pressed: bool,
}

impl Interaction {
    /// Pick a value by state: `pressed` wins over `hovered`, which wins over `rest`.
    pub fn pick<T>(self, rest: T, hover: T, pressed: T) -> T {
        if self.pressed {
            pressed
        } else if self.hovered {
            hover
        } else {
            rest
        }
    }
}

/// Keyed [`Interaction`] state for `id` (stored under `id` + `"interaction"`).
pub fn use_interaction(
    id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> (Entity<Interaction>, Interaction) {
    let state = window.use_keyed_state(child_id(id, "interaction"), cx, |_, _| {
        Interaction::default()
    });
    let value = *state.read(cx);
    (state, value)
}

/// Attach hover / press tracking to an interactive element.
pub trait TrackInteraction: Sized {
    /// Keep `state` in sync with the pointer.
    fn track_interaction(self, state: &Entity<Interaction>) -> Self;
}

impl TrackInteraction for Stateful<Div> {
    fn track_interaction(self, state: &Entity<Interaction>) -> Self {
        let hover = state.clone();
        let down = state.clone();
        let up = state.clone();
        let up_out = state.clone();
        self.on_hover(move |hovered, _, cx| {
            hover.update(cx, |s, cx| {
                if s.hovered != *hovered {
                    s.hovered = *hovered;
                    if !*hovered {
                        s.pressed = false;
                    }
                    cx.notify();
                }
            })
        })
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            down.update(cx, |s, cx| {
                if !s.pressed {
                    s.pressed = true;
                    cx.notify();
                }
            })
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            up.update(cx, |s, cx| {
                if s.pressed {
                    s.pressed = false;
                    cx.notify();
                }
            })
        })
        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
            up_out.update(cx, |s, cx| {
                if s.pressed {
                    s.pressed = false;
                    cx.notify();
                }
            })
        })
    }
}

/// A keyed, tab-stop [`FocusHandle`](gpui::FocusHandle) for `id`, so `render` can ask
/// whether the element is focused (`handle.is_focused(window)`). Attach it with
/// `.track_focus(&handle)`.
pub fn use_focus_handle(
    id: &ElementId,
    tab_index: isize,
    window: &mut Window,
    cx: &mut App,
) -> gpui::FocusHandle {
    let state = window.use_keyed_state(child_id(id, "focus"), cx, |_, cx| cx.focus_handle());
    state.read(cx).clone().tab_index(tab_index).tab_stop(true)
}

/// Last laid-out bounds of a container and its items — used to slide an
/// indicator (Tabs underline, SegmentedControl thumb, Command highlight) between
/// items, the way the web kit measures `offsetLeft` / `offsetWidth`.
#[derive(Clone, Debug, Default)]
pub struct LayoutProbe {
    /// Container bounds.
    pub container: Option<Bounds<Pixels>>,
    /// Item bounds by index.
    pub items: Vec<Option<Bounds<Pixels>>>,
}

impl LayoutProbe {
    /// Item `ix` relative to the container: `(left, top, width, height)`.
    pub fn relative(&self, ix: usize) -> Option<Bounds<Pixels>> {
        let container = self.container?;
        let item = (*self.items.get(ix)?)?;
        Some(Bounds::new(
            point(item.left() - container.left(), item.top() - container.top()),
            item.size,
        ))
    }
}

/// Keyed [`LayoutProbe`] for `id`.
pub fn use_layout_probe(id: &ElementId, window: &mut Window, cx: &mut App) -> Entity<LayoutProbe> {
    window.use_keyed_state(child_id(id, "probe"), cx, |_, _| LayoutProbe::default())
}

/// An invisible, absolutely positioned element that records its parent's bounds
/// into `probe` (`slot = None` → container, `Some(ix)` → item `ix`). Put it as a
/// child of a `relative()` element. Triggers one extra frame when bounds change.
pub fn probe(probe: &Entity<LayoutProbe>, slot: Option<usize>) -> impl IntoElement {
    let probe = probe.clone();
    canvas(
        move |bounds, window, cx| {
            let changed = probe.update(cx, |p, _| {
                let target = match slot {
                    None => &mut p.container,
                    Some(ix) => {
                        if p.items.len() <= ix {
                            p.items.resize(ix + 1, None);
                        }
                        &mut p.items[ix]
                    }
                };
                if *target != Some(bounds) {
                    *target = Some(bounds);
                    true
                } else {
                    false
                }
            });
            if changed {
                window.refresh();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Whether focus rings should show (`:focus-visible`): true after keyboard input,
/// false after pointer input. Maintained by [`crate::root`].
#[derive(Clone, Copy, Debug, Default)]
pub struct FocusVisible(pub bool);
impl gpui::Global for FocusVisible {}

/// `true` when the last input was from the keyboard.
pub fn focus_visible(cx: &App) -> bool {
    cx.try_global::<FocusVisible>().is_some_and(|f| f.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_endpoints() {
        let a = gpui::hsla(0.0, 1.0, 0.5, 1.0);
        let b = gpui::hsla(0.6, 1.0, 0.5, 1.0);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn mix_to_transparent_keeps_hue() {
        let white = gpui::hsla(0.0, 0.0, 1.0, 1.0);
        let m: Rgba = mix(white, gpui::transparent_black(), 0.5).into();
        assert!(
            (m.r - 1.0).abs() < 1e-3,
            "premultiplied mix should not darken"
        );
        assert!((m.a - 0.5).abs() < 1e-3);
    }

    #[test]
    fn pulse_shape() {
        assert!((skeleton_pulse_opacity(0.0) - 1.0).abs() < 1e-3);
        assert!((skeleton_pulse_opacity(0.5) - 0.5).abs() < 1e-3);
        assert!((skeleton_pulse_opacity(1.0) - 1.0).abs() < 1e-3);
    }
}
