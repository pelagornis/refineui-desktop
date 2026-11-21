//! Recipes: ready-made materials for common desktop surfaces.
//!
//! Each recipe starts from a foundation [`MaterialRole`] and sets the radius,
//! elevation and light that surface uses in RefineUI. Adjust the result with
//! the [`Material`] builders.
//!
//! ```ignore
//! material_surface(recipes::popover(mode)).p_3().child(menu_items);
//! ```

use gpui::{Hsla, px};
use refineui_foundation::{MaterialRole, Mode, palette, radius, shadow::Elevation};

use crate::{InnerShadow, Lighting, Material, ShadowPreset, hsla, shadow::elevation_shadows};

/// A named recipe (for galleries and settings UIs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Recipe {
    /// Source list / navigation column over the window content.
    Sidebar,
    /// Window titlebar / unified toolbar over scrolling content.
    Titlebar,
    /// Popover and dropdown panels.
    Popover,
    /// Context and dropdown menus.
    Menu,
    /// Tooltips.
    Tooltip,
    /// Heads-up display: dark smoked glass for transient status (volume,
    /// progress, capture).
    Hud,
    /// Sheets and dialogs.
    Sheet,
    /// Toast notifications.
    Toast,
    /// Modal backdrop.
    Scrim,
    /// Expressive glass card.
    Glass,
}

impl Recipe {
    /// Every recipe.
    pub const ALL: [Recipe; 10] = [
        Self::Sidebar,
        Self::Titlebar,
        Self::Popover,
        Self::Menu,
        Self::Tooltip,
        Self::Hud,
        Self::Sheet,
        Self::Toast,
        Self::Scrim,
        Self::Glass,
    ];

    /// Stable name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Titlebar => "titlebar",
            Self::Popover => "popover",
            Self::Menu => "menu",
            Self::Tooltip => "tooltip",
            Self::Hud => "hud",
            Self::Sheet => "sheet",
            Self::Toast => "toast",
            Self::Scrim => "scrim",
            Self::Glass => "glass",
        }
    }

    /// The material for `mode`.
    pub fn material(self, mode: Mode) -> Material {
        match self {
            Self::Sidebar => sidebar(mode),
            Self::Titlebar => titlebar(mode),
            Self::Popover => popover(mode),
            Self::Menu => menu(mode),
            Self::Tooltip => tooltip(mode),
            Self::Hud => hud(),
            Self::Sheet => sheet(mode),
            Self::Toast => toast(mode),
            Self::Scrim => scrim(mode),
            Self::Glass => glass(mode),
        }
    }

    /// Text color that reads on this recipe in `mode`.
    pub fn foreground(self, mode: Mode) -> Hsla {
        let dark = matches!(self, Self::Hud) || mode == Mode::Dark;
        if dark {
            hsla(palette::NEUTRAL_WHITE)
        } else {
            hsla(palette::PRIMARY_BLACK)
        }
    }
}

/// Sidebar: chrome material, square, no shadow or border (the shell draws
/// the divider).
pub fn sidebar(mode: Mode) -> Material {
    Material::role(MaterialRole::Chrome, mode)
        .radius(px(radius::NONE))
        .border(None)
        .shadows(Vec::new())
}

/// Titlebar: chrome material, a touch more saturated so content scrolling
/// under it keeps its color.
pub fn titlebar(mode: Mode) -> Material {
    Material::role(MaterialRole::Chrome, mode)
        .radius(px(radius::NONE))
        .border(None)
        .shadows(Vec::new())
        .saturation(1.9)
}

/// Popover: regular material, 12px radius, overlay elevation, top light.
pub fn popover(mode: Mode) -> Material {
    Material::role(MaterialRole::Regular, mode)
        .radius(px(radius::X_LARGE))
        .shadows(elevation_shadows(Elevation::OVERLAY, mode))
}

/// Menu: regular material, 8px radius, floating elevation.
pub fn menu(mode: Mode) -> Material {
    Material::role(MaterialRole::Regular, mode)
        .radius(px(radius::LARGE))
        .shadows(elevation_shadows(Elevation::FLOATING, mode))
}

/// Tooltip: thin material, 6px radius.
pub fn tooltip(mode: Mode) -> Material {
    Material::role(MaterialRole::Thin, mode)
        .radius(px(radius::MEDIUM))
        .lighting(Lighting::NONE)
}

/// HUD: dark smoked glass in both appearances.
pub fn hud() -> Material {
    let black = hsla(palette::PRIMARY_BLACK);
    Material::role(MaterialRole::Thick, Mode::Dark)
        .blur(px(24.))
        .saturation(1.5)
        .tint(black.opacity(0.48))
        .radius(px(18.))
        .border(Some(gpui::white().opacity(0.12)))
        .lighting(Lighting::edge(0.22).specular(0.08))
        .shadows(ShadowPreset::Layered(Mode::Dark).shadows(Mode::Dark))
        .fallback(black.opacity(0.88))
}

/// Sheet: thick material, 16px radius, modal elevation and a soft glass rim.
pub fn sheet(mode: Mode) -> Material {
    Material::role(MaterialRole::Thick, mode)
        .radius(px(radius::XX_LARGE))
        .shadows(elevation_shadows(Elevation::MODAL, mode))
        .inner_shadow(InnerShadow::glass_rim(mode))
}

/// Toast: regular material, 12px radius, toast elevation.
pub fn toast(mode: Mode) -> Material {
    Material::role(MaterialRole::Regular, mode)
        .radius(px(radius::X_LARGE))
        .shadows(elevation_shadows(Elevation::TOAST, mode))
}

/// Scrim: the modal backdrop (light blur and dim, square).
pub fn scrim(mode: Mode) -> Material {
    Material::role(MaterialRole::Scrim, mode)
        .radius(px(radius::NONE))
        .border(None)
}

/// Glass: clear, vivid, lit glass with layered shadows and a rim glow.
pub fn glass(mode: Mode) -> Material {
    Material::glass(mode)
        .radius(px(20.))
        .shadows(ShadowPreset::Layered(mode).shadows(mode))
        .inner_shadow(InnerShadow::glass_rim(mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recipe_resolves_in_both_modes() {
        for mode in [Mode::Light, Mode::Dark] {
            for recipe in Recipe::ALL {
                let m = recipe.material(mode);
                assert!(m.blur > px(0.), "{}", recipe.name());
                assert!(m.fallback.a > 0.15, "{}", recipe.name());
            }
        }
        assert_eq!(sidebar(Mode::Light).radius, px(0.));
        assert!(sheet(Mode::Dark).shadows.len() >= 2);
        // HUD is dark in light mode too.
        assert!(Recipe::Hud.foreground(Mode::Light).l > 0.9);
    }
}
