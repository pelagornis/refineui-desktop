// todo("windows"): remove
#![cfg_attr(windows, allow(dead_code))]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AtlasTextureId, AtlasTile, Background, Bounds, ContentMask, Corners, Edges, Hsla, Pixels,
    Point, Radians, ScaledPixels, Size, bounds_tree::BoundsTree, point,
};
use std::{
    fmt::Debug,
    iter::Peekable,
    ops::{Add, Range, Sub},
    slice,
};

#[allow(non_camel_case_types, unused)]
pub(crate) type PathVertex_ScaledPixels = PathVertex<ScaledPixels>;

pub(crate) type DrawOrder = u32;

#[derive(Default)]
pub(crate) struct Scene {
    pub(crate) paint_operations: Vec<PaintOperation>,
    primitive_bounds: BoundsTree<ScaledPixels>,
    layer_stack: Vec<DrawOrder>,
    /// RefineUI patch: bounds of each open layer (parallel to `layer_stack`).
    layer_bounds: Vec<Bounds<ScaledPixels>>,
    pub(crate) shadows: Vec<Shadow>,
    pub(crate) quads: Vec<Quad>,
    pub(crate) paths: Vec<Path<ScaledPixels>>,
    pub(crate) underlines: Vec<Underline>,
    pub(crate) monochrome_sprites: Vec<MonochromeSprite>,
    pub(crate) polychrome_sprites: Vec<PolychromeSprite>,
    pub(crate) surfaces: Vec<PaintSurface>,
    // RefineUI patch: real backdrop blur.
    pub(crate) backdrop_blurs: Vec<BackdropBlur>,
}

impl Scene {
    pub fn clear(&mut self) {
        self.paint_operations.clear();
        self.primitive_bounds.clear();
        self.layer_stack.clear();
        self.layer_bounds.clear();
        self.paths.clear();
        self.shadows.clear();
        self.quads.clear();
        self.underlines.clear();
        self.monochrome_sprites.clear();
        self.polychrome_sprites.clear();
        self.surfaces.clear();
        self.backdrop_blurs.clear();
    }

    pub fn len(&self) -> usize {
        self.paint_operations.len()
    }

    pub fn push_layer(&mut self, bounds: Bounds<ScaledPixels>) {
        let order = self.primitive_bounds.insert(bounds);
        self.layer_stack.push(order);
        self.layer_bounds.push(bounds);
        self.paint_operations
            .push(PaintOperation::StartLayer(bounds));
    }

    pub fn pop_layer(&mut self) {
        self.layer_stack.pop();
        self.layer_bounds.pop();
        self.paint_operations.push(PaintOperation::EndLayer);
    }

    pub fn insert_primitive(&mut self, primitive: impl Into<Primitive>) {
        let mut primitive = primitive.into();
        let clipped_bounds = match &primitive {
            // A backdrop blur reads pixels around its shape too (the kernel
            // footprint), so it must be ordered after everything painted
            // earlier in that wider region.
            Primitive::BackdropBlur(blur) => blur.paint_bounds(),
            _ => primitive
                .bounds()
                .intersect(&primitive.content_mask().bounds),
        };

        if clipped_bounds.is_empty() {
            return;
        }

        let order = match (&primitive, self.layer_stack.last().copied()) {
            // RefineUI patch: a backdrop blur inside a layer must still come
            // after everything painted earlier under it, even in the same
            // layer. Order it by the tree (including the layer's own bounds)
            // and move the rest of the layer up to that order: later layer
            // content shares it and, sorted after the blur, draws on top.
            (Primitive::BackdropBlur(_), Some(layer_order)) => {
                // Insert the layer's bounds too, so primitives painted after
                // the layer still order above all of its content.
                let bounds = match self.layer_bounds.last() {
                    Some(layer) => clipped_bounds.union(layer),
                    None => clipped_bounds,
                };
                let order = self.primitive_bounds.insert(bounds).max(layer_order + 1);
                if let Some(top) = self.layer_stack.last_mut() {
                    *top = order;
                }
                order
            }
            (_, Some(layer_order)) => layer_order,
            (_, None) => self.primitive_bounds.insert(clipped_bounds),
        };
        match &mut primitive {
            Primitive::Shadow(shadow) => {
                shadow.order = order;
                self.shadows.push(shadow.clone());
            }
            Primitive::Quad(quad) => {
                quad.order = order;
                self.quads.push(quad.clone());
            }
            Primitive::Path(path) => {
                path.order = order;
                path.id = PathId(self.paths.len());
                self.paths.push(path.clone());
            }
            Primitive::Underline(underline) => {
                underline.order = order;
                self.underlines.push(underline.clone());
            }
            Primitive::MonochromeSprite(sprite) => {
                sprite.order = order;
                self.monochrome_sprites.push(sprite.clone());
            }
            Primitive::PolychromeSprite(sprite) => {
                sprite.order = order;
                self.polychrome_sprites.push(sprite.clone());
            }
            Primitive::Surface(surface) => {
                surface.order = order;
                self.surfaces.push(surface.clone());
            }
            Primitive::BackdropBlur(blur) => {
                blur.order = order;
                self.backdrop_blurs.push(blur.clone());
            }
        }
        self.paint_operations
            .push(PaintOperation::Primitive(primitive));
    }

    pub fn replay(&mut self, range: Range<usize>, prev_scene: &Scene) {
        for operation in &prev_scene.paint_operations[range] {
            match operation {
                PaintOperation::Primitive(primitive) => self.insert_primitive(primitive.clone()),
                PaintOperation::StartLayer(bounds) => self.push_layer(*bounds),
                PaintOperation::EndLayer => self.pop_layer(),
            }
        }
    }

