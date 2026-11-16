//! ScrollArea — web `ScrollArea` (vertical scroll with an overlay scrollbar).

use std::cell::RefCell;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, MouseMoveEvent,
    ParentElement, Pixels, RenderOnce, ScrollHandle, StatefulInteractiveElement, Styled, Window,
    canvas, div, point, prelude::*, px,
};
use refineui_tokens::spacing;
use smallvec::SmallVec;

use crate::ActiveTheme;
use crate::motion::{Motion, TrackInteraction, animate, child_id, use_interaction};

/// Scrollable region with a thin themed overlay scrollbar.
///
/// Web `ScrollArea`: the 10px rail floats over the content (inset 2px) and
/// shows while hovered, dragging or scrolling (fading over `duration-fast`).
///
/// Edge softening (web `data-scroll-edge-item` + `refineui-scroll-edge-falloff`):
/// rows wrapped in [`ScrollEdgeItem`] soften as they cross the outer 25% of
/// the viewport on an edge that **hides more content**. Like the web, the
/// amount is per row (from the row's `view()` cover progress: 1 at 0% / 100%,
/// 0 at 25% / 75%), scaled by that edge's factor, which ramps 0 → 1 over the
/// first 25% of scroll distance, so a list parked at its top is untouched at
/// the top and lists that do not scroll are never touched. Rows never shift
/// or shrink. GPUI has no `filter: blur`, so the row's `blur(2px)` is
/// approximated by fading the whole row (no colored overlay).
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    children: SmallVec<[AnyElement; 4]>,
    max_height: Option<Pixels>,
    edge_blur: bool,
    style: Option<crate::platform::ScrollbarStyle>,
}

