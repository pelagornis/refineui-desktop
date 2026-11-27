//! Backdrop blur radii — `global/blurs.ts` + `semantic/blur.ts`.
//!
//! Radii are Gaussian σ in px (CSS `blur()` semantics). `refineui-material`
//! draws them as real in-window backdrop blur (patched GPUI renderer).

/// `blurNone`
pub const NONE: f32 = 0.0;
/// `blur2`
pub const BLUR_2: f32 = 2.0;
/// `blur4`
pub const BLUR_4: f32 = 4.0;
/// `blur8`
pub const BLUR_8: f32 = 8.0;
/// `blur12`
pub const BLUR_12: f32 = 12.0;
/// `blur16`
pub const BLUR_16: f32 = 16.0;

/// Semantic blur roles (`semanticBlur`).
pub mod role {
    /// `overlayScrim` — modal scrims.
    pub const OVERLAY_SCRIM: f32 = super::BLUR_2;
    /// `surfaceFrost` — frosted floating panels (menus, popovers, peek).
    pub const SURFACE_FROST: f32 = super::BLUR_16;
    /// `scrollEdge` — scroll-area edge softening.
    pub const SCROLL_EDGE: f32 = super::BLUR_2;
}
