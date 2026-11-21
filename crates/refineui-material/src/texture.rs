//! Texture: grain. A small tile of deterministic noise is generated once
//! (per pixel size bucket), uploaded to GPUI's sprite atlas and painted as a
//! single image sprite at low opacity. Grain hides banding in large smooth
//! blurs and gives glass a physical feel; it is optional (opacity 0 = off).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use gpui::{Bounds, Corners, Pixels, RenderImage, Window};
use image::{Frame, RgbaImage};

/// Grain images are generated at these sizes (device px, square) and
/// stretched to the surface; one 256-px tile covers typical panels with
/// near-1:1 texels.
const TILE_SIZES: [u32; 3] = [128, 256, 512];

fn cache() -> &'static Mutex<HashMap<(u32, u8), Arc<RenderImage>>> {
    static CACHE: OnceLock<Mutex<HashMap<(u32, u8), Arc<RenderImage>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Deterministic white noise value for a pixel (0–255).
pub fn noise_value(x: u32, y: u32) -> u8 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h >> 24) as u8
}

/// The cached grain tile of `size` px whose alpha is `alpha` (0–255).
pub fn grain_tile(size: u32, alpha: u8) -> Arc<RenderImage> {
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    cache
        .entry((size, alpha))
        .or_insert_with(|| {
            let mut img = RgbaImage::new(size, size);
            for (x, y, p) in img.enumerate_pixels_mut() {
                let v = noise_value(x, y);
                // Premultiplied-friendly gray (BGRA and RGBA are the same
                // for gray).
                *p = image::Rgba([v, v, v, alpha]);
            }
            Arc::new(RenderImage::new(smallvec::smallvec![Frame::new(img)]))
        })
        .clone()
}

/// Paint grain over `bounds` at `opacity`.
pub fn paint_grain(bounds: Bounds<Pixels>, radius: Pixels, opacity: f32, window: &mut Window) {
    if opacity <= 0. {
        return;
    }
    let scale = window.scale_factor();
    let longest = (f32::from(bounds.size.width.max(bounds.size.height)) * scale) as u32;
    let size = TILE_SIZES
        .into_iter()
        .find(|s| *s >= longest / 2)
        .unwrap_or(TILE_SIZES[TILE_SIZES.len() - 1]);
    let alpha = (opacity.clamp(0., 1.) * 255.).round() as u8;
    let _ = window.paint_image(
        bounds,
        Corners::all(radius),
        grain_tile(size, alpha),
        0,
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_spread() {
        assert_eq!(noise_value(3, 4), noise_value(3, 4));
        let mean: f32 = (0..64)
            .flat_map(|y| (0..64).map(move |x| noise_value(x, y) as f32))
            .sum::<f32>()
            / 4096.;
        assert!((100.0..156.0).contains(&mean), "mean {mean}");
    }

    #[test]
    fn grain_tiles_are_cached() {
        let a = grain_tile(128, 6);
        let b = grain_tile(128, 6);
        assert!(Arc::ptr_eq(&a, &b));
    }
}
