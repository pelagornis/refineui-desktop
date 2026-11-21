//! # refineui-material — the RefineUI Material Engine
//!
//! **Layer C** of RefineUI Desktop. Materials are surfaces that show what is
//! behind them: real backdrop blur, saturation, tint with blend modes,
//! lighting (directional, ambient, edge, specular, reflection), GPU grain,
//! masks, drop and inner shadows, clipped to a rounded rectangle. The engine depends only on GPUI and
//! [`refineui_foundation`] — never on the component library — so any GPUI
//! view can use it, and components consume it like any other app.
//!
//! ```ignore
//! use refineui_material::prelude::*;
//!
//! material_surface(Material::role(MaterialRole::Regular, Mode::Light))
//!     .p_4()
//!     .child("Frosted panel");
//! ```
//!
//! ## Real blur
//!
//! [`backdrop`] paints `Window::paint_backdrop_blur`, a primitive added by a
//! small patch to GPUI (`vendor/gpui`, see `REFINEUI_PATCH.md`). The renderer
//! copies the pixels already drawn behind the surface, blurs them (downsample
//! and a separable Gaussian in a float texture) and composites them back inside
//! the rounded shape. Nothing is faked with translucency: where the renderer
//! has no blur (DirectX today) or reduced transparency is on, the surface is
//! drawn with the material's documented opaque [`Material::fallback`] fill.
//!
//! ## Modules
//!
//! - [`surface`] — [`Material`], the one description of a surface.
//! - [`backdrop`] — capability check, settings, the blur layer.
//! - [`lighting`] — directional / ambient / edge / specular / reflection.
//! - [`texture`] — grain: per-pixel GPU noise with real blur, a cached tile
//!   on the fallback path.
//! - [`shadow`] — elevation, ambient, directional, layered and inner shadows.
//! - [`composite`] — blend modes, masks, ordering / clipping / nesting rules.
//! - [`recipes`] — sidebar, titlebar, popover, menu, tooltip, HUD, sheet,
//!   toast, scrim, glass.
//! - [`element`] — [`MaterialSurface`], compositing everything in order.
//!
//! ## Caching
//!
//! The renderer fingerprints what is drawn under each blur; when it is
//! unchanged from the previous frame the blurred backdrop is reused and only
//! the composite runs (no copy, no blur passes). `GPUI_BLUR_CACHE=0`
//! disables this for benchmarking.

#![warn(missing_docs)]

pub mod backdrop;
pub mod composite;
pub mod element;
pub mod lighting;
pub mod recipes;
pub mod shadow;
pub mod surface;
pub mod texture;

pub use backdrop::{BlurSupport, MaterialSettings, backdrop_blur_supported, paint_backdrop};
pub use composite::{BlendMode, Mask};
pub use element::{MaterialSurface, material_surface};
pub use lighting::Lighting;
pub use recipes::Recipe;
pub use refineui_foundation::{MaterialRole, MaterialTokens, Mode};
pub use shadow::{InnerShadow, ShadowPreset, elevation_shadows};
pub use surface::Material;

/// Everything needed to build materials.
pub mod prelude {
    pub use crate::{
        BlendMode, BlurSupport, InnerShadow, Lighting, Mask, Material, MaterialRole,
        MaterialSettings, MaterialSurface, Mode, Recipe, ShadowPreset, backdrop_blur_supported,
        elevation_shadows, material_surface, recipes,
    };
}

/// Foundation color → GPUI color.
pub fn hsla(color: refineui_foundation::Color) -> gpui::Hsla {
    gpui::rgba(color.0).into()
}
