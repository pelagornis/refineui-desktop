//! RefineUI patch: renderer-independent planning for backdrop blur.
//!
//! Both the Blade and the Metal renderer run the same steps (copy → downsample
//! → Gaussian H → Gaussian V → composite) with the same uniform block, built
//! here so the two backends cannot drift apart. See `REFINEUI_PATCH.md`.

#![cfg_attr(windows, allow(dead_code))]

use crate::{point, BackdropBlur, Bounds, ScaledPixels, Size};

/// Uniform block shared by every blur step. Mirrors `BlurParams` in
/// `blade/backdrop_blur.wgsl` and `RefineBlurParams` in `mac/shaders.metal`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BackdropBlurParams {
    pub dst_rect: [f32; 4],
    pub target_size: [f32; 4],
    pub src_size: [f32; 4],
    pub src_clamp: [f32; 4],
    pub params: [f32; 4],
    pub shape: [f32; 4],
    pub radii: [f32; 4],
    pub tint: [f32; 4],
    pub extra: [f32; 4],
    pub light_dir: [f32; 4],
    pub light_a: [f32; 4],
    pub light_b: [f32; 4],
    pub light_color: [f32; 4],
    pub mask: [f32; 4],
    pub misc: [f32; 4],
    pub shadows: [[f32; 4]; 8],
}

/// Where one blur reads and writes, in drawable pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BackdropBlurPlan {
    /// Downsample factor (1, 2, 4 or 8).
    pub scale: f32,
    /// Gaussian sigma in drawable pixels.
    pub sigma: f32,
    /// Region copied out of the drawable (x0, y0, x1, y1), snapped to the
    /// downsample grid.
    pub copy: [f32; 4],
    /// The same region in downsampled texels, whole texels only.
    pub region_ds: [f32; 4],
    /// Rectangle the composite covers (shape + shadows, clipped).
    pub draw: Bounds<ScaledPixels>,
    /// Drawable size.
    pub target: [f32; 2],
}

impl BackdropBlurPlan {
    /// Size of the blurred result in downsampled texels.
    pub(crate) fn result_size(&self) -> (u32, u32) {
        (
            (self.region_ds[2] - self.region_ds[0]).max(1.0) as u32,
            (self.region_ds[3] - self.region_ds[1]).max(1.0) as u32,
        )
    }

    fn base(&self) -> BackdropBlurParams {
        let [width, height] = self.target;
        let r = self.region_ds;
        BackdropBlurParams {
            dst_rect: [r[0], r[1], r[2] - r[0], r[3] - r[1]],
            target_size: [width, height, 0.0, 0.0],
            src_size: [width, height, 1.0 / width, 1.0 / height],
            src_clamp: self.copy,
            params: [self.scale, 0.0, 0.0, 0.0],
            extra: [1.0, 1.0, 0.0, self.scale],
            ..Default::default()
        }
    }

    /// Box downsample of the copied region (full-size source texture).
    pub(crate) fn downsample_params(&self) -> BackdropBlurParams {
        self.base()
    }

    /// One Gaussian direction in downsampled space.
    pub(crate) fn gaussian_params(&self, horizontal: bool) -> BackdropBlurParams {
        let mut params = self.base();
        params.src_clamp = self.region_ds;
        let (x, y) = if horizontal { (1.0, 0.0) } else { (0.0, 1.0) };
        params.params = [1.0, self.sigma / self.scale, x, y];
        params
    }
}

/// The texture the composite samples.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BlurSource {
    /// The full-size ping-pong texture, result at `region_ds`.
    Targets,
    /// A cached texture holding exactly the region (origin at 0, 0).
    Cached,
}

