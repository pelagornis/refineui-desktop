//! [`Icon`] element.

use gpui::{
    App, Hsla, IntoElement, Pixels, RenderOnce, SharedString, Styled, Transformation, Window, px,
    svg,
};

use crate::{ActiveTheme, IconName};
use refineui_system_icons as rsi;

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
        use refineui_foundation::size::icon;
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
    glyph: rsi::IconName,
    style: IconStyle,
    size: IconSize,
    color: Option<Hsla>,
    transformation: Transformation,
    surface: Option<crate::SurfaceKind>,
}

impl Icon {
    /// New icon (regular style, 16px, current text color).
    pub fn new(name: IconName) -> Self {
        Self::named(name.glyph())
    }

    /// Any glyph of RefineUI System Icons (890 icons), e.g.
    /// `Icon::named(refineui::system_icons::icons::AIRPLANE)`.
    pub fn named(glyph: rsi::IconName) -> Self {
        Self {
            glyph,
            style: IconStyle::Regular,
            size: IconSize::XSmall,
            color: None,
            transformation: Transformation::default(),
            surface: None,
        }
    }

    /// Default color for a [`crate::Surface`] of `kind` (e.g. white on the
    /// HUD). An explicit [`Self::color`] still wins.
    pub fn on_surface(mut self, kind: crate::SurfaceKind) -> Self {
        self.surface = Some(kind);
        self
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
        let color = self.color.unwrap_or_else(|| match self.surface {
            Some(kind) => kind.foreground(cx),
            None => cx.theme().colors.foreground_primary,
        });
        let px_f32 = f32::from(self.size.px());
        svg()
            .flex_none()
            .size(self.size.px())
            .path(asset_path(self.glyph, source_size(px_f32), self.style))
            .text_color(color)
            .with_transformation(self.transformation)
    }
}

/// Source sizes the icon set is drawn at (optical sizes).
pub(crate) const SOURCE_SIZES: [u16; 6] = [16, 20, 24, 28, 32, 48];

/// The optical source size to rasterize for a rendered size: the smallest
/// drawn size >= the rendered one (12px renders the 16px drawing).
pub(crate) fn source_size(rendered: f32) -> u16 {
    SOURCE_SIZES
        .into_iter()
        .find(|s| f32::from(*s) + 0.5 >= rendered)
        .unwrap_or(48)
}

/// Asset path served by [`crate::Assets`]: `icons/<slug>-<size>-<style>.svg`.
pub(crate) fn asset_path(glyph: rsi::IconName, size: u16, style: IconStyle) -> SharedString {
    let style = match style {
        IconStyle::Regular => "regular",
        IconStyle::Filled => "filled",
    };
    SharedString::from(format!("icons/{}-{size}-{style}.svg", glyph.as_str()))
}

/// Resolves an asset path back to SVG source (with size fallback).
pub(crate) fn svg_for_path(path: &str) -> Option<&'static str> {
    let rest = path.strip_prefix("icons/")?.strip_suffix(".svg")?;
    let (rest, style) = rest.rsplit_once('-')?;
    let (slug, size) = rest.rsplit_once('-')?;
    let size: u16 = size.parse().ok()?;
    let glyph = *rsi::Icon::names().iter().find(|n| n.as_str() == slug)?;
    let base = rsi::Icon::new(glyph);
    let base = match style {
        "filled" => base.filled(),
        "regular" => base.regular(),
        _ => return None,
    };
    // Exact optical size first, then the nearest drawn size.
    std::iter::once(size)
        .chain(SOURCE_SIZES.into_iter().filter(move |s| *s > size))
        .chain(SOURCE_SIZES.into_iter().rev().filter(move |s| *s < size))
        .find_map(|s| base.at(s).svg())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_curated_icon_exists_at_every_size_and_style() {
        for name in IconName::ALL {
            for size in SOURCE_SIZES {
                for style in [IconStyle::Regular, IconStyle::Filled] {
                    let path = asset_path(name.glyph(), size, style);
                    assert!(svg_for_path(&path).is_some(), "missing {path}");
                    let exact = rsi::Icon::new(name.glyph()).at(size);
                    let exact = if style == IconStyle::Filled {
                        exact.filled()
                    } else {
                        exact.regular()
                    };
                    assert!(exact.is_available(), "{path} not drawn at {size}");
                }
            }
        }
    }

    #[test]
    fn optical_sizes() {
        assert_eq!(source_size(12.), 16);
        assert_eq!(source_size(16.), 16);
        assert_eq!(source_size(20.), 20);
        assert_eq!(source_size(22.), 24);
        assert_eq!(source_size(64.), 48);
        assert!(svg_for_path("icons/add-12-regular.svg").is_some());
        assert!(svg_for_path("icons/nope-20-regular.svg").is_none());
    }
}
