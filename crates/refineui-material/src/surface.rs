//! [`Material`]: one description of a surface.

use gpui::{BackdropShadow, BoxShadow, Hsla, Pixels, px};
use refineui_foundation::{MaterialRole, Mode};

use crate::{BlendMode, InnerShadow, Lighting, Mask, hsla, shadow::elevation_shadows};

/// Drop shadows the renderer draws with the blur (more are painted by GPUI).
pub const MAX_BACKDROP_SHADOWS: usize = 4;

/// Everything that defines how a material surface looks: backdrop
/// (blur, saturation), tint and its blend mode, opacity, mask, border,
/// radius, lighting, grain, drop and inner shadows. Build one from a foundation role with [`Material::role`] and
/// adjust with the builder methods.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// Backdrop blur radius (Gaussian σ, logical px). 0 = no blur.
    pub blur: Pixels,
    /// Backdrop saturation (1 = unchanged).
    pub saturation: f32,
    /// Tint over the blurred backdrop (alpha = amount).
    pub tint: Hsla,
    /// How the tint combines with the backdrop.
    pub tint_blend: BlendMode,
    /// Optional linear fade across the surface.
    pub mask: Option<Mask>,
    /// Opacity of the whole surface (backdrop + tint + light).
    pub opacity: f32,
    /// Corner radius.
    pub radius: Pixels,
    /// Hairline border color (`None` = no border).
    pub border: Option<Hsla>,
    /// Border width (default 1px).
    pub border_width: Pixels,
    /// Lighting (directional, ambient, edge, specular, reflection).
    pub lighting: Lighting,
    /// Grain amplitude (0 = off).
    pub noise: f32,
    /// Drop shadows (outside the shape).
    pub shadows: Vec<BoxShadow>,
    /// Inner shadows (inside the shape, above the plate).
    pub inner_shadows: Vec<InnerShadow>,
    /// Fill used when blur is unavailable or reduced transparency is on.
    pub fallback: Hsla,
}

impl Material {
    /// The foundation recipe for `role` in `mode`, with radius 16.
    pub fn role(role: MaterialRole, mode: Mode) -> Self {
        let t = role.tokens(mode);
        Self {
            blur: px(t.blur),
            saturation: t.saturation,
            tint: hsla(t.tint),
            tint_blend: BlendMode::Normal,
            mask: None,
            opacity: 1.,
            radius: px(16.),
            border: (t.border.a() > 0).then(|| hsla(t.border)),
            border_width: px(1.),
            lighting: if t.reflection > 0. || t.specular > 0. {
                Lighting::glass(t.edge_highlight, t.reflection, t.specular)
            } else {
                Lighting::edge(t.edge_highlight)
            },
            noise: t.noise,
            shadows: t
                .elevation
                .map(|e| elevation_shadows(e, mode))
                .unwrap_or_default(),
            inner_shadows: Vec::new(),
            fallback: hsla(t.fallback),
        }
    }

    /// Clear glass (`MaterialRole::Glass`).
    pub fn glass(mode: Mode) -> Self {
        Self::role(MaterialRole::Glass, mode)
    }

    /// Default frosted surface (`MaterialRole::Regular`).
    pub fn regular(mode: Mode) -> Self {
        Self::role(MaterialRole::Regular, mode)
    }

    /// Blur radius.
    pub fn blur(mut self, radius: Pixels) -> Self {
        self.blur = radius;
        self
    }

