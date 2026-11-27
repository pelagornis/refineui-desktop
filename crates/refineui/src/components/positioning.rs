//! Collision-aware placement for floating panels (Select, Menu, Popover).
//!
//! Pure functions over rectangles — no styling, no theme. Callers measure
//! a trigger and a viewport, pass a desired size, and get a clamped
//! [`FloatingLayout`] that never overflows the viewport (after
//! [`CollisionOptions::collision_padding`]).
//!
//! Item-aligned mode ([`place_item_aligned`], used by Select) mirrors Radix
//! `SelectItemAlignedPosition`: the panel covers the trigger with the
//! selected row laid exactly over the trigger value, shifting inside the
//! padded viewport and shrinking (with a scroll offset) when it can't.
//!
//! Popper mode ([`place_popper`]) opens on the preferred side
//! (default: below) and flips to the side with more room when the panel
//! would not fit, shrinking to the remaining height. The panel is anchored
//! to the trigger and never covers it.

use gpui::{Bounds, Pixels, Point, Size, point, px};

/// Default collision padding, matching web `CONTENT_MARGIN` / `foundationSize100`.
pub const DEFAULT_COLLISION_PADDING: f32 = 10.0;
/// Default gap between the trigger and a popper panel.
pub const DEFAULT_SIDE_OFFSET: f32 = 4.0;

/// A rectangle in the same coordinate space as the viewport (window).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementRect {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl PlacementRect {
    /// New rectangle.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// From a GPUI bounds (window coordinates).
    pub fn from_bounds(bounds: Bounds<Pixels>) -> Self {
        Self {
            x: f32::from(bounds.origin.x),
            y: f32::from(bounds.origin.y),
            width: f32::from(bounds.size.width),
            height: f32::from(bounds.size.height),
        }
    }

    /// Left edge.
    pub fn left(self) -> f32 {
        self.x
    }

    /// Top edge.
    pub fn top(self) -> f32 {
        self.y
    }

    /// Right edge.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// Horizontal center.
    pub fn center_x(self) -> f32 {
        self.x + self.width / 2.
    }

    /// Vertical center.
    pub fn center_y(self) -> f32 {
        self.y + self.height / 2.
    }
}

/// Viewport size the panel must stay inside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementViewport {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl PlacementViewport {
    /// New viewport.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// From a GPUI size (usually `window.viewport_size()`).
    pub fn from_size(size: Size<Pixels>) -> Self {
        Self {
            width: f32::from(size.width),
            height: f32::from(size.height),
        }
    }
}

/// Preferred (and resolved) vertical side of a floating panel. For an
/// item-aligned panel it is the edge the panel grows from: `Bottom` when it
/// aligns without top overflow, `Top` when it starts at the top edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FloatingSide {
    /// Below the trigger.
    #[default]
    Bottom,
    /// Above the trigger.
    Top,
}

impl FloatingSide {
    /// The opposite side.
    pub fn opposite(self) -> Self {
        match self {
            Self::Bottom => Self::Top,
            Self::Top => Self::Bottom,
        }
    }
}

/// Padding from the viewport edge and gap from the trigger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionOptions {
    /// Minimum gap between the panel and the viewport edge (web
    /// `collisionPadding` / `--radix-select-content-available-height` inset).
    pub collision_padding: f32,
    /// Gap between the trigger and a popper panel.
    pub side_offset: f32,
}

impl Default for CollisionOptions {
    fn default() -> Self {
        Self {
            collision_padding: DEFAULT_COLLISION_PADDING,
            side_offset: DEFAULT_SIDE_OFFSET,
        }
    }
}

impl CollisionOptions {
    /// Defaults (`collision_padding` 10, `side_offset` 4).
    pub fn new() -> Self {
        Self::default()
    }

    /// Viewport-edge padding.
    pub fn collision_padding(mut self, v: f32) -> Self {
        self.collision_padding = v.max(0.);
        self
    }

    /// Trigger-to-panel gap (popper).
    pub fn side_offset(mut self, v: f32) -> Self {
        self.side_offset = v.max(0.);
        self
    }
}

