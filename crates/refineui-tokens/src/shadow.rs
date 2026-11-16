//! Elevation shadows — `global/shadows.ts` + `semantic/shadows.ts`.
//!
//! Each level is a *key* + *ambient* layer pair, exactly like the web tokens.

use crate::{Color, Mode, palette};

/// One shadow layer (`x y blur spread color`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowLayer {
    /// Horizontal offset (px).
    pub x: f32,
    /// Vertical offset (px).
    pub y: f32,
    /// Blur radius (px).
    pub blur: f32,
    /// Spread radius (px).
    pub spread: f32,
    /// Layer color.
    pub color: Color,
}

/// Elevation steps (Figma `Shadow N`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Elevation {
    /// `shadow2` — `surface`
    Shadow2,
    /// `shadow4` — `raised`
    Shadow4,
    /// `shadow8` — `floating`
    Shadow8,
    /// `shadow16` — `overlay` / `toast`
    Shadow16,
    /// `shadow24` — `modal`
    Shadow24,
    /// `shadow32`
    Shadow32,
    /// `shadow64`
    Shadow64,
}

impl Elevation {
    /// Component role `surface`.
    pub const SURFACE: Self = Self::Shadow2;
    /// Component role `raised` (cards, buttons).
    pub const RAISED: Self = Self::Shadow4;
    /// Component role `floating` (menus, selects, tooltips).
    pub const FLOATING: Self = Self::Shadow8;
    /// Component role `overlay` (popovers).
    pub const OVERLAY: Self = Self::Shadow16;
    /// Component role `modal` (dialogs, drawers).
    pub const MODAL: Self = Self::Shadow24;
    /// Component role `toast`.
    pub const TOAST: Self = Self::Shadow16;

    /// `(key y/blur, ambient y/blur)` dimensions.
    const fn dims(self) -> ((f32, f32), (f32, f32)) {
        match self {
            Self::Shadow2 => ((1.0, 2.0), (2.0, 4.0)),
            Self::Shadow4 => ((2.0, 2.0), (4.0, 8.0)),
            Self::Shadow8 => ((4.0, 4.0), (8.0, 16.0)),
            Self::Shadow16 => ((8.0, 8.0), (16.0, 32.0)),
            Self::Shadow24 => ((12.0, 12.0), (24.0, 48.0)),
            Self::Shadow32 => ((16.0, 16.0), (32.0, 64.0)),
            Self::Shadow64 => ((24.0, 24.0), (48.0, 96.0)),
        }
    }

    /// Resolve the two layers for a mode.
    pub fn layers(self, mode: Mode) -> [ShadowLayer; 2] {
        let ((ky, kb), (mut ay, mut ab)) = self.dims();
        let (key, ambient) = match mode {
            Mode::Light => (
                palette::PRIMARY_BLACK.with_alpha(0.05),
                palette::PRIMARY_BLACK.with_alpha(0.04),
            ),
            Mode::Dark => {
                if self == Self::Shadow4 {
                    // Foundation override: Elevation Dark / Shadow 4 ambient `0 6px 10px 0`.
                    ay = 6.0;
                    ab = 10.0;
                }
                (
                    palette::NEUTRAL_BLACK.with_alpha(0.2),
                    palette::NEUTRAL_BLACK.with_alpha(0.15),
                )
            }
        };
        [
            ShadowLayer {
                x: 0.0,
                y: ky,
                blur: kb,
                spread: 0.0,
                color: key,
            },
            ShadowLayer {
                x: 0.0,
                y: ay,
                blur: ab,
                spread: 0.0,
                color: ambient,
            },
        ]
    }
}
