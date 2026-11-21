//! Message — web `Message` / `MessageAvatar` / `MessageContent` /
//! `MessageFooter` (`components/Message/style.ts`): one conversation turn.
//!
//! Row with `gap-small` (8px), items aligned to the bottom; `align(End)`
//! reverses it (avatar on the right). The avatar is pinned to the bottom
//! edge; the content column (`gap-xx-small`, 4px) holds [`crate::Bubble`]s or
//! a [`crate::BubbleGroup`] and lines them up with the sender edge. The
//! footer is `caption2` in `foregroundTertiary` with `px-small`, end-aligned
//! for outgoing turns.

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*, px,
};
use refineui_foundation::{spacing, typography::role};

use crate::{ActiveTheme, Avatar, BubbleAlign, StyledExt};

/// One conversation turn: avatar, bubbles and an optional footer.
#[derive(IntoElement)]
pub struct Message {
    align: BubbleAlign,
    avatar: Option<Avatar>,
    content: Vec<AnyElement>,
    footer: Option<SharedString>,
}

impl Message {
    /// New incoming (start-aligned) message.
    pub fn new() -> Self {
        Self {
            align: BubbleAlign::Start,
            avatar: None,
            content: Vec::new(),
            footer: None,
        }
    }

    /// Sender side (web `align`).
    pub fn align(mut self, align: BubbleAlign) -> Self {
        self.align = align;
        self
    }

    /// Sender avatar (web `MessageAvatar`).
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// Add content — usually a `Bubble` or `BubbleGroup` (web `MessageContent`).
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content.push(e.into_any_element());
        self
    }

    /// Delivery / time line under the bubbles (web `MessageFooter`).
    pub fn footer(mut self, text: impl Into<SharedString>) -> Self {
        self.footer = Some(text.into());
        self
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Message {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = &cx.theme().colors;
        let end = self.align == BubbleAlign::End;
        div()
            .flex()
            .w_full()
            .min_w_0()
            .items_end()
            .when(end, |d| d.flex_row_reverse())
            .gap(px(spacing::SMALL))
            .when_some(self.avatar, |d, a| d.child(div().flex_none().child(a)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(spacing::XX_SMALL))
                    .when(end, |d| d.items_end())
                    .when(!end, |d| d.items_start())
                    .children(self.content)
                    .when_some(self.footer, |d, f| {
                        d.child(
                            div()
                                .px(px(spacing::SMALL))
                                .text_style_token(role::CAPTION_MD)
                                .text_color(c.foreground_tertiary)
                                .when(end, |d| d.text_right())
                                .child(f),
                        )
                    }),
            )
    }
}
