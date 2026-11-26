//! Tooltip — web `Tooltip`.
//!
//! Wraps a trigger and, on hover, paints a beaked bubble anchored to that
//! trigger's bounds (not the pointer). The bubble is lifted with
//! [`crate::layer`] at [`crate::foundation::LayerOrder::Tooltip`] so it
//! always paints above the trigger — including when the preferred side is
//! [`AnchorSide::Top`] (Above), where an in-tree sibling would otherwise
//! be covered by the button and its overlapping beak.
//!
//! Attach with [`Tooltip::new`] (or `Button` / `Toggle`'s `.tooltip(text)`
//! shorthand). Placement flips when the preferred side does not fit.

use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, anchored, div, point, prelude::*, px,
};

use refineui_foundation::{radius, size::component, spacing, stroke, typography::role};

use crate::components::floating::{Beak, BeakAlign, BeakSide, floating_gap};
use crate::components::positioning::{
    AnchorAlign, AnchorSide, AnchoredLayout, CollisionOptions, DEFAULT_COLLISION_PADDING,
    PlacementRect, PlacementViewport, place_anchored,
};
use crate::foundation::LayerOrder;
use crate::motion::{Motion, child_id, presence, probe, use_layout_probe};
use crate::{ActiveTheme, Elevation, Kbd, StyledExt};

/// Web `Tooltip` hover delay (`delayMs = 120`).
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(120);

/// Gap between the trigger and the panel: beak height + `sizeXXSmall`.
pub fn tooltip_side_offset() -> f32 {
    f32::from(floating_gap(component::TOOLTIP_BEAK_HEIGHT))
}

/// Collision options used by a live tooltip (`collision_padding` 10, gap
/// [`tooltip_side_offset`] unless overridden).
pub fn tooltip_collision_options(side_offset: Option<f32>) -> CollisionOptions {
    CollisionOptions::new()
        .collision_padding(DEFAULT_COLLISION_PADDING)
        .side_offset(side_offset.unwrap_or_else(tooltip_side_offset))
}

/// Beak edge that faces the trigger for `placement`.
pub fn tooltip_beak_side(placement: AnchorSide) -> BeakSide {
    match placement {
        AnchorSide::Bottom => BeakSide::Top,
        AnchorSide::Top => BeakSide::Bottom,
        AnchorSide::Left => BeakSide::Right,
        AnchorSide::Right => BeakSide::Left,
    }
}

/// Place a tooltip panel on `preferred`, centered on the trigger edge,
/// flipping when that side does not fit. The returned `beak_center` aims
/// at the trigger midpoint.
pub fn place_tooltip(
    trigger: PlacementRect,
    viewport: PlacementViewport,
    desired_width: f32,
    desired_height: f32,
    preferred: AnchorSide,
    align: AnchorAlign,
    opts: CollisionOptions,
) -> AnchoredLayout {
    place_anchored(
        trigger,
        viewport,
        desired_width,
        desired_height,
        preferred,
        align,
        opts,
    )
}

/// A tooltip attached to a trigger child.
///
/// ```ignore
/// Tooltip::new("save-tip", "Save")
///     .shortcut("⌘S")
///     .placement(AnchorSide::Top)
///     .child(Button::new("save", "Save"))
/// ```
#[derive(IntoElement)]
pub struct Tooltip {
    id: ElementId,
    text: SharedString,
    shortcut: Option<SharedString>,
    action: Option<SharedString>,
    placement: AnchorSide,
    align: AnchorAlign,
    side_offset: Option<f32>,
    delay: Duration,
    /// When `Some`, the open state is controlled (used by docs / screenshots).
    open: Option<bool>,
    trigger: Option<AnyElement>,
}

impl Tooltip {
    /// Attach a hover tooltip to a following [`Self::child`].
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            shortcut: None,
            action: None,
            placement: AnchorSide::Bottom,
            align: AnchorAlign::Center,
            side_offset: None,
            delay: TOOLTIP_DELAY,
            open: None,
            trigger: None,
        }
    }

    /// Tooltip with a keyboard shortcut hint, e.g.
    /// `Tooltip::new("save-tip", "Save").shortcut("⌘S")`.
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Resolve the shortcut from the installed keymap for this action
    /// (`⇧⌘P` / `Ctrl+Shift+P`); no hint when unbound.
    pub fn action(mut self, action: impl Into<SharedString>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Preferred side (web `position`). Defaults to [`AnchorSide::Bottom`].
    /// Flips when that side does not fit in the window.
    pub fn placement(mut self, side: AnchorSide) -> Self {
        self.placement = side;
        self
    }

    /// Alignment along the facing edge. Defaults to [`AnchorAlign::Center`].
    pub fn align(mut self, align: AnchorAlign) -> Self {
        self.align = align;
        self
    }

    /// Gap between the trigger and the panel (beak height is already
    /// reserved by the default). `0` is allowed.
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = Some(offset.max(0.));
        self
    }

    /// Hover delay before the tooltip appears (web default 120ms).
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Force the tooltip open or closed, skipping hover. Used by docs /
    /// screenshots so every side can be captured without a pointer.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// The control the tooltip is anchored to.
    pub fn child(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Render a static tooltip bubble (used by docs / screenshots).
    pub fn bubble(text: impl Into<SharedString>, cx: &App) -> gpui::Div {
        bubble(text.into(), None, None, cx)
    }
}