    pub fn finish(&mut self) {
        self.shadows.sort_by_key(|shadow| shadow.order);
        self.quads.sort_by_key(|quad| quad.order);
        self.paths.sort_by_key(|path| path.order);
        self.underlines.sort_by_key(|underline| underline.order);
        self.monochrome_sprites
            .sort_by_key(|sprite| (sprite.order, sprite.tile.tile_id));
        self.polychrome_sprites
            .sort_by_key(|sprite| (sprite.order, sprite.tile.tile_id));
        self.surfaces.sort_by_key(|surface| surface.order);
        self.backdrop_blurs.sort_by_key(|blur| blur.order);
    }

    #[cfg_attr(
        all(
            any(target_os = "linux", target_os = "freebsd"),
            not(any(feature = "x11", feature = "wayland"))
        ),
        allow(dead_code)
    )]
    pub(crate) fn batches(&self) -> impl Iterator<Item = PrimitiveBatch<'_>> {
        BatchIterator {
            shadows: &self.shadows,
            shadows_start: 0,
            shadows_iter: self.shadows.iter().peekable(),
            quads: &self.quads,
            quads_start: 0,
            quads_iter: self.quads.iter().peekable(),
            paths: &self.paths,
            paths_start: 0,
            paths_iter: self.paths.iter().peekable(),
            underlines: &self.underlines,
            underlines_start: 0,
            underlines_iter: self.underlines.iter().peekable(),
            monochrome_sprites: &self.monochrome_sprites,
            monochrome_sprites_start: 0,
            monochrome_sprites_iter: self.monochrome_sprites.iter().peekable(),
            polychrome_sprites: &self.polychrome_sprites,
            polychrome_sprites_start: 0,
            polychrome_sprites_iter: self.polychrome_sprites.iter().peekable(),
            surfaces: &self.surfaces,
            surfaces_start: 0,
            surfaces_iter: self.surfaces.iter().peekable(),
            backdrop_blurs: &self.backdrop_blurs,
            backdrop_blurs_start: 0,
            backdrop_blurs_iter: self.backdrop_blurs.iter().peekable(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
#[cfg_attr(
    all(
        any(target_os = "linux", target_os = "freebsd"),
        not(any(feature = "x11", feature = "wayland"))
    ),
    allow(dead_code)
)]
pub(crate) enum PrimitiveKind {
    // First, so that within one draw order the blur runs before the quads
    // and text it sits under.
    BackdropBlur,
    Shadow,
    #[default]
    Quad,
    Path,
    Underline,
    MonochromeSprite,
    PolychromeSprite,
    Surface,
}

pub(crate) enum PaintOperation {
    Primitive(Primitive),
    StartLayer(Bounds<ScaledPixels>),
    EndLayer,
}

#[derive(Clone)]
pub(crate) enum Primitive {
    Shadow(Shadow),
    Quad(Quad),
    Path(Path<ScaledPixels>),
    Underline(Underline),
    MonochromeSprite(MonochromeSprite),
    PolychromeSprite(PolychromeSprite),
    Surface(PaintSurface),
    BackdropBlur(BackdropBlur),
}

impl Primitive {
    pub fn bounds(&self) -> &Bounds<ScaledPixels> {
        match self {
            Primitive::Shadow(shadow) => &shadow.bounds,
            Primitive::Quad(quad) => &quad.bounds,
            Primitive::Path(path) => &path.bounds,
            Primitive::Underline(underline) => &underline.bounds,
            Primitive::MonochromeSprite(sprite) => &sprite.bounds,
            Primitive::PolychromeSprite(sprite) => &sprite.bounds,
            Primitive::Surface(surface) => &surface.bounds,
            Primitive::BackdropBlur(blur) => &blur.bounds,
        }
    }

    pub fn content_mask(&self) -> &ContentMask<ScaledPixels> {
        match self {
            Primitive::Shadow(shadow) => &shadow.content_mask,
            Primitive::Quad(quad) => &quad.content_mask,
            Primitive::Path(path) => &path.content_mask,
            Primitive::Underline(underline) => &underline.content_mask,
            Primitive::MonochromeSprite(sprite) => &sprite.content_mask,
            Primitive::PolychromeSprite(sprite) => &sprite.content_mask,
            Primitive::Surface(surface) => &surface.content_mask,
            Primitive::BackdropBlur(blur) => &blur.content_mask,
        }
    }
}

#[cfg_attr(
    all(
        any(target_os = "linux", target_os = "freebsd"),
        not(any(feature = "x11", feature = "wayland"))
    ),
    allow(dead_code)
)]
struct BatchIterator<'a> {
    shadows: &'a [Shadow],
    shadows_start: usize,
    shadows_iter: Peekable<slice::Iter<'a, Shadow>>,
    quads: &'a [Quad],
    quads_start: usize,
    quads_iter: Peekable<slice::Iter<'a, Quad>>,
    paths: &'a [Path<ScaledPixels>],
    paths_start: usize,
    paths_iter: Peekable<slice::Iter<'a, Path<ScaledPixels>>>,
    underlines: &'a [Underline],
    underlines_start: usize,
    underlines_iter: Peekable<slice::Iter<'a, Underline>>,
    monochrome_sprites: &'a [MonochromeSprite],
    monochrome_sprites_start: usize,
    monochrome_sprites_iter: Peekable<slice::Iter<'a, MonochromeSprite>>,
    polychrome_sprites: &'a [PolychromeSprite],
    polychrome_sprites_start: usize,
    polychrome_sprites_iter: Peekable<slice::Iter<'a, PolychromeSprite>>,
    surfaces: &'a [PaintSurface],
    surfaces_start: usize,
    surfaces_iter: Peekable<slice::Iter<'a, PaintSurface>>,
    backdrop_blurs: &'a [BackdropBlur],
    backdrop_blurs_start: usize,
    backdrop_blurs_iter: Peekable<slice::Iter<'a, BackdropBlur>>,
}

