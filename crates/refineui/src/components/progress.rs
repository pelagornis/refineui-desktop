//! Progress (bar) — web `Progress` (`sm | lg`).
//!
//! `brandSubtle` circular track (2px / 4px), `brand` fill whose width
//! transitions over `duration-panel` with ease-out. The step indicator lives in
//! [`crate::ProgressStepper`].

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, px, relative,
};
use refineui_tokens::{motion::easing::EASE_IN_OUT, size::component, spacing, typography::role};

use crate::motion::{Motion, animate_from, child_id};
use crate::{ActiveTheme, StyledExt};

/// Linear progress bar.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    value: Option<f32>,
    large: bool,
    label: Option<SharedString>,
    show_value: bool,
}

impl Progress {
    /// Determinate bar, `value` in `0..=1`.
    pub fn new(id: impl Into<ElementId>, value: f32) -> Self {
        Self {
            id: id.into(),
            value: Some(value.clamp(0., 1.)),
            large: false,
            label: None,
            show_value: false,
        }
    }

    /// Indeterminate bar (desktop addition: a brand segment sweeps the track).
    pub fn indeterminate(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            large: false,
            label: None,
            show_value: false,
        }
    }

    /// `lg` track (4px) instead of `sm` (2px).
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Caption above the bar (desktop addition).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show the percentage next to the label.
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }
}

impl RenderOnce for Progress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let h = if self.large {
            component::PROGRESS_TRACK_LG
        } else {
            component::PROGRESS_TRACK_SM
        };
        let track = div()
            .relative()
            .w_full()
            .min_w_0()
            .h(px(h))
            .rounded_full()
            .bg(c.background_brand_subtle)
            .overflow_hidden();
        let bar = match self.value {
            Some(v) => {
                let w = animate_from(
                    child_id(&self.id, "w"),
                    0.0f32,
                    v,
                    Motion::PROGRESS,
                    window,
                    cx,
                );
                track
                    .child(
                        div()
                            .h_full()
                            .w(relative(w))
                            .rounded_full()
                            .bg(c.background_brand),
                    )
                    .into_any_element()
            }
            None => track
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .h_full()
                        .w(relative(0.35))
                        .rounded_full()
                        .bg(c.background_brand)
                        .with_animation(
                            child_id(&self.id, "sweep"),
                            Animation::new(Duration::from_millis(1400))
                                .repeat()
                                .with_easing(|t| EASE_IN_OUT.ease(t)),
                            |d, t| d.left(relative(t * 1.35 - 0.35)),
                        ),
                )
                .into_any_element(),
        };
        let percent = self
            .value
            .map(|v| format!("{}%", (v * 100.).round() as i32));
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::X_SMALL))
            .w_full()
            .when(self.label.is_some() || self.show_value, |d| {
                d.child(
                    div()
                        .flex()
                        .justify_between()
                        .text_style_token(role::CAPTION_LG)
                        .text_color(c.foreground_secondary)
                        .child(self.label.unwrap_or_default())
                        .when(self.show_value, |d| d.children(percent)),
                )
            })
            .child(bar)
    }
}
