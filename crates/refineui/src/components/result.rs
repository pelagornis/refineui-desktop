//! Result — web `Result` (`components/Result/style.ts`): the outcome page of
//! a flow (success / info / warning / error).
//!
//! Named `ResultView` in Rust so it does not shadow `std::result::Result`
//! under `use refineui::prelude::*`.
//!
//! Centered column, `gap-large` (16px), `px-xx-large` / `py-xxx-large`
//! (24 / 32px). Figure: 48px box with a 28px (`iconSizes.large`) regular icon
//! in the variant's foreground (`checkmark`, `info`, `warning`,
//! `error-circle`). Title `body1` semibold primary, description `body2`
//! secondary (max 320px wide, `gap-small`). Actions stack full-width buttons
//! (max 200px, `gap-small`).

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*, px,
};
use refineui_tokens::{spacing, typography::role};

use crate::{ActiveTheme, Button, Icon, IconName, IconSize, StyledExt};

/// Outcome tone (web `ResultVariant`, default `Info`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResultVariant {
    /// Checkmark in `foregroundSuccess`.
    Success,
    /// Info glyph in `foregroundInfo` (default).
    #[default]
    Info,
    /// Warning glyph in `foregroundWarning`.
    Warning,
    /// Error circle in `foregroundError`.
    Error,
}

impl ResultVariant {
    /// All variants.
    pub const ALL: [ResultVariant; 4] = [Self::Success, Self::Info, Self::Warning, Self::Error];

    /// Default icon (web `variantIcons`).
    pub fn icon(self) -> IconName {
        match self {
            Self::Success => IconName::Checkmark,
            Self::Info => IconName::Info,
            Self::Warning => IconName::Warning,
            Self::Error => IconName::ErrorCircle,
        }
    }
}

/// Outcome panel: figure, title, description and actions.
#[derive(IntoElement)]
pub struct ResultView {
    variant: ResultVariant,
    icon: Option<IconName>,
    figure: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    actions: Vec<Button>,
}

impl ResultView {
    /// New result of a variant.
    pub fn new(variant: ResultVariant) -> Self {
        Self {
            variant,
            icon: None,
            figure: None,
            title: None,
            description: None,
            actions: Vec::new(),
        }
    }

    /// Heading.
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Supporting text (newlines are kept, web `whitespace-pre-line`).
    pub fn description(mut self, t: impl Into<SharedString>) -> Self {
        self.description = Some(t.into());
        self
    }

    /// Replace the variant icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Replace the figure with any element (illustration, avatar…).
    pub fn figure(mut self, figure: impl IntoElement) -> Self {
        self.figure = Some(figure.into_any_element());
        self
    }

    /// Add a full-width action button (web `ResultButton`).
    pub fn action(mut self, button: Button) -> Self {
        self.actions.push(button);
        self
    }
}

impl RenderOnce for ResultView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = &cx.theme().colors;
        let tone = match self.variant {
            ResultVariant::Success => c.foreground_success,
            ResultVariant::Info => c.foreground_info,
            ResultVariant::Warning => c.foreground_warning,
            ResultVariant::Error => c.foreground_error,
        };
        let icon = self.icon.unwrap_or(self.variant.icon());
        div()
            .flex()
            .flex_col()
            .w_full()
            .items_center()
            .gap(px(spacing::LARGE))
            .px(px(spacing::XX_LARGE))
            .py(px(spacing::XXX_LARGE))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .size(px(48.))
                    .items_center()
                    .justify_center()
                    .text_color(tone)
                    .map(|d| match self.figure {
                        Some(f) => d.child(f),
                        None => d.child(Icon::new(icon).size(IconSize::Large).color(tone)),
                    }),
            )
            .when(self.title.is_some() || self.description.is_some(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .max_w(px(320.))
                        .gap(px(spacing::SMALL))
                        .text_center()
                        .when_some(self.title, |d, t| {
                            d.child(
                                div()
                                    .text_style_token(role::BODY_LG)
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(c.foreground_primary)
                                    .child(t),
                            )
                        })
                        .when_some(self.description, |d, t| {
                            d.child(
                                div()
                                    .text_style_token(role::BODY_MD)
                                    .text_color(c.foreground_secondary)
                                    .children(t.split('\n').map(|l| div().child(l.to_string()))),
                            )
                        }),
                )
            })
            .when(!self.actions.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .w_full()
                        .max_w(px(200.))
                        .gap(px(spacing::SMALL))
                        .children(self.actions.into_iter().map(|b| b.full_width(true))),
                )
            })
    }
}
