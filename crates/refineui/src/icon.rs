//! [`Icon`] element.

use gpui::{App, Hsla, IntoElement, Pixels, RenderOnce, Styled, Transformation, Window, px, svg};

use crate::{ActiveTheme, IconName};

/// Icon outline style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconStyle {
    /// Outline (default).
    #[default]
    Regular,
    /// Solid.
    Filled,
}

/// Icon size steps — `iconSizes` (`global/sizes.ts`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum IconSize {
    /// `xxsmall` — 12px
    XXSmall,
    /// `xsmall` — 16px (default)
    #[default]
    XSmall,
    /// `small` — 20px
    Small,
    /// `medium` — 24px
    Medium,
    /// `large` — 28px
    Large,
    /// `xlarge` — 32px
    XLarge,
    /// `xxlarge` — 48px
    XXLarge,
    /// Explicit size.
    Custom(Pixels),
}

impl IconSize {
    /// Size in pixels.
    pub fn px(self) -> Pixels {
        use refineui_tokens::size::icon;
        px(match self {
            IconSize::XXSmall => icon::XX_SMALL,
            IconSize::XSmall => icon::X_SMALL,
            IconSize::Small => icon::SMALL,
            IconSize::Medium => icon::MEDIUM,
            IconSize::Large => icon::LARGE,
            IconSize::XLarge => icon::X_LARGE,
            IconSize::XXLarge => icon::XX_LARGE,
            IconSize::Custom(p) => return p,
        })
    }
}

/// A monochrome SVG icon from the RefineUI System Icons set.
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    style: IconStyle,
    size: IconSize,
    color: Option<Hsla>,
    transformation: Transformation,
}

impl Icon {
    /// New icon (regular style, 16px, current text color).
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            style: IconStyle::Regular,
            size: IconSize::XSmall,
            color: None,
            transformation: Transformation::default(),
        }
    }

    /// Filled / regular.
    pub fn style(mut self, style: IconStyle) -> Self {
        self.style = style;
        self
    }

    /// Size.
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Color (defaults to `foreground_primary`).
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Rotation / scale transform.
    pub fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = transformation;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.color.unwrap_or(cx.theme().colors.foreground_primary);
        svg()
            .flex_none()
            .size(self.size.px())
            .path(self.name.path(self.style))
            .text_color(color)
            .with_transformation(self.transformation)
    }
}
