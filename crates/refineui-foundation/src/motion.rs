//! Motion — `global/motion.ts` + semantic roles from `semantic/interaction.ts`.

/// Semantic durations in milliseconds.
pub mod duration {
    /// `instant`
    pub const INSTANT: u64 = 0;
    /// `fast` — hover / press feedback.
    pub const FAST: u64 = 150;
    /// `normal`
    pub const NORMAL: u64 = 180;
    /// `medium`
    pub const MEDIUM: u64 = 200;
    /// `slow`
    pub const SLOW: u64 = 240;
    /// `overlay` — popovers, menus.
    pub const OVERLAY: u64 = 280;
    /// `panel` — drawers, sheets.
    pub const PANEL: u64 = 320;
    /// `accordionPanel`
    pub const ACCORDION_PANEL: u64 = 380;
    /// `accordionContent`
    pub const ACCORDION_CONTENT: u64 = 260;
    /// `toastEnter`
    pub const TOAST_ENTER: u64 = 280;
    /// `toastLeave`
    pub const TOAST_LEAVE: u64 = 200;
    /// `skeleton` pulse cycle.
    pub const SKELETON: u64 = 1500;
    /// `spinner` rotation cycle.
    pub const SPINNER: u64 = 800;
}

/// A cubic-bezier easing curve `(x1, y1, x2, y2)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier(pub f32, pub f32, pub f32, pub f32);

impl CubicBezier {
    /// Evaluate the curve's y for a time `t` in `0..=1` (Newton–Raphson on x).
    pub fn ease(&self, t: f32) -> f32 {
        let CubicBezier(x1, y1, x2, y2) = *self;
        let bez = |p1: f32, p2: f32, s: f32| {
            let u = 1.0 - s;
            3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s
        };
        let d_bez = |p1: f32, p2: f32, s: f32| {
            let u = 1.0 - s;
            3.0 * u * u * p1 + 6.0 * u * s * (p2 - p1) + 3.0 * s * s * (1.0 - p2)
        };
        let t = t.clamp(0.0, 1.0);
        let mut s = t;
        for _ in 0..8 {
            let x = bez(x1, x2, s) - t;
            let dx = d_bez(x1, x2, s);
            if dx.abs() < 1e-6 {
                break;
            }
            s = (s - x / dx).clamp(0.0, 1.0);
        }
        bez(y1, y2, s)
    }
}

/// Semantic easing curves.
pub mod easing {
    use super::CubicBezier;
    /// `standard` — `cubic-bezier(0.2, 0, 0, 1)`.
    pub const STANDARD: CubicBezier = CubicBezier(0.2, 0.0, 0.0, 1.0);
    /// `emphasized` — `cubic-bezier(0.16, 1, 0.3, 1)`.
    pub const EMPHASIZED: CubicBezier = CubicBezier(0.16, 1.0, 0.3, 1.0);
    /// `panel` — `cubic-bezier(0.32, 0.72, 0, 1)`.
    pub const PANEL: CubicBezier = CubicBezier(0.32, 0.72, 0.0, 1.0);
    /// `content` — `cubic-bezier(0.4, 0, 0.2, 1)`.
    pub const CONTENT: CubicBezier = CubicBezier(0.4, 0.0, 0.2, 1.0);
    /// `easeOut`
    pub const EASE_OUT: CubicBezier = CubicBezier(0.0, 0.0, 0.58, 1.0);
    /// CSS `ease-in` — `cubic-bezier(0.42, 0, 1, 1)`. Exit counterpart of `easeOut`.
    pub const EASE_IN: CubicBezier = CubicBezier(0.42, 0.0, 1.0, 1.0);
    /// `easeInOut`
    pub const EASE_IN_OUT: CubicBezier = CubicBezier(0.42, 0.0, 0.58, 1.0);
    /// `linear`
    pub const LINEAR: CubicBezier = CubicBezier(0.0, 0.0, 1.0, 1.0);
    /// CSS `ease` — the browser default when a transition names no easing.
    pub const EASE: CubicBezier = CubicBezier(0.25, 0.1, 0.25, 1.0);
}

/// Semantic scale factors.
pub mod scale {
    /// `enter` — dialog enter under-scale.
    pub const ENTER: f32 = 0.96;
    /// `press` — pressed control squash.
    pub const PRESS: f32 = 0.98;
    /// Button `:active` scale (`interaction.buttonActive` = `scale96`).
    pub const BUTTON_PRESS: f32 = 0.96;
    /// `hoverGrow` — slider thumb hover.
    pub const HOVER_GROW: f32 = 1.15;
}

/// `distance.float` — small enter offset (px).
pub const FLOAT_DISTANCE: f32 = 4.0;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bezier_endpoints() {
        for c in [
            easing::STANDARD,
            easing::EMPHASIZED,
            easing::PANEL,
            easing::CONTENT,
        ] {
            assert!(c.ease(0.0).abs() < 1e-3);
            assert!((c.ease(1.0) - 1.0).abs() < 1e-3);
        }
    }
}
