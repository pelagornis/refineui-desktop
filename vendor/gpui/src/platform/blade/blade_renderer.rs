// Doing `if let` gives you nice scoping with passes/encoders
#![allow(irrefutable_let_patterns)]

use super::{BladeAtlas, BladeContext};
use crate::backdrop_blur::{
    BLUR_CACHE_MAX_AGE, BLUR_CACHE_MAX_ENTRIES, BackdropBlurParams, BackdropBlurPlan, BlurSource,
    blur_cache_enabled,
};
use crate::{
    BackdropBlur, Background, Bounds, DevicePixels, GpuSpecs, MonochromeSprite, Path, Point,
    PolychromeSprite, PrimitiveBatch, Quad, ScaledPixels, Scene, Shadow, Size, Underline,
};
use blade_graphics as gpu;
use blade_util::{BufferBelt, BufferBeltDescriptor};
use bytemuck::{Pod, Zeroable};
#[cfg(target_os = "macos")]
use media::core_video::CVMetalTextureCache;
use collections::HashMap;
use std::sync::Arc;

const MAX_FRAME_TIME_MS: u32 = 10000;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlobalParams {
    viewport_size: [f32; 2],
    premultiplied_alpha: u32,
    pad: u32,
}

//Note: we can't use `Bounds` directly here because
// it doesn't implement Pod + Zeroable
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PodBounds {
    origin: [f32; 2],
    size: [f32; 2],
}

