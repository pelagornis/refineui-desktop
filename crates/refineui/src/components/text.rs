//! Text, Heading and Label — web `Text` / `Label` with RefineUI typography tokens.

use gpui::{
    App, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};
use refineui_foundation::typography::{self, TextStyle};

use crate::{ActiveTheme, ControlSize, StyledExt};

/// Typography roles (web `typographys.ts`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextVariant {
    /// 64 / 86 heavy
    Heading1,
    /// 40 / 60 bold
    Heading2,
    /// 32 / 40 bold
    Heading3,
    /// 28 / 36 semibold
    Heading4,
    /// 32 / 40 semibold
    Title1,
    /// 28 / 36 semibold
    Title2,
    /// 24 / 32 semibold
    Title3,
    /// 20 / 28 semibold
    SubTitle1,
    /// 16 / 24 semibold
    SubTitle2,
    /// 16 / 24 medium
    Body1,
    /// 14 / 20 medium
    Body2,
    /// 14 / 20 regular (default)
    #[default]
    Body3,
    /// 12 / 16 regular
    Body4,
    /// 12 / 16 medium
    Caption1,
    /// 12 / 16 regular
    Caption2,
    /// 10 / 14 regular
    Caption3,
}

impl TextVariant {
    /// All roles in scale order.
    pub const ALL: [TextVariant; 16] = [
        Self::Heading1,
        Self::Heading2,
        Self::Heading3,
        Self::Heading4,
        Self::Title1,
        Self::Title2,
        Self::Title3,
        Self::SubTitle1,
        Self::SubTitle2,
        Self::Body1,
        Self::Body2,
        Self::Body3,
        Self::Body4,
        Self::Caption1,
        Self::Caption2,
        Self::Caption3,
    ];

    /// Underlying token.
    pub fn token(self) -> TextStyle {
        typography::ALL[self as usize]
    }
}

/// Text color roles.
/// Semantic text role (web `Text variant`, `semantic/text.ts`) — what the
/// web components reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum TextRole {
    DisplayLg,
    DisplayMd,
    DisplaySm,
    HeadingLg,
    HeadingMd,
    HeadingSm,
    TitleLg,
    TitleMd,
    TitleSm,
    SubtitleLg,
    SubtitleMd,
    BodyLg,
    #[default]
    BodyMd,
    BodySm,
    BodyXs,
    LabelLg,
    LabelMd,
    LabelSm,
    CaptionLg,
    CaptionMd,
    CaptionSm,
}

impl TextRole {
    /// All roles, largest first.
    pub const ALL: [TextRole; 21] = [
        Self::DisplayLg,
        Self::DisplayMd,
        Self::DisplaySm,
        Self::HeadingLg,
        Self::HeadingMd,
        Self::HeadingSm,
        Self::TitleLg,
        Self::TitleMd,
        Self::TitleSm,
        Self::SubtitleLg,
        Self::SubtitleMd,
        Self::BodyLg,
        Self::BodyMd,
        Self::BodySm,
        Self::BodyXs,
        Self::LabelLg,
        Self::LabelMd,
        Self::LabelSm,
        Self::CaptionLg,
        Self::CaptionMd,
        Self::CaptionSm,
    ];

    /// Web name (`bodyMd`, …).
    pub fn name(self) -> &'static str {
        [
            "displayLg",
            "displayMd",
            "displaySm",
            "headingLg",
            "headingMd",
            "headingSm",
            "titleLg",
            "titleMd",
            "titleSm",
            "subtitleLg",
            "subtitleMd",
            "bodyLg",
            "bodyMd",
            "bodySm",
            "bodyXs",
            "labelLg",
            "labelMd",
            "labelSm",
            "captionLg",
            "captionMd",
            "captionSm",
        ][self as usize]
    }

    /// Resolved text style.
    pub fn token(self) -> TextStyle {
        use typography::role::*;
        [
            DISPLAY_LG,
            DISPLAY_MD,
            DISPLAY_SM,
            HEADING_LG,
            HEADING_MD,
            HEADING_SM,
            TITLE_LG,
            TITLE_MD,
            TITLE_SM,
            SUBTITLE_LG,
            SUBTITLE_MD,
            BODY_LG,
            BODY_MD,
            BODY_SM,
            BODY_XS,
            LABEL_LG,
            LABEL_MD,
            LABEL_SM,
            CAPTION_LG,
            CAPTION_MD,
            CAPTION_SM,
        ][self as usize]
    }
}

