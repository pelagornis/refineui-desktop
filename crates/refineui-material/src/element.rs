//! [`MaterialSurface`]: composites a [`Material`] in a fixed order.
//!
//! Real-blur path (bottom → top):
//! 1. one renderer pass: drop shadows (outside the shape), blurred backdrop,
//!    saturation, tint (blend mode), lighting, grain, dithering, mask,
//! 2. inner shadows,
//! 3. border,
//! 4. children.
//!
//! Fallback path (no blur in the renderer, or reduced transparency): GPUI
//! drop shadows, the opaque fallback fill (faded by the mask), grain tile,
//! quad lighting, inner shadows, border, children.
//!
//! Nested materials work naturally: an inner surface blurs whatever the
//! outer one already composited.

use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    canvas, div, linear_color_stop, linear_gradient, px, quad, transparent_black,
};
use smallvec::SmallVec;

use crate::{Material, MaterialSettings, texture::paint_grain};

/// A container drawn as a material. Style it like a `div` (size, padding,
/// flex); the corner radius comes from the material.
#[derive(IntoElement)]
pub struct MaterialSurface {
    base: Div,
    material: Material,
    children: SmallVec<[AnyElement; 2]>,
}

/// Create a [`MaterialSurface`].
pub fn material_surface(material: Material) -> MaterialSurface {
    MaterialSurface {
        base: div(),
        material,
        children: SmallVec::new(),
    }
}

impl MaterialSurface {
    /// Replace the material.
    pub fn material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }
}

impl Styled for MaterialSurface {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for MaterialSurface {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MaterialSurface {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m = self.material;
        // The layer canvas is absolutely positioned, so the surface must be
        // a containing block; keep an explicit `absolute()` from the caller.
        if self.base.style().position.is_none() {
            self.base = self.base.relative();
        }
        let blur = MaterialSettings::blur_enabled(cx) && m.blur > px(0.);
        // With real blur the renderer draws the first shadows itself, outside
        // the shape, so they are not blurred into the backdrop.
        let shadows = if blur {
            m.extra_shadows().to_vec()
        } else {
            m.shadows.clone()
        };
        let radius = m.radius;
        let layers = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let corners = gpui::Corners::all(m.radius);
                if blur {
                    // Stretching this canvas with all four insets (below)
                    // keeps `bounds` equal to the surface, including after
                    // the parent grows or is padded. `paint_backdrop_blur`
                    // still early-returns when opacity is 0 (enter/exit);
                    // the renderer composites a tinted plate whenever the
                    // shape is visible, even if the blur sample is clipped
                    // to a sliver at the window edge.
                    window.paint_backdrop_blur(bounds, m.backdrop_style());
                } else {
                    let fill = m.fallback.opacity(m.opacity);
                    let background: gpui::Background = match m.mask {
                        Some(mask) => linear_gradient(
                            mask.angle,
                            linear_color_stop(fill, mask.start),
                            linear_color_stop(fill.opacity(0.), mask.end),
                        ),
                        None => fill.into(),
                    };
                    window.paint_quad(quad(
                        bounds,
                        corners,
                        background,
                        px(0.),
                        transparent_black(),
                        Default::default(),
                    ));
                    paint_grain(bounds, m.radius, m.noise * m.opacity, window);
                    m.lighting.paint(bounds, m.radius, m.opacity, window);
                }
                for inner in &m.inner_shadows {
                    inner.paint(bounds, m.radius, m.opacity, window);
                }
                if let Some(border) = m.border {
                    window.paint_quad(quad(
                        bounds,
                        corners,
                        transparent_black(),
                        m.border_width,
                        border.opacity(m.opacity),
                        Default::default(),
                    ));
                }
            },
        )
        // Insets on all four sides, not `size_full()`: a percentage height
        // is not always definite on an inset-stretched absolute parent, so
        // the canvas could paint only a first-frame or content-box sliver
        // and leave the rest of the (transparent-fill) plate see-through.
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0();

        self.base
            .rounded(radius)
            .shadow(shadows)
            .child(layers)
            .children(self.children)
    }
}
