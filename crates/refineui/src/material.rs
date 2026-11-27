//! Components on the Material Engine.
//!
//! With the **`material`** feature, frosted components (Popover, Tooltip,
//! Menu, Select, the Dialog scrim) and [`crate::Surface`] sample the pixels
//! behind them through `refineui-material` (real backdrop blur, tint,
//! lighting, grain, shadows drawn outside the shape). Glass is opt-in for
//! [`crate::Button::glass`], [`crate::Input::glass`] and
//! [`crate::Toast::material`].
//!
//! Without the feature — or where blur is unavailable (DirectX, the
//! "reduce transparency" setting, native macOS frost) — every component draws
//! its regular opaque web-parity surface, so the public API is the same
//! either way. [`materials_active`] tells which path is taken.
//!
//! The feature needs the vendored gpui (`[patch.crates-io] gpui`, see
//! `vendor/gpui/REFINEUI_PATCH.md`); it is off by default so stock gpui 0.2.2
//! keeps building.

use gpui::{AnyElement, App, BoxShadow, Hsla, ParentElement, Pixels, Styled};

use crate::ActiveTheme;

/// What a surface is for. Picks the material recipe when materials are
/// active and the opaque fallback fill otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// Generic raised panel (card-like). Regular material.
    #[default]
    Panel,
    /// Source list / navigation column. Chrome material.
    Sidebar,
    /// Titlebar / unified toolbar. Chrome material.
    Titlebar,
    /// Popover and dropdown panels. Regular material.
    Popover,
    /// Menus. Regular material.
    Menu,
    /// Tooltips. Thin material.
    Tooltip,
    /// Heads-up display: dark smoked glass in both appearances.
    Hud,
    /// Sheets and dialogs. Thick material with a glass rim.
    Sheet,
    /// Toast notifications. Regular material.
    Toast,
    /// Modal backdrop.
    Scrim,
    /// Expressive glass. Glass material.
    Glass,
}