/// Text color role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextColor {
    /// `foregroundPrimary` (default)
    #[default]
    Primary,
    /// `foregroundSecondary`
    Secondary,
    /// `foregroundTertiary`
    Tertiary,
    /// `foregroundDisabled`
    Disabled,
    /// `foregroundSuccess`
    Success,
    /// `foregroundWarning`
    Warning,
    /// `foregroundError`
    Error,
    /// `foregroundInfo`
    Info,
    /// `foregroundBrand`
    Brand,
}

impl TextColor {
    pub(crate) fn resolve(self, cx: &App) -> Hsla {
        let c = &cx.theme().colors;
        match self {
            Self::Primary => c.foreground_primary,
            Self::Secondary => c.foreground_secondary,
            Self::Tertiary => c.foreground_tertiary,
            Self::Disabled => c.foreground_disabled,
            Self::Success => c.foreground_success,
            Self::Warning => c.foreground_warning,
            Self::Error => c.foreground_error,
            Self::Info => c.foreground_info,
            Self::Brand => c.foreground_brand,
        }
    }
}

/// A run of text styled with a typography token.
#[derive(IntoElement)]
pub struct Text {
    text: SharedString,
    style: TextStyle,
    color: TextColor,
    truncate: bool,
    surface: Option<crate::SurfaceKind>,
}

impl Text {
    /// Body text.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            style: TextRole::BodyMd.token(),
            color: TextColor::Primary,
            truncate: false,
            surface: None,
        }
    }

    /// Read on a [`crate::Surface`] of `kind`: primary / secondary /
    /// tertiary colors follow the surface (e.g. light text on the HUD);
    /// intent colors are unchanged.
    pub fn on_surface(mut self, kind: crate::SurfaceKind) -> Self {
        self.surface = Some(kind);
        self
    }

    /// Typography role.
    pub fn variant(mut self, variant: TextVariant) -> Self {
        self.style = variant.token();
        self
    }

    /// Semantic role (web `variant`, e.g. [`TextRole::BodyMd`]).
    pub fn role(mut self, role: TextRole) -> Self {
        self.style = role.token();
        self
    }

    /// Color role.
    pub fn color(mut self, color: TextColor) -> Self {
        self.color = color;
        self
    }

    /// Single line with ellipsis.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }
}

impl RenderOnce for Text {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = match (self.surface, self.color) {
            (Some(kind), TextColor::Primary) => kind.foreground(cx),
            (Some(kind), TextColor::Secondary | TextColor::Tertiary) => {
                kind.foreground_secondary(cx)
            }
            (_, color) => color.resolve(cx),
        };
        let el = div().text_style_token(self.style).text_color(color);
        let el = if self.truncate { el.truncate() } else { el };
        el.child(self.text)
    }
}

/// Shorthand for a heading-level [`Text`].
pub struct Heading;

#[allow(clippy::new_ret_no_self)]
impl Heading {
    /// `heading1`…`heading4` (`level` is clamped to 1..=4).
    pub fn new(level: u8, text: impl Into<SharedString>) -> Text {
        let role = match level {
            0 | 1 => TextRole::HeadingLg,
            2 => TextRole::HeadingMd,
            3 => TextRole::HeadingSm,
            4 => TextRole::TitleMd,
            _ => TextRole::TitleSm,
        };
        Text::new(text).role(role)
    }
}

/// Form label — web `Label` (`sm | md | lg`, required marker).
#[derive(IntoElement)]
pub struct Label {
    text: SharedString,
    size: ControlSize,
    required: bool,
    disabled: bool,
}

impl Label {
    /// New label.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            size: ControlSize::Medium,
            required: false,
            disabled: false,
        }
    }

    /// Size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Append a required `*`.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Disabled color.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let fg = if self.disabled {
            theme.colors.foreground_disabled
        } else {
            theme.colors.foreground_primary
        };
        // `labelSizeTypo`: sm labelSm, md labelMd, lg labelLg; required `*` with xxx-small gap.
        let style = match self.size {
            ControlSize::Small => refineui_foundation::typography::role::LABEL_SM,
            ControlSize::Medium => refineui_foundation::typography::role::LABEL_MD,
            ControlSize::Large => refineui_foundation::typography::role::LABEL_LG,
        };
        div()
            .flex()
            .gap(px(refineui_foundation::spacing::XXX_SMALL))
            .text_style_token(style)
            .text_color(fg)
            .child(self.text)
            .when(self.required, |this| {
                this.child(div().text_color(theme.colors.foreground_error).child("*"))
            })
    }
}

use gpui::prelude::FluentBuilder as _;
