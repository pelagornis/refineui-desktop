//! Result — web `Result` (`components/Result/style.ts`): the outcome of a
//! flow (success / info / warning / error) and, with [`ResultVariant::Empty`],
//! the placeholder a pane shows when it has nothing yet.
//!
//! Named `ResultView` in Rust so it does not shadow `std::result::Result`
//! under `use refineui::prelude::*`.
//!
//! Centered column, `gap-large` (16px), `px-xx-large` / `py-xxx-large`
//! (24 / 32px). Figure: 48px box with a 28px (`iconSizes.large`) regular icon
//! in the variant's foreground (`checkmark`, `info`, `warning`,
//! `error-circle`). `Empty` uses a secondary `folder` glyph on a subtle 48px
//! plate. Title `body1` semibold primary, description `body2` secondary
//! (max 320px wide, `gap-small`). Actions stack full-width buttons (max
//! 200px, `gap-small`).
//!
//! `compact(true)` tightens spacing, shrinks the typography, drops the plate
//! and lays actions in a horizontal row — for narrow panels and lists.
//! `details` adds an optional monospace well under the description.

use gpui::{
    App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

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
    /// Neutral empty state: folder glyph in `foregroundSecondary`, with a
    /// subtle 48px plate unless [`ResultView::compact`] or overridden by
    /// [`ResultView::plate`].
    Empty,
}

impl ResultVariant {
    /// All variants.
    pub const ALL: [ResultVariant; 5] = [
        Self::Success,
        Self::Info,
        Self::Warning,
        Self::Error,
        Self::Empty,
    ];

    /// Default icon (web `variantIcons`, plus a folder for `Empty`).
    pub fn icon(self) -> IconName {
        match self {
            Self::Success => IconName::Checkmark,
            Self::Info => IconName::Info,
            Self::Warning => IconName::Warning,
            Self::Error => IconName::ErrorCircle,
            Self::Empty => IconName::Folder,
        }
    }
}

/// Outcome panel: figure, title, description, optional details and actions.
#[derive(IntoElement)]
pub struct ResultView {
    variant: ResultVariant,
    icon: Option<IconName>,
    figure: Option<gpui::AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    details: Option<SharedString>,
    actions: Vec<Button>,
    compact: bool,
    plate: Option<bool>,
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
            details: None,
            actions: Vec::new(),
            compact: false,
            plate: None,
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

    /// Technical details (error code, path), shown in a monospace well.
    pub fn details(mut self, t: impl Into<SharedString>) -> Self {
        self.details = Some(t.into());
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

    /// Add an action button (web `ResultButton`). Use this for retry too:
    /// `action(Button::new("retry", "Try Again"))`.
    pub fn action(mut self, button: Button) -> Self {
        self.actions.push(button);
        self
    }

    /// Smaller layout for narrow panels and list placeholders: tighter
    /// padding, a smaller glyph and title, no plate, and a horizontal
    /// action row.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Show or hide the 48px rounded plate behind the icon. Defaults to
    /// on for [`ResultVariant::Empty`] when not compact, off for the web
    /// `Result` variants (so Success / Info / Warning / Error stay
    /// faithful to the web kit).
    pub fn plate(mut self, plate: bool) -> Self {
        self.plate = Some(plate);
        self
    }

    fn show_plate(&self) -> bool {
        self.plate
            .unwrap_or(self.variant == ResultVariant::Empty && !self.compact)
    }
}

impl RenderOnce for ResultView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let c = &theme.colors;
        let tone = match self.variant {
            ResultVariant::Success => c.foreground_success,
            ResultVariant::Info => c.foreground_info,
            ResultVariant::Warning => c.foreground_warning,
            ResultVariant::Error => c.foreground_error,
            ResultVariant::Empty => c.foreground_secondary,
        };
        let compact = self.compact;
        let icon = self.icon.unwrap_or(self.variant.icon());
        let show_plate = self.show_plate();
        let plate_fill = match self.variant {
            ResultVariant::Error => {
                if theme.is_dark() {
                    c.foreground_error.opacity(0.16)
                } else {
                    c.background_error_subtle
                }
            }
            ResultVariant::Success => c.background_success_subtle,
            ResultVariant::Warning => c.background_warning_subtle,
            ResultVariant::Info => c.background_info_subtle,
            ResultVariant::Empty => c.background_surface_hover,
        };
        let icon_size = if compact {
            IconSize::Small
        } else if show_plate {
            IconSize::Medium
        } else {
            IconSize::Large
        };
        let glyph = Icon::new(icon).size(icon_size).color(tone);
        let figure = match self.figure {
            Some(f) => div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .child(f),
            None if show_plate => div()
                .flex()
                .flex_none()
                .size(px(48.))
                .items_center()
                .justify_center()
                .rounded(px(radius::X_LARGE))
                .bg(plate_fill)
                .child(glyph),
            None => div()
                .flex()
                .flex_none()
                .size(px(48.))
                .items_center()
                .justify_center()
                .text_color(tone)
                .child(glyph),
        };
        let max_copy = if compact || self.variant == ResultVariant::Empty || self.details.is_some()
        {
            360.
        } else {
            320.
        };
        div()
            .flex()
            .flex_col()
            .w_full()
            .items_center()
            .justify_center()
            .text_center()
            .gap(px(if compact {
                spacing::X_SMALL
            } else {
                spacing::LARGE
            }))
            .px(px(if compact {
                spacing::MEDIUM
            } else {
                spacing::XX_LARGE
            }))
            .py(px(if compact {
                spacing::LARGE
            } else {
                spacing::XXX_LARGE
            }))
            .child(figure)
            .when(self.title.is_some() || self.description.is_some(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .max_w(px(max_copy))
                        .gap(px(spacing::SMALL))
                        .text_center()
                        .when_some(self.title, |d, t| {
                            d.child(
                                div()
                                    .text_style_token(if compact {
                                        role::BODY_MD
                                    } else {
                                        role::BODY_LG
                                    })
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(c.foreground_primary)
                                    .child(t),
                            )
                        })
                        .when_some(self.description, |d, t| {
                            d.child(
                                div()
                                    .text_style_token(if compact {
                                        role::BODY_SM
                                    } else {
                                        role::BODY_MD
                                    })
                                    .text_color(c.foreground_secondary)
                                    .children(t.split('\n').map(|l| div().child(l.to_string()))),
                            )
                        }),
                )
            })
            .when_some(self.details, |d, t| {
                d.child(
                    div()
                        .max_w(px(max_copy))
                        .px(px(spacing::SMALL))
                        .py(px(spacing::X_SMALL))
                        .rounded(px(radius::MEDIUM))
                        .bg(c.background_surface_hover)
                        .font_family(theme.mono_font_family.clone())
                        .text_style_token(role::CAPTION_MD)
                        .text_color(c.foreground_secondary)
                        .child(t),
                )
            })
            .when(!self.actions.is_empty(), |d| {
                d.child(if compact {
                    div()
                        .pt(px(spacing::XX_SMALL))
                        .flex()
                        .gap(px(spacing::X_SMALL))
                        .children(self.actions)
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .w_full()
                        .max_w(px(200.))
                        .gap(px(spacing::SMALL))
                        .children(self.actions.into_iter().map(|b| b.full_width(true)))
                        .into_any_element()
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_a_variant_with_a_neutral_icon() {
        assert_eq!(ResultVariant::ALL.len(), 5);
        assert_eq!(ResultVariant::Empty.icon(), IconName::Folder);
        assert_eq!(ResultVariant::Error.icon(), IconName::ErrorCircle);
    }
}