impl SurfaceKind {
    /// Every kind.
    pub const ALL: [SurfaceKind; 11] = [
        Self::Panel,
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
            Self::Panel => "panel",
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

    /// Opaque fill used without materials (web parity: frosted surfaces use
    /// [`crate::Theme::floating_fill`], panels `backgroundPrimary`).
    pub fn fallback_fill(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        let c = &theme.colors;
        match self {
            Self::Panel | Self::Sheet | Self::Toast => c.background_primary,
            Self::Sidebar | Self::Titlebar => c.background_surface,
            Self::Popover | Self::Menu | Self::Tooltip | Self::Glass => theme.floating_fill(),
            Self::Hud => gpui::hsla(0., 0., 0.08, 0.9),
            Self::Scrim => c.surface_overlay,
        }
    }

    /// Text / icon color that reads on this surface (the HUD is always
    /// dark; everything else follows the theme).
    pub fn foreground(self, cx: &App) -> Hsla {
        match self {
            Self::Hud => gpui::white(),
            _ => cx.theme().colors.foreground_primary,
        }
    }

    /// Secondary text color on this surface.
    pub fn foreground_secondary(self, cx: &App) -> Hsla {
        match self {
            Self::Hud => gpui::white().opacity(0.72),
            _ => cx.theme().colors.foreground_secondary,
        }
    }
}

/// True when components draw real materials: the `material` feature is on,
/// the renderer supports backdrop blur and reduced transparency is off.
pub fn materials_active(cx: &App) -> bool {
    #[cfg(feature = "material")]
    {
        refineui_material::MaterialSettings::blur_enabled(cx)
    }
    #[cfg(not(feature = "material"))]
    {
        let _ = cx;
        false
    }
}

/// How a component should paint its plate this frame.
pub(crate) struct Backing {
    /// Fill for the component's own box (transparent under a material).
    pub fill: Hsla,
    /// Border color for the component's own box (transparent under a
    /// material: the material draws the border after the blur).
    pub border: Hsla,
    /// Shadows for the component's own box (empty under a material: the
    /// material draws them outside its shape).
    pub shadows: Vec<BoxShadow>,
    /// The material layer to add as the component's **first** child.
    pub layer: Option<AnyElement>,
}

impl Backing {
    /// The plain opaque plate (no material).
    pub(crate) fn opaque(fill: Hsla, shadows: Vec<BoxShadow>) -> Self {
        Self {
            fill,
            border: gpui::transparent_black(),
            shadows,
            layer: None,
        }
    }
}

/// Plate for a component of `kind`: the material layer when
/// [`materials_active`], else the given opaque `fill`, `border` and `shadows`.
///
/// `box_border` is the border width of the component's own box: the layer
/// is outset by it (absolute children sit inside the border) and the
/// material strokes `border` at that width.
pub(crate) fn backing(
    kind: SurfaceKind,
    radius: Pixels,
    box_border: f32,
    fill: Hsla,
    border: Option<Hsla>,
    shadows: Vec<BoxShadow>,
    cx: &App,
) -> Backing {
    #[cfg(feature = "material")]
    if materials_active(cx) {
        let material = material_for(kind, cx)
            .radius(radius)
            .border(border)
            .border_width(gpui::px(box_border.max(0.5)))
            .shadows(shadows);
        return Backing {
            fill: gpui::transparent_black(),
            border: gpui::transparent_black(),
            shadows: Vec::new(),
            layer: Some(material_layer(material, box_border)),
        };
    }
    let _ = (kind, radius, box_border, cx);
    Backing {
        fill,
        border: border.unwrap_or(gpui::transparent_black()),
        shadows,
        layer: None,
    }
}

/// Window chrome (titlebar, status bar, bars, docks, tab strips): a square
/// `kind` material with no border or shadow of its own, so the caller's
/// one-sided hairline (drawn in the border box, outside the layer) stays
/// visible. Returns the fill for the box (transparent under a material) and
/// the layer to add as its **first** child.
pub(crate) fn chrome(kind: SurfaceKind, fill: Hsla, cx: &App) -> (Hsla, Option<AnyElement>) {
    let b = backing(kind, gpui::px(0.), 0., fill, None, Vec::new(), cx);
    (b.fill, b.layer)
}

/// [`crate::StyledExt::popover_surface`] on a material: the frosted panel
/// chrome (hairline `borderDefault`, `roundedXXLarge`, surface shadow) drawn
/// by the `kind` material when [`materials_active`], else the opaque fill.
/// Call it before adding children (the material layer must come first).
///
/// Put padding on an *inner* child, not on `el`. The material layer is an
/// absolutely positioned child; padding on the same node can leave a ring
/// of the (transparent) plate unblurred depending on how the containing
/// block is resolved.
pub(crate) fn frosted_plate<E: Styled + ParentElement>(el: E, kind: SurfaceKind, cx: &App) -> E {
    use crate::StyledExt;
    let theme = cx.theme();
    let radius = gpui::px(refineui_foundation::radius::XX_LARGE);
    let b = backing(
        kind,
        radius,
        refineui_foundation::stroke::HAIRLINE,
        theme.floating_fill(),
        Some(theme.colors.border_default),
        theme.shadow(crate::Elevation::SURFACE),
        cx,
    );
    match b.layer {
        Some(layer) => el
            .rounded(radius)
            .border(gpui::px(refineui_foundation::stroke::HAIRLINE))
            .border_color(b.border)
            .child(layer),
        None => el.popover_surface(cx),
    }
}

#[cfg(feature = "material")]
fn material_for(kind: SurfaceKind, cx: &App) -> refineui_material::Material {
    use refineui_material::{Material, MaterialRole, recipes};
    let mode = cx.theme().appearance;
    match kind {
        SurfaceKind::Panel => Material::role(MaterialRole::Regular, mode),
        SurfaceKind::Sidebar => recipes::sidebar(mode),
        SurfaceKind::Titlebar => recipes::titlebar(mode),
        SurfaceKind::Popover => recipes::popover(mode),
        SurfaceKind::Menu => recipes::menu(mode),
        SurfaceKind::Tooltip => recipes::tooltip(mode),
        SurfaceKind::Hud => recipes::hud(),
        SurfaceKind::Sheet => recipes::sheet(mode),
        SurfaceKind::Toast => recipes::toast(mode),
        SurfaceKind::Scrim => recipes::scrim(mode),
        SurfaceKind::Glass => recipes::glass(mode),
    }
}

/// The material as an absolutely positioned layer covering its parent's
/// border box (`outset` = the parent's border width).
#[cfg(feature = "material")]
pub(crate) fn material_layer(material: refineui_material::Material, outset: f32) -> AnyElement {
    use gpui::{IntoElement, Styled, px};
    refineui_material::material_surface(material)
        .absolute()
        .top(px(-outset))
        .left(px(-outset))
        .right(px(-outset))
        .bottom(px(-outset))
        .into_any_element()
}

/// The material for `kind` in the current appearance (feature `material`),
/// for building custom surfaces with `refineui_material::material_surface`.
#[cfg(feature = "material")]
pub fn material(kind: SurfaceKind, cx: &App) -> refineui_material::Material {
    material_for(kind, cx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique() {
        let mut names: Vec<_> = SurfaceKind::ALL.iter().map(|k| k.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SurfaceKind::ALL.len());
    }
}