/// Snap a sample rectangle to the downsample grid, clamped to the drawable.
/// Always returns a non-empty region when the drawable has a positive size,
/// so a visible sliver at the edge still gets a tinted composite.
fn snap_copy_region(
    sample: Bounds<ScaledPixels>,
    width: f32,
    height: f32,
    s: f32,
) -> ([f32; 4], [f32; 4]) {
    let mut x0 = ((sample.origin.x.0 / s).floor() * s).max(0.0);
    let mut y0 = ((sample.origin.y.0 / s).floor() * s).max(0.0);
    let mut x1 = (((sample.origin.x.0 + sample.size.width.0) / s).ceil() * s).min(width);
    let mut y1 = (((sample.origin.y.0 + sample.size.height.0) / s).ceil() * s).min(height);
    let mut region_ds = [x0 / s, y0 / s, (x1 / s).floor(), (y1 / s).floor()];
    if region_ds[2] <= region_ds[0] || region_ds[3] <= region_ds[1] {
        let max_x = (width / s).floor().max(1.0);
        let max_y = (height / s).floor().max(1.0);
        let ds_x0 = (x0 / s).floor().clamp(0.0, max_x - 1.0);
        let ds_y0 = (y0 / s).floor().clamp(0.0, max_y - 1.0);
        region_ds = [ds_x0, ds_y0, ds_x0 + 1.0, ds_y0 + 1.0];
        x0 = region_ds[0] * s;
        y0 = region_ds[1] * s;
        x1 = (region_ds[2] * s).min(width).max(x0 + 1.0);
        y1 = (region_ds[3] * s).min(height).max(y0 + 1.0);
    }
    ([x0, y0, x1, y1], region_ds)
}

impl BackdropBlur {
    /// Plan the blur for a drawable of `width` × `height`; `None` when
    /// nothing of the shape is visible.
    pub(crate) fn plan(&self, width: f32, height: f32) -> Option<BackdropBlurPlan> {
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let viewport = Bounds {
            origin: point(ScaledPixels(0.0), ScaledPixels(0.0)),
            size: Size {
                width: ScaledPixels(width),
                height: ScaledPixels(height),
            },
        };
        let visible = self
            .bounds
            .intersect(&self.content_mask.bounds)
            .intersect(&viewport);
        if visible.is_empty() {
            return None;
        }
        let draw = self.draw_bounds().intersect(&viewport);
        let sigma = self.sigma.0.max(0.0);
        let s: f32 = if sigma <= 4.0 {
            1.0
        } else if sigma <= 10.0 {
            2.0
        } else if sigma <= 24.0 {
            4.0
        } else {
            8.0
        };
        let sample = self.sample_bounds().intersect(&viewport);
        // A region clipped by the drawable edge can end mid-texel in the
        // downsampled space; only whole texels are written. If snapping
        // would leave a 0-texel copy, `snap_copy_region` keeps 1 texel so
        // the composite still paints the tinted shape.
        let (copy, region_ds) = snap_copy_region(sample, width, height, s);
        Some(BackdropBlurPlan {
            scale: s,
            sigma,
            copy,
            region_ds,
            draw,
            target: [width, height],
        })
    }

