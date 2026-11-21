//! Badge — web `Badge` (`default | neutral | outline | success | warning | danger`, label / number).

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::motion::{Interaction, Motion, TrackInteraction, animate, child_id, use_interaction};
use crate::{ActiveTheme, StyledExt};

/// Badge color variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
    /// Brand fill (default).
    #[default]
    Default,
    /// Subtle brand fill.
    Neutral,
    /// Bordered.
    Outline,
    /// Green.
    Success,
    /// Yellow.
    Warning,
    /// Red.
    Danger,
}

impl BadgeVariant {
    /// All variants.
    pub const ALL: [BadgeVariant; 6] = [
        Self::Default,
        Self::Neutral,
        Self::Outline,
        Self::Success,
        Self::Warning,
        Self::Danger,
    ];

    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Neutral => "Neutral",
            Self::Outline => "Outline",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Danger => "Danger",
        }
    }
}

/// A small status label or count.
/// Small status label or count.
///
/// `captionMd`, `sizeXXSmall × sizeSmall` padding, `roundedMedium` (label) or
/// circular (number). With [`Badge::id`] it gets the Web Kit hover / pressed
/// fills, transitioned over `duration-fast`.
#[derive(IntoElement)]
pub struct Badge {
    id: Option<ElementId>,
    label: SharedString,
    variant: BadgeVariant,
    number: bool,
}

impl Badge {
    /// Label badge.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            id: None,
            label: label.into(),
            variant: BadgeVariant::Default,
            number: false,
        }
    }

    /// Circular count badge (`99+` above 99), `danger` by default.
    pub fn count(count: usize) -> Self {
        let label = if count > 99 {
            "99+".to_string()
        } else {
            count.to_string()
        };
        Self {
            id: None,
            label: label.into(),
            variant: BadgeVariant::Danger,
            number: true,
        }
    }

    /// Variant.
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Make the badge interactive (hover / pressed fills).
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let (state, i) = match &self.id {
            Some(id) => {
                let (s, i) = use_interaction(id, window, cx);
                (Some(s), i)
            }
            None => (None, Interaction::default()),
        };
        let clear = c.background_primary.opacity(0.);
        let (bg, fg, border) = match self.variant {
            BadgeVariant::Default => (
                i.pick(
                    c.background_brand,
                    c.background_brand_hover,
                    c.background_brand_active,
                ),
                c.foreground_inversed,
                None,
            ),
            BadgeVariant::Neutral => (
                i.pick(
                    c.background_brand_subtle,
                    c.background_primary_hover,
                    c.background_primary_active,
                ),
                c.foreground_brand_strong,
                None,
            ),
            BadgeVariant::Outline => (
                clear,
                c.foreground_primary,
                Some(i.pick(c.border_default, c.border_default, c.border_strong)),
            ),
            BadgeVariant::Success => (
                i.pick(
                    c.background_success,
                    c.background_success_hover,
                    c.background_success_subtle,
                ),
                c.foreground_inversed,
                None,
            ),
            BadgeVariant::Warning => (
                i.pick(
                    c.background_warning,
                    c.background_warning_hover,
                    c.background_warning_subtle,
                ),
                c.foreground_inversed,
                None,
            ),
            BadgeVariant::Danger => (
                i.pick(
                    c.background_error,
                    c.background_error_hover,
                    c.background_error_subtle,
                ),
                c.foreground_inversed,
                None,
            ),
        };
        let (bg, border) = match &self.id {
            Some(id) => (
                animate(child_id(id, "bg"), bg, Motion::FAST_EASE, window, cx),
                border.map(|b| animate(child_id(id, "bd"), b, Motion::FAST_EASE, window, cx)),
            ),
            None => (bg, border),
        };
        let id = self.id.clone().unwrap_or_else(|| "refineui-badge".into());
        div()
            .id(id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .py(px(spacing::XX_SMALL))
            .px(px(spacing::SMALL))
            .bg(bg)
            .text_color(fg)
            .text_style_token(role::CAPTION_MD)
            .whitespace_nowrap()
            .when_some(border, |d, b| d.border_1().border_color(b))
            .when_some(state, |d, s| d.track_interaction(&s))
            .map(|d| {
                if self.number {
                    d.rounded_full()
                } else {
                    d.rounded(px(radius::MEDIUM))
                }
            })
            .child(self.label)
    }
}