impl<'a> Iterator for BatchIterator<'a> {
    type Item = PrimitiveBatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut orders_and_kinds = [
            (
                self.backdrop_blurs_iter.peek().map(|b| b.order),
                PrimitiveKind::BackdropBlur,
            ),
            (
                self.shadows_iter.peek().map(|s| s.order),
                PrimitiveKind::Shadow,
            ),
            (self.quads_iter.peek().map(|q| q.order), PrimitiveKind::Quad),
            (self.paths_iter.peek().map(|q| q.order), PrimitiveKind::Path),
            (
                self.underlines_iter.peek().map(|u| u.order),
                PrimitiveKind::Underline,
            ),
            (
                self.monochrome_sprites_iter.peek().map(|s| s.order),
                PrimitiveKind::MonochromeSprite,
            ),
            (
                self.polychrome_sprites_iter.peek().map(|s| s.order),
                PrimitiveKind::PolychromeSprite,
            ),
            (
                self.surfaces_iter.peek().map(|s| s.order),
                PrimitiveKind::Surface,
            ),
        ];
        orders_and_kinds.sort_by_key(|(order, kind)| (order.unwrap_or(u32::MAX), *kind));

        let first = orders_and_kinds[0];
        let second = orders_and_kinds[1];
        let (batch_kind, max_order_and_kind) = if first.0.is_some() {
            (first.1, (second.0.unwrap_or(u32::MAX), second.1))
        } else {
            return None;
        };

        match batch_kind {
            PrimitiveKind::BackdropBlur => {
                let start = self.backdrop_blurs_start;
                let mut end = start + 1;
                self.backdrop_blurs_iter.next();
                while self
                    .backdrop_blurs_iter
                    .next_if(|blur| (blur.order, batch_kind) < max_order_and_kind)
                    .is_some()
                {
                    end += 1;
                }
                self.backdrop_blurs_start = end;
                Some(PrimitiveBatch::BackdropBlurs(
                    &self.backdrop_blurs[start..end],
                ))
            }
            PrimitiveKind::Shadow => {
                let shadows_start = self.shadows_start;
                let mut shadows_end = shadows_start + 1;
                self.shadows_iter.next();
                while self
                    .shadows_iter
                    .next_if(|shadow| (shadow.order, batch_kind) < max_order_and_kind)
                    .is_some()
                {
                    shadows_end += 1;
                }
                self.shadows_start = shadows_end;
                Some(PrimitiveBatch::Shadows(
                    &self.shadows[shadows_start..shadows_end],
                ))
            }
            PrimitiveKind::Quad => {
                let quads_start = self.quads_start;
                let mut quads_end = quads_start + 1;
                self.quads_iter.next();
                while self
                    .quads_iter
                    .next_if(|quad| (quad.order, batch_kind) < max_order_and_kind)
                    .is_some()
                {
                    quads_end += 1;
                }
                self.quads_start = quads_end;
                Some(PrimitiveBatch::Quads(&self.quads[quads_start..quads_end]))
            }
            PrimitiveKind::Path => {
                let paths_start = self.paths_start;
                let mut paths_end = paths_start + 1;
                self.paths_iter.next();
                while self
                    .paths_iter
                    .next_if(|path| (path.order, batch_kind) < max_order_and_kind)
                    .is_some()
                {
                    paths_end += 1;
                }
                self.paths_start = paths_end;
                Some(PrimitiveBatch::Paths(&self.paths[paths_start..paths_end]))
            }
            PrimitiveKind::Underline => {
                let underlines_start = self.underlines_start;
                let mut underlines_end = underlines_start + 1;
                self.underlines_iter.next();
                while self
                    .underlines_iter
                    .next_if(|underline| (underline.order, batch_kind) < max_order_and_kind)
                    .is_some()
                {
                    underlines_end += 1;
                }
                self.underlines_start = underlines_end;
                Some(PrimitiveBatch::Underlines(
                    &self.underlines[underlines_start..underlines_end],
                ))
            }
            PrimitiveKind::MonochromeSprite => {
                let texture_id = self.monochrome_sprites_iter.peek().unwrap().tile.texture_id;
                let sprites_start = self.monochrome_sprites_start;
                let mut sprites_end = sprites_start + 1;
                self.monochrome_sprites_iter.next();
                while self
                    .monochrome_sprites_iter
                    .next_if(|sprite| {
                        (sprite.order, batch_kind) < max_order_and_kind
                            && sprite.tile.texture_id == texture_id
                    })
                    .is_some()
                {
                    sprites_end += 1;
                }
                self.monochrome_sprites_start = sprites_end;
                Some(PrimitiveBatch::MonochromeSprites {
                    texture_id,
                    sprites: &self.monochrome_sprites[sprites_start..sprites_end],
                })
            }
            PrimitiveKind::PolychromeSprite => {
                let texture_id = self.polychrome_sprites_iter.peek().unwrap().tile.texture_id;
                let sprites_start = self.polychrome_sprites_start;
                let mut sprites_end = self.polychrome_sprites_start + 1;
                self.polychrome_sprites_iter.next();
                while self
                    .polychrome_sprites_iter
                    .next_if(|sprite| {
                        (sprite.order, batch_kind) < max_order_and_kind
                            && sprite.tile.texture_id == texture_id
                    })
                    .is_some()
                {
                    sprites_end += 1;
                }
                self.polychrome_sprites_start = sprites_end;
                Some(PrimitiveBatch::PolychromeSprites {
                    texture_id,
                    sprites: &self.polychrome_sprites[sprites_start..sprites_end],
                })
            }
            PrimitiveKind::Surface => {
                let surfaces_start = self.surfaces_start;
                let mut surfaces_end = surfaces_start + 1;
                self.surfaces_iter.next();
                while self
                    .surfaces_iter
                    .next_if(|surface| (surface.order, batch_kind) < max_order_and_kind)
                    .is_some()
                {
                    surfaces_end += 1;
                }
                self.surfaces_start = surfaces_end;
                Some(PrimitiveBatch::Surfaces(
                    &self.surfaces[surfaces_start..surfaces_end],
                ))
            }
        }
    }
}

