# Vendored gpui 0.2.2 — RefineUI patch

This is crates.io `gpui 0.2.2` (normalized manifest; examples/tests dropped)
wired in with `[patch.crates-io] gpui = { path = "vendor/gpui" }`. It has the
same version as crates.io, so the graph still has exactly one gpui.

The complete change is `refineui.patch` (unified diff against the pristine
crate sources). To re-vendor a newer gpui, copy its sources here and
re-apply that patch.

## Why the public API is not enough

gpui 0.2.2 elements can only add primitives (quads, shadows, paths, sprites,
surfaces) to a `Scene`. The renderers draw the scene in a single render pass
into the drawable, and no public API exposes the pixels already drawn,
offscreen targets, or custom shaders. Backdrop blur needs to read what was
drawn *behind* a surface mid-frame, so it has to live inside the renderer.

## What changed

| File | Change |
| --- | --- |
| `src/scene.rs` | New `BackdropBlur` primitive + `PrimitiveKind::BackdropBlur` (sorted first within one draw order) and `PrimitiveBatch::BackdropBlurs`. Its draw order is computed from the *sampled* region (shape + 3σ), so everything painted earlier under the kernel footprint is drawn before it. |
| `src/scene.rs` (Phase 3) | `BackdropBlur` carries tint blend mode, grain, lighting (`BackdropBlurLight`), a linear mask and up to 4 drop shadows (`BackdropBlurShadow`); `paint_bounds()` includes the shadow extent. Blurs inserted inside a `paint_layer` are ordered by `union(paint bounds, layer bounds)` and bump the layer order (`Scene::layer_bounds`), so later content in the layer draws above the blur. `backdrop_blur_keys()` hashes the primitives under each blur for the cache. |
| `src/backdrop_blur.rs` (new) | Renderer-independent planning: `BackdropBlurParams` (368-byte uniform shared by WGSL/MSL), `BackdropBlurPlan` (copy region, downsample, Gaussian params), cache constants, `blur_cache_enabled()` (`GPUI_BLUR_CACHE=0` disables). `plan()` never returns `None` for a visible shape: a sliver at the drawable edge that snaps to a 0-texel copy is given a 1-texel region so the composite still paints the tinted plate (otherwise Blade/Metal skipped the primitive and the surface went fully transparent). |
| `src/window.rs` | `Window::paint_backdrop_blur(bounds, BackdropBlurStyle)`, `BackdropBlurStyle` (+ `BackdropBlend`, `BackdropLight`, `BackdropMask`, `BackdropShadow`), `backdrop_blur_supported()` (false on Windows). |
| `src/platform/blade/blade_renderer.rs` + `backdrop_blur.wgsl` | Surface usage `TARGET \| COPY`. At a blur batch the main pass is closed; the region is copied out of the drawable, box-downsampled (×1/2/4/8 by σ) into an `Rgba16Float` texture, Gaussian-blurred horizontally and vertically, then composited (saturation, tint, rounded-rect SDF coverage, opacity) and the main pass resumes with `Load` — the same pattern gpui already uses for paths. Textures are cached per drawable size and recreated on resize. |
| `src/platform/blade/*` (Phase 3) | Composite pass evaluates saturation, blended tint, lighting, grain + dithering, mask and shadows (outside the shape). Blur cache: per-key result textures; hits composite inside the main pass, misses run the full pipeline and store the result; eviction after 3 unused frames (max 24); cleared on resize. |
| `src/platform/mac/metal_renderer.rs` + `shaders.metal` | The same algorithm and cache for the Metal renderer (`framebufferOnly = false` so the drawable can be blitted). **Not verified on hardware** — Rust type-checked with `cargo check --target aarch64-apple-darwin` on Linux; the MSL composite is syntax-checked against a C++ shim only. |
| `src/platform/windows/directx_renderer.rs` | No-op match arm (callers paint the material fallback). |
| `src/taffy.rs` | `0.0_f32` literals (silences a future-incompat warning now that gpui is a path dependency). |

## Cost

Per blurred surface per frame: one copy of the sampled region, three small
fragment passes over the downsampled region (≤ 64 taps each way), one
composite pass over the visible shape (plus its shadow extent). When the
region under a blur is unchanged since the last frame, only the composite
runs (cached result). Cache textures are allocated on a miss and reused.