/// Local point inside the panel used as a CSS-style transform origin.
///
/// Maps to the web custom property
/// `--refineui-select-content-transform-origin` (a CSS string such as
/// `"160px 0px"`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformOrigin {
    /// X from the panel's left edge.
    pub x: f32,
    /// Y from the panel's top edge.
    pub y: f32,
}

impl TransformOrigin {
    /// CSS value for `--refineui-select-content-transform-origin`.
    pub fn to_css(self) -> String {
        format!("{}px {}px", fmt_px(self.x), fmt_px(self.y))
    }
}

fn fmt_px(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.2}")
    }
}

/// Result of collision-aware placement.
///
/// Field mapping to the web Select CSS variables:
///
/// | Field | Web custom property |
/// | --- | --- |
/// | [`Self::available_height`] | `--refineui-select-content-available-height` |
/// | [`Self::trigger_height`] | `--refineui-select-trigger-height` |
/// | [`Self::transform_origin`] (via [`TransformOrigin::to_css`]) | `--refineui-select-content-transform-origin` |
/// | [`Self::side`] | data-side (`top` / `bottom`) |
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingLayout {
    /// Panel left, in viewport coordinates.
    pub x: f32,
    /// Panel top, in viewport coordinates.
    pub y: f32,
    /// Clamped panel width.
    pub width: f32,
    /// Clamped panel height (may be less than the desired content height).
    pub height: f32,
    /// Height the panel may occupy on the resolved side (the constraint,
    /// before min-ing with the content). Maps to
    /// `--refineui-select-content-available-height`.
    pub available_height: f32,
    /// Trigger height. Maps to `--refineui-select-trigger-height`.
    pub trigger_height: f32,
    /// Open-animation origin, in the panel's local space.
    pub transform_origin: TransformOrigin,
    /// Side the panel was placed on.
    pub side: FloatingSide,
    /// Initial vertical scroll of the panel's list so an aligned row sits
    /// over the trigger (item-aligned only; `0` for popper).
    pub scroll_top: f32,
}

impl FloatingLayout {
    /// Top-left origin as a GPUI point.
    pub fn origin(self) -> Point<Pixels> {
        point(px(self.x), px(self.y))
    }
}

/// Room above and below the trigger after padding and side offset,
/// `(above, below)`.
pub fn vertical_room(
    trigger: PlacementRect,
    viewport: PlacementViewport,
    opts: CollisionOptions,
) -> (f32, f32) {
    let pad = opts.collision_padding;
    let off = opts.side_offset;
    let below = (viewport.height - pad - trigger.bottom() - off).max(0.);
    let above = (trigger.top() - pad - off).max(0.);
    (above, below)
}

fn clamp_horizontal(
    desired_x: f32,
    width: f32,
    viewport: PlacementViewport,
    pad: f32,
) -> (f32, f32) {
    let max_w = (viewport.width - 2. * pad).max(0.);
    let width = width.max(0.).min(max_w);
    let max_x = (viewport.width - pad - width).max(pad);
    let x = desired_x.clamp(pad, max_x);
    (x, width)
}

fn origin_for(
    side: FloatingSide,
    trigger: PlacementRect,
    x: f32,
    width: f32,
    height: f32,
) -> TransformOrigin {
    let ox = (trigger.center_x() - x).clamp(0., width.max(0.));
    let oy = match side {
        FloatingSide::Bottom => 0.,
        FloatingSide::Top => height.max(0.),
    };
    TransformOrigin { x: ox, y: oy }
}

/// Resolve which vertical side to use: fit on the preferred side, else the
/// opposite if it fits, else the side with more room (preferred on a tie).
pub fn resolve_side(
    preferred: FloatingSide,
    desired_height: f32,
    above: f32,
    below: f32,
) -> FloatingSide {
    let (pref_room, other_room, other) = match preferred {
        FloatingSide::Bottom => (below, above, FloatingSide::Top),
        FloatingSide::Top => (above, below, FloatingSide::Bottom),
    };
    if desired_height <= pref_room {
        preferred
    } else if desired_height <= other_room {
        other
    } else if below > above {
        FloatingSide::Bottom
    } else if above > below {
        FloatingSide::Top
    } else {
        preferred
    }
}

