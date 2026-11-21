//! # refineui-foundation
//!
//! RefineUI foundation tokens as framework-agnostic Rust constants. This crate
//! is a faithful port of the web package `@refineui/tokens` and has **no
//! dependencies** — the `refineui` crate turns these values into GPUI types.
//!
//! Layering (same as the web design system):
//!
//! 1. **Global / foundation** — [`palette`], [`spacing`], [`radius`],
//!    [`typography`], [`shadow`], [`motion`], [`size`], [`stroke`], [`opacity`].
//! 2. **Semantic (alias)** — [`semantic::SemanticColors`], one resolved set per
//!    appearance ([`semantic::SemanticColors::LIGHT`] / [`semantic::SemanticColors::DARK`]).
//! 3. **Component** — lives in the `refineui` crate (desktop density aware).
//!
//! Desktop additions (RefineUI Desktop, Foundation layer): [`size::Density`]
//! (compact / comfortable / spacious, with [`size::DesktopMetrics`]),
//! [`z_index`] layer order, [`blur`] roles, [`material`] recipes (thin /
//! regular / thick / chrome / glass / scrim), per-platform [`window`] metrics,
//! and machine-readable component [`contracts`]. Everything here is plain
//! data with no GPUI dependency, so other renderers can share it.

#![warn(missing_docs)]

pub mod blur;
pub mod contracts;
pub mod material;
pub mod motion;
pub mod opacity;
pub mod palette;
pub mod radius;
pub mod semantic;
pub mod shadow;
pub mod size;
pub mod spacing;
pub mod stroke;
pub mod typography;
pub mod window;
pub mod z_index;

pub use material::{MaterialRole, MaterialTokens};
pub use semantic::SemanticColors;

/// An sRGB color stored as `0xRRGGBBAA`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub u32);

impl Color {
    /// Opaque color from `0xRRGGBB`.
    pub const fn rgb(hex: u32) -> Self {
        Self((hex << 8) | 0xFF)
    }

    /// Color from `0xRRGGBBAA`.
    pub const fn rgba(hex: u32) -> Self {
        Self(hex)
    }

    /// Red channel (0–255).
    pub const fn r(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Green channel (0–255).
    pub const fn g(self) -> u8 {
        (self.0 >> 16) as u8
    }

    /// Blue channel (0–255).
    pub const fn b(self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Alpha channel (0–255).
    pub const fn a(self) -> u8 {
        self.0 as u8
    }

    /// Same color with a new alpha in `0.0..=1.0` (const-friendly rounding).
    pub const fn with_alpha(self, alpha: f32) -> Self {
        let a = (alpha * 255.0 + 0.5) as u32;
        let a = if a > 255 { 255 } else { a };
        Self((self.0 & 0xFFFF_FF00) | a)
    }

    /// `#RRGGBB` or `#RRGGBBAA` (when not opaque).
    pub fn to_hex(self) -> String {
        if self.a() == 0xFF {
            format!("#{:02X}{:02X}{:02X}", self.r(), self.g(), self.b())
        } else {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r(),
                self.g(),
                self.b(),
                self.a()
            )
        }
    }
}

/// Light or dark appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Light appearance (default).
    #[default]
    Light,
    /// Dark appearance.
    Dark,
}

impl Mode {
    /// Semantic colors for this mode.
    pub const fn colors(self) -> &'static SemanticColors {
        match self {
            Mode::Light => &SemanticColors::LIGHT,
            Mode::Dark => &SemanticColors::DARK,
        }
    }

    /// The other mode.
    pub const fn toggled(self) -> Self {
        match self {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Light,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_packing() {
        let c = Color::rgb(0x2351F7);
        assert_eq!((c.r(), c.g(), c.b(), c.a()), (0x23, 0x51, 0xF7, 0xFF));
        assert_eq!(c.to_hex(), "#2351F7");
        assert_eq!(c.with_alpha(0.5).a(), 128);
    }

    #[test]
    fn palette_matches_web_tokens() {
        assert_eq!(palette::PRIMARY_BLACK, Color::rgb(0x212121));
        assert_eq!(palette::BLUE_800, Color::rgb(0x2351F7));
        assert_eq!(palette::ALL.len(), 114);
    }

    #[test]
    fn semantic_pairs_resolve() {
        let light = Mode::Light.colors();
        let dark = Mode::Dark.colors();
        assert_eq!(light.background_brand, palette::PRIMARY_BLACK);
        assert_eq!(dark.background_brand, palette::PRIMARY_GRAY);
        assert_eq!(light.foreground_primary, palette::PRIMARY_BLACK);
        assert_eq!(dark.foreground_primary, palette::NEUTRAL_WHITE);
        assert_eq!(light.entries().len(), 71);
    }

    #[test]
    fn typography_scale() {
        assert_eq!(typography::BODY_3.size, 14.0);
        assert_eq!(typography::HEADING_1.weight, typography::FontWeight::HEAVY);
    }
}
