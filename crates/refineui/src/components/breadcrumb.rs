//! Breadcrumb — web `Breadcrumb`.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::components::plate::row_plate;
use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, StyledExt};

/// Path trail. The last item is the current page.
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    items: Vec<SharedString>,
    on_select: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Breadcrumb {
    /// New trail.
    pub fn new(
        id: impl Into<ElementId>,
        items: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().map(Into::into).collect(),
            on_select: None,
        }
    }

    /// Called with the index of a clicked ancestor.
    pub fn on_select(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let last = self.items.len().saturating_sub(1);
        let parent = self.id.clone();
        let show_ring = focus_visible(cx);
        let mut children = Vec::new();
        for (ix, label) in self.items.into_iter().enumerate() {
            if ix > 0 {
                // Web default separator: `/` in `foregroundTertiary`.
                children.push(
                    div()
                        .flex_none()
                        .text_color(c.foreground_tertiary)
                        .child("/")
                        .into_any_element(),
                );
            }
            if ix == last {
                children.push(
                    div()
                        .flex()
                        .items_center()
                        .px(px(spacing::MEDIUM))
                        .py(px(spacing::X_SMALL))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(c.foreground_primary)
                        .child(label)
                        .into_any_element(),
                );
                continue;
            }
            let id = ElementId::NamedChild(Box::new(parent.clone()), format!("crumb-{ix}").into());
            let (state, i) = use_interaction(&id, window, cx);
            let focus = use_focus_handle(&id, 0, window, cx);
            let (bg, fg) = row_plate(i, false, false, c.foreground_tertiary, cx);
            let bg = animate(child_id(&id, "bg"), bg, Motion::FAST_EASE, window, cx);
            let fg = animate(child_id(&id, "fg"), fg, Motion::FAST_EASE, window, cx);
            let focused = show_ring && focus.is_focused(window);
            let handler = self.on_select.clone();
            let key_handler = self.on_select.clone();
            children.push(
                div()
                    .id(id)
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_h(px(theme.button_metrics(crate::ControlSize::Small).height))
                    .px(px(spacing::MEDIUM))
                    .py(px(spacing::X_SMALL))
                    .rounded(px(radius::LARGE))
                    .bg(bg)
                    .text_color(fg)
                    .cursor_action()
                    .track_focus(&focus)
                    .track_interaction(&state)
                    .when(focused, |d| d.shadow(theme.focus_ring()))
                    .child(label)
                    .when_some(handler, |d, h| d.on_click(move |_, w, cx| h(&ix, w, cx)))
                    .when_some(key_handler, |d, h| {
                        d.on_key_down(move |e: &gpui::KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                cx.stop_propagation();
                                h(&ix, w, cx)
                            }
                        })
                    })
                    .into_any_element(),
            );
        }
        div()
            .id(self.id)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(spacing::MEDIUM))
            .py(px(spacing::X_SMALL))
            .text_style_token(role::CAPTION_LG)
            .whitespace_nowrap()
            .children(children)
    }
}
