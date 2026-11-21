//! Shadow and elevation: foundation elevation levels as GPUI shadows, plus
//! presets for directional and layered shadows.
//!
//! Drop shadows are analytic Gaussian rounded rectangles (no extra pass).
//! On the real-blur path the composite pass draws them *outside* the shape
//! only, so a surface's own shadow is never blurred into its backdrop.
//!
//! [`InnerShadow`] is drawn inside the shape as a stack of rounded borders
//! (one per device-ish pixel of falloff), which follows the corner radius
//! exactly and needs no renderer support.

use gpui::{
    Bounds, BoxShadow, Corners, Edges, Hsla, Pixels, Point, Window, point, px, quad,
    transparent_black,
};
use refineui_foundation::{Mode, shadow::Elevation};

use crate::hsla;

/// Key + ambient layers of a foundation elevation level.
pub fn elevation_shadows(elevation: Elevation, mode: Mode) -> Vec<BoxShadow> {
    elevation
        .layers(mode)
        .iter()
        .map(|l| BoxShadow {
            color: hsla(l.color),
            offset: point(px(l.x), px(l.y)),
            blur_radius: px(l.blur),
            spread_radius: px(l.spread),
        })
        .collect()
}

/// Shadow presets beyond the elevation scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShadowPreset {
    /// A foundation elevation level (key + ambient).
    Elevation(Elevation),
    /// Light from `angle` degrees (0 = top): the shadow falls the other way,
    /// `distance` px, softened by `blur` px.
    Directional {
        /// Light direction in degrees.
        angle: f32,
        /// Offset length.
        distance: f32,
        /// Blur radius.
        blur: f32,
        /// Shadow color.
        color: Hsla,
    },
    /// Three stacked layers (contact, mid, ambient) for floating glass.
    Layered(Mode),
    /// Even, offset-free glow all round (`blur` px).
    Ambient {
        /// Blur radius.
        blur: f32,
        /// Shadow color.
        color: Hsla,
    },
}

impl ShadowPreset {
    /// Resolve to shadow layers.
    pub fn shadows(self, mode: Mode) -> Vec<BoxShadow> {
        match self {
            Self::Elevation(e) => elevation_shadows(e, mode),
            Self::Directional {
                angle,
                distance,
                blur,
                color,
            } => {
                let r = angle.to_radians();
                vec![BoxShadow {
                    color,
                    offset: point(px(-r.sin() * distance), px(r.cos() * distance)),
                    blur_radius: px(blur),
                    spread_radius: px(0.),
                }]
            }
            Self::Ambient { blur, color } => vec![BoxShadow {
                color,
                offset: point(px(0.), px(0.)),
                blur_radius: px(blur),
                spread_radius: px(0.),
            }],
            Self::Layered(m) => {
                let k: f32 = if m == Mode::Dark { 2.2 } else { 1.0 };
                [(1., 2., 0.06_f32), (6., 16., 0.06), (18., 48., 0.08)]
                    .into_iter()
                    .map(|(y, b, a)| BoxShadow {
                        color: gpui::black().opacity((a * k).min(0.5)),
                        offset: point(px(0.), px(y)),
                        blur_radius: px(b),
                        spread_radius: px(0.),
                    })
                    .collect()
            }
        }
    }
}

/// A shadow cast *inside* a surface (CSS `box-shadow: inset`): pressed
/// controls, wells, glass thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InnerShadow {
    /// Shadow color.
    pub color: Hsla,
    /// Offset: a positive `y` darkens the top edge (light from above).
    pub offset: Point<Pixels>,
    /// Falloff distance.
    pub blur: Pixels,
    /// Solid band before the falloff starts.
    pub spread: Pixels,
}

impl InnerShadow {
    /// A pressed control: a soft dark band along the top.
    pub fn pressed(mode: Mode) -> Self {
        Self {
            color: gpui::black().opacity(if mode == Mode::Dark { 0.45 } else { 0.14 }),
            offset: point(px(0.), px(1.5)),
            blur: px(4.),
            spread: px(0.),
        }
    }

    /// A recessed well (inputs, tracks): even darkening all round.
    pub fn well(mode: Mode) -> Self {
        Self {
            color: gpui::black().opacity(if mode == Mode::Dark { 0.35 } else { 0.08 }),
            offset: point(px(0.), px(1.)),
            blur: px(6.),
            spread: px(0.),
        }
    }

    /// Glass thickness: a light inner glow around the rim.
    pub fn glass_rim(mode: Mode) -> Self {
        Self {
            color: gpui::white().opacity(if mode == Mode::Dark { 0.10 } else { 0.30 }),
            offset: point(px(0.), px(0.)),
            blur: px(10.),
            spread: px(0.),
        }
    }

    /// Paint inside `bounds` (rounded by `radius`) at `opacity`.
    ///
    /// Nested rounded borders of growing width, each with an equal share of
    /// the alpha, add up to a smooth ramp from `color` at the edge to
    /// nothing at `spread + blur` (offset shifts it per side).
    pub fn paint(&self, bounds: Bounds<Pixels>, radius: Pixels, opacity: f32, window: &mut Window) {
        let alpha = self.color.a * opacity;
        if alpha <= 0. {
            return;
        }
        let scale = window.scale_factor();
        let blur = f32::from(self.blur).max(0.);
        let steps = ((blur * scale).ceil() as usize).clamp(1, 16);
        // Equal per-layer alpha so the overlap of all layers is `alpha`.
        let layer_alpha = 1. - (1. - alpha.min(0.999)).powf(1. / steps as f32);
        let color = self.color.opacity(layer_alpha);
        let (ox, oy) = (f32::from(self.offset.x), f32::from(self.offset.y));
        let spread = f32::from(self.spread).max(0.);
        let max_width = f32::from(bounds.size.width.min(bounds.size.height)) / 2.;
        for i in 0..steps {
            let w = spread + blur * (i + 1) as f32 / steps as f32;
            let side = |v: f32| px((w + v).clamp(0., max_width));
            let widths = Edges {
                top: side(oy),
                right: side(-ox),
                bottom: side(-oy),
                left: side(ox),
            };
            window.paint_quad(quad(
                bounds,
                Corners::all(radius),
                transparent_black(),
                widths,
                color,
                Default::default(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_shadow_presets_are_inset_and_soft() {
        for mode in [Mode::Light, Mode::Dark] {
            let p = InnerShadow::pressed(mode);
            assert!(p.offset.y > px(0.) && p.blur > px(0.));
            assert!(InnerShadow::well(mode).color.a > 0.);
            assert!(InnerShadow::glass_rim(mode).color.l > 0.9);
        }
        let ambient = ShadowPreset::Ambient {
            blur: 20.,
            color: gpui::black(),
        }
        .shadows(Mode::Light);
        assert_eq!(ambient[0].offset, point(px(0.), px(0.)));
    }

    #[test]
    fn directional_shadow_falls_away_from_light() {
        let s = ShadowPreset::Directional {
            angle: 0.,
            distance: 8.,
            blur: 12.,
            color: gpui::black(),
        }
        .shadows(Mode::Light);
        assert!(s[0].offset.y > px(0.));
        assert_eq!(elevation_shadows(Elevation::MODAL, Mode::Light).len(), 2);
        assert_eq!(
            ShadowPreset::Layered(Mode::Dark).shadows(Mode::Dark).len(),
            3
        );
    }
}
