//! Floating surface chrome shared by Popover and Tooltip — web
//! `shared/FloatingSurface.tsx` + `shared/floatingOutline.ts`: the panel and
//! its beak (arrow) are **one closed outline** (rounded rect + a triangular
//! beak with a quadratic-rounded tip), filled with the frosted material and
//! stroked with a single hairline, so the beak is seamless with the panel.

use gpui::{Bounds, Hsla, IntoElement, PathBuilder, Pixels, Point, Styled, canvas, point, px};

/// Which panel edge carries the beak (opposite of the placement side).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeakSide {
    /// Beak on the top edge, pointing up (panel below its trigger).
    Top,
    /// Beak on the bottom edge, pointing down (panel above its trigger).
    Bottom,
    /// Beak on the left edge, pointing left (panel right of its trigger).
    Left,
    /// Beak on the right edge, pointing right (panel left of its trigger).
    Right,
}

impl BeakSide {
    fn vertical(self) -> bool {
        matches!(self, BeakSide::Top | BeakSide::Bottom)
    }
}

/// Where the beak sits along its edge — web `FloatingBeakAlign`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BeakAlign {
    /// `insetStart + width/2` from the start corner.
    Start,
    /// Centered on the edge.
    #[default]
    Center,
    /// `insetEnd + width/2` from the end corner.
    End,
}

/// Beak geometry (all px) — web `componentSizes.*Beak*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beak {
    /// Edge carrying the beak.
    pub side: BeakSide,
    /// Placement along the edge.
    pub align: BeakAlign,
    /// Base width.
    pub width: f32,
    /// Protrusion.
    pub height: f32,
    /// Tip rounding (quadratic, measured along each slanted edge).
    pub tip_radius: f32,
    /// Start inset (for [`BeakAlign::Start`]).
    pub inset_start: f32,
    /// End inset (for [`BeakAlign::End`]).
    pub inset_end: f32,
    /// Explicit center along the edge (from the panel's start), e.g. the
    /// trigger's midpoint. Clamped off the corner arcs like the aligned values.
    pub center: Option<f32>,
}

impl Beak {
    /// Tooltip beak: 16×8, tip radius 2, insets `sizeMedium`.
    pub fn tooltip(side: BeakSide, align: BeakAlign) -> Self {
        use refineui_foundation::size::component::*;
        Self {
            side,
            align,
            width: TOOLTIP_BEAK_WIDTH,
            height: TOOLTIP_BEAK_HEIGHT,
            tip_radius: TOOLTIP_BEAK_TIP_RADIUS,
            inset_start: refineui_foundation::spacing::MEDIUM,
            inset_end: refineui_foundation::spacing::MEDIUM,
            center: None,
        }
    }

    /// Popover beak: 24×12, tip radius 4, insets 34 / 30.
    pub fn popover(side: BeakSide, align: BeakAlign) -> Self {
        use refineui_foundation::size::component::*;
        Self {
            side,
            align,
            width: POPOVER_BEAK_WIDTH,
            height: POPOVER_BEAK_HEIGHT,
            tip_radius: POPOVER_BEAK_TIP_RADIUS,
            inset_start: POPOVER_BEAK_INSET_START,
            inset_end: POPOVER_BEAK_INSET_END,
            center: None,
        }
    }

    /// Point the beak at `center` (distance along the edge from the start).
    pub fn with_center(mut self, center: f32) -> Self {
        self.center = Some(center);
        self
    }
}

/// Web `resolveBeakCenter`: clamp the beak off the corner arcs.
pub fn resolve_beak_center(edge: f32, beak: &Beak, radius: f32) -> f32 {
    let half = beak.width / 2.;
    let min = radius + half;
    let max = edge - radius - half;
    if max < min {
        return edge / 2.;
    }
    let wanted = if let Some(c) = beak.center {
        c
    } else {
        match beak.align {
            BeakAlign::Center => edge / 2.,
            BeakAlign::Start => beak.inset_start + half,
            BeakAlign::End => edge - beak.inset_end - half,
        }
    };
    wanted.clamp(min, max)
}

/// Extra canvas inset (top, right, bottom, left) so the beak sits in the shape box.
pub fn beak_insets(beak: Option<&Beak>) -> (f32, f32, f32, f32) {
    match beak {
        Some(b) if b.height > 0. && b.width > 0. => match b.side {
            BeakSide::Top => (b.height, 0., 0., 0.),
            BeakSide::Right => (0., b.height, 0., 0.),
            BeakSide::Bottom => (0., 0., b.height, 0.),
            BeakSide::Left => (0., 0., 0., b.height),
        },
        _ => (0., 0., 0., 0.),
    }
}