    /// Uniforms for the composite step. `premultiplied` selects the output
    /// alpha convention of the drawable's blend state.
    pub(crate) fn composite_params(
        &self,
        plan: &BackdropBlurPlan,
        source: BlurSource,
        premultiplied: bool,
    ) -> BackdropBlurParams {
        let [width, height] = plan.target;
        let (src_size, src_clamp, offset) = match source {
            BlurSource::Targets => (
                [width, height, 1.0 / width, 1.0 / height],
                plan.region_ds,
                [0.0, 0.0],
            ),
            BlurSource::Cached => {
                let (w, h) = plan.result_size();
                let (w, h) = (w as f32, h as f32);
                (
                    [w, h, 1.0 / w, 1.0 / h],
                    [0.0, 0.0, w, h],
                    [plan.region_ds[0], plan.region_ds[1]],
                )
            }
        };
        // The drawable is a non-sRGB UNORM format (gpui writes gamma-space
        // colors), so colors stay in gamma space.
        let tint = self.tint.to_rgb();
        let light = &self.light;
        let light_color = light.color.to_rgb();
        let mut shadows = [[0.0; 4]; 8];
        let mut count = 0;
        for shadow in self.shadows.iter().filter(|s| s.is_visible()) {
            let color = shadow.color.to_rgb();
            shadows[count * 2] = [
                shadow.offset.x.0,
                shadow.offset.y.0,
                shadow.blur_radius.0.max(0.0),
                shadow.spread_radius.0,
            ];
            shadows[count * 2 + 1] = [color.r, color.g, color.b, color.a];
            count += 1;
        }
        BackdropBlurParams {
            dst_rect: [
                plan.draw.origin.x.0,
                plan.draw.origin.y.0,
                plan.draw.size.width.0,
                plan.draw.size.height.0,
            ],
            target_size: [width, height, 0.0, 0.0],
            src_size,
            src_clamp,
            params: [1.0, 0.0, 0.0, 0.0],
            shape: [
                self.bounds.origin.x.0,
                self.bounds.origin.y.0,
                self.bounds.size.width.0,
                self.bounds.size.height.0,
            ],
            radii: [
                self.corner_radii.top_left.0,
                self.corner_radii.top_right.0,
                self.corner_radii.bottom_right.0,
                self.corner_radii.bottom_left.0,
            ],
            tint: [tint.r, tint.g, tint.b, tint.a],
            extra: [
                self.saturation,
                self.opacity.clamp(0.0, 1.0),
                if premultiplied { 1.0 } else { 0.0 },
                plan.scale,
            ],
            light_dir: [
                light.direction[0],
                light.direction[1],
                light.directional.max(0.0),
                light.ambient.max(0.0),
            ],
            light_a: [
                light.edge.max(0.0),
                light.edge_width.0,
                light.specular.max(0.0),
                light.specular_size.0,
            ],
            light_b: [
                light.specular_position[0],
                light.specular_position[1],
                light.reflection.max(0.0),
                self.noise,
            ],
            light_color: [
                light_color.r,
                light_color.g,
                light_color.b,
                self.tint_blend as f32,
            ],
            mask: self.mask,
            misc: [count as f32, offset[0], offset[1], 0.0],
            shadows,
        }
    }
}

/// Whether the blur cache is on (default) — `GPUI_BLUR_CACHE=0` turns it
/// off, e.g. to benchmark the uncached path.
pub(crate) fn blur_cache_enabled() -> bool {
    std::env::var("GPUI_BLUR_CACHE").map_or(true, |v| v != "0")
}

