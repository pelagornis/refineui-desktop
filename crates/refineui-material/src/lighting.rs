//! Lighting: directional, ambient, edge, specular and reflection.
//!
//! With real blur, light is evaluated per pixel on the GPU inside the
//! composite pass (exact rounded rim, dithered, no extra passes). On the
//! fallback path it is approximated with gradient quads and a soft glow.
//!
//! `angle` is the direction the light comes *from*, in degrees (0 = top,
//! 90 = right), matching CSS gradient angles.

use gpui::{
    BackdropLight, Bounds, BoxShadow, Corners, Hsla, Pixels, Point, Window, linear_color_stop,
    linear_gradient, point, px, quad, size, transparent_white,
};

/// Light on a material surface. Every strength is 0–1 (0 = off).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    /// Direction the light comes from (degrees; 0 = top).
    pub angle: f32,
    /// Light color.
    pub color: Hsla,
    /// Uniform lift.
    pub ambient: f32,
    /// Sheen that brightens the side facing the light and fades across.
    pub directional: f32,
    /// Rim highlight, strongest on the edge facing the light.
    pub edge: f32,
    /// Rim falloff width.
    pub edge_width: Pixels,
    /// Soft specular highlight.
    pub specular: f32,
    /// Shape-relative position (0–1) of the specular highlight.
    pub specular_position: Point<f32>,
    /// Size (Gaussian σ) of the specular highlight.
    pub specular_size: Pixels,
    /// Glossy diagonal reflection band (GPU path only).
    pub reflection: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self::NONE
    }
}

impl Lighting {
    /// No light.
    pub const NONE: Self = Self {
        angle: 0.,
        color: Hsla {
            h: 0.,
            s: 0.,
            l: 1.,
            a: 1.,
        },
        ambient: 0.,
        directional: 0.,
        edge: 0.,
        edge_width: px(1.5),
        specular: 0.,
        specular_position: Point { x: 0.22, y: 0.1 },
        specular_size: px(40.),
        reflection: 0.,
    };

    /// Top light: an edge highlight of `strength` with a matching soft
    /// directional sheen.
    pub fn edge(strength: f32) -> Self {
        Self {
            edge: strength,
            directional: strength * 0.18,
            ..Self::NONE
        }
    }

    /// Glass light: rim, sheen, reflection band and a specular highlight.
    pub fn glass(edge: f32, reflection: f32, specular: f32) -> Self {
        Self {
            edge,
            directional: edge * 0.15,
            reflection,
            specular,
            ..Self::NONE
        }
    }

    /// Light direction (degrees; 0 = top).
    pub fn angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    /// Light color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    /// Ambient lift.
    pub fn ambient(mut self, ambient: f32) -> Self {
        self.ambient = ambient;
        self
    }

    /// Directional sheen.
    pub fn directional(mut self, directional: f32) -> Self {
        self.directional = directional;
        self
    }

    /// Rim highlight strength and falloff width.
    pub fn rim(mut self, edge: f32, width: Pixels) -> Self {
        self.edge = edge;
        self.edge_width = width;
        self
    }

    /// Specular highlight strength.
    pub fn specular(mut self, specular: f32) -> Self {
        self.specular = specular;
        self
    }

    /// Specular highlight position (shape-relative, 0–1) and size.
    pub fn specular_at(mut self, position: Point<f32>, size: Pixels) -> Self {
        self.specular_position = position;
        self.specular_size = size;
        self
    }

    /// Reflection band strength.
    pub fn reflection(mut self, reflection: f32) -> Self {
        self.reflection = reflection;
        self
    }

    /// `true` when nothing would be drawn.
    pub fn is_none(&self) -> bool {
        self.ambient <= 0.
            && self.directional <= 0.
            && self.edge <= 0.
            && self.specular <= 0.
            && self.reflection <= 0.
    }

    /// Unit vector pointing *towards* the light (screen space, y down).
    pub fn direction(&self) -> Point<f32> {
        let r = self.angle.to_radians();
        point(r.sin(), -r.cos())
    }

    /// The renderer's per-pixel lighting parameters (real-blur path).
    pub fn to_backdrop(&self) -> BackdropLight {
        BackdropLight {
            direction: self.direction(),
            color: self.color,
            directional: self.directional,
            ambient: self.ambient,
            edge: self.edge,
            edge_width: self.edge_width,
            specular: self.specular,
            specular_position: self.specular_position,
            specular_size: self.specular_size,
            reflection: self.reflection,
        }
    }

