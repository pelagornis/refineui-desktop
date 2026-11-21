//! Link — web `Link`. Opens a URL with the OS handler or runs a callback.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, ClickHandler, Icon, IconName, IconSize, StyledExt};

/// Inline text link.
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    label: SharedString,
    href: Option<SharedString>,
    external: bool,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl Link {
    /// New link.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            href: None,
            external: false,
            disabled: false,
            on_click: None,
        }
    }

    /// URL opened with `cx.open_url` when clicked.
    pub fn href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Show the "opens externally" glyph.
    pub fn external(mut self, external: bool) -> Self {
        self.external = external;
        self
    }

    /// Custom click handler (instead of / in addition to `href`).
    /// Disabled (`foregroundDisabled`, no underline, not clickable).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Click handler (runs before `href` opens).
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Link {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let (state, i) = use_interaction(&self.id, window, cx);
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus_visible(cx) && focus.is_focused(window);
        // `refineui.css`: rest link, hover linkHover + underline, active
        // linkVisited + underline, focus-visible linkActive + underline.
        let fg = if self.disabled {
            c.foreground_disabled
        } else if i.pressed {
            c.foreground_link_visited
        } else if i.hovered {
            c.foreground_link_hover
        } else if focused {
            c.foreground_link_active
        } else {
            c.foreground_link
        };
        let fg = animate(child_id(&self.id, "fg"), fg, Motion::FAST_EASE, window, cx);
        let underline = !self.disabled && (i.hovered || i.pressed || focused);
        let href = self.href.clone();
        let handler = self.on_click.clone();
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(spacing::XX_SMALL))
            .text_style_token(role::BODY_LG)
            .text_color(fg)
            .rounded(px(radius::SMALL))
            .when(underline, |d| d.underline())
            .when(focused, |d| d.shadow(theme.focus_ring()))
            .when(self.disabled, |d| {
                d.cursor(gpui::CursorStyle::OperationNotAllowed)
            })
            .when(!self.disabled, |d| {
                d.cursor_pointer()
                    .track_focus(&focus)
                    .track_interaction(&state)
                    .on_click(move |e, w, cx| {
                        if let Some(h) = &handler {
                            h(e, w, cx);
                        }
                        if let Some(url) = &href {
                            cx.open_url(url);
                        }
                    })
            })
            .child(self.label)
            .when(self.external, |this| {
                this.child(Icon::new(IconName::Open).size(IconSize::XSmall).color(fg))
            })
    }
}