#[derive(Default)]
struct TipHover {
    hovered: bool,
    open: bool,
    generation: u64,
}

fn bubble(
    text: SharedString,
    shortcut: Option<SharedString>,
    beak: Option<Beak>,
    cx: &App,
) -> gpui::Div {
    let theme = cx.theme();
    let c = theme.colors;
    // Web `Tooltip`: transparent hairline-border panel (`roundedMedium`,
    // `bodyXs`, px `sizeMedium` / py `sizeSmall`) over a `FloatingSurface`
    // that fills + strokes the panel and its beak as one outline.
    // With materials active the panel is the Tooltip material (thin, real
    // blur) and the outline only strokes it and fills the beak.
    let (chrome, shadows) = crate::components::floating::floating_chrome(
        crate::SurfaceKind::Tooltip,
        radius::MEDIUM,
        beak,
        theme.floating_fill(),
        c.border_default,
        theme.shadow(Elevation::FLOATING),
        cx,
    );
    div()
        .relative()
        .max_w(px(component::TOOLTIP_MAX_WIDTH))
        .flex()
        .flex_col()
        .rounded(px(radius::MEDIUM))
        .shadow(shadows)
        .children(chrome)
        .text_color(c.foreground_primary)
        .font_family(theme.font_family.clone())
        .text_style_token(role::BODY_XS)
        .whitespace_nowrap()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::X_SMALL))
                .px(px(spacing::MEDIUM + stroke::HAIRLINE))
                .py(px(spacing::SMALL + stroke::HAIRLINE))
                .child(text)
                .when_some(shortcut, |this, s| this.child(Kbd::new(s))),
        )
}

/// A tooltip bubble with its 16×8 beak (2px rounded tip) inset `sizeMedium`
/// from the start edge — web default `align="Start"`. `BeakSide::Bottom` is a
/// tooltip shown above its trigger; `Top` one shown below. Used by docs /
/// screenshots.
pub fn tooltip_with_beak(text: impl Into<SharedString>, side: BeakSide, cx: &App) -> gpui::Div {
    tooltip_with_beak_aligned(text, side, BeakAlign::Start, cx)
}

/// [`tooltip_with_beak`] with an explicit beak alignment.
pub fn tooltip_with_beak_aligned(
    text: impl Into<SharedString>,
    side: BeakSide,
    align: BeakAlign,
    cx: &App,
) -> gpui::Div {
    bubble(text.into(), None, Some(Beak::tooltip(side, align)), cx)
}