/// Place a panel below (or above) the trigger, flipping and shrinking so it
/// stays inside the viewport.
///
/// `desired_width` / `desired_height` are the unclamped content size. The
/// returned height is never larger than the available space on the resolved
/// side (not a fixed max-height alone).
pub fn place_popper(
    trigger: PlacementRect,
    viewport: PlacementViewport,
    desired_width: f32,
    desired_height: f32,
    preferred: FloatingSide,
    opts: CollisionOptions,
) -> FloatingLayout {
    let (above, below) = vertical_room(trigger, viewport, opts);
    let side = resolve_side(preferred, desired_height, above, below);
    let available_height = match side {
        FloatingSide::Bottom => below,
        FloatingSide::Top => above,
    };
    let height = desired_height.max(0.).min(available_height);
    let y = match side {
        FloatingSide::Bottom => trigger.bottom() + opts.side_offset,
        FloatingSide::Top => trigger.top() - opts.side_offset - height,
    };
    let (x, width) = clamp_horizontal(trigger.x, desired_width, viewport, opts.collision_padding);
    let transform_origin = origin_for(side, trigger, x, width, height);
    FloatingLayout {
        x,
        y,
        width,
        height,
        available_height,
        trigger_height: trigger.height,
        transform_origin,
        side,
        scroll_top: 0.,
    }
}

/// Measurements of an item-aligned list, all in the panel's local space
/// (`0` = the panel's outer top / left edge, including its border).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemAlignedMetrics {
    /// Distance from the trigger's left edge to its value text.
    pub trigger_value_inset: f32,
    /// Distance from the panel's left edge to an item's text.
    pub item_text_inset: f32,
    /// Full (unclipped) panel height: border + padding + every row.
    pub content_height: f32,
    /// Top of the aligned (selected) row.
    pub item_top: f32,
    /// Height of the aligned row.
    pub item_height: f32,
    /// Panel border (top / bottom) width.
    pub border: f32,
    /// List padding (top / bottom) inside the border.
    pub padding: f32,
    /// The aligned row is the first row of the list.
    pub is_first: bool,
    /// The aligned row is the last row of the list.
    pub is_last: bool,
}

/// Place a panel over the trigger so the selected row sits exactly on the
/// trigger value (Radix `SelectItemAlignedPosition`).
///
/// - Horizontal: the row text lines up with the trigger value; the panel is
///   at least as wide as the trigger (plus the text offset) and is clamped
///   inside the viewport.
/// - Vertical: when the rows above the selected one fit between the top
///   edge and the trigger middle, the panel grows down from the aligned top
///   to the bottom edge ([`FloatingSide::Bottom`]); otherwise it starts at
///   the top edge and the list is scrolled ([`FloatingLayout::scroll_top`])
///   so the row still lands on the trigger ([`FloatingSide::Top`]).
/// - Collision: everything stays inside `collision_padding`; the panel is
///   at least five rows tall (or the whole list), at most the padded
///   viewport, and shifts instead of overflowing.
pub fn place_item_aligned(
    trigger: PlacementRect,
    viewport: PlacementViewport,
    m: ItemAlignedMetrics,
    opts: CollisionOptions,
) -> FloatingLayout {
    let pad = opts.collision_padding;
    // Horizontal: align texts, at least trigger-wide.
    let left = trigger.x + m.trigger_value_inset - m.item_text_inset;
    let min_width = trigger.width + (trigger.x - left);
    let (x, width) = clamp_horizontal(left, min_width, viewport, pad);

    // Vertical (Radix item-aligned).
    let full = m.content_height.max(0.);
    let half = m.item_height / 2.;
    let available_height = (viewport.height - pad * 2.).max(0.);
    let min_height = (m.item_height * 5.).min(full).min(available_height);
    let top_edge_to_trigger_middle = trigger.center_y() - pad;
    let trigger_middle_to_bottom_edge = available_height - top_edge_to_trigger_middle;
    let content_top_to_item_middle = m.item_top + half;
    let item_middle_to_content_bottom = full - content_top_to_item_middle;
    let aligns_without_top_overflow = content_top_to_item_middle <= top_edge_to_trigger_middle;

    let (y, height, scroll_top, side) = if aligns_without_top_overflow {
        // Wrapper anchored to the bottom edge, growing up to the aligned top.
        let bottom_need = half + if m.is_last { m.padding } else { 0. } + m.border;
        let wrapper = (content_top_to_item_middle + trigger_middle_to_bottom_edge.max(bottom_need))
            .max(min_height)
            .min(available_height);
        let y = viewport.height - pad - wrapper;
        (y, full.min(wrapper), 0., FloatingSide::Bottom)
    } else {
        // Wrapper anchored to the top edge; scroll the row onto the trigger.
        let top_need = m.border + if m.is_first { m.padding } else { 0. } + half;
        let wrapper = (top_edge_to_trigger_middle.max(top_need) + item_middle_to_content_bottom)
            .max(min_height)
            .min(available_height);
        let height = full.min(wrapper);
        let scroll = (content_top_to_item_middle - top_edge_to_trigger_middle)
            .clamp(0., (full - height).max(0.));
        (pad, height, scroll, FloatingSide::Top)
    };
    let transform_origin = TransformOrigin {
        x: (trigger.center_x() - x).clamp(0., width.max(0.)),
        y: (trigger.center_y() - y).clamp(0., height.max(0.)),
    };
    FloatingLayout {
        x,
        y,
        width,
        height,
        available_height,
        trigger_height: trigger.height,
        transform_origin,
        side,
        scroll_top,
    }
}

