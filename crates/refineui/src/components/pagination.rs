//! Pagination — web `Pagination`.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{radius, size::component, spacing, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, StyledExt};

/// Page navigator (1-based pages).
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    page: usize,
    total: usize,
    size: ControlSize,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Pagination {
    /// New paginator showing `page` of `total`.
    pub fn new(id: impl Into<ElementId>, page: usize, total: usize) -> Self {
        Self {
            id: id.into(),
            page: page.clamp(1, total.max(1)),
            total: total.max(1),
            size: ControlSize::Small,
            on_change: None,
        }
    }

    /// Button size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Called with the new page.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

/// Page numbers to show (`None` = ellipsis).
pub(crate) fn page_window(page: usize, total: usize) -> Vec<Option<usize>> {
    if total <= 7 {
        return (1..=total).map(Some).collect();
    }
    let mut out = vec![Some(1)];
    let start = page.saturating_sub(1).max(2);
    let end = (page + 1).min(total - 1);
    if start > 2 {
        out.push(None);
    }
    out.extend((start..=end).map(Some));
    if end < total - 1 {
        out.push(None);
    }
    out.push(Some(total));
    out
}

enum Item {
    Page(usize),
    Nav(IconName, usize, bool),
    Ellipsis,
}

/// One 40×40 `roundedXLarge` shell: thin `borderDefault` on `backgroundPrimary`;
/// hover `surfaceHover` + `borderHover`; selected page 2px `borderFocus` on
/// `surfaceSelected`; focus-visible `borderFocus` + 0.5px halo; disabled
/// `surfaceDisabled` / `borderDisabled`. Transitions over `duration-fast`.
fn shell(
    id: ElementId,
    item: Item,
    current: usize,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let size = px(component::PAGINATION_BUTTON);
    let base = div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(size)
        .p(px(spacing::X_SMALL))
        .rounded(px(radius::X_LARGE));
    let (target, disabled, selected) = match item {
        Item::Ellipsis => {
            return base
                .child(
                    Icon::new(IconName::MoreHorizontal)
                        .size(IconSize::Medium)
                        .color(c.foreground_secondary),
                )
                .into_any_element();
        }
        Item::Page(p) => (p, false, p == current),
        Item::Nav(_, t, disabled) => (t, disabled, false),
    };
    let (state, i) = use_interaction(&id, window, cx);
    let focus = use_focus_handle(&id, 0, window, cx);
    let focused = !disabled && focus_visible(cx) && focus.is_focused(window);
    let (bg, border, fg) = if disabled {
        (
            c.background_surface_disabled,
            c.border_disabled,
            c.foreground_disabled,
        )
    } else if selected {
        (
            c.background_surface_selected,
            c.border_focus,
            c.foreground_primary,
        )
    } else if focused {
        (c.background_primary, c.border_focus, c.foreground_primary)
    } else if i.hovered {
        (
            c.background_surface_hover,
            c.border_hover,
            c.foreground_primary,
        )
    } else {
        (c.background_primary, c.border_default, c.foreground_primary)
    };
    let bg = animate(child_id(&id, "bg"), bg, Motion::FAST_EASE, window, cx);
    let border = animate(child_id(&id, "bd"), border, Motion::FAST_EASE, window, cx);
    let width = animate(
        child_id(&id, "bw"),
        if selected {
            refineui_tokens::stroke::THICK
        } else {
            refineui_tokens::stroke::THIN
        },
        Motion::FAST_EASE,
        window,
        cx,
    );
    let content: gpui::AnyElement = match item {
        Item::Page(p) => div()
            .text_style_token(role::BODY_LG)
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(p.to_string())
            .into_any_element(),
        Item::Nav(icon, ..) => Icon::new(icon)
            .size(IconSize::Medium)
            .color(fg)
            .into_any_element(),
        Item::Ellipsis => unreachable!(),
    };
    base.id(id)
        .border(px(width))
        .border_color(border)
        .bg(bg)
        .text_color(fg)
        .when(focused, |d| d.shadow(theme.ring(c.border_focus, 0.5)))
        .when(disabled, |d| {
            d.cursor(gpui::CursorStyle::OperationNotAllowed)
        })
        .when(!disabled, |d| {
            d.cursor_action()
                .track_focus(&focus)
                .track_interaction(&state)
                .when_some(on_change, |d, h| {
                    let hk = h.clone();
                    d.on_click(move |_, w, cx| h(&target, w, cx)).on_key_down(
                        move |e: &gpui::KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                cx.stop_propagation();
                                hk(&target, w, cx)
                            }
                        },
                    )
                })
        })
        .child(content)
        .into_any_element()
}

impl RenderOnce for Pagination {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let cid = |s: String| ElementId::NamedChild(Box::new(id.clone()), s.into());
        let mut items = vec![(
            cid("prev".into()),
            Item::Nav(
                IconName::ChevronLeft,
                self.page.saturating_sub(1).max(1),
                self.page <= 1,
            ),
        )];
        for (i, entry) in page_window(self.page, self.total).into_iter().enumerate() {
            items.push(match entry {
                Some(p) => (cid(format!("page-{p}")), Item::Page(p)),
                None => (cid(format!("gap-{i}")), Item::Ellipsis),
            });
        }
        items.push((
            cid("next".into()),
            Item::Nav(
                IconName::ChevronRight,
                (self.page + 1).min(self.total),
                self.page >= self.total,
            ),
        ));
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(spacing::MEDIUM));
        for (id, item) in items {
            row = row.child(shell(
                id,
                item,
                self.page,
                self.on_change.clone(),
                window,
                cx,
            ));
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::page_window;
    #[test]
    fn windows() {
        assert_eq!(page_window(1, 3), vec![Some(1), Some(2), Some(3)]);
        assert_eq!(
            page_window(5, 10),
            vec![Some(1), None, Some(4), Some(5), Some(6), None, Some(10)]
        );
        assert_eq!(page_window(1, 10), vec![Some(1), Some(2), None, Some(10)]);
    }
}