fn towards(from: Point<f32>, to: Point<f32>, d: f32) -> Point<f32> {
    let len = ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt();
    if len == 0. {
        return from;
    }
    let t = d / len;
    point(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
}

/// Port of web `buildFloatingOutline` into a GPUI path builder. `o` is the
/// panel's top-left; the beak protrudes outside the panel box.
fn build_outline(
    b: &mut PathBuilder,
    o: Point<Pixels>,
    w: f32,
    h: f32,
    radius: f32,
    beak: &Beak,
    center: f32,
    inset: f32,
) {
    let (ox, oy) = (f32::from(o.x), f32::from(o.y));
    let p = |q: Point<f32>| point(px(ox + q.x), px(oy + q.y));
    let x0 = inset;
    let y0 = inset;
    let x1 = w - inset;
    let y1 = h - inset;
    let r = (radius - inset)
        .min((x1 - x0) / 2.)
        .min((y1 - y0) / 2.)
        .max(0.);
    let half = beak.width / 2.;
    let sin_half = half / (half * half + beak.height * beak.height).sqrt();
    let reach = beak.height + inset - inset / sin_half;
    let spread = half * reach / beak.height;
    let tip_r = beak.tip_radius;

    let beak_seg = |b: &mut PathBuilder, first: Point<f32>, tip: Point<f32>, second: Point<f32>| {
        let before = towards(tip, first, tip_r);
        let after = towards(tip, second, tip_r);
        b.line_to(p(first));
        b.line_to(p(before));
        b.curve_to(p(after), p(tip));
        b.line_to(p(second));
    };
    let arc = |b: &mut PathBuilder, to: Point<f32>| {
        if r > 0. {
            b.arc_to(point(px(r), px(r)), px(0.), false, true, p(to));
        } else {
            b.line_to(p(to));
        }
    };

    b.move_to(p(point(x0 + r, y0)));
    if beak.side == BeakSide::Top {
        beak_seg(
            b,
            point(center - spread, y0),
            point(center, y0 - reach),
            point(center + spread, y0),
        );
    }
    b.line_to(p(point(x1 - r, y0)));
    arc(b, point(x1, y0 + r));
    if beak.side == BeakSide::Right {
        beak_seg(
            b,
            point(x1, center - spread),
            point(x1 + reach, center),
            point(x1, center + spread),
        );
    }
    b.line_to(p(point(x1, y1 - r)));
    arc(b, point(x1 - r, y1));
    if beak.side == BeakSide::Bottom {
        beak_seg(
            b,
            point(center + spread, y1),
            point(center, y1 + reach),
            point(center - spread, y1),
        );
    }
    b.line_to(p(point(x0 + r, y1)));
    arc(b, point(x0, y1 - r));
    if beak.side == BeakSide::Left {
        beak_seg(
            b,
            point(x0, center + spread),
            point(x0 - reach, center),
            point(x0, center - spread),
        );
    }
    b.line_to(p(point(x0, y0 + r)));
    arc(b, point(x0 + r, y0));
    b.close();
}

/// Paint the panel + beak as **one** filled and stroked outline.
///
/// `bounds` is the **shape box** (panel plus the beak protrusion). When a
/// beak is present the canvas is grown by [`beak_insets`] so the path is not
/// clipped and the hairline continues around the arrow with no seam.
pub fn paint_floating_surface(
    bounds: Bounds<Pixels>,
    radius: f32,
    beak: Option<&Beak>,
    fill: Hsla,
    border: Hsla,
    window: &mut gpui::Window,
) {
    paint_outline(bounds, radius, beak, fill, border, true, window)
}

/// Beak region plus a short overlap into the panel, so a rectangular
/// material plate's edge is hidden under the beak fill.
fn beak_fill_mask(
    origin: gpui::Point<Pixels>,
    w: f32,
    h: f32,
    beak: &Beak,
    overlap: f32,
) -> Option<Bounds<Pixels>> {
    if beak.width <= 0. || beak.height <= 0. {
        return None;
    }
    let reach = px(beak.height + overlap);
    Some(match beak.side {
        BeakSide::Top => Bounds::new(
            point(origin.x, origin.y - px(beak.height)),
            gpui::size(px(w), reach),
        ),
        BeakSide::Bottom => Bounds::new(
            point(origin.x, origin.y + px(h) - px(overlap)),
            gpui::size(px(w), reach),
        ),
        BeakSide::Left => Bounds::new(
            point(origin.x - px(beak.height), origin.y),
            gpui::size(reach, px(h)),
        ),
        BeakSide::Right => Bounds::new(
            point(origin.x + px(w) - px(overlap), origin.y),
            gpui::size(reach, px(h)),
        ),
    })
}

fn panel_in_shape(bounds: Bounds<Pixels>, beak: Option<&Beak>) -> (gpui::Point<Pixels>, f32, f32) {
    let (top, right, bottom, left) = beak_insets(beak);
    let origin = point(bounds.origin.x + px(left), bounds.origin.y + px(top));
    let w = (f32::from(bounds.size.width) - left - right).max(0.);
    let h = (f32::from(bounds.size.height) - top - bottom).max(0.);
    (origin, w, h)
}

fn paint_outline(
    bounds: Bounds<Pixels>,
    radius: f32,
    beak: Option<&Beak>,
    fill: Hsla,
    border: Hsla,
    fill_panel: bool,
    window: &mut gpui::Window,
) {
    let (origin, w, h) = panel_in_shape(bounds, beak);
    let stroke = refineui_foundation::stroke::HAIRLINE;
    let none = Beak {
        side: BeakSide::Top,
        align: BeakAlign::Center,
        width: 0.,
        height: 1.,
        tip_radius: 0.,
        inset_start: 0.,
        inset_end: 0.,
        center: None,
    };
    let (beak, center) = match beak {
        Some(b) => {
            let edge = if b.side.vertical() { w } else { h };
            (*b, resolve_beak_center(edge, b, radius))
        }
        None => (none, 0.),
    };
    // One closed path for the panel and the beak. The hairline runs around
    // the arrow so there is no join line. When a material plate is under the
    // panel, only the beak (plus a short overlap) is filled.
    let mut f = PathBuilder::fill();
    build_outline(&mut f, origin, w, h, radius, &beak, center, 0.);
    if let Ok(path) = f.build() {
        if fill_panel {
            window.paint_path(path, fill);
        } else if let Some(mask) = beak_fill_mask(origin, w, h, &beak, 2.) {
            window.with_content_mask(Some(gpui::ContentMask { bounds: mask }), |window| {
                window.paint_path(path, fill)
            });
        }
    }
    let mut s = PathBuilder::stroke(px(stroke));
    build_outline(&mut s, origin, w, h, radius, &beak, center, stroke / 2.);
    if let Ok(path) = s.build() {
        window.paint_path(path, border);
    }
}

fn position_shape_canvas(canvas: gpui::Canvas<()>, beak: Option<&Beak>) -> gpui::Canvas<()> {
    let (top, right, bottom, left) = beak_insets(beak);
    canvas
        .absolute()
        .top(px(-top))
        .right(px(-right))
        .bottom(px(-bottom))
        .left(px(-left))
}

/// An absolutely positioned layer that paints the floating material (fill +
/// hairline + beak) behind a panel's content. Add it as the panel's **first**
/// child; the panel itself must have no background or border of its own. The
/// canvas extends past the panel by the beak height so the arrow is part of
/// the same shape.
pub fn floating_surface(
    radius: f32,
    beak: Option<Beak>,
    fill: Hsla,
    border: Hsla,
) -> impl IntoElement + Styled {
    position_shape_canvas(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                paint_floating_surface(bounds, radius, beak.as_ref(), fill, border, window)
            },
        ),
        beak.as_ref(),
    )
}

