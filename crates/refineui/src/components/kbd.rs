//! Kbd — web `Kbd` (`components/Kbd/style.ts`): keyboard shortcut hint.
//!
//! `caption2`, min 20×20px, `roundedMedium` (6px), thin `borderStrong`,
//! `backgroundSurface`, `px-x-small` (6px), `foregroundSecondary`.

use gpui::{
    App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};

use crate::{ActiveTheme, StyledExt};

/// A keycap label like `⌘K`.
#[derive(IntoElement)]
pub struct Kbd {
    keys: SharedString,
    inverted: bool,
}

impl Kbd {
    /// New keycap.
    pub fn new(keys: impl Into<SharedString>) -> Self {
        Self {
            keys: keys.into(),
            inverted: false,
        }
    }

    /// Use on dark surfaces (tooltips).
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = &cx.theme().colors;
        let (fg, border) = if self.inverted {
            (
                c.foreground_inversed.opacity(0.75),
                c.foreground_inversed.opacity(0.3),
            )
        } else {
            (c.foreground_secondary, c.border_strong)
        };
        div()
            .flex_none()
            .px(px(refineui_tokens::spacing::X_SMALL))
            .min_w(px(20.))
            .min_h(px(20.))
            .whitespace_nowrap()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(refineui_tokens::radius::MEDIUM))
            .when(!self.inverted, |d| d.bg(c.background_surface))
            .border_1()
            .border_color(border)
            .text_style_token(refineui_tokens::typography::role::CAPTION_MD)
            .text_color(fg)
            .child(self.keys)
    }
}
