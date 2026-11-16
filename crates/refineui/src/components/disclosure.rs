//! Shared expand / collapse panel (web `Accordion`, `Collapsible`, `Tree`).
//!
//! Web animates `grid-template-rows: 0fr → 1fr` over `accordionPanel` (380ms,
//! panel curve) while the inner content fades and slides from
//! `translateY(-sizeXSmall)` over `accordionContent` (260ms, content curve).
//! GPUI has no `fr` rows, so the content height is measured with a
//! [`LayoutProbe`] and the clip height is tweened instead.

use gpui::{AnyElement, App, ElementId, IntoElement, ParentElement, Styled, Window, div, px};
use refineui_tokens::spacing;

use crate::motion::{Motion, animate, child_id, probe, use_layout_probe};

/// Render `content` (built only while visible) inside an animated clip.
pub(crate) fn collapse_panel(
    id: &ElementId,
    open: bool,
    window: &mut Window,
    cx: &mut App,
    content: impl FnOnce(&mut Window, &mut App) -> AnyElement,
) -> Option<AnyElement> {
    let height_t = animate(
        child_id(id, "h"),
        if open { 1.0f32 } else { 0.0 },
        Motion::ACCORDION_PANEL,
        window,
        cx,
    );
    let content_t = animate(
        child_id(id, "c"),
        if open { 1.0f32 } else { 0.0 },
        Motion::ACCORDION_CONTENT,
        window,
        cx,
    );
    if !open && height_t <= 0.001 {
        return None;
    }
    let measure = use_layout_probe(&child_id(id, "probe"), window, cx);
    let measured = measure.read(cx).container.map(|b| b.size.height);
    let inner = div()
        .relative()
        .flex_none()
        .w_full()
        .opacity(content_t)
        .mt(px(-(1. - content_t) * spacing::X_SMALL))
        .child(content(window, cx))
        .child(probe(&measure, None));
    let settled_open = open && height_t >= 0.999;
    Some(
        div()
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .when_some(measured.filter(|_| !settled_open), |d, h| d.h(h * height_t))
            .when(measured.is_none() && !open, |d| d.h(px(0.)))
            .child(inner)
            .into_any_element(),
    )
}

use gpui::prelude::FluentBuilder as _;