fn schedule_show(
    state: Entity<TipHover>,
    generation: u64,
    delay: Duration,
    window: &mut Window,
    cx: &mut App,
) {
    if delay.is_zero() {
        state.update(cx, |s, cx| {
            if s.generation == generation && s.hovered && !s.open {
                s.open = true;
                cx.notify();
            }
        });
        return;
    }
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(delay).await;
            cx.update(|_, cx| {
                state.update(cx, |s, cx| {
                    if s.generation == generation && s.hovered && !s.open {
                        s.open = true;
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
}

impl RenderOnce for Tooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let shortcut = match (&self.shortcut, &self.action) {
            (Some(s), _) => Some(s.clone()),
            (None, Some(action)) => {
                crate::interaction::keymap::shortcut_for(action, cx).map(Into::into)
            }
            (None, None) => None,
        };
        let hover =
            window.use_keyed_state(child_id(&self.id, "hover"), cx, |_, _| TipHover::default());
        let is_open = self.open.unwrap_or_else(|| hover.read(cx).open);
        let p = presence(
            child_id(&self.id, "presence"),
            is_open,
            Motion::new(
                crate::motion::duration::FAST,
                crate::motion::easing::EMPHASIZED,
            ),
            Motion::FAST,
            window,
            cx,
        );
        let measure = use_layout_probe(&self.id, window, cx);
        let preferred = self.placement;
        let align = self.align;
        let opts = tooltip_collision_options(self.side_offset);
        let viewport = PlacementViewport::from_size(window.viewport_size());
        let placed = {
            let m = measure.read(cx);
            match (m.container, m.items.first().copied().flatten()) {
                (Some(t), Some(panel)) => Some(place_tooltip(
                    PlacementRect::from_bounds(t),
                    viewport,
                    f32::from(panel.size.width),
                    f32::from(panel.size.height),
                    preferred,
                    align,
                    opts,
                )),
                _ => None,
            }
        };
        let measured = placed.is_some();
        let (pos, resolved_side, beak_center) = match (placed, measure.read(cx).container) {
            (Some(l), _) => (point(px(l.x), px(l.y)), l.side, Some(l.beak_center)),
            (None, Some(t)) => {
                let gap = px(opts.side_offset);
                let pos = match preferred {
                    AnchorSide::Bottom => point(t.origin.x, t.bottom() + gap),
                    AnchorSide::Top => point(t.origin.x, t.origin.y),
                    AnchorSide::Right => point(t.right() + gap, t.origin.y),
                    AnchorSide::Left => point(t.origin.x, t.origin.y),
                };
                (pos, preferred, None)
            }
            _ => (point(px(0.), px(0.)), preferred, None),
        };
        let delay = self.delay;
        let controlled = self.open.is_some();
        let slide = spacing::X_SMALL * (1. - p.progress);
        let slide_pt = match resolved_side {
            AnchorSide::Bottom => point(px(0.), px(slide)),
            AnchorSide::Top => point(px(0.), px(-slide)),
            AnchorSide::Right => point(px(slide), px(0.)),
            AnchorSide::Left => point(px(-slide), px(0.)),
        };

        let overlay = p.mounted.then(|| {
            let mut beak = Beak::tooltip(
                tooltip_beak_side(resolved_side),
                match align {
                    AnchorAlign::Start => BeakAlign::Start,
                    AnchorAlign::Center => BeakAlign::Center,
                    AnchorAlign::End => BeakAlign::End,
                },
            );
            if let Some(c) = beak_center {
                beak = beak.with_center(c);
            }
            let panel =
                bubble(self.text.clone(), shortcut, Some(beak), cx).child(probe(&measure, Some(0)));
            crate::layer(
                anchored()
                    .position(pos + slide_pt)
                    .snap_to_window_with_margin(px(opts.collision_padding))
                    .child(
                        div()
                            .id("tooltip-panel")
                            .opacity(if measured { p.progress } else { 0. })
                            .child(panel),
                    ),
            )
            .priority(LayerOrder::Tooltip.priority())
            .into_any_element()
        });

        div()
            .id(self.id)
            .relative()
            .flex_none()
            .child(
                div()
                    .id("trigger")
                    .relative()
                    .child(probe(&measure, None))
                    .when(!controlled, |d| {
                        let hover_in = hover.clone();
                        d.on_hover(move |hovered, window, cx| {
                            let hovered = *hovered;
                            let (generation, should_show) = hover_in.update(cx, |s, cx| {
                                if s.hovered == hovered {
                                    return (s.generation, false);
                                }
                                s.hovered = hovered;
                                s.generation = s.generation.wrapping_add(1);
                                if !hovered {
                                    s.open = false;
                                }
                                cx.notify();
                                (s.generation, hovered)
                            });
                            if should_show {
                                schedule_show(hover_in.clone(), generation, delay, window, cx);
                            }
                        })
                    })
                    .children(self.trigger),
            )
            .children(overlay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: PlacementViewport = PlacementViewport {
        width: 800.,
        height: 600.,
    };

    fn opts() -> CollisionOptions {
        tooltip_collision_options(None)
    }

    fn trigger(x: f32, y: f32, w: f32, h: f32) -> PlacementRect {
        PlacementRect::new(x, y, w, h)
    }

    fn place(t: PlacementRect, tip_w: f32, tip_h: f32, preferred: AnchorSide) -> AnchoredLayout {
        place_tooltip(
            t,
            VIEW,
            tip_w,
            tip_h,
            preferred,
            AnchorAlign::Center,
            opts(),
        )
    }

    #[test]
    fn default_gap_is_beak_plus_xx_small() {
        assert_eq!(
            tooltip_side_offset(),
            component::TOOLTIP_BEAK_HEIGHT + spacing::XX_SMALL
        );
    }

    #[test]
    fn below_when_there_is_room() {
        let t = trigger(200., 80., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Bottom);
        assert_eq!(l.side, AnchorSide::Bottom);
        assert_eq!(l.y, t.bottom() + tooltip_side_offset());
        assert!((l.x + l.width / 2. - t.center_x()).abs() < 0.01);
        assert!((l.beak_center - (t.center_x() - l.x)).abs() < 0.01);
    }

    #[test]
    fn above_when_requested_and_there_is_room() {
        let t = trigger(200., 200., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Top);
        assert_eq!(l.side, AnchorSide::Top);
        assert_eq!(l.y + l.height + tooltip_side_offset(), t.top());
        assert!((l.x + l.width / 2. - t.center_x()).abs() < 0.01);
        assert!((l.beak_center - (t.center_x() - l.x)).abs() < 0.01);
    }

    #[test]
    fn flips_to_top_when_the_bottom_does_not_fit() {
        let t = trigger(200., 560., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Bottom);
        assert_eq!(l.side, AnchorSide::Top);
        assert!(l.y + l.height + tooltip_side_offset() <= t.top() + 0.01);
        assert!(l.y >= DEFAULT_COLLISION_PADDING - 0.01);
    }

    #[test]
    fn flips_to_bottom_when_the_top_does_not_fit() {
        let t = trigger(200., 16., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Top);
        assert_eq!(l.side, AnchorSide::Bottom);
        assert_eq!(l.y, t.bottom() + tooltip_side_offset());
    }

    #[test]
    fn right_when_there_is_room() {
        let t = trigger(200., 200., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Right);
        assert_eq!(l.side, AnchorSide::Right);
        assert_eq!(l.x, t.right() + tooltip_side_offset());
        assert!((l.y + l.height / 2. - t.center_y()).abs() < 0.01);
        assert!((l.beak_center - (t.center_y() - l.y)).abs() < 0.01);
    }

    #[test]
    fn left_when_there_is_room() {
        let t = trigger(200., 200., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Left);
        assert_eq!(l.side, AnchorSide::Left);
        assert_eq!(l.x + l.width + tooltip_side_offset(), t.left());
        assert!((l.y + l.height / 2. - t.center_y()).abs() < 0.01);
    }

    #[test]
    fn flips_to_left_when_the_right_does_not_fit() {
        let t = trigger(700., 200., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Right);
        assert_eq!(l.side, AnchorSide::Left);
    }

    #[test]
    fn flips_to_right_when_the_left_does_not_fit() {
        let t = trigger(16., 200., 80., 32.);
        let l = place(t, 120., 28., AnchorSide::Left);
        assert_eq!(l.side, AnchorSide::Right);
    }

    #[test]
    fn beak_stays_aimed_at_the_trigger_center_when_the_panel_is_clamped() {
        // Near the left edge, a wide tooltip cannot stay centered; the beak
        // still points at the trigger midpoint.
        let t = trigger(20., 200., 40., 32.);
        let l = place(t, 240., 28., AnchorSide::Bottom);
        assert!(l.x >= DEFAULT_COLLISION_PADDING - 0.01);
        assert!((l.beak_center - (t.center_x() - l.x)).abs() < 0.01);
    }

    #[test]
    fn placement_ignores_a_pointer_position() {
        // The helper takes only the trigger rect — a moving pointer inside
        // the trigger cannot change the result.
        let t = trigger(200., 200., 80., 32.);
        let a = place(t, 120., 28., AnchorSide::Top);
        let b = place(t, 120., 28., AnchorSide::Top);
        assert_eq!(a, b);
        assert_eq!(a.side, AnchorSide::Top);
    }

    #[test]
    fn custom_side_offset_is_respected() {
        let t = trigger(200., 80., 80., 32.);
        let l = place_tooltip(
            t,
            VIEW,
            120.,
            28.,
            AnchorSide::Bottom,
            AnchorAlign::Center,
            tooltip_collision_options(Some(12.)),
        );
        assert_eq!(l.y, t.bottom() + 12.);
    }

    #[test]
    fn beak_side_faces_the_trigger() {
        assert_eq!(tooltip_beak_side(AnchorSide::Top), BeakSide::Bottom);
        assert_eq!(tooltip_beak_side(AnchorSide::Bottom), BeakSide::Top);
        assert_eq!(tooltip_beak_side(AnchorSide::Left), BeakSide::Right);
        assert_eq!(tooltip_beak_side(AnchorSide::Right), BeakSide::Left);
    }
}