    /// Paint an approximation of the light over `bounds` with quads (the
    /// fallback path, after the opaque fill). The reflection band is GPU
    /// only.
    pub fn paint(&self, bounds: Bounds<Pixels>, radius: Pixels, opacity: f32, window: &mut Window) {
        if self.is_none() || opacity <= 0. {
            return;
        }
        let corners = Corners::all(radius);
        let light = self.color;
        if self.ambient > 0. {
            window.paint_quad(quad(
                bounds,
                corners,
                light.opacity(self.ambient * opacity),
                px(0.),
                transparent_white(),
                Default::default(),
            ));
        }
        if self.directional > 0. {
            // Fades from the lit edge to nothing past the middle (gradient
            // angle points *away* from the light).
            let sheen = linear_gradient(
                self.angle + 180.,
                linear_color_stop(light.opacity(self.directional * opacity), 0.),
                linear_color_stop(light.opacity(0.), 0.55),
            );
            window.paint_quad(quad(
                bounds,
                corners,
                sheen,
                px(0.),
                transparent_white(),
                Default::default(),
            ));
        }
        if self.specular > 0. {
            // A soft glow: a light-colored Gaussian shadow, clipped to the
            // surface.
            let center = point(
                bounds.origin.x + bounds.size.width * self.specular_position.x,
                bounds.origin.y + bounds.size.height * self.specular_position.y,
            );
            let core = self.specular_size * 0.5;
            let spot = Bounds::new(
                point(center.x - core / 2., center.y - core / 2.),
                size(core, core),
            );
            window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                window.paint_shadows(
                    spot,
                    Corners::all(core / 2.),
                    &[BoxShadow {
                        color: light.opacity(self.specular * opacity),
                        offset: point(px(0.), px(0.)),
                        blur_radius: self.specular_size * 0.5,
                        spread_radius: px(0.),
                    }],
                );
            });
        }
        if self.edge > 0. {
            self.paint_rim(bounds, radius, opacity, window);
        }
    }

    fn paint_rim(&self, bounds: Bounds<Pixels>, radius: Pixels, opacity: f32, window: &mut Window) {
        let corners = Corners::all(radius);
        let light = self.color;
        // A faint 1px border all round, plus a brighter line along the lit
        // edge that fades out toward both corners.
        window.paint_quad(quad(
            bounds,
            corners,
            transparent_white(),
            px(1.),
            light.opacity(self.edge * 0.35 * opacity),
            Default::default(),
        ));
        let inset = (radius * 0.6).max(px(1.));
        let (origin, length, vertical) = match ((self.angle % 360. + 360.) % 360.) as u32 {
            45..=134 => (
                point(bounds.right() - px(1.), bounds.top() + inset),
                bounds.size.height - inset * 2.,
                true,
            ),
            135..=224 => (
                point(bounds.left() + inset, bounds.bottom() - px(1.)),
                bounds.size.width - inset * 2.,
                false,
            ),
            225..=314 => (
                point(bounds.left(), bounds.top() + inset),
                bounds.size.height - inset * 2.,
                true,
            ),
            _ => (
                point(bounds.left() + inset, bounds.top()),
                bounds.size.width - inset * 2.,
                false,
            ),
        };
        if length <= px(0.) {
            return;
        }
        let color = light.opacity(self.edge * opacity);
        let angle = if vertical { 180. } else { 90. };
        let half = if vertical {
            size(px(1.), length / 2.)
        } else {
            size(length / 2., px(1.))
        };
        let second = if vertical {
            point(origin.x, origin.y + length / 2.)
        } else {
            point(origin.x + length / 2., origin.y)
        };
        let fade_in = linear_gradient(
            angle,
            linear_color_stop(color.opacity(0.), 0.),
            linear_color_stop(color, 1.),
        );
        let fade_out = linear_gradient(
            angle,
            linear_color_stop(color, 0.),
            linear_color_stop(color.opacity(0.), 1.),
        );
        for (b, bg) in [
            (Bounds::new(origin, half), fade_in),
            (Bounds::new(second, half), fade_out),
        ] {
            window.paint_quad(quad(
                b,
                Corners::default(),
                bg,
                px(0.),
                transparent_white(),
                Default::default(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_light_has_a_soft_sheen() {
        let l = Lighting::edge(0.6);
        assert_eq!(l.edge, 0.6);
        assert!(l.directional > 0. && l.directional < l.edge);
        assert!(Lighting::NONE.is_none());
        assert!(!l.is_none());
    }

    #[test]
    fn angle_points_towards_the_light() {
        let top = Lighting::NONE.angle(0.).direction();
        assert!(top.y < -0.99 && top.x.abs() < 1e-5);
        let right = Lighting::NONE.angle(90.).direction();
        assert!(right.x > 0.99);
        let b = Lighting::glass(0.8, 0.3, 0.2).angle(90.).to_backdrop();
        assert!(b.direction.x > 0.99);
        assert_eq!(b.reflection, 0.3);
        assert_eq!(b.specular, 0.2);
    }
}
