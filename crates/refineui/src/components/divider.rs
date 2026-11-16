//! Divider — web `Divider` (`default | content | icon` layouts, `center | left | right`).
//!
//! 1px (`strokeThin`) `borderDefault` line; content label `captionMd` in
//! `foregroundBrand` with `sizeMedium` gaps; icon layout shows a 13.3px brand
//! ring in a 20px slot. Short ends (left / right alignment) are 8px.

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};
use refineui_tokens::{size::component::DIVIDER_SHORT_END, spacing, typography::role};

use crate::{ActiveTheme, StyledExt};

/// Divider orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Full-width rule (default).
    #[default]
    Horizontal,
    /// Full-height rule (desktop toolbars / split panes).
    Vertical,
}

/// Where the label / icon sits on a horizontal divider (web `align`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DividerAlign {
    /// Centered (default).
    #[default]
    Center,
    /// Short 8px leading segment.
    Left,
    /// Short 8px trailing segment.
    Right,
}

#[derive(Clone)]
enum Content {
    None,
    Label(SharedString),
    Icon,
}

/// A separating rule.
#[derive(IntoElement)]
pub struct Divider {
    axis: Axis,
    content: Content,
    align: DividerAlign,
    strong: bool,
}

impl Divider {
    /// Horizontal divider.
    pub fn horizontal() -> Self {
        Self {
            axis: Axis::Horizontal,
            content: Content::None,
            align: DividerAlign::Center,
            strong: false,
        }
    }

    /// Vertical divider.
    pub fn vertical() -> Self {
        Self {
            axis: Axis::Vertical,
            ..Self::horizontal()
        }
    }

    /// Content layout with a label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.content = Content::Label(label.into());
        self
    }

    /// Icon layout (brand ring glyph).
    pub fn icon(mut self) -> Self {
        self.content = Content::Icon;
        self
    }

    /// Label / icon alignment.
    pub fn align(mut self, align: DividerAlign) -> Self {
        self.align = align;
        self
    }

    /// Use `borderStrong` (desktop addition for pane splitters).
    pub fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = &cx.theme().colors;
        let color = if self.strong {
            c.border_strong
        } else {
            c.border_default
        };
        let thin = px(refineui_tokens::stroke::THIN);
        let segment = |grow: bool| {
            let d = div().h(thin).min_h(thin).flex_none().bg(color);
            if grow {
                d.flex_1().min_w_0()
            } else {
                d.w(px(DIVIDER_SHORT_END))
            }
        };
        let row = |middle: gpui::AnyElement, align: DividerAlign| {
            div()
                .flex()
                .w_full()
                .items_center()
                .justify_center()
                .gap(px(spacing::MEDIUM))
                .overflow_hidden()
                .child(segment(align != DividerAlign::Left))
                .child(middle)
                .child(segment(align != DividerAlign::Right))
        };
        match (self.axis, self.content) {
            (Axis::Vertical, _) => div().flex_none().w(thin).h_full().min_h(px(16.)).bg(color),
            (Axis::Horizontal, Content::None) => div().flex_none().h(thin).w_full().bg(color),
            (Axis::Horizontal, Content::Label(label)) => row(
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .text_style_token(role::CAPTION_MD)
                    .text_color(c.foreground_brand)
                    .child(label)
                    .into_any_element(),
                self.align,
            ),
            (Axis::Horizontal, Content::Icon) => row(
                div()
                    .relative()
                    .flex_none()
                    .size(px(20.))
                    .child(
                        div()
                            .absolute()
                            .left(px(3.))
                            .top(px(3.))
                            .size(px(13.3))
                            .rounded_full()
                            .border_1()
                            .border_color(c.foreground_brand),
                    )
                    .into_any_element(),
                self.align,
            ),
        }
    }
}
