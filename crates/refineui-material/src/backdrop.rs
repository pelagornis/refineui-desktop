//! Backdrop blur: capability, settings and the paint call.

use gpui::{App, BackdropBlurStyle, Bounds, Global, Pixels, Window};

/// What the current renderer can do for backdrop blur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurSupport {
    /// Real blur of the pixels behind the surface (Blade on Linux / FreeBSD,
    /// Metal on macOS).
    Real,
    /// No blur in this renderer (DirectX on Windows): materials draw their
    /// opaque fallback fill.
    Fallback,
}

impl BlurSupport {
    /// Support in this build.
    pub const fn current() -> Self {
        if gpui::backdrop_blur_supported() {
            Self::Real
        } else {
            Self::Fallback
        }
    }
}

/// `true` when the renderer draws real backdrop blur.
pub const fn backdrop_blur_supported() -> bool {
    gpui::backdrop_blur_supported()
}

/// App-wide material settings (a GPUI global).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaterialSettings {
    /// Accessibility "reduce transparency": draw fallback fills, no blur.
    pub reduced_transparency: bool,
}

impl Global for MaterialSettings {}

impl MaterialSettings {
    /// Current settings (defaults when never set).
    pub fn get(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    /// Replace the settings and refresh every window.
    pub fn set(settings: Self, cx: &mut App) {
        cx.set_global(settings);
        cx.refresh_windows();
    }

    /// Whether surfaces should draw real blur right now.
    pub fn blur_enabled(cx: &App) -> bool {
        backdrop_blur_supported() && !Self::get(cx).reduced_transparency
    }
}

/// Paint a real backdrop blur (no-op where unsupported; check
/// [`MaterialSettings::blur_enabled`] first and paint a fallback instead).
pub fn paint_backdrop(window: &mut Window, bounds: Bounds<Pixels>, style: BackdropBlurStyle) {
    window.paint_backdrop_blur(bounds, style);
}
