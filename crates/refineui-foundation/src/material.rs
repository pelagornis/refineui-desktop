//! Material tokens — the data behind the Material Engine (`refineui-material`).
//!
//! A material is a recipe for a surface that shows what is behind it:
//! backdrop blur, saturation, a tint, an edge highlight, a hairline border,
//! optional grain and an elevation. The values here are plain data so any
//! renderer can share them; `refineui-material` turns them into GPUI paint
//! calls (real backdrop blur where the renderer supports it, the opaque
//! [`MaterialTokens::fallback`] fill where it does not).
//!
//! Roles follow desktop practice (thin → thick by how much they hide), plus
//! `Glass` (clear, vivid, strongly lit) and `Scrim` (modal backdrop). Tints
//! are the semantic `backgroundPrimary` of each appearance, so materials stay
//! on-brand when the palette changes.

use crate::{Color, Mode, blur, palette, semantic::SemanticColors, shadow::Elevation};

/// Material roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialRole {
    /// Light frosting for small transient surfaces (tooltips, peeks).
    Thin,
    /// Default frosted surface (menus, popovers, command dialog).
    Regular,
    /// Heavy frosting; content behind is only a soft color field (sheets).
    Thick,
    /// Window chrome (titlebars, sidebars, toolbars) over scrolled content.
    Chrome,
    /// Clear glass: little tint, vivid backdrop, lit edges.
    Glass,
    /// Modal scrim: a slight blur and darkening behind dialogs.
    Scrim,
}

/// Resolved material values for one appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialTokens {
    /// Backdrop blur radius (Gaussian σ in px, CSS `blur()` semantics).
    pub blur: f32,
    /// Backdrop saturation (1 = unchanged).
    pub saturation: f32,
    /// Tint laid over the blurred backdrop (alpha = amount).
    pub tint: Color,
    /// Strength of the top-edge highlight (white alpha at the edge).
    pub edge_highlight: f32,
    /// Hairline border.
    pub border: Color,
    /// Grain amplitude (0 = off).
    pub noise: f32,
    /// Glossy reflection band strength (glass only).
    pub reflection: f32,
    /// Soft specular highlight strength (glass only).
    pub specular: f32,
    /// Drop shadow under the surface, if any.
    pub elevation: Option<Elevation>,
    /// Fill used when the renderer cannot blur (or reduced transparency is
    /// on). Nearly opaque so text stays readable over any backdrop.
    pub fallback: Color,
}

impl MaterialRole {
    /// Every role.
    pub const ALL: [MaterialRole; 6] = [
        Self::Thin,
        Self::Regular,
        Self::Thick,
        Self::Chrome,
        Self::Glass,
        Self::Scrim,
    ];

    /// Stable token name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Thin => "thin",
            Self::Regular => "regular",
            Self::Thick => "thick",
            Self::Chrome => "chrome",
            Self::Glass => "glass",
            Self::Scrim => "scrim",
        }
    }

    /// Resolve the role for an appearance.
    pub fn tokens(self, mode: Mode) -> MaterialTokens {
        let dark = mode == Mode::Dark;
        let semantic = if dark {
            SemanticColors::DARK
        } else {
            SemanticColors::LIGHT
        };
        let base = semantic.background_primary;
        let pick = |light: f32, dark_value: f32| if dark { dark_value } else { light };
        let white = palette::NEUTRAL_WHITE;
        let hairline = if dark {
            white.with_alpha(0.10)
        } else {
            palette::PRIMARY_BLACK.with_alpha(0.08)
        };
        match self {
            Self::Thin => MaterialTokens {
                blur: blur::BLUR_12,
                saturation: 1.6,
                tint: base.with_alpha(pick(0.45, 0.40)),
                edge_highlight: pick(0.50, 0.10),
                border: hairline,
                noise: 0.0,
                reflection: 0.0,
                specular: 0.0,
                elevation: Some(Elevation::RAISED),
                fallback: base.with_alpha(0.94),
            },
            Self::Regular => MaterialTokens {
                blur: blur::role::SURFACE_FROST,
                saturation: 1.8,
                tint: base.with_alpha(pick(0.62, 0.58)),
                edge_highlight: pick(0.60, 0.12),
                border: hairline,
                noise: 0.0,
                reflection: 0.0,
                specular: 0.0,
                elevation: Some(Elevation::FLOATING),
                fallback: base.with_alpha(0.97),
            },
            Self::Thick => MaterialTokens {
                blur: 28.0,
                saturation: 1.8,
                tint: base.with_alpha(pick(0.78, 0.74)),
                edge_highlight: pick(0.60, 0.12),
                border: hairline,
                noise: 0.0,
                reflection: 0.0,
                specular: 0.0,
                elevation: Some(Elevation::OVERLAY),
                fallback: base,
            },
            Self::Chrome => MaterialTokens {
                blur: 24.0,
                saturation: 1.8,
                tint: base.with_alpha(pick(0.72, 0.66)),
                edge_highlight: 0.0,
                border: hairline,
                noise: 0.0,
                reflection: 0.0,
                specular: 0.0,
                elevation: None,
                fallback: base,
            },
            Self::Glass => MaterialTokens {
                blur: 18.0,
                saturation: 1.9,
                tint: base.with_alpha(pick(0.22, 0.16)),
                edge_highlight: pick(0.85, 0.28),
                border: white.with_alpha(pick(0.45, 0.14)),
                noise: 0.025,
                reflection: pick(0.30, 0.12),
                specular: pick(0.35, 0.12),
                elevation: Some(Elevation::FLOATING),
                fallback: base.with_alpha(0.92),
            },
            Self::Scrim => MaterialTokens {
                blur: blur::role::OVERLAY_SCRIM,
                saturation: 1.0,
                tint: palette::PRIMARY_BLACK.with_alpha(pick(0.10, 0.35)),
                edge_highlight: 0.0,
                border: Color::rgba(0),
                noise: 0.0,
                reflection: 0.0,
                specular: 0.0,
                elevation: None,
                fallback: palette::PRIMARY_BLACK.with_alpha(pick(0.18, 0.45)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(c: Color) -> u8 {
        c.a()
    }

    #[test]
    fn thickness_orders_tint_and_blur() {
        for mode in [Mode::Light, Mode::Dark] {
            let t = |r: MaterialRole| r.tokens(mode);
            let (thin, regular, thick) = (
                t(MaterialRole::Thin),
                t(MaterialRole::Regular),
                t(MaterialRole::Thick),
            );
            assert!(alpha(thin.tint) < alpha(regular.tint));
            assert!(alpha(regular.tint) < alpha(thick.tint));
            assert!(thin.blur < regular.blur && regular.blur < thick.blur);
            // Glass is the clearest surface.
            assert!(alpha(t(MaterialRole::Glass).tint) < alpha(thin.tint));
        }
    }

    #[test]
    fn every_role_has_a_readable_fallback() {
        for mode in [Mode::Light, Mode::Dark] {
            for role in MaterialRole::ALL {
                let m = role.tokens(mode);
                assert!(m.blur > 0.0, "{} must blur", role.name());
                assert!(m.saturation >= 1.0);
                if role != MaterialRole::Scrim {
                    // Fallback hides the backdrop far more than the tint does.
                    assert!(alpha(m.fallback) >= 230, "{}", role.name());
                    assert!(alpha(m.fallback) > alpha(m.tint));
                }
            }
        }
    }
}