/// Gap between a trigger and a floating panel: beak height + `sizeXXSmall`.
pub fn floating_gap(beak_height: f32) -> Pixels {
    px(beak_height + refineui_foundation::spacing::XX_SMALL)
}

fn floating_outline_over_material(
    radius: f32,
    beak: Option<Beak>,
    beak_fill: Hsla,
    border: Hsla,
) -> impl IntoElement + Styled {
    position_shape_canvas(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                paint_outline(
                    bounds,
                    radius,
                    beak.as_ref(),
                    beak_fill,
                    border,
                    false,
                    window,
                )
            },
        ),
        beak.as_ref(),
    )
}

/// Material-aware floating chrome. The panel may sit on a material plate
/// (live blur). The beak is filled with `fill` and the hairline strokes the
/// **combined** path (panel + beak), so there is no rectangular border
/// through the arrow. GPUI can only clip backdrop blur to a rectangle, so
/// the beak itself is a tinted fill rather than live blur.
pub(crate) fn floating_chrome(
    kind: crate::SurfaceKind,
    radius: f32,
    beak: Option<Beak>,
    fill: Hsla,
    border: Hsla,
    shadows: Vec<gpui::BoxShadow>,
    cx: &gpui::App,
) -> (Vec<gpui::AnyElement>, Vec<gpui::BoxShadow>) {
    let beaked = beak.as_ref().is_some_and(|b| b.width > 0. && b.height > 0.);
    let b = crate::material::backing(kind, px(radius), 0., fill, None, shadows, cx);
    let mut layers = Vec::with_capacity(2);
    if let Some(layer) = b.layer {
        // Panel-only material (no rect border). The outline canvas extends
        // to include the beak and strokes the combined path.
        layers.push(layer);
        if beaked {
            layers.push(
                floating_outline_over_material(radius, beak, fill, border).into_any_element(),
            );
        } else {
            layers.push(floating_surface(radius, beak, fill, border).into_any_element());
        }
    } else {
        layers.push(floating_surface(radius, beak, fill, border).into_any_element());
    }
    (layers, b.shadows)
}