/// Radix `SelectViewport` scroll expansion for an item-aligned panel.
///
/// An item-aligned panel opened shorter than the window (it was cut at the
/// trigger side) grows by the scrolled distance before the list really
/// scrolls, until it reaches the padded window height (or the whole list):
///
/// - [`FloatingSide::Top`] (anchored at the top edge): the panel grows
///   down; the list keeps its scroll, so rows move with the gesture and
///   more rows appear below.
/// - [`FloatingSide::Bottom`] (anchored at the bottom edge): the panel
///   grows up and the growth is taken back out of the scroll, so the rows
///   move exactly as far as the gesture while the top edge rises.
///
/// `layout` is the panel as currently shown (already grown), `prev_scroll`
/// / `scroll` the list offset before and after the gesture. Returns the
/// new panel and the list offset to apply.
pub fn expand_on_scroll(
    layout: FloatingLayout,
    content_height: f32,
    viewport: PlacementViewport,
    opts: CollisionOptions,
    prev_scroll: f32,
    scroll: f32,
) -> (FloatingLayout, f32) {
    let scrolled_by = (scroll - prev_scroll).abs();
    let max_height = (viewport.height - 2. * opts.collision_padding)
        .max(0.)
        .min(content_height.max(0.));
    if scrolled_by <= 0. || layout.height >= max_height {
        return (layout, scroll);
    }
    let next = (layout.height + scrolled_by).min(max_height);
    let grown = next - layout.height;
    let mut out = layout;
    out.height = next;
    let scroll = match layout.side {
        FloatingSide::Top => scroll.min((content_height - next).max(0.)),
        FloatingSide::Bottom => {
            out.y -= grown;
            (scroll - grown).max(0.)
        }
    };
    (out, scroll)
}

/// `base` grown by `grow` px the way [`expand_on_scroll`] grows it (down
/// from a top-anchored panel, up from a bottom-anchored one).
pub fn grow_item_aligned(base: FloatingLayout, grow: f32) -> FloatingLayout {
    let grow = grow.max(0.);
    let mut out = base;
    out.height += grow;
    if base.side == FloatingSide::Bottom {
        out.y -= grow;
    }
    out
}

/// Placement side for a beaked floating panel (Popover / Tooltip).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnchorSide {
    /// Above the trigger; the beak sits on the panel's bottom edge.
    Top,
    /// Below the trigger; the beak sits on the panel's top edge.
    #[default]
    Bottom,
    /// Leading side of the trigger; the beak sits on the panel's right edge.
    Left,
    /// Trailing side of the trigger; the beak sits on the panel's left edge.
    Right,
}