/// Cache entries older than this many frames are released.
pub(crate) const BLUR_CACHE_MAX_AGE: u64 = 3;
/// At most this many blurred backdrops are kept.
pub(crate) const BLUR_CACHE_MAX_ENTRIES: usize = 24;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{size, BackdropBlurLight, BackdropBlurShadow, ContentMask, Corners, Hsla};

    fn blur(x: f32, y: f32, w: f32, h: f32, sigma: f32) -> BackdropBlur {
        let bounds = Bounds {
            origin: point(ScaledPixels(x), ScaledPixels(y)),
            size: size(ScaledPixels(w), ScaledPixels(h)),
        };
        BackdropBlur {
            order: 0,
            bounds,
            content_mask: ContentMask {
                bounds: Bounds {
                    origin: point(ScaledPixels(0.0), ScaledPixels(0.0)),
                    size: size(ScaledPixels(4000.0), ScaledPixels(4000.0)),
                },
            },
            corner_radii: Corners::default(),
            sigma: ScaledPixels(sigma),
            tint: Hsla::transparent_black(),
            saturation: 1.0,
            opacity: 1.0,
            tint_blend: 0,
            noise: 0.0,
            light: BackdropBlurLight::default(),
            mask: [0.0; 4],
            shadows: [BackdropBlurShadow::default(); 4],
        }
    }

    #[test]
    fn plan_snaps_to_the_downsample_grid() {
        let plan = blur(101.0, 50.0, 200.0, 100.0, 16.0)
            .plan(1000.0, 800.0)
            .unwrap();
        assert_eq!(plan.scale, 4.0);
        for v in plan.copy {
            assert_eq!(v % 4.0, 0.0);
        }
        // The copy covers the shape plus the 3σ kernel footprint.
        assert!(plan.copy[0] <= 101.0 - 48.0);
        assert!(plan.copy[2] >= 301.0 + 48.0);
        assert_eq!(plan.region_ds[0] * 4.0, plan.copy[0]);
    }

    #[test]
    fn plan_clips_to_the_drawable_and_skips_hidden_shapes() {
        let plan = blur(-50.0, -50.0, 100.0, 100.0, 8.0)
            .plan(500.0, 500.0)
            .unwrap();
        assert_eq!(plan.copy[0], 0.0);
        assert_eq!(plan.copy[1], 0.0);
        assert!(blur(600.0, 0.0, 50.0, 50.0, 8.0)
            .plan(500.0, 500.0)
            .is_none());
    }

    #[test]
    fn plan_still_composites_a_visible_sliver_at_the_drawable_edge() {
        // A 1px sample at the drawable edge with downsample ×8 used to snap
        // to a 0-texel region (`floor(10/8) == 8/8`), so `plan()` returned
        // None and the renderer left a transparent hole.
        let sample = Bounds {
            origin: point(ScaledPixels(9.0), ScaledPixels(0.0)),
            size: size(ScaledPixels(1.0), ScaledPixels(10.0)),
        };
        let (copy, region_ds) = snap_copy_region(sample, 10.0, 10.0, 8.0);
        assert!(region_ds[2] > region_ds[0], "{region_ds:?}");
        assert!(region_ds[3] > region_ds[1], "{region_ds:?}");
        assert!(copy[2] > copy[0]);
        assert!(copy[3] > copy[1]);

        let mut b = blur(9.0, 0.0, 1.0, 10.0, 40.0);
        b.tint = Hsla::white().opacity(0.62);
        let plan = b
            .plan(10.0, 10.0)
            .expect("visible sliver must still be planned");
        assert!(plan.region_ds[2] > plan.region_ds[0]);
        assert!(plan.region_ds[3] > plan.region_ds[1]);
        let viewport = Bounds {
            origin: point(ScaledPixels(0.0), ScaledPixels(0.0)),
            size: size(ScaledPixels(10.0), ScaledPixels(10.0)),
        };
        let visible = b.bounds.intersect(&viewport);
        assert!(!plan.draw.intersect(&visible).is_empty());
    }

    #[test]
    fn shadows_extend_the_draw_rect_but_not_the_sample() {
        let mut b = blur(100.0, 100.0, 100.0, 100.0, 4.0);
        let plain = b.plan(1000.0, 1000.0).unwrap();
        b.shadows[0] = BackdropBlurShadow {
            offset: point(ScaledPixels(0.0), ScaledPixels(20.0)),
            blur_radius: ScaledPixels(10.0),
            spread_radius: ScaledPixels(0.0),
            color: Hsla::black().opacity(0.3),
        };
        let shadowed = b.plan(1000.0, 1000.0).unwrap();
        assert_eq!(plain.copy, shadowed.copy);
        assert!(shadowed.draw.size.height.0 > plain.draw.size.height.0 + 40.0);
        let params = b.composite_params(&shadowed, BlurSource::Cached, false);
        assert_eq!(params.misc[0], 1.0);
        assert_eq!(params.misc[1], shadowed.region_ds[0]);
    }

    #[test]
    fn params_layout_matches_the_shaders() {
        // 15 vec4 + 8 vec4 of shadows.
        assert_eq!(std::mem::size_of::<BackdropBlurParams>(), 23 * 16);
    }
}
