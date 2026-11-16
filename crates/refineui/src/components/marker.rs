//! Marker — web `Marker` (`components/Marker/style.ts`): a quiet inline
//! annotation in a conversation ("Today", "Alex joined", "Thinking…").
//!
//! Row with `gap-small` (8px) in `foregroundTertiary`; optional 16px icon in
//! `foregroundSecondary`; text `caption1`. The `Divider` variant draws a 1px
//! `borderDefault` rule on both sides (`gap-medium`, 10px). `status(true)`
//! (web `role="status"`) pulses the text like an empty Input OTP slot
//! (`duration-skeleton`, ease-in-out); static under reduced motion.

use gpui::{
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{spacing, stroke, typography::role};

use crate::motion::{reduced_motion, skeleton_pulse_opacity};
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// Marker layout (web `MarkerVariant`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MarkerVariant {
    /// Icon + text.
    #[default]
    Default,
    /// Text centered between two rules.
    Divider,
}

/// An inline conversation annotation.
#[derive(IntoElement)]
pub struct Marker {
    id: ElementId,
    text: SharedString,
    icon: Option<IconName>,
    variant: MarkerVariant,
    status: bool,
}

impl Marker {
    /// New marker.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            icon: None,
            variant: MarkerVariant::Default,
            status: false,
        }
    }

    /// Leading icon (web `MarkerIcon`).
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Layout variant.
    pub fn variant(mut self, variant: MarkerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Live status text (web `role="status"`): pulses.
    pub fn status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }
}

impl RenderOnce for Marker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = &cx.theme().colors;
        let pulse = self.status && !reduced_motion(cx);
        let text = div()
            .id(self.id.clone())
            .flex_none()
            .whitespace_nowrap()
            .text_style_token(role::CAPTION_LG)
            .child(self.text);
        let text = if pulse {
            text.with_animation(
                self.id,
                Animation::new(std::time::Duration::from_millis(
                    refineui_tokens::motion::duration::SKELETON,
                ))
                .repeat(),
                |d, t| d.opacity(skeleton_pulse_opacity(t)),
            )
            .into_any_element()
        } else {
            text.into_any_element()
        };
        let rule = || {
            div()
                .flex_1()
                .min_w_0()
                .h(px(stroke::THIN))
                .bg(c.border_default)
        };
        div()
            .flex()
            .min_w_0()
            .w_full()
            .items_center()
            .gap(px(spacing::SMALL))
            .text_color(c.foreground_tertiary)
            .when_some(self.icon, |d, icon| {
                d.child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .color(c.foreground_secondary),
                )
            })
            .map(|d| match self.variant {
                MarkerVariant::Default => d.child(text),
                MarkerVariant::Divider => d.child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap(px(spacing::MEDIUM))
                        .child(rule())
                        .child(text)
                        .child(rule()),
                ),
            })
    }
}