    /// Backdrop saturation.
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation;
        self
    }

    /// Tint color.
    pub fn tint(mut self, tint: Hsla) -> Self {
        self.tint = tint;
        self
    }

    /// Tint blend mode.
    pub fn tint_blend(mut self, mode: BlendMode) -> Self {
        self.tint_blend = mode;
        self
    }

    /// Linear fade mask.
    pub fn mask(mut self, mask: Mask) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Add an inner shadow.
    pub fn inner_shadow(mut self, shadow: InnerShadow) -> Self {
        self.inner_shadows.push(shadow);
        self
    }

    /// Whole-surface opacity.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Corner radius.
    pub fn radius(mut self, radius: Pixels) -> Self {
        self.radius = radius;
        self
    }

    /// Border color (or none).
    pub fn border(mut self, border: Option<Hsla>) -> Self {
        self.border = border;
        self
    }

    /// Border width (e.g. a 0.5px hairline).
    pub fn border_width(mut self, width: Pixels) -> Self {
        self.border_width = width;
        self
    }

    /// Lighting.
    pub fn lighting(mut self, lighting: Lighting) -> Self {
        self.lighting = lighting;
        self
    }

    /// Grain opacity.
    pub fn noise(mut self, noise: f32) -> Self {
        self.noise = noise;
        self
    }

    /// Shadow layers.
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> Self {
        self.shadows = shadows;
        self
    }

    /// Fallback fill.
    pub fn fallback(mut self, fallback: Hsla) -> Self {
        self.fallback = fallback;
        self
    }

    /// The blur style handed to the renderer: blur, tint, blend, light,
    /// grain, mask and the first [`MAX_BACKDROP_SHADOWS`] drop shadows.
    pub fn backdrop_style(&self) -> gpui::BackdropBlurStyle {
        let mut shadows = [None; MAX_BACKDROP_SHADOWS];
        for (slot, s) in shadows.iter_mut().zip(&self.shadows) {
            *slot = Some(BackdropShadow {
                color: s.color,
                offset: s.offset,
                blur_radius: s.blur_radius,
                spread_radius: s.spread_radius,
            });
        }
        gpui::BackdropBlurStyle {
            radius: self.blur,
            corner_radii: self.radius.into(),
            tint: self.tint,
            saturation: self.saturation,
            opacity: self.opacity,
            tint_blend: self.tint_blend.to_backdrop(),
            noise: self.noise,
            light: self.lighting.to_backdrop(),
            mask: self.mask.map(Mask::to_backdrop),
            shadows,
        }
    }

    /// Shadows the renderer cannot take with the blur (painted by GPUI).
    pub fn extra_shadows(&self) -> &[BoxShadow] {
        self.shadows.get(MAX_BACKDROP_SHADOWS..).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_resolves_foundation_tokens() {
        for mode in [Mode::Light, Mode::Dark] {
            for role in MaterialRole::ALL {
                let m = Material::role(role, mode);
                let t = role.tokens(mode);
                assert_eq!(m.blur, px(t.blur));
                assert_eq!(m.saturation, t.saturation);
                assert_eq!(m.shadows.is_empty(), t.elevation.is_none());
                assert!(m.fallback.a >= m.tint.a);
            }
        }
    }

    #[test]
    fn backdrop_style_carries_shape_and_tint() {
        let m = Material::glass(Mode::Dark).radius(px(12.)).opacity(0.5);
        let s = m.backdrop_style();
        assert_eq!(s.corner_radii.top_left, px(12.));
        assert_eq!(s.opacity, 0.5);
        assert_eq!(s.tint, m.tint);
        assert_eq!(s.noise, m.noise);
        // Elevation shadows travel with the blur.
        assert_eq!(s.shadows.iter().flatten().count(), m.shadows.len().min(4));
        assert!(m.extra_shadows().is_empty());
    }

    #[test]
    fn glass_role_gets_reflection_and_specular() {
        let g = Material::glass(Mode::Light);
        assert!(g.lighting.reflection > 0. && g.lighting.specular > 0.);
        let r = Material::regular(Mode::Light);
        assert_eq!(r.lighting.reflection, 0.);
        let masked = r
            .clone()
            .mask(Mask::fade_bottom(0.5))
            .tint_blend(BlendMode::Multiply);
        let s = masked.backdrop_style();
        assert!(s.mask.is_some());
        assert_eq!(s.tint_blend, gpui::BackdropBlend::Multiply);
    }
}
