//! Surface — a plate on the Material Engine.
//!
//! `Surface::new(SurfaceKind::Popover)` is a `div` whose background is the
//! matching material (real backdrop blur, tint, light, grain and shadows) when
//! [`crate::materials_active`], and the opaque web-parity fill, hairline border
//! and elevation shadow otherwise. Style and fill it like any `div`.

use gpui::{
    AnyElement, App, Div, Hsla, IntoElement, ParentElement, Pixels, RenderOnce, StyleRefinement,
    Styled, Window, div, px,
};
use refineui_foundation::{radius, stroke};
use smallvec::SmallVec;

use crate::{ActiveTheme, Elevation, SurfaceKind};

/// A material-backed plate (see the module docs).
#[derive(IntoElement)]
pub struct Surface {
    kind: SurfaceKind,
    base: Div,
    radius: Option<Pixels>,
    elevation: Option<Elevation>,
    border: Option<Option<Hsla>>,
    children: SmallVec<[AnyElement; 2]>,
}

impl Surface {
    /// A surface of `kind`.
    pub fn new(kind: SurfaceKind) -> Self {
        Self {
            kind,
            base: div(),
            radius: None,
            elevation: None,
            border: None,
            children: SmallVec::new(),
        }
    }

    /// Corner radius (default: the kind's radius token).
    pub fn radius(mut self, radius: Pixels) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Elevation (default: the kind's elevation; the material recipe's own
    /// shadows when materials are active).
    pub fn elevation(mut self, elevation: Elevation) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Border color; `None` removes it (default: `borderDefault` /
    /// the recipe's border).
    pub fn border_color_override(mut self, border: Option<Hsla>) -> Self {
        self.border = Some(border);
        self
    }

    /// The kind's default corner radius.
    pub fn default_radius(kind: SurfaceKind) -> Pixels {
        px(match kind {
            SurfaceKind::Panel | SurfaceKind::Menu => radius::LARGE,
            SurfaceKind::Sidebar | SurfaceKind::Titlebar | SurfaceKind::Scrim => radius::NONE,
            SurfaceKind::Popover | SurfaceKind::Toast => radius::X_LARGE,
            SurfaceKind::Tooltip => radius::MEDIUM,
            SurfaceKind::Hud => 18.,
            SurfaceKind::Sheet => radius::XX_LARGE,
            SurfaceKind::Glass => 20.,
        })
    }

    /// The kind's default elevation (opaque path).
    pub fn default_elevation(kind: SurfaceKind) -> Option<Elevation> {
        match kind {
            SurfaceKind::Panel => Some(Elevation::RAISED),
            SurfaceKind::Popover | SurfaceKind::Menu | SurfaceKind::Tooltip => {
                Some(Elevation::FLOATING)
            }
            SurfaceKind::Toast => Some(Elevation::TOAST),
            SurfaceKind::Sheet | SurfaceKind::Hud | SurfaceKind::Glass => Some(Elevation::OVERLAY),
            SurfaceKind::Sidebar | SurfaceKind::Titlebar | SurfaceKind::Scrim => None,
        }
    }
}

impl Styled for Surface {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Surface {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Surface {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let kind = self.kind;
        let theme = cx.theme();
        let radius = self.radius.unwrap_or(Self::default_radius(kind));
        let default_border = match kind {
            SurfaceKind::Sidebar | SurfaceKind::Titlebar | SurfaceKind::Scrim => None,
            SurfaceKind::Hud => Some(gpui::white().opacity(0.12)),
            _ => Some(theme.colors.border_default),
        };
        let border = self.border.unwrap_or(default_border);
        let shadows = self
            .elevation
            .or(Self::default_elevation(kind))
            .map(|e| theme.shadow(e))
            .unwrap_or_default();
        let fill = kind.fallback_fill(cx);
        let foreground = kind.foreground(cx);

        let mut base = self.base;
        if base.style().position.is_none() {
            base = base.relative();
        }
        let base = base.rounded(radius).text_color(foreground);

        #[cfg(feature = "material")]
        if crate::materials_active(cx) {
            let mut material = crate::material::material(kind, cx).radius(radius);
            if let Some(border) = self.border {
                material = material.border(border);
            }
            if self.elevation.is_some() {
                material = material.shadows(shadows);
            }
            let material = material.border_width(px(stroke::HAIRLINE));
            let layer = crate::material::material_layer(material, stroke::HAIRLINE);
            return base
                .border(px(stroke::HAIRLINE))
                .border_color(gpui::transparent_black())
                .child(layer)
                .children(self.children);
        }

        base.bg(fill)
            .border(px(stroke::HAIRLINE))
            .border_color(border.unwrap_or(gpui::transparent_black()))
            .shadow(shadows)
            .children(self.children)
    }
}
