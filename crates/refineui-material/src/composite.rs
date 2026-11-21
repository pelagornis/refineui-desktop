//! Compositing: blend modes, masks and the rules for alpha, ordering,
//! clipping and nesting.
//!
//! - **Alpha**: `Material::opacity` scales the whole surface (plate, light,
//!   grain, border, shadows) and multiplies with GPUI's element opacity.
//! - **Ordering**: a surface samples everything painted before it under its
//!   blur footprint and draws below its own children. The renderer orders a
//!   blur after every earlier primitive it reads, also inside text layers.
//! - **Clipping**: the plate is clipped to its rounded shape (anti-aliased
//!   SDF) and to the current content mask (`overflow_hidden` ancestors).
//! - **Masks**: [`Mask`] fades the plate linearly across the shape.
//! - **Blend**: [`BlendMode`] sets how the tint combines with the blurred
//!   backdrop.
//! - **Nesting**: an inner surface blurs what the outer one composited, so
//!   materials stack like real panes.

use gpui::{BackdropBlend, BackdropMask, point};

/// How a material's tint combines with the blurred backdrop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Mix the tint over the backdrop (alpha = amount). Frosted surfaces.
    #[default]
    Normal,
    /// Darken by the tint: colored, smoked glass.
    Multiply,
    /// Lighten by the tint: milky, luminous glass.
    Screen,
    /// Contrast: multiply darks, screen lights.
    Overlay,
    /// Gentle contrast (W3C soft-light).
    SoftLight,
}

impl BlendMode {
    /// Every mode.
    pub const ALL: [BlendMode; 5] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::SoftLight,
    ];

    /// Display name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::SoftLight => "soft-light",
        }
    }

    pub(crate) fn to_backdrop(self) -> BackdropBlend {
        match self {
            Self::Normal => BackdropBlend::Normal,
            Self::Multiply => BackdropBlend::Multiply,
            Self::Screen => BackdropBlend::Screen,
            Self::Overlay => BackdropBlend::Overlay,
            Self::SoftLight => BackdropBlend::SoftLight,
        }
    }
}

/// A linear alpha mask across a surface: opaque before `start`,
/// transparent after `end` (positions 0–1 across the shape along `angle`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mask {
    /// Direction the fade runs towards, degrees (0 = top, 180 = bottom).
    pub angle: f32,
    /// Fade start.
    pub start: f32,
    /// Fade end.
    pub end: f32,
}

impl Mask {
    /// Fade towards `angle` between `start` and `end`.
    pub fn linear(angle: f32, start: f32, end: f32) -> Self {
        Self { angle, start, end }
    }

    /// Solid at the top, fading out towards the bottom from `start`.
    pub fn fade_bottom(start: f32) -> Self {
        Self::linear(180., start, 1.)
    }

    /// Solid at the bottom, fading out towards the top.
    pub fn fade_top(start: f32) -> Self {
        Self::linear(0., start, 1.)
    }

    pub(crate) fn to_backdrop(self) -> BackdropMask {
        let r = self.angle.to_radians();
        BackdropMask {
            direction: point(r.sin(), -r.cos()),
            start: self.start,
            end: self.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_point_where_they_fade() {
        let m = Mask::fade_bottom(0.4).to_backdrop();
        assert!(m.direction.y > 0.99);
        assert_eq!((m.start, m.end), (0.4, 1.));
        assert_eq!(BlendMode::ALL.len(), 5);
        assert_eq!(BlendMode::Screen.to_backdrop(), BackdropBlend::Screen);
    }
}