#[derive(Debug)]
#[cfg_attr(
    all(
        any(target_os = "linux", target_os = "freebsd"),
        not(any(feature = "x11", feature = "wayland"))
    ),
    allow(dead_code)
)]
pub(crate) enum PrimitiveBatch<'a> {
    Shadows(&'a [Shadow]),
    Quads(&'a [Quad]),
    Paths(&'a [Path<ScaledPixels>]),
    Underlines(&'a [Underline]),
    MonochromeSprites {
        texture_id: AtlasTextureId,
        sprites: &'a [MonochromeSprite],
    },
    PolychromeSprites {
        texture_id: AtlasTextureId,
        sprites: &'a [PolychromeSprite],
    },
    Surfaces(&'a [PaintSurface]),
    BackdropBlurs(&'a [BackdropBlur]),
}

#[derive(Default, Debug, Clone)]
#[repr(C)]
pub(crate) struct Quad {
    pub order: DrawOrder,
    pub border_style: BorderStyle,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub background: Background,
    pub border_color: Hsla,
    pub corner_radii: Corners<ScaledPixels>,
    pub border_widths: Edges<ScaledPixels>,
}

impl From<Quad> for Primitive {
    fn from(quad: Quad) -> Self {
        Primitive::Quad(quad)
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub(crate) struct Underline {
    pub order: DrawOrder,
    pub pad: u32, // align to 8 bytes
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: Hsla,
    pub thickness: ScaledPixels,
    pub wavy: u32,
}

impl From<Underline> for Primitive {
    fn from(underline: Underline) -> Self {
        Primitive::Underline(underline)
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub(crate) struct Shadow {
    pub order: DrawOrder,
    pub blur_radius: ScaledPixels,
    pub bounds: Bounds<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: Hsla,
}

impl From<Shadow> for Primitive {
    fn from(shadow: Shadow) -> Self {
        Primitive::Shadow(shadow)
    }
}

/// The style of a border.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[repr(C)]
pub enum BorderStyle {
    /// A solid border.
    #[default]
    Solid = 0,
    /// A dashed border.
    Dashed = 1,
}

/// A data type representing a 2 dimensional transformation that can be applied to an element.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct TransformationMatrix {
    /// 2x2 matrix containing rotation and scale,
    /// stored row-major
    pub rotation_scale: [[f32; 2]; 2],
    /// translation vector
    pub translation: [f32; 2],
}

impl Eq for TransformationMatrix {}

impl TransformationMatrix {
    /// The unit matrix, has no effect.
    pub fn unit() -> Self {
        Self {
            rotation_scale: [[1.0, 0.0], [0.0, 1.0]],
            translation: [0.0, 0.0],
        }
    }

    /// Move the origin by a given point
    pub fn translate(mut self, point: Point<ScaledPixels>) -> Self {
        self.compose(Self {
            rotation_scale: [[1.0, 0.0], [0.0, 1.0]],
            translation: [point.x.0, point.y.0],
        })
    }

    /// Clockwise rotation in radians around the origin
    pub fn rotate(self, angle: Radians) -> Self {
        self.compose(Self {
            rotation_scale: [
                [angle.0.cos(), -angle.0.sin()],
                [angle.0.sin(), angle.0.cos()],
            ],
            translation: [0.0, 0.0],
        })
    }

    /// Scale around the origin
    pub fn scale(self, size: Size<f32>) -> Self {
        self.compose(Self {
            rotation_scale: [[size.width, 0.0], [0.0, size.height]],
            translation: [0.0, 0.0],
        })
    }

    /// Perform matrix multiplication with another transformation
    /// to produce a new transformation that is the result of
    /// applying both transformations: first, `other`, then `self`.
    #[inline]
    pub fn compose(self, other: TransformationMatrix) -> TransformationMatrix {
        if other == Self::unit() {
            return self;
        }
        // Perform matrix multiplication
        TransformationMatrix {
            rotation_scale: [
                [
                    self.rotation_scale[0][0] * other.rotation_scale[0][0]
                        + self.rotation_scale[0][1] * other.rotation_scale[1][0],
                    self.rotation_scale[0][0] * other.rotation_scale[0][1]
                        + self.rotation_scale[0][1] * other.rotation_scale[1][1],
                ],
                [
                    self.rotation_scale[1][0] * other.rotation_scale[0][0]
                        + self.rotation_scale[1][1] * other.rotation_scale[1][0],
                    self.rotation_scale[1][0] * other.rotation_scale[0][1]
                        + self.rotation_scale[1][1] * other.rotation_scale[1][1],
                ],
            ],
            translation: [
                self.translation[0]
                    + self.rotation_scale[0][0] * other.translation[0]
                    + self.rotation_scale[0][1] * other.translation[1],
                self.translation[1]
                    + self.rotation_scale[1][0] * other.translation[0]
                    + self.rotation_scale[1][1] * other.translation[1],
            ],
        }
    }

    /// Apply transformation to a point, mainly useful for debugging
    pub fn apply(&self, point: Point<Pixels>) -> Point<Pixels> {
        let input = [point.x.0, point.y.0];
        let mut output = self.translation;
        for (i, output_cell) in output.iter_mut().enumerate() {
            for (k, input_cell) in input.iter().enumerate() {
                *output_cell += self.rotation_scale[i][k] * *input_cell;
            }
        }
        Point::new(output[0].into(), output[1].into())
    }
}

impl Default for TransformationMatrix {
    fn default() -> Self {
        Self::unit()
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct MonochromeSprite {
    pub order: DrawOrder,
    pub pad: u32, // align to 8 bytes
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub color: Hsla,
    pub tile: AtlasTile,
    pub transformation: TransformationMatrix,
}

impl From<MonochromeSprite> for Primitive {
    fn from(sprite: MonochromeSprite) -> Self {
        Primitive::MonochromeSprite(sprite)
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct PolychromeSprite {
    pub order: DrawOrder,
    pub pad: u32, // align to 8 bytes
    pub grayscale: bool,
    pub opacity: f32,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    pub tile: AtlasTile,
}

impl From<PolychromeSprite> for Primitive {
    fn from(sprite: PolychromeSprite) -> Self {
        Primitive::PolychromeSprite(sprite)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PaintSurface {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    #[cfg(target_os = "macos")]
    pub image_buffer: core_video::pixel_buffer::CVPixelBuffer,
}

impl From<PaintSurface> for Primitive {
    fn from(surface: PaintSurface) -> Self {
        Primitive::Surface(surface)
    }
}

/// RefineUI patch: blur whatever was painted behind `bounds`, then tint it.
///
/// The renderer copies the sampled region of the frame, blurs it (downsample
/// + separable Gaussian) and composites the result inside the rounded shape.
#[derive(Clone, Debug)]
pub(crate) struct BackdropBlur {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub corner_radii: Corners<ScaledPixels>,
    /// Gaussian standard deviation in device pixels (CSS `blur()` semantics).
    pub sigma: ScaledPixels,
    /// Color laid over the blurred backdrop (its alpha is the mix amount).
    pub tint: Hsla,
    /// 1 = unchanged, 0 = grayscale, > 1 = more vivid.
    pub saturation: f32,
    /// Overall opacity of the blurred plate (element opacity).
    pub opacity: f32,
    /// How the tint combines with the backdrop (`BackdropBlend as u32`).
    pub tint_blend: u32,
    /// Monochrome grain amplitude (0 = none). Dithering is always on.
    pub noise: f32,
    /// Lighting, evaluated on the GPU inside the shape.
    pub light: BackdropBlurLight,
    /// Linear alpha mask across the shape: (dir.x, dir.y, start, end);
    /// a zero direction disables it.
    pub mask: [f32; 4],
    /// Drop shadows drawn by the composite pass *outside* the shape only
    /// (CSS `box-shadow` clipping), so they never end up in the backdrop.
    pub shadows: [BackdropBlurShadow; 4],
}

/// RefineUI patch: [`BackdropBlur`] lighting in device pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BackdropBlurLight {
    pub direction: [f32; 2],
    pub color: Hsla,
    pub directional: f32,
    pub ambient: f32,
    pub edge: f32,
    pub edge_width: ScaledPixels,
    pub specular: f32,
    /// Shape-relative (0..1) position of the specular highlight.
    pub specular_position: [f32; 2],
    pub specular_size: ScaledPixels,
    pub reflection: f32,
}

/// RefineUI patch: a [`BackdropBlur`] drop shadow in device pixels; a
/// transparent color means unused.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BackdropBlurShadow {
    pub offset: Point<ScaledPixels>,
    pub blur_radius: ScaledPixels,
    pub spread_radius: ScaledPixels,
    pub color: Hsla,
}

impl BackdropBlurShadow {
    pub(crate) fn is_visible(&self) -> bool {
        self.color.a > 0.0
    }

    /// The area the shadow can touch, around `shape`.
    pub(crate) fn extent(&self, shape: &Bounds<ScaledPixels>) -> Bounds<ScaledPixels> {
        let grow = self.spread_radius.0.max(0.0) + self.blur_radius.0.max(0.0) * 3.0 + 1.0;
        Bounds {
            origin: point(
                shape.origin.x + self.offset.x - ScaledPixels(grow),
                shape.origin.y + self.offset.y - ScaledPixels(grow),
            ),
            size: Size {
                width: shape.size.width + ScaledPixels(grow * 2.0),
                height: shape.size.height + ScaledPixels(grow * 2.0),
            },
        }
    }
}

impl BackdropBlur {
    /// The region the composite writes: the shape and its shadows, clipped
    /// to the content mask.
    pub(crate) fn draw_bounds(&self) -> Bounds<ScaledPixels> {
        let mut bounds = self.bounds;
        for shadow in self.shadows.iter().filter(|s| s.is_visible()) {
            bounds = bounds.union(&shadow.extent(&self.bounds));
        }
        bounds.intersect(&self.content_mask.bounds)
    }

    /// Everything the blur reads or writes, used for draw ordering.
    pub(crate) fn paint_bounds(&self) -> Bounds<ScaledPixels> {
        let sample = self.sample_bounds();
        let draw = self.draw_bounds();
        if sample.is_empty() {
            return sample;
        }
        sample.union(&draw)
    }

    /// The region the blur reads: the visible shape grown by the kernel
    /// footprint (3σ).
    pub(crate) fn sample_bounds(&self) -> Bounds<ScaledPixels> {
        let visible = self.bounds.intersect(&self.content_mask.bounds);
        if visible.is_empty() {
            return visible;
        }
        let grow = ScaledPixels((self.sigma.0 * 3.0).ceil() + 8.0);
        Bounds {
            origin: point(visible.origin.x - grow, visible.origin.y - grow),
            size: Size {
                width: visible.size.width + grow + grow,
                height: visible.size.height + grow + grow,
            },
        }
    }
}

impl From<BackdropBlur> for Primitive {
    fn from(blur: BackdropBlur) -> Self {
        Primitive::BackdropBlur(blur)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PathId(pub(crate) usize);

/// A line made up of a series of vertices and control points.
#[derive(Clone, Debug)]
pub struct Path<P: Clone + Debug + Default + PartialEq> {
    pub(crate) id: PathId,
    pub(crate) order: DrawOrder,
    pub(crate) bounds: Bounds<P>,
    pub(crate) content_mask: ContentMask<P>,
    pub(crate) vertices: Vec<PathVertex<P>>,
    pub(crate) color: Background,
    start: Point<P>,
    current: Point<P>,
    contour_count: usize,
}

impl Path<Pixels> {
    /// Create a new path with the given starting point.
    pub fn new(start: Point<Pixels>) -> Self {
        Self {
            id: PathId(0),
            order: DrawOrder::default(),
            vertices: Vec::new(),
            start,
            current: start,
            bounds: Bounds {
                origin: start,
                size: Default::default(),
            },
            content_mask: Default::default(),
            color: Default::default(),
            contour_count: 0,
        }
    }

    /// Scale this path by the given factor.
    pub fn scale(&self, factor: f32) -> Path<ScaledPixels> {
        Path {
            id: self.id,
            order: self.order,
            bounds: self.bounds.scale(factor),
            content_mask: self.content_mask.scale(factor),
            vertices: self
                .vertices
                .iter()
                .map(|vertex| vertex.scale(factor))
                .collect(),
            start: self.start.map(|start| start.scale(factor)),
            current: self.current.scale(factor),
            contour_count: self.contour_count,
            color: self.color,
        }
    }

    /// Move the start, current point to the given point.
    pub fn move_to(&mut self, to: Point<Pixels>) {
        self.contour_count += 1;
        self.start = to;
        self.current = to;
    }

    /// Draw a straight line from the current point to the given point.
    pub fn line_to(&mut self, to: Point<Pixels>) {
        self.contour_count += 1;
        if self.contour_count > 1 {
            self.push_triangle(
                (self.start, self.current, to),
                (point(0., 1.), point(0., 1.), point(0., 1.)),
            );
        }
        self.current = to;
    }

    /// Draw a curve from the current point to the given point, using the given control point.
    pub fn curve_to(&mut self, to: Point<Pixels>, ctrl: Point<Pixels>) {
        self.contour_count += 1;
        if self.contour_count > 1 {
            self.push_triangle(
                (self.start, self.current, to),
                (point(0., 1.), point(0., 1.), point(0., 1.)),
            );
        }

        self.push_triangle(
            (self.current, ctrl, to),
            (point(0., 0.), point(0.5, 0.), point(1., 1.)),
        );
        self.current = to;
    }

    /// Push a triangle to the Path.
    pub fn push_triangle(
        &mut self,
        xy: (Point<Pixels>, Point<Pixels>, Point<Pixels>),
        st: (Point<f32>, Point<f32>, Point<f32>),
    ) {
        self.bounds = self
            .bounds
            .union(&Bounds {
                origin: xy.0,
                size: Default::default(),
            })
            .union(&Bounds {
                origin: xy.1,
                size: Default::default(),
            })
            .union(&Bounds {
                origin: xy.2,
                size: Default::default(),
            });

        self.vertices.push(PathVertex {
            xy_position: xy.0,
            st_position: st.0,
            content_mask: Default::default(),
        });
        self.vertices.push(PathVertex {
            xy_position: xy.1,
            st_position: st.1,
            content_mask: Default::default(),
        });
        self.vertices.push(PathVertex {
            xy_position: xy.2,
            st_position: st.2,
            content_mask: Default::default(),
        });
    }
}

impl<T> Path<T>
where
    T: Clone + Debug + Default + PartialEq + PartialOrd + Add<T, Output = T> + Sub<Output = T>,
{
    #[allow(unused)]
    pub(crate) fn clipped_bounds(&self) -> Bounds<T> {
        self.bounds.intersect(&self.content_mask.bounds)
    }
}

impl From<Path<ScaledPixels>> for Primitive {
    fn from(path: Path<ScaledPixels>) -> Self {
        Primitive::Path(path)
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct PathVertex<P: Clone + Debug + Default + PartialEq> {
    pub(crate) xy_position: Point<P>,
    pub(crate) st_position: Point<f32>,
    pub(crate) content_mask: ContentMask<P>,
}

impl PathVertex<Pixels> {
    pub fn scale(&self, factor: f32) -> PathVertex<ScaledPixels> {
        PathVertex {
            xy_position: self.xy_position.scale(factor),
            st_position: self.st_position,
            content_mask: self.content_mask.scale(factor),
        }
    }
}

// RefineUI patch: backdrop blur cache keys.
//
// A blurred backdrop only depends on what was drawn before the blur inside the
// region it samples. Hashing those primitives gives a key; when a blur has the
// same key as in the previous frame, the renderer reuses its blurred texture
// and only runs the cheap composite (no copy, no blur passes, no pass break).
mod backdrop_keys {
    use super::*;
    use std::hash::{DefaultHasher, Hash, Hasher};

    fn f(h: &mut DefaultHasher, x: f32) {
        h.write_u32(x.to_bits());
    }
    fn sp(h: &mut DefaultHasher, x: ScaledPixels) {
        f(h, x.0);
    }
    fn bounds(h: &mut DefaultHasher, b: &Bounds<ScaledPixels>) {
        sp(h, b.origin.x);
        sp(h, b.origin.y);
        sp(h, b.size.width);
        sp(h, b.size.height);
    }
    fn hsla(h: &mut DefaultHasher, c: &Hsla) {
        f(h, c.h);
        f(h, c.s);
        f(h, c.l);
        f(h, c.a);
    }
    fn corners(h: &mut DefaultHasher, c: &Corners<ScaledPixels>) {
        sp(h, c.top_left);
        sp(h, c.top_right);
        sp(h, c.bottom_right);
        sp(h, c.bottom_left);
    }
    fn background(h: &mut DefaultHasher, b: &Background) {
        h.write_u32(b.tag as u32);
        h.write_u32(b.color_space as u32);
        hsla(h, &b.solid);
        f(h, b.gradient_angle_or_pattern_height);
        for stop in &b.colors {
            hsla(h, &stop.color);
            f(h, stop.percentage);
        }
    }
    fn tile(h: &mut DefaultHasher, t: &AtlasTile) {
        t.texture_id.hash(h);
        h.write_u32(t.tile_id.0);
        h.write_i32(t.bounds.origin.x.0);
        h.write_i32(t.bounds.origin.y.0);
        h.write_i32(t.bounds.size.width.0);
        h.write_i32(t.bounds.size.height.0);
    }

    fn touches(
        b: &Bounds<ScaledPixels>,
        mask: &ContentMask<ScaledPixels>,
        region: &Bounds<ScaledPixels>,
    ) -> bool {
        !b.intersect(&mask.bounds).intersect(region).is_empty()
    }

    /// Hash of the blur's own parameters that affect its composite output.
    pub(super) fn hash_blur_output(h: &mut DefaultHasher, b: &BackdropBlur) {
        bounds(h, &b.bounds);
        bounds(h, &b.content_mask.bounds);
        corners(h, &b.corner_radii);
        sp(h, b.sigma);
        hsla(h, &b.tint);
        f(h, b.saturation);
        f(h, b.opacity);
        h.write_u32(b.tint_blend);
        f(h, b.noise);
        let l = &b.light;
        f(h, l.direction[0]);
        f(h, l.direction[1]);
        hsla(h, &l.color);
        for x in [l.directional, l.ambient, l.edge, l.edge_width.0, l.specular] {
            f(h, x);
        }
        for x in [
            l.specular_position[0],
            l.specular_position[1],
            l.specular_size.0,
            l.reflection,
        ] {
            f(h, x);
        }
        for x in b.mask {
            f(h, x);
        }
        for s in &b.shadows {
            sp(h, s.offset.x);
            sp(h, s.offset.y);
            sp(h, s.blur_radius);
            sp(h, s.spread_radius);
            hsla(h, &s.color);
        }
    }

    impl Scene {
        /// One key per entry of `backdrop_blurs` (after `finish`); `None`
        /// means the backdrop must be re-blurred (e.g. a video surface is
        /// underneath). `viewport` is the drawable size.
        pub(crate) fn backdrop_blur_keys(&self, viewport: Size<ScaledPixels>) -> Vec<Option<u64>> {
            let mut keys: Vec<Option<u64>> = Vec::with_capacity(self.backdrop_blurs.len());
            for (index, blur) in self.backdrop_blurs.iter().enumerate() {
                let region = blur.sample_bounds();
                let order = blur.order;
                let mut h = DefaultHasher::new();
                f(&mut h, viewport.width.0);
                f(&mut h, viewport.height.0);
                bounds(&mut h, &region);
                sp(&mut h, blur.sigma);

                let mut cacheable = true;
                for s in self.surfaces.iter().take_while(|s| s.order < order) {
                    if touches(&s.bounds, &s.content_mask, &region) {
                        cacheable = false;
                    }
                }
                for s in self.shadows.iter().take_while(|s| s.order < order) {
                    if touches(&s.bounds, &s.content_mask, &region) {
                        h.write_u8(1);
                        bounds(&mut h, &s.bounds);
                        bounds(&mut h, &s.content_mask.bounds);
                        corners(&mut h, &s.corner_radii);
                        sp(&mut h, s.blur_radius);
                        hsla(&mut h, &s.color);
                    }
                }
                for q in self.quads.iter().take_while(|q| q.order < order) {
                    if touches(&q.bounds, &q.content_mask, &region) {
                        h.write_u8(2);
                        h.write_u32(q.border_style as u32);
                        bounds(&mut h, &q.bounds);
                        bounds(&mut h, &q.content_mask.bounds);
                        background(&mut h, &q.background);
                        hsla(&mut h, &q.border_color);
                        corners(&mut h, &q.corner_radii);
                        let w = &q.border_widths;
                        for x in [w.top, w.right, w.bottom, w.left] {
                            sp(&mut h, x);
                        }
                    }
                }
                for p in self.paths.iter().take_while(|p| p.order < order) {
                    if touches(&p.bounds, &p.content_mask, &region) {
                        h.write_u8(3);
                        bounds(&mut h, &p.bounds);
                        bounds(&mut h, &p.content_mask.bounds);
                        background(&mut h, &p.color);
                        h.write_usize(p.vertices.len());
                        for v in &p.vertices {
                            sp(&mut h, v.xy_position.x);
                            sp(&mut h, v.xy_position.y);
                            f(&mut h, v.st_position.x);
                            f(&mut h, v.st_position.y);
                        }
                    }
                }
                for u in self.underlines.iter().take_while(|u| u.order < order) {
                    if touches(&u.bounds, &u.content_mask, &region) {
                        h.write_u8(4);
                        bounds(&mut h, &u.bounds);
                        bounds(&mut h, &u.content_mask.bounds);
                        hsla(&mut h, &u.color);
                        sp(&mut h, u.thickness);
                        h.write_u32(u.wavy);
                    }
                }
                for s in self.monochrome_sprites.iter().take_while(|s| s.order < order) {
                    if touches(&s.bounds, &s.content_mask, &region) {
                        h.write_u8(5);
                        bounds(&mut h, &s.bounds);
                        bounds(&mut h, &s.content_mask.bounds);
                        hsla(&mut h, &s.color);
                        tile(&mut h, &s.tile);
                        let t = &s.transformation;
                        for x in [
                            t.rotation_scale[0][0],
                            t.rotation_scale[0][1],
                            t.rotation_scale[1][0],
                            t.rotation_scale[1][1],
                            t.translation[0],
                            t.translation[1],
                        ] {
                            f(&mut h, x);
                        }
                    }
                }
                for s in self.polychrome_sprites.iter().take_while(|s| s.order < order) {
                    if touches(&s.bounds, &s.content_mask, &region) {
                        h.write_u8(6);
                        h.write_u8(s.grayscale as u8);
                        f(&mut h, s.opacity);
                        bounds(&mut h, &s.bounds);
                        bounds(&mut h, &s.content_mask.bounds);
                        corners(&mut h, &s.corner_radii);
                        tile(&mut h, &s.tile);
                    }
                }
                // Earlier blurs drawn into the region: their output depends on
                // their own key and composite parameters.
                for (earlier, key) in self.backdrop_blurs[..index].iter().zip(&keys) {
                    if earlier.order < order && !earlier.draw_bounds().intersect(&region).is_empty() {
                        match key {
                            Some(key) => {
                                h.write_u8(7);
                                h.write_u64(*key);
                                hash_blur_output(&mut h, earlier);
                            }
                            None => cacheable = false,
                        }
                    }
                }
                keys.push(cacheable.then(|| h.finish()));
            }
            keys
        }
    }
}

// RefineUI patch: ordering + cache-key tests.
#[cfg(test)]
mod backdrop_tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<ScaledPixels> {
        Bounds {
            origin: point(ScaledPixels(x), ScaledPixels(y)),
            size: Size {
                width: ScaledPixels(w),
                height: ScaledPixels(h),
            },
        }
    }

    fn quad(b: Bounds<ScaledPixels>, color: Hsla) -> Quad {
        Quad {
            bounds: b,
            content_mask: ContentMask {
                bounds: rect(0., 0., 4000., 4000.),
            },
            background: color.into(),
            ..Default::default()
        }
    }

    fn blur(b: Bounds<ScaledPixels>) -> BackdropBlur {
        BackdropBlur {
            order: 0,
            bounds: b,
            content_mask: ContentMask {
                bounds: rect(0., 0., 4000., 4000.),
            },
            corner_radii: Corners::default(),
            sigma: ScaledPixels(8.0),
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

    fn red() -> Hsla {
        Hsla {
            h: 0.,
            s: 1.,
            l: 0.5,
            a: 1.,
        }
    }

    fn scene(under: Hsla, elsewhere: Hsla) -> Scene {
        let mut s = Scene::default();
        s.insert_primitive(quad(rect(0., 0., 300., 300.), under));
        s.insert_primitive(quad(rect(900., 900., 50., 50.), elsewhere));
        s.insert_primitive(blur(rect(50., 50., 100., 100.)));
        s.finish();
        s
    }

    #[test]
    fn blur_in_a_layer_draws_below_later_layer_content() {
        let mut s = Scene::default();
        s.insert_primitive(quad(rect(0., 0., 300., 300.), red()));
        s.push_layer(rect(40., 40., 200., 200.));
        s.insert_primitive(quad(rect(40., 40., 200., 200.), red()));
        s.insert_primitive(blur(rect(50., 50., 100., 100.)));
        // e.g. a label painted on the material, same layer
        s.insert_primitive(quad(rect(60., 60., 20., 10.), Hsla::white()));
        s.pop_layer();
        s.insert_primitive(quad(rect(100., 100., 20., 10.), Hsla::white()));
        s.finish();
        let blur_order = s.backdrop_blurs[0].order;
        let orders: Vec<_> = s.quads.iter().map(|q| q.order).collect();
        // background + layer fill before the blur, label and later content at/after it
        assert!(orders[0] < blur_order && orders[1] < blur_order);
        assert!(orders[2] >= blur_order && orders[3] >= blur_order);
    }

    #[test]
    fn cache_key_tracks_only_the_sampled_region() {
        let view = Size {
            width: ScaledPixels(1000.),
            height: ScaledPixels(1000.),
        };
        let a = scene(red(), red()).backdrop_blur_keys(view);
        let same = scene(red(), red()).backdrop_blur_keys(view);
        let outside = scene(red(), Hsla::white()).backdrop_blur_keys(view);
        let under = scene(Hsla::white(), red()).backdrop_blur_keys(view);
        assert!(a[0].is_some());
        assert_eq!(a, same);
        assert_eq!(a, outside);
        assert_ne!(a, under);
    }
}
