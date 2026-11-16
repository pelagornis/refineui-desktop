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
}

impl Beak {
    /// Tooltip beak: 16×8, tip radius 2, insets `sizeMedium`.
    pub fn tooltip(side: BeakSide, align: BeakAlign) -> Self {
        use refineui_tokens::size::component::*;
        Self {
            side,
            align,
            width: TOOLTIP_BEAK_WIDTH,
            height: TOOLTIP_BEAK_HEIGHT,
            tip_radius: TOOLTIP_BEAK_TIP_RADIUS,
            inset_start: refineui_tokens::spacing::MEDIUM,
            inset_end: refineui_tokens::spacing::MEDIUM,
        }
    }

    /// Popover beak: 24×12, tip radius 4, insets 34 / 30.
    pub fn popover(side: BeakSide, align: BeakAlign) -> Self {
        use refineui_tokens::size::component::*;
        Self {
            side,
            align,
            width: POPOVER_BEAK_WIDTH,
            height: POPOVER_BEAK_HEIGHT,
            tip_radius: POPOVER_BEAK_TIP_RADIUS,
            inset_start: POPOVER_BEAK_INSET_START,
            inset_end: POPOVER_BEAK_INSET_END,
        }
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
    let wanted = match beak.align {
        BeakAlign::Center => edge / 2.,
        BeakAlign::Start => beak.inset_start + half,
        BeakAlign::End => edge - beak.inset_end - half,
    };
    wanted.clamp(min, max)
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

/// Paint the panel + beak outline into `bounds` (the panel border box).
pub fn paint_floating_surface(
    bounds: Bounds<Pixels>,
    radius: f32,
    beak: Option<&Beak>,
    fill: Hsla,
    border: Hsla,
    window: &mut gpui::Window,
) {
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    let stroke = refineui_tokens::stroke::HAIRLINE;
    let none = Beak {
        side: BeakSide::Top,
        align: BeakAlign::Center,
        width: 0.,
        height: 1.,
        tip_radius: 0.,
        inset_start: 0.,
        inset_end: 0.,
    };
    let (beak, center) = match beak {
        Some(b) => {
            let edge = if b.side.vertical() { w } else { h };
            (*b, resolve_beak_center(edge, b, radius))
        }
        None => (none, 0.),
    };
    let mut f = PathBuilder::fill();
    build_outline(&mut f, bounds.origin, w, h, radius, &beak, center, 0.);
    if let Ok(path) = f.build() {
        window.paint_path(path, fill);
    }
    let mut s = PathBuilder::stroke(px(stroke));
    build_outline(
        &mut s,
        bounds.origin,
        w,
        h,
        radius,
        &beak,
        center,
        stroke / 2.,
    );
    if let Ok(path) = s.build() {
        window.paint_path(path, border);
    }
}

/// An absolutely positioned layer that paints the floating material (fill +
/// hairline + beak) behind a panel's content. Add it as the panel's **first**
/// child; the panel itself must have no background or border of its own.
pub fn floating_surface(
    radius: f32,
    beak: Option<Beak>,
    fill: Hsla,
    border: Hsla,
) -> impl IntoElement + Styled {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            paint_floating_surface(bounds, radius, beak.as_ref(), fill, border, window)
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Gap between a trigger and a floating panel: beak height + `sizeXXSmall`.
pub fn floating_gap(beak_height: f32) -> Pixels {
    px(beak_height + refineui_tokens::spacing::XX_SMALL)
}