impl AnchorSide {
    /// The opposite side.
    pub fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// `true` when the panel is above or below the trigger.
    pub fn vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// Start / middle / end alignment of a panel against its trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnchorAlign {
    /// Panel start edge flush with the trigger start edge.
    Start,
    /// Centers the panel on the trigger (and aims the beak at the midpoint).
    #[default]
    Center,
    /// Panel end edge flush with the trigger end edge.
    End,
}

/// Result of [`place_anchored`]: a clamped panel rect and the beak center
/// along the edge that faces the trigger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchoredLayout {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Resolved side.
    pub side: AnchorSide,
    /// Beak center along the facing edge, from that edge's start.
    pub beak_center: f32,
}

/// Place a beaked panel on `preferred`, flipping to the opposite side (or
/// the side with more room) when it would overflow. The returned
/// `beak_center` aims at the trigger midpoint and is ready to clamp against
/// the panel radius.
pub fn place_anchored(
    trigger: PlacementRect,
    viewport: PlacementViewport,
    desired_width: f32,
    desired_height: f32,
    preferred: AnchorSide,
    align: AnchorAlign,
    opts: CollisionOptions,
) -> AnchoredLayout {
    let pad = opts.collision_padding;
    let off = opts.side_offset;
    let below = (viewport.height - pad - trigger.bottom() - off).max(0.);
    let above = (trigger.top() - pad - off).max(0.);
    let right = (viewport.width - pad - trigger.right() - off).max(0.);
    let left = (trigger.left() - pad - off).max(0.);
    let need = if preferred.vertical() {
        desired_height
    } else {
        desired_width
    };
    let room = |s: AnchorSide| match s {
        AnchorSide::Bottom => below,
        AnchorSide::Top => above,
        AnchorSide::Right => right,
        AnchorSide::Left => left,
    };
    let side = if room(preferred) + 0.01 >= need {
        preferred
    } else if room(preferred.opposite()) + 0.01 >= need {
        preferred.opposite()
    } else {
        [
            AnchorSide::Bottom,
            AnchorSide::Top,
            AnchorSide::Right,
            AnchorSide::Left,
        ]
        .into_iter()
        .max_by(|a, b| {
            room(*a)
                .partial_cmp(&room(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(preferred)
    };
    let (width, height, x, y) = match side {
        AnchorSide::Bottom | AnchorSide::Top => {
            let (x, width) = {
                let desired_x = match align {
                    AnchorAlign::Start => trigger.x,
                    AnchorAlign::Center => trigger.center_x() - desired_width / 2.,
                    AnchorAlign::End => trigger.right() - desired_width,
                };
                clamp_horizontal(desired_x, desired_width, viewport, pad)
            };
            let avail = room(side);
            let height = desired_height.max(0.).min(avail);
            let y = if side == AnchorSide::Bottom {
                trigger.bottom() + off
            } else {
                trigger.top() - off - height
            };
            (width, height, x, y)
        }
        AnchorSide::Left | AnchorSide::Right => {
            let max_h = (viewport.height - 2. * pad).max(0.);
            let height = desired_height.max(0.).min(max_h);
            let desired_y = match align {
                AnchorAlign::Start => trigger.y,
                AnchorAlign::Center => trigger.center_y() - height / 2.,
                AnchorAlign::End => trigger.bottom() - height,
            };
            let max_y = (viewport.height - pad - height).max(pad);
            let y = desired_y.clamp(pad, max_y);
            let avail = room(side);
            let width = desired_width.max(0.).min(avail);
            let x = if side == AnchorSide::Right {
                trigger.right() + off
            } else {
                trigger.left() - off - width
            };
            (width, height, x, y)
        }
    };
    let beak_center = if side.vertical() {
        trigger.center_x() - x
    } else {
        trigger.center_y() - y
    };
    AnchoredLayout {
        x,
        y,
        width,
        height,
        side,
        beak_center,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTS: CollisionOptions = CollisionOptions {
        collision_padding: 10.,
        side_offset: 4.,
    };
    const VIEW: PlacementViewport = PlacementViewport {
        width: 800.,
        height: 600.,
    };

    fn trigger(x: f32, y: f32, w: f32, h: f32) -> PlacementRect {
        PlacementRect::new(x, y, w, h)
    }

    #[test]
    fn popper_stays_below_when_there_is_room() {
        let t = trigger(40., 40., 200., 32.);
        let l = place_popper(t, VIEW, 200., 200., FloatingSide::Bottom, OPTS);
        assert_eq!(l.side, FloatingSide::Bottom);
        assert_eq!(l.y, 40. + 32. + 4.);
        assert_eq!(l.x, 40.);
        assert_eq!(l.width, 200.);
        assert_eq!(l.height, 200.);
        assert_eq!(l.trigger_height, 32.);
        assert!(l.available_height >= 200.);
        assert_eq!(l.transform_origin.y, 0.);
        // Anchored below: the panel never covers the trigger.
        assert!(l.y >= t.bottom() + OPTS.side_offset - 0.01);
    }

    fn overlaps_trigger(l: &FloatingLayout, t: PlacementRect) -> bool {
        l.y < t.bottom() && l.y + l.height > t.top()
    }

    #[test]
    fn popper_never_overlaps_the_trigger() {
        let cases = [
            trigger(40., 40., 200., 32.),
            trigger(40., 500., 200., 32.),
            trigger(40., 250., 200., 32.),
            trigger(700., 40., 200., 32.),
        ];
        for t in cases {
            let l = place_popper(t, VIEW, 200., 400., FloatingSide::Bottom, OPTS);
            assert!(
                !overlaps_trigger(&l, t),
                "panel ({}, {} + {}) overlaps trigger ({}, {} + {}) side={:?}",
                l.y,
                l.height,
                l.y + l.height,
                t.y,
                t.height,
                t.bottom(),
                l.side,
            );
            match l.side {
                FloatingSide::Bottom => {
                    assert!(l.y >= t.bottom() + OPTS.side_offset - 0.01);
                }
                FloatingSide::Top => {
                    assert!(l.y + l.height <= t.top() - OPTS.side_offset + 0.01);
                }
            }
        }
    }

    #[test]
    fn popper_flips_to_top_when_the_bottom_does_not_fit() {
        // Trigger near the bottom: 200px panel cannot fit below.
        let t = trigger(40., 500., 200., 32.);
        let l = place_popper(t, VIEW, 200., 200., FloatingSide::Bottom, OPTS);
        assert_eq!(l.side, FloatingSide::Top);
        assert_eq!(l.y, 500. - 4. - 200.);
        assert_eq!(l.height, 200.);
        assert_eq!(l.transform_origin.y, 200.);
        // Stays inside the padded viewport.
        assert!(l.y >= OPTS.collision_padding);
        assert!(l.y + l.height <= 500. - OPTS.side_offset);
    }

    #[test]
    fn popper_picks_the_side_with_more_room_when_neither_fits() {
        // Mid-viewport trigger: 400px fits on neither side; more room below.
        let t = trigger(40., 250., 200., 32.);
        let l = place_popper(t, VIEW, 200., 400., FloatingSide::Top, OPTS);
        let (above, below) = vertical_room(t, VIEW, OPTS);
        assert!(400. > above && 400. > below);
        assert!(below > above);
        assert_eq!(l.side, FloatingSide::Bottom);
        assert_eq!(l.height, below);
        assert_eq!(l.available_height, below);
        assert!(l.y + l.height <= VIEW.height - OPTS.collision_padding + 0.01);
    }

    #[test]
    fn popper_height_is_the_available_space_not_a_fixed_max() {
        let t = trigger(40., 500., 200., 32.);
        let (above, below) = vertical_room(t, VIEW, OPTS);
        assert!(below < 80., "expected little room below, got {below}");
        let l = place_popper(t, VIEW, 200., 2000., FloatingSide::Bottom, OPTS);
        // Flips to top and shrinks to `above`, not to some hardcoded max.
        assert_eq!(l.side, FloatingSide::Top);
        assert_eq!(l.available_height, above);
        assert_eq!(l.height, above);
        assert!(l.height < 2000.);
    }

    #[test]
    fn horizontal_clamp_keeps_the_panel_inside_the_viewport() {
        let t = trigger(700., 40., 200., 32.); // would overflow the right edge
        let l = place_popper(t, VIEW, 200., 80., FloatingSide::Bottom, OPTS);
        assert!(l.x >= OPTS.collision_padding);
        assert!(l.x + l.width <= VIEW.width - OPTS.collision_padding + 0.01);
        assert_eq!(l.width, 200.);

        let t = trigger(-50., 40., 200., 32.);
        let l = place_popper(t, VIEW, 200., 80., FloatingSide::Bottom, OPTS);
        assert_eq!(l.x, OPTS.collision_padding);
    }

    #[test]
    fn width_shrinks_when_wider_than_the_viewport() {
        let t = trigger(10., 40., 200., 32.);
        let l = place_popper(t, VIEW, 900., 80., FloatingSide::Bottom, OPTS);
        assert_eq!(l.width, VIEW.width - 2. * OPTS.collision_padding);
        assert_eq!(l.x, OPTS.collision_padding);
    }

    #[test]
    fn collision_padding_is_respected_on_every_edge() {
        let tight = CollisionOptions {
            collision_padding: 24.,
            side_offset: 4.,
        };
        let t = trigger(10., 10., 120., 24.);
        let l = place_popper(t, VIEW, 400., 80., FloatingSide::Bottom, tight);
        assert!(l.x >= 24.);
        assert!(l.x + l.width <= VIEW.width - 24. + 0.01);
        assert!(l.y >= t.bottom());
    }

    #[test]
    fn transform_origin_css_matches_the_web_variable_format() {
        let o = TransformOrigin { x: 160., y: 0. };
        assert_eq!(o.to_css(), "160px 0px");
    }

    fn metrics(item_top: f32, content_height: f32) -> ItemAlignedMetrics {
        ItemAlignedMetrics {
            trigger_value_inset: 13.,
            item_text_inset: 33.,
            content_height,
            item_top,
            item_height: 32.,
            border: 1.,
            padding: 4.,
            is_first: false,
            is_last: false,
        }
    }

    #[test]
    fn item_aligned_puts_the_row_over_the_trigger_middle() {
        let t = trigger(200., 250., 200., 32.);
        let l = place_item_aligned(t, VIEW, metrics(70., 200.), OPTS);
        assert_eq!(l.side, FloatingSide::Bottom);
        assert_eq!(l.scroll_top, 0.);
        assert!((l.y + 70. + 16. - t.center_y()).abs() < 0.01);
        assert_eq!(l.height, 200.);
        // Row text over the trigger value; at least trigger wide.
        assert!((l.x + 33. - (t.x + 13.)).abs() < 0.01);
        assert!((l.width - (t.width + 20.)).abs() < 0.01);
        assert!(l.y <= t.top() && l.y + l.height >= t.bottom());
    }

    #[test]
    fn item_aligned_top_overflow_anchors_to_the_edge_and_scrolls() {
        let t = trigger(200., 40., 200., 32.);
        let l = place_item_aligned(t, VIEW, metrics(300., 800.), OPTS);
        assert_eq!(l.side, FloatingSide::Top);
        assert_eq!(l.y, OPTS.collision_padding);
        assert!(l.scroll_top > 0.);
        assert!((l.y + 300. + 16. - l.scroll_top - t.center_y()).abs() < 0.01);
        assert!(l.y + l.height <= VIEW.height - OPTS.collision_padding + 0.01);
    }

    #[test]
    fn item_aligned_stays_inside_the_padded_viewport() {
        for y in [0., 10., 200., 540., 580.] {
            for x in [-40., 0., 300., 700.] {
                let t = trigger(x, y, 200., 32.);
                for top in [5., 100., 400., 900.] {
                    let l = place_item_aligned(t, VIEW, metrics(top, 1000.), OPTS);
                    let p = OPTS.collision_padding - 0.01;
                    assert!(l.x >= p && l.x + l.width <= VIEW.width - p, "{l:?}");
                    assert!(l.y >= p && l.y + l.height <= VIEW.height - p, "{l:?}");
                    assert!(l.scroll_top >= 0. && l.scroll_top <= 1000. - l.height);
                }
            }
        }
    }

    #[test]
    fn item_aligned_keeps_a_five_row_minimum_near_the_bottom() {
        let t = trigger(40., 556., 200., 32.);
        let l = place_item_aligned(t, VIEW, metrics(5., 400.), OPTS);
        assert!(l.height >= 160. - 0.01);
        assert!(l.y + l.height <= VIEW.height - OPTS.collision_padding + 0.01);
        assert!(l.y <= t.top() && l.y + l.height >= t.bottom());
    }

    #[test]
    fn expand_on_scroll_grows_a_top_anchored_panel_down() {
        let t = trigger(200., 40., 200., 32.);
        let l = place_item_aligned(t, VIEW, metrics(300., 800.), OPTS);
        assert_eq!(l.side, FloatingSide::Top);
        let s0 = l.scroll_top;
        // Scroll up by 30: the panel grows 30 down, the scroll is kept.
        let (g, s) = expand_on_scroll(l, 800., VIEW, OPTS, s0, s0 - 30.);
        assert_eq!(g.y, l.y);
        assert!((g.height - (l.height + 30.)).abs() < 0.01);
        assert_eq!(s, s0 - 30.);
        // Grows until the padded window height, then just scrolls.
        let (g, _) = expand_on_scroll(g, 800., VIEW, OPTS, 0., 2000.);
        assert!((g.height - (VIEW.height - 2. * OPTS.collision_padding)).abs() < 0.01);
        let (g2, s2) = expand_on_scroll(g, 800., VIEW, OPTS, 0., 50.);
        assert_eq!(g2, g);
        assert_eq!(s2, 50.);
    }

    #[test]
    fn expand_on_scroll_grows_a_bottom_anchored_panel_up_and_keeps_rows_moving() {
        let t = trigger(200., 250., 200., 32.);
        let l = place_item_aligned(t, VIEW, metrics(70., 900.), OPTS);
        assert_eq!(l.side, FloatingSide::Bottom);
        assert_eq!(l.scroll_top, 0.);
        let bottom = l.y + l.height;
        // Scroll down by 40: the panel grows 40 up and the list stays at 0,
        // so the first row moves up 40 on screen, as a real scroll would.
        let (g, s) = expand_on_scroll(l, 900., VIEW, OPTS, 0., 40.);
        assert!((g.y + g.height - bottom).abs() < 0.01);
        assert!((g.height - (l.height + 40.)).abs() < 0.01);
        assert_eq!(s, 0.);
        assert!(((g.y - s) - (l.y - 40.)).abs() < 0.01);
        // Past the window height the remainder becomes a real scroll.
        let room = VIEW.height - 2. * OPTS.collision_padding - g.height;
        let (g2, s2) = expand_on_scroll(g, 900., VIEW, OPTS, 0., room + 25.);
        assert!((g2.y - OPTS.collision_padding).abs() < 0.01);
        assert!((s2 - 25.).abs() < 0.01);
    }

    #[test]
    fn expand_on_scroll_never_exceeds_the_list() {
        let t = trigger(200., 40., 200., 32.);
        let l = place_item_aligned(t, VIEW, metrics(300., 420.), OPTS);
        let (g, _) = expand_on_scroll(l, 420., VIEW, OPTS, l.scroll_top, 0.);
        assert!(g.height <= 420. + 0.01);
        assert_eq!(grow_item_aligned(l, g.height - l.height), g);
    }

    #[test]
    fn anchored_flips_to_the_side_with_more_room() {
        let t = trigger(100., 40., 80., 24.);
        let l = place_anchored(
            t,
            VIEW,
            200.,
            400.,
            AnchorSide::Top,
            AnchorAlign::Center,
            OPTS,
        );
        // 400px does not fit above a trigger at y=40 (room ≈ 26), so it flips down.
        assert_eq!(l.side, AnchorSide::Bottom);
        assert!(l.y >= t.bottom());
        assert!(l.x >= OPTS.collision_padding);
    }

    #[test]
    fn anchored_beak_aims_at_the_trigger_center() {
        let t = trigger(200., 200., 80., 32.);
        let l = place_anchored(
            t,
            VIEW,
            240.,
            80.,
            AnchorSide::Bottom,
            AnchorAlign::Center,
            OPTS,
        );
        assert_eq!(l.side, AnchorSide::Bottom);
        assert!((l.x + l.beak_center - t.center_x()).abs() < 0.5);
    }
}