impl ScrollArea {
    /// New scroll area (fills its parent's height unless [`Self::max_height`] is set).
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: SmallVec::new(),
            max_height: None,
            edge_blur: true,
            style: None,
        }
    }

    /// Scrollbar behaviour; defaults to the platform's
    /// ([`crate::platform::PlatformAdapter::scrollbar`]: overlay on macOS /
    /// Linux, persistent on Windows).
    pub fn scrollbar(mut self, style: crate::platform::ScrollbarStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Edge softening of [`ScrollEdgeItem`] rows on / off (default on).
    pub fn edge_blur(mut self, on: bool) -> Self {
        self.edge_blur = on;
        self
    }

    /// Cap the height.
    pub fn max_height(mut self, h: Pixels) -> Self {
        self.max_height = Some(h);
        self
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

#[derive(Default, Clone, Copy)]
struct Drag {
    start_y: Option<(Pixels, Pixels)>,
}

/// Last seen scroll offset and when it changed (web `data-scrolling`).
#[derive(Clone, Copy)]
struct Scrolling {
    offset: Pixels,
    changed: Option<Instant>,
}

/// How long the rail stays up after the last scroll.
const SCROLLING_LINGER: Duration = Duration::from_millis(600);
/// Web `SCROLL_EDGE_FALLOFF_RATIO`.
const EDGE_FALLOFF: f32 = 0.25;
/// Opacity lost at full edge amount — stands in for `blur(scrollEdge)` (2px),
/// which costs a row roughly this much contrast.
const EDGE_FADE: f32 = 0.5;

/// What a [`ScrollEdgeItem`] needs from its enclosing [`ScrollArea`].
#[derive(Clone)]
struct EdgeCtx {
    handle: ScrollHandle,
    top: f32,
    bottom: f32,
}

thread_local! {
    static EDGE: RefCell<Vec<EdgeCtx>> = const { RefCell::new(Vec::new()) };
}

fn current_edge() -> Option<EdgeCtx> {
    EDGE.with(|e| e.borrow().last().cloned())
}

/// Web edge amount for a row at `content_top` (content coordinates) of
/// `height`, in a viewport of `vh` scrolled by `offset`: the row's `view()`
/// cover progress maps 0% → 1, 25% → 0 (bottom edge) and 75% → 0, 100% → 1
/// (top edge), each scaled by that edge's factor.
fn edge_amount(content_top: f32, height: f32, offset: f32, vh: f32, top: f32, bottom: f32) -> f32 {
    let span = vh + height;
    if span <= 0. {
        return 0.;
    }
    let row_top = content_top - offset;
    let cover = ((vh - row_top) / span).clamp(0., 1.);
    let end = (1. - cover / EDGE_FALLOFF).clamp(0., 1.);
    let start = ((cover - (1. - EDGE_FALLOFF)) / EDGE_FALLOFF).clamp(0., 1.);
    (end * bottom).max(start * top)
}

/// Pushes the area's [`EdgeCtx`] while its content lays out and prepaints, so
/// nested [`ScrollEdgeItem`]s (at any depth) can find it.
struct EdgeScope {
    ctx: EdgeCtx,
    child: AnyElement,
}

impl IntoElement for EdgeScope {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for EdgeScope {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, ()) {
        EDGE.with(|e| e.borrow_mut().push(self.ctx.clone()));
        let id = self.child.request_layout(window, cx);
        EDGE.with(|e| e.borrow_mut().pop());
        (id, ())
    }
    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        EDGE.with(|e| e.borrow_mut().push(self.ctx.clone()));
        self.child.prepaint(window, cx);
        EDGE.with(|e| e.borrow_mut().pop());
    }
    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

/// A row that softens at a [`ScrollArea`] edge (web `data-scroll-edge-item`).
/// Put it anywhere inside a scroll area's content; outside one it is a plain
/// wrapper.
#[derive(IntoElement)]
pub struct ScrollEdgeItem {
    id: ElementId,
    child: AnyElement,
}

impl ScrollEdgeItem {
    /// Wrap `child`; `id` must be unique within the scroll area.
    pub fn new(id: impl Into<ElementId>, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            child: child.into_any_element(),
        }
    }
}

/// Shorthand for [`ScrollEdgeItem::new`].
pub fn scroll_edge_item(id: impl Into<ElementId>, child: impl IntoElement) -> ScrollEdgeItem {
    ScrollEdgeItem::new(id, child)
}

impl RenderOnce for ScrollEdgeItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ctx = current_edge();
        // Row position in content coordinates (`top`, `height`), measured last frame.
        let pos =
            window.use_keyed_state(
                child_id(&self.id, "edge-pos"),
                cx,
                |_, _| None::<(f32, f32)>,
            );
        let amount = match (&ctx, *pos.read(cx)) {
            (Some(ctx), Some((top, h))) if ctx.top > 0.001 || ctx.bottom > 0.001 => edge_amount(
                top,
                h,
                -f32::from(ctx.handle.offset().y),
                f32::from(ctx.handle.bounds().size.height),
                ctx.top,
                ctx.bottom,
            ),
            _ => 0.,
        };
        div()
            .relative()
            .opacity(1. - EDGE_FADE * amount)
            .child(self.child)
            .when(ctx.is_some(), |d| {
                d.child(
                    canvas(
                        move |bounds, window, cx| {
                            let Some(ctx) = current_edge() else { return };
                            let vp = ctx.handle.bounds();
                            let top = f32::from(bounds.top() - vp.top() - ctx.handle.offset().y);
                            let next = Some((top, f32::from(bounds.size.height)));
                            if *pos.read(cx) != next {
                                pos.update(cx, |p, _| *p = next);
                                // `refresh` is a no-op while drawing.
                                window.request_animation_frame();
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            })
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window
            .use_keyed_state(child_id(&self.id, "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let drag = window.use_keyed_state(child_id(&self.id, "drag"), cx, |_, _| Drag::default());
        let dragging = drag.read(cx).start_y.is_some();
        let c = cx.theme().colors;
        let (root_state, root_i) = use_interaction(&self.id, window, cx);
        let thumb_id = child_id(&self.id, "thumb");
        let (thumb_state, thumb_i) = use_interaction(&thumb_id, window, cx);

        // Web `ScrollArea` (type `hover`): 10px overlay rail inset 2px, track
        // `surfaceSelected`, thumb `borderStrong` (hover / drag
        // `foregroundSecondary`), min thumb 20px; fades over `duration-fast`.
        let inset = spacing::XXX_SMALL;
        let viewport = handle.bounds().size.height;
        let rail = viewport - px(2. * inset);
        let max = handle.max_offset().height;
        let offset = -handle.offset().y;
        let thumb = if max > px(0.) && rail > px(0.) {
            let content = viewport + max;
            let h = (rail * (viewport / content)).max(px(spacing::X_LARGE));
            let top = (rail - h) * (offset / max).clamp(0., 1.);
            Some((top, h))
        } else {
            None
        };
        let now = Instant::now();
        let scrolling_state =
            window.use_keyed_state(child_id(&self.id, "scrolling"), cx, |_, _| Scrolling {
                offset: px(0.),
                changed: None,
            });
        let last = *scrolling_state.read(cx);
        if last.offset != offset {
            scrolling_state.update(cx, |s, _| {
                s.offset = offset;
                s.changed = Some(now);
            });
        }
        let scrolling = scrolling_state
            .read(cx)
            .changed
            .is_some_and(|t| now.duration_since(t) < SCROLLING_LINGER);
        if scrolling {
            window.request_animation_frame();
        }
        // Edge factors (web `resolveEdgeFactors`).
        let zone = f32::from(viewport) * EDGE_FALLOFF;
        let (edge_top, edge_bottom) = if self.edge_blur && max > px(0.) && zone > 0. {
            (
                (f32::from(offset) / zone).clamp(0., 1.),
                (f32::from(max - offset) / zone).clamp(0., 1.),
            )
        } else {
            (0., 0.)
        };
        let visible = animate(
            child_id(&self.id, "vis"),
            if root_i.hovered
                || dragging
                || scrolling
                || self
                    .style
                    .unwrap_or_else(|| crate::platform::active(cx).scrollbar())
                    == crate::platform::ScrollbarStyle::Persistent
            {
                1.0f32
            } else {
                0.0
            },
            Motion::FAST,
            window,
            cx,
        );
        let thumb_bg = animate(
            child_id(&self.id, "tbg"),
            if thumb_i.hovered || dragging {
                c.foreground_secondary
            } else {
                c.border_strong
            },
            Motion::FAST,
            window,
            cx,
        );
        let inner = child_id(&self.id, "viewport");
        let move_drag = drag.clone();
        let move_handle = handle.clone();
        let up_drag = drag.clone();
        let up_drag2 = drag.clone();
        div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .when_some(self.max_height, |d, h| d.max_h(h))
            .when(self.max_height.is_none(), |d| d.h_full())
            .track_interaction(&root_state)
            .on_mouse_move(move |e: &MouseMoveEvent, window, cx| {
                let Some((start_y, start_off)) = move_drag.read(cx).start_y else {
                    return;
                };
                let viewport = move_handle.bounds().size.height;
                let max = move_handle.max_offset().height;
                let rail = viewport - px(2. * inset);
                let content = viewport + max;
                let h = (rail * (viewport / content)).max(px(spacing::X_LARGE));
                let travel = rail - h;
                if travel <= px(0.) {
                    return;
                }
                let next =
                    (start_off + max * ((e.position.y - start_y) / travel)).clamp(px(0.), max);
                move_handle.set_offset(point(px(0.), -next));
                window.refresh();
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                up_drag.update(cx, |d, _| d.start_y = None);
                window.refresh();
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                up_drag2.update(cx, |d, _| d.start_y = None);
                window.refresh();
            })
            .child(
                div()
                    .id(inner)
                    .size_full()
                    .when_some(self.max_height, |d, h| d.max_h(h))
                    .overflow_y_scroll()
                    .track_scroll(&handle)
                    .child(EdgeScope {
                        ctx: EdgeCtx {
                            handle: handle.clone(),
                            top: edge_top,
                            bottom: edge_bottom,
                        },
                        child: div().w_full().children(self.children).into_any_element(),
                    }),
            )
            .when_some(thumb.filter(|_| visible > 0.001), |d, (top, h)| {
                let down_handle = handle.clone();
                d.child(
                    div()
                        .absolute()
                        .top(px(inset))
                        .bottom(px(inset))
                        .right(px(inset))
                        .w(px(spacing::MEDIUM))
                        .rounded_full()
                        .overflow_hidden()
                        .bg(c.background_surface_selected)
                        .opacity(visible)
                        .child(
                            div()
                                .id(thumb_id)
                                .absolute()
                                .left_0()
                                .w_full()
                                .top(top)
                                .h(h)
                                .rounded_full()
                                .bg(thumb_bg)
                                .track_interaction(&thumb_state)
                                .on_mouse_down(MouseButton::Left, move |e, window, cx| {
                                    cx.stop_propagation();
                                    let off = -down_handle.offset().y;
                                    drag.update(cx, |d, _| d.start_y = Some((e.position.y, off)));
                                    window.refresh();
                                }),
                        ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::edge_amount;

    #[test]
    fn edge_amount_follows_web_cover_range() {
        let (vh, h) = (200., 40.);
        // Row just entering at the bottom edge (cover 0%) → full amount.
        assert!((edge_amount(200., h, 0., vh, 0., 1.) - 1.).abs() < 1e-4);
        // Row in the middle → untouched.
        assert_eq!(edge_amount(80., h, 0., vh, 1., 1.), 0.);
        // Bottom edge factor 0 (nothing hidden below) → untouched.
        assert_eq!(edge_amount(200., h, 0., vh, 0., 0.), 0.);
        // Row leaving at the top (cover 100%) scaled by the top factor.
        assert!((edge_amount(0., h, 40., vh, 0.5, 0.) - 0.5).abs() < 1e-4);
    }
}