impl From<Bounds<ScaledPixels>> for PodBounds {
    fn from(bounds: Bounds<ScaledPixels>) -> Self {
        Self {
            origin: [bounds.origin.x.0, bounds.origin.y.0],
            size: [bounds.size.width.0, bounds.size.height.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SurfaceParams {
    bounds: PodBounds,
    content_mask: PodBounds,
}

#[derive(blade_macros::ShaderData)]
struct ShaderQuadsData {
    globals: GlobalParams,
    b_quads: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct ShaderShadowsData {
    globals: GlobalParams,
    b_shadows: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct ShaderPathRasterizationData {
    globals: GlobalParams,
    b_path_vertices: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct ShaderPathsData {
    globals: GlobalParams,
    t_sprite: gpu::TextureView,
    s_sprite: gpu::Sampler,
    b_path_sprites: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct ShaderUnderlinesData {
    globals: GlobalParams,
    b_underlines: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct ShaderMonoSpritesData {
    globals: GlobalParams,
    gamma_ratios: [f32; 4],
    grayscale_enhanced_contrast: f32,
    t_sprite: gpu::TextureView,
    s_sprite: gpu::Sampler,
    b_mono_sprites: gpu::BufferPiece,
}

#[derive(blade_macros::ShaderData)]
struct ShaderPolySpritesData {
    globals: GlobalParams,
    t_sprite: gpu::TextureView,
    s_sprite: gpu::Sampler,
    b_poly_sprites: gpu::BufferPiece,
}

// RefineUI patch: backdrop blur. Mirrors `BlurParams` in backdrop_blur.wgsl.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct BlurParams(BackdropBlurParams);

// SAFETY: `BackdropBlurParams` is `repr(C)` and made only of `f32` arrays
// (no padding, every bit pattern valid).
unsafe impl Zeroable for BlurParams {}
unsafe impl Pod for BlurParams {}

#[derive(blade_macros::ShaderData)]
struct ShaderBlurData {
    blur_params: BlurParams,
    t_blur: gpu::TextureView,
    s_blur: gpu::Sampler,
}

/// Format of the intermediate blur textures: float, so wide blurs of smooth
/// gradients do not band.
const BLUR_TEXTURE_FORMAT: gpu::TextureFormat = gpu::TextureFormat::Rgba16Float;

/// Cached intermediate textures for backdrop blur, sized to the drawable.
#[derive(Clone, Copy)]
enum BlurStep {
    Downsample,
    Gaussian,
}

/// A blurred backdrop kept for reuse while what is under it is unchanged.
struct CachedBlur {
    texture: gpu::Texture,
    view: gpu::TextureView,
    size: (u32, u32),
    last_used: u64,
}

struct BlurTargets {
    /// Copy of the frame region behind the plate (surface format).
    source: (gpu::Texture, gpu::TextureView),
    /// Ping-pong float textures (downsampled space).
    a: (gpu::Texture, gpu::TextureView),
    b: (gpu::Texture, gpu::TextureView),
    size: gpu::Extent,
    needs_init: bool,
}

#[derive(blade_macros::ShaderData)]
struct ShaderSurfacesData {
    globals: GlobalParams,
    surface_locals: SurfaceParams,
    t_y: gpu::TextureView,
    t_cb_cr: gpu::TextureView,
    s_surface: gpu::Sampler,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(C)]
struct PathSprite {
    bounds: Bounds<ScaledPixels>,
}

#[derive(Clone, Debug)]
#[repr(C)]
struct PathRasterizationVertex {
    xy_position: Point<ScaledPixels>,
    st_position: Point<f32>,
    color: Background,
    bounds: Bounds<ScaledPixels>,
}

struct BladePipelines {
    quads: gpu::RenderPipeline,
    shadows: gpu::RenderPipeline,
    path_rasterization: gpu::RenderPipeline,
    paths: gpu::RenderPipeline,
    underlines: gpu::RenderPipeline,
    mono_sprites: gpu::RenderPipeline,
    poly_sprites: gpu::RenderPipeline,
    surfaces: gpu::RenderPipeline,
    blur_downsample: gpu::RenderPipeline,
    blur_gaussian: gpu::RenderPipeline,
    blur_composite: gpu::RenderPipeline,
}

impl BladePipelines {
    fn new(gpu: &gpu::Context, surface_info: gpu::SurfaceInfo, path_sample_count: u32) -> Self {
        use gpu::ShaderData as _;

        log::info!(
            "Initializing Blade pipelines for surface {:?}",
            surface_info
        );
        let shader = gpu.create_shader(gpu::ShaderDesc {
            source: include_str!("shaders.wgsl"),
        });
        shader.check_struct_size::<GlobalParams>();
        shader.check_struct_size::<SurfaceParams>();
        shader.check_struct_size::<Quad>();
        shader.check_struct_size::<Shadow>();
        shader.check_struct_size::<PathRasterizationVertex>();
        shader.check_struct_size::<PathSprite>();
        shader.check_struct_size::<Underline>();
        shader.check_struct_size::<MonochromeSprite>();
        shader.check_struct_size::<PolychromeSprite>();
        let blur_shader = gpu.create_shader(gpu::ShaderDesc {
            source: include_str!("backdrop_blur.wgsl"),
        });
        blur_shader.check_struct_size::<BlurParams>();
        let blur_targets = &[gpu::ColorTargetState {
            format: BLUR_TEXTURE_FORMAT,
            blend: None,
            write_mask: gpu::ColorWrites::default(),
        }];

        // See https://apoorvaj.io/alpha-compositing-opengl-blending-and-premultiplied-alpha/
        let blend_mode = match surface_info.alpha {
            gpu::AlphaMode::Ignored => gpu::BlendState::ALPHA_BLENDING,
            gpu::AlphaMode::PreMultiplied => gpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            gpu::AlphaMode::PostMultiplied => gpu::BlendState::ALPHA_BLENDING,
        };
        let color_targets = &[gpu::ColorTargetState {
            format: surface_info.format,
            blend: Some(blend_mode),
            write_mask: gpu::ColorWrites::default(),
        }];

        Self {
            quads: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "quads",
                data_layouts: &[&ShaderQuadsData::layout()],
                vertex: shader.at("vs_quad"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_quad")),
                color_targets,
                multisample_state: gpu::MultisampleState::default(),
            }),
            shadows: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "shadows",
                data_layouts: &[&ShaderShadowsData::layout()],
                vertex: shader.at("vs_shadow"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_shadow")),
                color_targets,
                multisample_state: gpu::MultisampleState::default(),
            }),
            path_rasterization: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "path_rasterization",
                data_layouts: &[&ShaderPathRasterizationData::layout()],
                vertex: shader.at("vs_path_rasterization"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_path_rasterization")),
                // The original implementation was using ADDITIVE blende mode,
                // I don't know why
                // color_targets: &[gpu::ColorTargetState {
                //     format: PATH_TEXTURE_FORMAT,
                //     blend: Some(gpu::BlendState::ADDITIVE),
                //     write_mask: gpu::ColorWrites::default(),
                // }],
                color_targets: &[gpu::ColorTargetState {
                    format: surface_info.format,
                    blend: Some(gpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: gpu::ColorWrites::default(),
                }],
                multisample_state: gpu::MultisampleState {
                    sample_count: path_sample_count,
                    ..Default::default()
                },
            }),
            paths: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "paths",
                data_layouts: &[&ShaderPathsData::layout()],
                vertex: shader.at("vs_path"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_path")),
                color_targets: &[gpu::ColorTargetState {
                    format: surface_info.format,
                    blend: Some(gpu::BlendState {
                        color: gpu::BlendComponent::OVER,
                        alpha: gpu::BlendComponent::ADDITIVE,
                    }),
                    write_mask: gpu::ColorWrites::default(),
                }],
                multisample_state: gpu::MultisampleState::default(),
            }),
            underlines: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "underlines",
                data_layouts: &[&ShaderUnderlinesData::layout()],
                vertex: shader.at("vs_underline"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_underline")),
                color_targets,
                multisample_state: gpu::MultisampleState::default(),
            }),
            mono_sprites: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "mono-sprites",
                data_layouts: &[&ShaderMonoSpritesData::layout()],
                vertex: shader.at("vs_mono_sprite"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_mono_sprite")),
                color_targets,
                multisample_state: gpu::MultisampleState::default(),
            }),
            poly_sprites: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "poly-sprites",
                data_layouts: &[&ShaderPolySpritesData::layout()],
                vertex: shader.at("vs_poly_sprite"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_poly_sprite")),
                color_targets,
                multisample_state: gpu::MultisampleState::default(),
            }),
            surfaces: gpu.create_render_pipeline(gpu::RenderPipelineDesc {
                name: "surfaces",
                data_layouts: &[&ShaderSurfacesData::layout()],
                vertex: shader.at("vs_surface"),
                vertex_fetches: &[],
                primitive: gpu::PrimitiveState {
                    topology: gpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                fragment: Some(shader.at("fs_surface")),
                color_targets,
                multisample_state: gpu::MultisampleState::default(),
            }),
            blur_downsample: blur_pipeline(gpu, &blur_shader, "blur_downsample", "fs_downsample", blur_targets),
            blur_gaussian: blur_pipeline(gpu, &blur_shader, "blur_gaussian", "fs_blur", blur_targets),
            blur_composite: blur_pipeline(gpu, &blur_shader, "blur_composite", "fs_composite", color_targets),
        }
    }

    fn destroy(&mut self, gpu: &gpu::Context) {
        gpu.destroy_render_pipeline(&mut self.quads);
        gpu.destroy_render_pipeline(&mut self.shadows);
        gpu.destroy_render_pipeline(&mut self.path_rasterization);
        gpu.destroy_render_pipeline(&mut self.paths);
        gpu.destroy_render_pipeline(&mut self.underlines);
        gpu.destroy_render_pipeline(&mut self.mono_sprites);
        gpu.destroy_render_pipeline(&mut self.poly_sprites);
        gpu.destroy_render_pipeline(&mut self.surfaces);
        gpu.destroy_render_pipeline(&mut self.blur_downsample);
        gpu.destroy_render_pipeline(&mut self.blur_gaussian);
        gpu.destroy_render_pipeline(&mut self.blur_composite);
    }
}

fn blur_pipeline(
    gpu: &gpu::Context,
    shader: &gpu::Shader,
    name: &str,
    fragment: &str,
    color_targets: &[gpu::ColorTargetState],
) -> gpu::RenderPipeline {
    use gpu::ShaderData as _;
    gpu.create_render_pipeline(gpu::RenderPipelineDesc {
        name,
        data_layouts: &[&ShaderBlurData::layout()],
        vertex: shader.at("vs_blur"),
        vertex_fetches: &[],
        primitive: gpu::PrimitiveState {
            topology: gpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: None,
        fragment: Some(shader.at(fragment)),
        color_targets,
        multisample_state: gpu::MultisampleState::default(),
    })
}

pub struct BladeSurfaceConfig {
    pub size: gpu::Extent,
    pub transparent: bool,
}

//Note: we could see some of these fields moved into `BladeContext`
// so that they are shared between windows. E.g. `pipelines`.
// But that is complicated by the fact that pipelines depend on
// the format and alpha mode.
pub struct BladeRenderer {
    gpu: Arc<gpu::Context>,
    surface: gpu::Surface,
    surface_config: gpu::SurfaceConfig,
    command_encoder: gpu::CommandEncoder,
    last_sync_point: Option<gpu::SyncPoint>,
    pipelines: BladePipelines,
    instance_belt: BufferBelt,
    atlas: Arc<BladeAtlas>,
    atlas_sampler: gpu::Sampler,
    #[cfg(target_os = "macos")]
    core_video_texture_cache: CVMetalTextureCache,
    path_intermediate_texture: gpu::Texture,
    path_intermediate_texture_view: gpu::TextureView,
    path_intermediate_msaa_texture: Option<gpu::Texture>,
    path_intermediate_msaa_texture_view: Option<gpu::TextureView>,
    rendering_parameters: RenderingParameters,
    blur_targets: Option<BlurTargets>,
    blur_cache: HashMap<u64, CachedBlur>,
    blur_cache_enabled: bool,
    frame_index: u64,
}

impl BladeRenderer {
    pub fn new<I: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle>(
        context: &BladeContext,
        window: &I,
        config: BladeSurfaceConfig,
    ) -> anyhow::Result<Self> {
        let surface_config = gpu::SurfaceConfig {
            size: config.size,
            // COPY: backdrop blur copies the region behind a plate out of
            // the drawable (RefineUI patch).
            usage: gpu::TextureUsage::TARGET | gpu::TextureUsage::COPY,
            display_sync: gpu::DisplaySync::Recent,
            color_space: gpu::ColorSpace::Srgb,
            allow_exclusive_full_screen: false,
            transparent: config.transparent,
        };
        let surface = context
            .gpu
            .create_surface_configured(window, surface_config)
            .map_err(|err| anyhow::anyhow!("Failed to create surface: {err:?}"))?;

        let command_encoder = context.gpu.create_command_encoder(gpu::CommandEncoderDesc {
            name: "main",
            buffer_count: 2,
        });
        let rendering_parameters = RenderingParameters::from_env(context);
        let pipelines = BladePipelines::new(
            &context.gpu,
            surface.info(),
            rendering_parameters.path_sample_count,
        );
        let instance_belt = BufferBelt::new(BufferBeltDescriptor {
            memory: gpu::Memory::Shared,
            min_chunk_size: 0x1000,
            alignment: 0x40, // Vulkan `minStorageBufferOffsetAlignment` on Intel Xe
        });
        let atlas = Arc::new(BladeAtlas::new(&context.gpu));
        let atlas_sampler = context.gpu.create_sampler(gpu::SamplerDesc {
            name: "path rasterization sampler",
            mag_filter: gpu::FilterMode::Linear,
            min_filter: gpu::FilterMode::Linear,
            ..Default::default()
        });

        let (path_intermediate_texture, path_intermediate_texture_view) =
            create_path_intermediate_texture(
                &context.gpu,
                surface.info().format,
                config.size.width,
                config.size.height,
            );
        let (path_intermediate_msaa_texture, path_intermediate_msaa_texture_view) =
            create_msaa_texture_if_needed(
                &context.gpu,
                surface.info().format,
                config.size.width,
                config.size.height,
                rendering_parameters.path_sample_count,
            )
            .unzip();

        #[cfg(target_os = "macos")]
        let core_video_texture_cache = unsafe {
            CVMetalTextureCache::new(
                objc2::rc::Retained::as_ptr(&context.gpu.metal_device()) as *mut _
            )
            .unwrap()
        };

        Ok(Self {
            gpu: Arc::clone(&context.gpu),
            surface,
            surface_config,
            command_encoder,
            last_sync_point: None,
            pipelines,
            instance_belt,
            atlas,
            atlas_sampler,
            #[cfg(target_os = "macos")]
            core_video_texture_cache,
            path_intermediate_texture,
            path_intermediate_texture_view,
            path_intermediate_msaa_texture,
            path_intermediate_msaa_texture_view,
            rendering_parameters,
            blur_targets: None,
            blur_cache: HashMap::default(),
            blur_cache_enabled: blur_cache_enabled(),
            frame_index: 0,
        })
    }

    fn wait_for_gpu(&mut self) {
        if let Some(last_sp) = self.last_sync_point.take()
            && !self.gpu.wait_for(&last_sp, MAX_FRAME_TIME_MS)
        {
            log::error!("GPU hung");
            #[cfg(target_os = "linux")]
            if self.gpu.device_information().driver_name == "radv" {
                log::error!(
                    "there's a known bug with amdgpu/radv, try setting ZED_PATH_SAMPLE_COUNT=0 as a workaround"
                );
                log::error!(
                    "if that helps you're running into https://github.com/zed-industries/zed/issues/26143"
                );
            }
            log::error!(
                "your device information is: {:?}",
                self.gpu.device_information()
            );
            while !self.gpu.wait_for(&last_sp, MAX_FRAME_TIME_MS) {}
        }
    }

    pub fn update_drawable_size(&mut self, size: Size<DevicePixels>) {
        self.update_drawable_size_impl(size, false);
    }

    /// Like `update_drawable_size` but skips the check that the size has changed. This is useful in
    /// cases like restoring a window from minimization where the size is the same but the
    /// renderer's swap chain needs to be recreated.
    #[cfg_attr(
        any(target_os = "macos", target_os = "linux", target_os = "freebsd"),
        allow(dead_code)
    )]
    pub fn update_drawable_size_even_if_unchanged(&mut self, size: Size<DevicePixels>) {
        self.update_drawable_size_impl(size, true);
    }

    fn update_drawable_size_impl(&mut self, size: Size<DevicePixels>, always_resize: bool) {
        let gpu_size = gpu::Extent {
            width: size.width.0 as u32,
            height: size.height.0 as u32,
            depth: 1,
        };

        if always_resize || gpu_size != self.surface_config.size {
            self.wait_for_gpu();
            self.surface_config.size = gpu_size;
            self.gpu
                .reconfigure_surface(&mut self.surface, self.surface_config);
            self.destroy_blur_targets();
            self.destroy_blur_cache();
            self.gpu.destroy_texture(self.path_intermediate_texture);
            self.gpu
                .destroy_texture_view(self.path_intermediate_texture_view);
            if let Some(msaa_texture) = self.path_intermediate_msaa_texture {
                self.gpu.destroy_texture(msaa_texture);
            }
            if let Some(msaa_view) = self.path_intermediate_msaa_texture_view {
                self.gpu.destroy_texture_view(msaa_view);
            }
            let (path_intermediate_texture, path_intermediate_texture_view) =
                create_path_intermediate_texture(
                    &self.gpu,
                    self.surface.info().format,
                    gpu_size.width,
                    gpu_size.height,
                );
            self.path_intermediate_texture = path_intermediate_texture;
            self.path_intermediate_texture_view = path_intermediate_texture_view;
            let (path_intermediate_msaa_texture, path_intermediate_msaa_texture_view) =
                create_msaa_texture_if_needed(
                    &self.gpu,
                    self.surface.info().format,
                    gpu_size.width,
                    gpu_size.height,
                    self.rendering_parameters.path_sample_count,
                )
                .unzip();
            self.path_intermediate_msaa_texture = path_intermediate_msaa_texture;
            self.path_intermediate_msaa_texture_view = path_intermediate_msaa_texture_view;
        }
    }

    pub fn update_transparency(&mut self, transparent: bool) {
        if transparent != self.surface_config.transparent {
            self.wait_for_gpu();
            self.surface_config.transparent = transparent;
            self.gpu
                .reconfigure_surface(&mut self.surface, self.surface_config);
            self.pipelines.destroy(&self.gpu);
            self.pipelines = BladePipelines::new(
                &self.gpu,
                self.surface.info(),
                self.rendering_parameters.path_sample_count,
            );
        }
    }

    #[cfg_attr(
        any(target_os = "macos", feature = "wayland", target_os = "windows"),
        allow(dead_code)
    )]
    pub fn viewport_size(&self) -> gpu::Extent {
        self.surface_config.size
    }

    pub fn sprite_atlas(&self) -> &Arc<BladeAtlas> {
        &self.atlas
    }

    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub fn gpu_specs(&self) -> GpuSpecs {
        let info = self.gpu.device_information();

        GpuSpecs {
            is_software_emulated: info.is_software_emulated,
            device_name: info.device_name.clone(),
            driver_name: info.driver_name.clone(),
            driver_info: info.driver_info.clone(),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn layer(&self) -> metal::MetalLayer {
        unsafe { foreign_types::ForeignType::from_ptr(self.layer_ptr()) }
    }

    #[cfg(target_os = "macos")]
    pub fn layer_ptr(&self) -> *mut metal::CAMetalLayer {
        objc2::rc::Retained::as_ptr(&self.surface.metal_layer()) as *mut _
    }

    #[profiling::function]
    fn draw_paths_to_intermediate(
        &mut self,
        paths: &[Path<ScaledPixels>],
        width: f32,
        height: f32,
    ) {
        self.command_encoder
            .init_texture(self.path_intermediate_texture);
        if let Some(msaa_texture) = self.path_intermediate_msaa_texture {
            self.command_encoder.init_texture(msaa_texture);
        }

        let target = if let Some(msaa_view) = self.path_intermediate_msaa_texture_view {
            gpu::RenderTarget {
                view: msaa_view,
                init_op: gpu::InitOp::Clear(gpu::TextureColor::TransparentBlack),
                finish_op: gpu::FinishOp::ResolveTo(self.path_intermediate_texture_view),
            }
        } else {
            gpu::RenderTarget {
                view: self.path_intermediate_texture_view,
                init_op: gpu::InitOp::Clear(gpu::TextureColor::TransparentBlack),
                finish_op: gpu::FinishOp::Store,
            }
        };
        if let mut pass = self.command_encoder.render(
            "rasterize paths",
            gpu::RenderTargetSet {
                colors: &[target],
                depth_stencil: None,
            },
        ) {
            let globals = GlobalParams {
                viewport_size: [width, height],
                premultiplied_alpha: 0,
                pad: 0,
            };
            let mut encoder = pass.with(&self.pipelines.path_rasterization);

            let mut vertices = Vec::new();
            for path in paths {
                vertices.extend(path.vertices.iter().map(|v| PathRasterizationVertex {
                    xy_position: v.xy_position,
                    st_position: v.st_position,
                    color: path.color,
                    bounds: path.clipped_bounds(),
                }));
            }
            let vertex_buf = unsafe { self.instance_belt.alloc_typed(&vertices, &self.gpu) };
            encoder.bind(
                0,
                &ShaderPathRasterizationData {
                    globals,
                    b_path_vertices: vertex_buf,
                },
            );
            encoder.draw(0, vertices.len() as u32, 0, 1);
        }
    }

    pub fn destroy(&mut self) {
        self.wait_for_gpu();
        self.atlas.destroy();
        self.gpu.destroy_sampler(self.atlas_sampler);
        self.instance_belt.destroy(&self.gpu);
        self.gpu.destroy_command_encoder(&mut self.command_encoder);
        self.pipelines.destroy(&self.gpu);
        self.gpu.destroy_surface(&mut self.surface);
        self.destroy_blur_targets();
        self.destroy_blur_cache();
        self.gpu.destroy_texture(self.path_intermediate_texture);
        self.gpu
            .destroy_texture_view(self.path_intermediate_texture_view);
        if let Some(msaa_texture) = self.path_intermediate_msaa_texture {
            self.gpu.destroy_texture(msaa_texture);
        }
        if let Some(msaa_view) = self.path_intermediate_msaa_texture_view {
            self.gpu.destroy_texture_view(msaa_view);
        }
    }

    fn destroy_blur_targets(&mut self) {
        if let Some(targets) = self.blur_targets.take() {
            for (texture, view) in [targets.source, targets.a, targets.b] {
                self.gpu.destroy_texture_view(view);
                self.gpu.destroy_texture(texture);
            }
        }
    }

    fn ensure_blur_targets(&mut self) {
        let size = self.surface_config.size;
        if self.blur_targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        self.destroy_blur_targets();
        let make = |name: &str, format: gpu::TextureFormat, usage: gpu::TextureUsage| {
            let texture = self.gpu.create_texture(gpu::TextureDesc {
                name,
                format,
                size,
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: gpu::TextureDimension::D2,
                usage,
                external: None,
            });
            let view = self.gpu.create_texture_view(
                texture,
                gpu::TextureViewDesc {
                    name,
                    format,
                    dimension: gpu::ViewDimension::D2,
                    subresources: &Default::default(),
                },
            );
            (texture, view)
        };
        let source = make(
            "backdrop source",
            self.surface.info().format,
            gpu::TextureUsage::COPY | gpu::TextureUsage::RESOURCE,
        );
        let float_usage =
            gpu::TextureUsage::RESOURCE | gpu::TextureUsage::TARGET | gpu::TextureUsage::COPY;
        let a = make("backdrop blur a", BLUR_TEXTURE_FORMAT, float_usage);
        let b = make("backdrop blur b", BLUR_TEXTURE_FORMAT, float_usage);
        self.blur_targets = Some(BlurTargets {
            source,
            a,
            b,
            size,
            needs_init: true,
        });
    }

    fn destroy_blur_cache(&mut self) {
        for (_, entry) in self.blur_cache.drain() {
            self.gpu.destroy_texture_view(entry.view);
            self.gpu.destroy_texture(entry.texture);
        }
    }

    /// Release cached blurs not used in the last few frames.
    fn evict_blur_cache(&mut self) {
        let frame = self.frame_index;
        let stale: Vec<u64> = self
            .blur_cache
            .iter()
            .filter(|(_, e)| frame.saturating_sub(e.last_used) > BLUR_CACHE_MAX_AGE)
            .map(|(k, _)| *k)
            .collect();
        for key in stale {
            if let Some(entry) = self.blur_cache.remove(&key) {
                self.gpu.destroy_texture_view(entry.view);
                self.gpu.destroy_texture(entry.texture);
            }
        }
    }

    /// RefineUI patch: copy the frame region behind `blur`, downsample, run a
    /// separable Gaussian and composite it back inside the rounded shape.
    /// Called between render passes (the main pass is closed). With a `key`
    /// the blurred result is also kept for reuse in later frames.
    fn draw_backdrop_blur(
        &mut self,
        blur: &BackdropBlur,
        plan: &BackdropBlurPlan,
        key: Option<u64>,
        frame: &gpu::Frame,
        premultiplied: bool,
    ) {
        let (x0, y0, x1, y1) = (plan.copy[0], plan.copy[1], plan.copy[2], plan.copy[3]);
        self.ensure_blur_targets();
        let Some(targets) = self.blur_targets.as_mut() else {
            return;
        };
        if targets.needs_init {
            targets.needs_init = false;
            self.command_encoder.init_texture(targets.source.0);
            self.command_encoder.init_texture(targets.a.0);
            self.command_encoder.init_texture(targets.b.0);
            // Clear the float textures once: bilinear taps at the edge of a
            // region touch neighbouring texels with weight 0, and garbage
            // there can be NaN, which survives a zero weight.
            for view in [targets.a.1, targets.b.1] {
                let _clear = self.command_encoder.render(
                    "backdrop clear",
                    gpu::RenderTargetSet {
                        colors: &[gpu::RenderTarget {
                            view,
                            init_op: gpu::InitOp::Clear(gpu::TextureColor::TransparentBlack),
                            finish_op: gpu::FinishOp::Store,
                        }],
                        depth_stencil: None,
                    },
                );
            }
        }
        let (source, a, b) = (targets.source, targets.a, targets.b);

        // 1. Copy the region out of the drawable.
        if let mut transfer = self.command_encoder.transfer("backdrop copy") {
            transfer.copy_texture_to_texture(
                gpu::TexturePiece {
                    texture: frame.texture(),
                    mip_level: 0,
                    array_layer: 0,
                    origin: [x0 as u32, y0 as u32, 0],
                },
                gpu::TexturePiece {
                    texture: source.0,
                    mip_level: 0,
                    array_layer: 0,
                    origin: [x0 as u32, y0 as u32, 0],
                },
                gpu::Extent {
                    width: (x1 - x0) as u32,
                    height: (y1 - y0) as u32,
                    depth: 1,
                },
            );
        }

        // 2. Downsample into `a`.
        let down = plan.downsample_params();
        self.blur_pass("backdrop downsample", a.1, source.1, down, BlurStep::Downsample);
        // 3. Gaussian, horizontal into `b`, vertical back into `a`.
        let h = plan.gaussian_params(true);
        self.blur_pass("backdrop blur h", b.1, a.1, h, BlurStep::Gaussian);
        let v = plan.gaussian_params(false);
        self.blur_pass("backdrop blur v", a.1, b.1, v, BlurStep::Gaussian);

        // 4. Keep the result for later frames.
        if let Some(key) = key
            && self.blur_cache_enabled
        {
            self.store_cached_blur(key, plan, a.0);
        }

        // 5. Composite into the drawable.
        let composite = blur.composite_params(plan, BlurSource::Targets, premultiplied);
        if let mut pass = self.command_encoder.render(
            "backdrop composite",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Load,
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            let mut encoder = pass.with(&self.pipelines.blur_composite);
            encoder.bind(
                0,
                &ShaderBlurData {
                    blur_params: BlurParams(composite),
                    t_blur: a.1,
                    s_blur: self.atlas_sampler,
                },
            );
            encoder.draw(0, 4, 0, 1);
        }
    }

    /// Copy the blurred region of `result` into a cache texture for `key`.
    fn store_cached_blur(&mut self, key: u64, plan: &BackdropBlurPlan, result: gpu::Texture) {
        let size = plan.result_size();
        let reusable = self.blur_cache.get(&key).is_some_and(|e| e.size == size);
        if !reusable {
            if self.blur_cache.len() >= BLUR_CACHE_MAX_ENTRIES {
                return;
            }
            if let Some(old) = self.blur_cache.remove(&key) {
                self.gpu.destroy_texture_view(old.view);
                self.gpu.destroy_texture(old.texture);
            }
            let extent = gpu::Extent {
                width: size.0,
                height: size.1,
                depth: 1,
            };
            let texture = self.gpu.create_texture(gpu::TextureDesc {
                name: "backdrop blur cache",
                format: BLUR_TEXTURE_FORMAT,
                size: extent,
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: gpu::TextureDimension::D2,
                usage: gpu::TextureUsage::COPY | gpu::TextureUsage::RESOURCE,
                external: None,
            });
            let view = self.gpu.create_texture_view(
                texture,
                gpu::TextureViewDesc {
                    name: "backdrop blur cache",
                    format: BLUR_TEXTURE_FORMAT,
                    dimension: gpu::ViewDimension::D2,
                    subresources: &Default::default(),
                },
            );
            self.command_encoder.init_texture(texture);
            self.blur_cache.insert(
                key,
                CachedBlur {
                    texture,
                    view,
                    size,
                    last_used: self.frame_index,
                },
            );
        }
        let Some(entry) = self.blur_cache.get_mut(&key) else {
            return;
        };
        entry.last_used = self.frame_index;
        let texture = entry.texture;
        if let mut transfer = self.command_encoder.transfer("backdrop cache store") {
            transfer.copy_texture_to_texture(
                gpu::TexturePiece {
                    texture: result,
                    mip_level: 0,
                    array_layer: 0,
                    origin: [plan.region_ds[0] as u32, plan.region_ds[1] as u32, 0],
                },
                gpu::TexturePiece {
                    texture,
                    mip_level: 0,
                    array_layer: 0,
                    origin: [0, 0, 0],
                },
                gpu::Extent {
                    width: size.0,
                    height: size.1,
                    depth: 1,
                },
            );
        }
    }

    fn blur_pass(
        &mut self,
        label: &str,
        target: gpu::TextureView,
        source: gpu::TextureView,
        params: BackdropBlurParams,
        step: BlurStep,
    ) {
        if let mut pass = self.command_encoder.render(
            label,
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: target,
                    init_op: gpu::InitOp::Load,
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        ) {
            let pipeline = match step {
                BlurStep::Downsample => &self.pipelines.blur_downsample,
                BlurStep::Gaussian => &self.pipelines.blur_gaussian,
            };
            let mut encoder = pass.with(pipeline);
            encoder.bind(
                0,
                &ShaderBlurData {
                    blur_params: BlurParams(params),
                    t_blur: source,
                    s_blur: self.atlas_sampler,
                },
            );
            encoder.draw(0, 4, 0, 1);
        }
    }

    pub fn draw(&mut self, scene: &Scene) {
        self.command_encoder.start();
        self.atlas.before_frame(&mut self.command_encoder);

        let frame = {
            profiling::scope!("acquire frame");
            self.surface.acquire_frame()
        };
        self.command_encoder.init_texture(frame.texture());

        let globals = GlobalParams {
            viewport_size: [
                self.surface_config.size.width as f32,
                self.surface_config.size.height as f32,
            ],
            premultiplied_alpha: match self.surface.info().alpha {
                gpu::AlphaMode::Ignored | gpu::AlphaMode::PostMultiplied => 0,
                gpu::AlphaMode::PreMultiplied => 1,
            },
            pad: 0,
        };

        let mut pass = self.command_encoder.render(
            "main",
            gpu::RenderTargetSet {
                colors: &[gpu::RenderTarget {
                    view: frame.texture_view(),
                    init_op: gpu::InitOp::Clear(gpu::TextureColor::TransparentBlack),
                    finish_op: gpu::FinishOp::Store,
                }],
                depth_stencil: None,
            },
        );

        // RefineUI patch: backdrop blur cache keys for this frame.
        self.frame_index += 1;
        let blur_keys = if self.blur_cache_enabled && !scene.backdrop_blurs.is_empty() {
            scene.backdrop_blur_keys(Size {
                width: ScaledPixels(self.surface_config.size.width as f32),
                height: ScaledPixels(self.surface_config.size.height as f32),
            })
        } else {
            Vec::new()
        };

        profiling::scope!("render pass");
        for batch in scene.batches() {
            match batch {
                PrimitiveBatch::Quads(quads) => {
                    let instance_buf = unsafe { self.instance_belt.alloc_typed(quads, &self.gpu) };
                    let mut encoder = pass.with(&self.pipelines.quads);
                    encoder.bind(
                        0,
                        &ShaderQuadsData {
                            globals,
                            b_quads: instance_buf,
                        },
                    );
                    encoder.draw(0, 4, 0, quads.len() as u32);
                }
                PrimitiveBatch::Shadows(shadows) => {
                    let instance_buf =
                        unsafe { self.instance_belt.alloc_typed(shadows, &self.gpu) };
                    let mut encoder = pass.with(&self.pipelines.shadows);
                    encoder.bind(
                        0,
                        &ShaderShadowsData {
                            globals,
                            b_shadows: instance_buf,
                        },
                    );
                    encoder.draw(0, 4, 0, shadows.len() as u32);
                }
                PrimitiveBatch::Paths(paths) => {
                    let Some(first_path) = paths.first() else {
                        continue;
                    };
                    drop(pass);
                    self.draw_paths_to_intermediate(
                        paths,
                        self.surface_config.size.width as f32,
                        self.surface_config.size.height as f32,
                    );
                    pass = self.command_encoder.render(
                        "main",
                        gpu::RenderTargetSet {
                            colors: &[gpu::RenderTarget {
                                view: frame.texture_view(),
                                init_op: gpu::InitOp::Load,
                                finish_op: gpu::FinishOp::Store,
                            }],
                            depth_stencil: None,
                        },
                    );
                    let mut encoder = pass.with(&self.pipelines.paths);
                    // When copying paths from the intermediate texture to the drawable,
                    // each pixel must only be copied once, in case of transparent paths.
                    //
                    // If all paths have the same draw order, then their bounds are all
                    // disjoint, so we can copy each path's bounds individually. If this
                    // batch combines different draw orders, we perform a single copy
                    // for a minimal spanning rect.
                    let sprites = if paths.last().unwrap().order == first_path.order {
                        paths
                            .iter()
                            .map(|path| PathSprite {
                                bounds: path.clipped_bounds(),
                            })
                            .collect()
                    } else {
                        let mut bounds = first_path.clipped_bounds();
                        for path in paths.iter().skip(1) {
                            bounds = bounds.union(&path.clipped_bounds());
                        }
                        vec![PathSprite { bounds }]
                    };
                    let instance_buf =
                        unsafe { self.instance_belt.alloc_typed(&sprites, &self.gpu) };
                    encoder.bind(
                        0,
                        &ShaderPathsData {
                            globals,
                            t_sprite: self.path_intermediate_texture_view,
                            s_sprite: self.atlas_sampler,
                            b_path_sprites: instance_buf,
                        },
                    );
                    encoder.draw(0, 4, 0, sprites.len() as u32);
                }
                PrimitiveBatch::Underlines(underlines) => {
                    let instance_buf =
                        unsafe { self.instance_belt.alloc_typed(underlines, &self.gpu) };
                    let mut encoder = pass.with(&self.pipelines.underlines);
                    encoder.bind(
                        0,
                        &ShaderUnderlinesData {
                            globals,
                            b_underlines: instance_buf,
                        },
                    );
                    encoder.draw(0, 4, 0, underlines.len() as u32);
                }
                PrimitiveBatch::MonochromeSprites {
                    texture_id,
                    sprites,
                } => {
                    let tex_info = self.atlas.get_texture_info(texture_id);
                    let instance_buf =
                        unsafe { self.instance_belt.alloc_typed(sprites, &self.gpu) };
                    let mut encoder = pass.with(&self.pipelines.mono_sprites);
                    encoder.bind(
                        0,
                        &ShaderMonoSpritesData {
                            globals,
                            gamma_ratios: self.rendering_parameters.gamma_ratios,
                            grayscale_enhanced_contrast: self
                                .rendering_parameters
                                .grayscale_enhanced_contrast,
                            t_sprite: tex_info.raw_view,
                            s_sprite: self.atlas_sampler,
                            b_mono_sprites: instance_buf,
                        },
                    );
                    encoder.draw(0, 4, 0, sprites.len() as u32);
                }
                PrimitiveBatch::PolychromeSprites {
                    texture_id,
                    sprites,
                } => {
                    let tex_info = self.atlas.get_texture_info(texture_id);
                    let instance_buf =
                        unsafe { self.instance_belt.alloc_typed(sprites, &self.gpu) };
                    let mut encoder = pass.with(&self.pipelines.poly_sprites);
                    encoder.bind(
                        0,
                        &ShaderPolySpritesData {
                            globals,
                            t_sprite: tex_info.raw_view,
                            s_sprite: self.atlas_sampler,
                            b_poly_sprites: instance_buf,
                        },
                    );
                    encoder.draw(0, 4, 0, sprites.len() as u32);
                }
                PrimitiveBatch::BackdropBlurs(blurs) => {
                    let premultiplied = globals.premultiplied_alpha != 0;
                    let (width, height) = (
                        self.surface_config.size.width as f32,
                        self.surface_config.size.height as f32,
                    );
                    for blur in blurs {
                        let Some(plan) = blur.plan(width, height) else {
                            continue;
                        };
                        // SAFETY: `blurs` is a sub-slice of `scene.backdrop_blurs`.
                        let index = unsafe {
                            (blur as *const BackdropBlur).offset_from(scene.backdrop_blurs.as_ptr())
                        } as usize;
                        let key = blur_keys.get(index).copied().flatten();
                        let hit = key.and_then(|key| {
                            let entry = self.blur_cache.get_mut(&key)?;
                            (entry.size == plan.result_size()
                                && entry.last_used + 1 >= self.frame_index)
                                .then(|| {
                                    entry.last_used = self.frame_index;
                                    entry.view
                                })
                        });
                        if let Some(view) = hit {
                            // Same backdrop as before: composite only, inside
                            // the main pass.
                            let params = blur.composite_params(&plan, BlurSource::Cached, premultiplied);
                            let mut encoder = pass.with(&self.pipelines.blur_composite);
                            encoder.bind(
                                0,
                                &ShaderBlurData {
                                    blur_params: BlurParams(params),
                                    t_blur: view,
                                    s_blur: self.atlas_sampler,
                                },
                            );
                            encoder.draw(0, 4, 0, 1);
                            continue;
                        }
                        drop(pass);
                        self.draw_backdrop_blur(blur, &plan, key, &frame, premultiplied);
                        pass = self.command_encoder.render(
                            "main",
                            gpu::RenderTargetSet {
                                colors: &[gpu::RenderTarget {
                                    view: frame.texture_view(),
                                    init_op: gpu::InitOp::Load,
                                    finish_op: gpu::FinishOp::Store,
                                }],
                                depth_stencil: None,
                            },
                        );
                    }
                }
                PrimitiveBatch::Surfaces(surfaces) => {
                    let mut _encoder = pass.with(&self.pipelines.surfaces);

                    for surface in surfaces {
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = surface;
                            continue;
                        };

                        #[cfg(target_os = "macos")]
                        {
                            let (t_y, t_cb_cr) = unsafe {
                                use core_foundation::base::TCFType as _;
                                use std::ptr;

                                assert_eq!(
                                        surface.image_buffer.get_pixel_format(),
                                        core_video::pixel_buffer::kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
                                    );

                                let y_texture = self
                                    .core_video_texture_cache
                                    .create_texture_from_image(
                                        surface.image_buffer.as_concrete_TypeRef(),
                                        ptr::null(),
                                        metal::MTLPixelFormat::R8Unorm,
                                        surface.image_buffer.get_width_of_plane(0),
                                        surface.image_buffer.get_height_of_plane(0),
                                        0,
                                    )
                                    .unwrap();
                                let cb_cr_texture = self
                                    .core_video_texture_cache
                                    .create_texture_from_image(
                                        surface.image_buffer.as_concrete_TypeRef(),
                                        ptr::null(),
                                        metal::MTLPixelFormat::RG8Unorm,
                                        surface.image_buffer.get_width_of_plane(1),
                                        surface.image_buffer.get_height_of_plane(1),
                                        1,
                                    )
                                    .unwrap();
                                (
                                    gpu::TextureView::from_metal_texture(
                                        &objc2::rc::Retained::retain(
                                            foreign_types::ForeignTypeRef::as_ptr(
                                                y_texture.as_texture_ref(),
                                            )
                                                as *mut objc2::runtime::ProtocolObject<
                                                    dyn objc2_metal::MTLTexture,
                                                >,
                                        )
                                        .unwrap(),
                                        gpu::TexelAspects::COLOR,
                                    ),
                                    gpu::TextureView::from_metal_texture(
                                        &objc2::rc::Retained::retain(
                                            foreign_types::ForeignTypeRef::as_ptr(
                                                cb_cr_texture.as_texture_ref(),
                                            )
                                                as *mut objc2::runtime::ProtocolObject<
                                                    dyn objc2_metal::MTLTexture,
                                                >,
                                        )
                                        .unwrap(),
                                        gpu::TexelAspects::COLOR,
                                    ),
                                )
                            };

                            _encoder.bind(
                                0,
                                &ShaderSurfacesData {
                                    globals,
                                    surface_locals: SurfaceParams {
                                        bounds: surface.bounds.into(),
                                        content_mask: surface.content_mask.bounds.into(),
                                    },
                                    t_y,
                                    t_cb_cr,
                                    s_surface: self.atlas_sampler,
                                },
                            );

                            _encoder.draw(0, 4, 0, 1);
                        }
                    }
                }
            }
        }
        drop(pass);

        self.command_encoder.present(frame);
        let sync_point = self.gpu.submit(&mut self.command_encoder);

        profiling::scope!("finish");
        self.instance_belt.flush(&sync_point);
        self.atlas.after_frame(&sync_point);

        self.wait_for_gpu();
        self.last_sync_point = Some(sync_point);
        // RefineUI patch: the previous frame is done with its textures.
        self.evict_blur_cache();
    }
}

fn create_path_intermediate_texture(
    gpu: &gpu::Context,
    format: gpu::TextureFormat,
    width: u32,
    height: u32,
) -> (gpu::Texture, gpu::TextureView) {
    let texture = gpu.create_texture(gpu::TextureDesc {
        name: "path intermediate",
        format,
        size: gpu::Extent {
            width,
            height,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: gpu::TextureDimension::D2,
        usage: gpu::TextureUsage::COPY | gpu::TextureUsage::RESOURCE | gpu::TextureUsage::TARGET,
        external: None,
    });
    let texture_view = gpu.create_texture_view(
        texture,
        gpu::TextureViewDesc {
            name: "path intermediate view",
            format,
            dimension: gpu::ViewDimension::D2,
            subresources: &Default::default(),
        },
    );
    (texture, texture_view)
}

fn create_msaa_texture_if_needed(
    gpu: &gpu::Context,
    format: gpu::TextureFormat,
    width: u32,
    height: u32,
    sample_count: u32,
) -> Option<(gpu::Texture, gpu::TextureView)> {
    if sample_count <= 1 {
        return None;
    }
    let texture_msaa = gpu.create_texture(gpu::TextureDesc {
        name: "path intermediate msaa",
        format,
        size: gpu::Extent {
            width,
            height,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count,
        dimension: gpu::TextureDimension::D2,
        usage: gpu::TextureUsage::TARGET,
        external: None,
    });
    let texture_view_msaa = gpu.create_texture_view(
        texture_msaa,
        gpu::TextureViewDesc {
            name: "path intermediate msaa view",
            format,
            dimension: gpu::ViewDimension::D2,
            subresources: &Default::default(),
        },
    );

    Some((texture_msaa, texture_view_msaa))
}

/// A set of parameters that can be set using a corresponding environment variable.
struct RenderingParameters {
    // Env var: ZED_PATH_SAMPLE_COUNT
    // workaround for https://github.com/zed-industries/zed/issues/26143
    path_sample_count: u32,

    // Env var: ZED_FONTS_GAMMA
    // Allowed range [1.0, 2.2], other values are clipped
    // Default: 1.8
    gamma_ratios: [f32; 4],
    // Env var: ZED_FONTS_GRAYSCALE_ENHANCED_CONTRAST
    // Allowed range: [0.0, ..), other values are clipped
    // Default: 1.0
    grayscale_enhanced_contrast: f32,
}

impl RenderingParameters {
    fn from_env(context: &BladeContext) -> Self {
        use std::env;

        let path_sample_count = env::var("ZED_PATH_SAMPLE_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| {
                [4, 2, 1]
                    .into_iter()
                    .find(|&n| (context.gpu.capabilities().sample_count_mask & n) != 0)
            })
            .unwrap_or(1);
        let gamma = env::var("ZED_FONTS_GAMMA")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.8_f32)
            .clamp(1.0, 2.2);
        let gamma_ratios = Self::get_gamma_ratios(gamma);
        let grayscale_enhanced_contrast = env::var("ZED_FONTS_GRAYSCALE_ENHANCED_CONTRAST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0_f32)
            .max(0.0);

        Self {
            path_sample_count,
            gamma_ratios,
            grayscale_enhanced_contrast,
        }
    }

    // Gamma ratios for brightening/darkening edges for better contrast
    // https://github.com/microsoft/terminal/blob/1283c0f5b99a2961673249fa77c6b986efb5086c/src/renderer/atlas/dwrite.cpp#L50
    fn get_gamma_ratios(gamma: f32) -> [f32; 4] {
        const GAMMA_INCORRECT_TARGET_RATIOS: [[f32; 4]; 13] = [
            [0.0000 / 4.0, 0.0000 / 4.0, 0.0000 / 4.0, 0.0000 / 4.0], // gamma = 1.0
            [0.0166 / 4.0, -0.0807 / 4.0, 0.2227 / 4.0, -0.0751 / 4.0], // gamma = 1.1
            [0.0350 / 4.0, -0.1760 / 4.0, 0.4325 / 4.0, -0.1370 / 4.0], // gamma = 1.2
            [0.0543 / 4.0, -0.2821 / 4.0, 0.6302 / 4.0, -0.1876 / 4.0], // gamma = 1.3
            [0.0739 / 4.0, -0.3963 / 4.0, 0.8167 / 4.0, -0.2287 / 4.0], // gamma = 1.4
            [0.0933 / 4.0, -0.5161 / 4.0, 0.9926 / 4.0, -0.2616 / 4.0], // gamma = 1.5
            [0.1121 / 4.0, -0.6395 / 4.0, 1.1588 / 4.0, -0.2877 / 4.0], // gamma = 1.6
            [0.1300 / 4.0, -0.7649 / 4.0, 1.3159 / 4.0, -0.3080 / 4.0], // gamma = 1.7
            [0.1469 / 4.0, -0.8911 / 4.0, 1.4644 / 4.0, -0.3234 / 4.0], // gamma = 1.8
            [0.1627 / 4.0, -1.0170 / 4.0, 1.6051 / 4.0, -0.3347 / 4.0], // gamma = 1.9
            [0.1773 / 4.0, -1.1420 / 4.0, 1.7385 / 4.0, -0.3426 / 4.0], // gamma = 2.0
            [0.1908 / 4.0, -1.2652 / 4.0, 1.8650 / 4.0, -0.3476 / 4.0], // gamma = 2.1
            [0.2031 / 4.0, -1.3864 / 4.0, 1.9851 / 4.0, -0.3501 / 4.0], // gamma = 2.2
        ];

        const NORM13: f32 = ((0x10000 as f64) / (255.0 * 255.0) * 4.0) as f32;
        const NORM24: f32 = ((0x100 as f64) / (255.0) * 4.0) as f32;

        let index = ((gamma * 10.0).round() as usize).clamp(10, 22) - 10;
        let ratios = GAMMA_INCORRECT_TARGET_RATIOS[index];

        [
            ratios[0] * NORM13,
            ratios[1] * NORM24,
            ratios[2] * NORM13,
            ratios[3] * NORM24,
        ]
    }
}
