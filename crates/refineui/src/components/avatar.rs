//! Avatar and Avatars (group) — web `Avatar` / `AvatarGroup`.
//!
//! Nine sizes (`3xs`…`3xl` = 16–60px), circular shell with a thin border,
//! `image | icon | initials` layouts, ten Global tint colors, and the Web Kit
//! status glyphs (online / away / unavailable / offline).

use gpui::{
    App, Hsla, ImageSource, IntoElement, ObjectFit, ParentElement, PathBuilder, RenderOnce,
    SharedString, Styled, StyledImage, Window, canvas, div, img, point, prelude::*, px,
};
use refineui_tokens::{
    palette,
    size::component::AVATAR_SIZES,
    size::icon,
    spacing,
    typography::{TextStyle, role},
};

use crate::theme::to_hsla;
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// Avatar size (web `3xs | 2xs | xs | sm | md | lg | xl | 2xl | 3xl`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarSize {
    /// 16px
    XXXSmall,
    /// 20px
    XXSmall,
    /// 24px
    XSmall,
    /// 28px
    Small,
    /// 32px (default)
    #[default]
    Medium,
    /// 36px
    Large,
    /// 48px
    XLarge,
    /// 54px
    XXLarge,
    /// 60px
    XXXLarge,
}

impl AvatarSize {
    /// All sizes, smallest first.
    pub const ALL: [AvatarSize; 9] = [
        Self::XXXSmall,
        Self::XXSmall,
        Self::XSmall,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::XLarge,
        Self::XXLarge,
        Self::XXXLarge,
    ];

    /// Web alias (`md`, `2xl`, …).
    pub fn label(self) -> &'static str {
        ["3xs", "2xs", "xs", "sm", "md", "lg", "xl", "2xl", "3xl"][self as usize]
    }

    /// Diameter in px.
    pub fn px(self) -> f32 {
        AVATAR_SIZES[self as usize]
    }

    fn icon(self) -> f32 {
        [
            icon::XX_SMALL,
            icon::X_SMALL,
            icon::SMALL,
            icon::SMALL,
            icon::MEDIUM,
            icon::MEDIUM,
            icon::LARGE,
            icon::X_LARGE,
            icon::XX_LARGE,
        ][self as usize]
    }

    fn initials_neutral(self) -> TextStyle {
        [
            role::BODY_XS,
            role::BODY_MD,
            role::BODY_LG,
            role::BODY_LG,
            role::BODY_LG,
            role::BODY_LG,
            role::SUBTITLE_LG,
            role::TITLE_SM,
            role::TITLE_MD,
        ][self as usize]
    }

    fn initials_accent(self) -> TextStyle {
        match self {
            Self::XLarge => role::SUBTITLE_LG,
            Self::XXLarge => role::TITLE_SM,
            Self::XXXLarge => role::TITLE_MD,
            _ => role::BODY_LG,
        }
    }

    fn count_text(self) -> TextStyle {
        [
            role::CAPTION_SM,
            role::CAPTION_SM,
            role::CAPTION_MD,
            role::CAPTION_MD,
            role::CAPTION_LG,
            role::CAPTION_LG,
            role::BODY_MD,
            role::BODY_MD,
            role::BODY_MD,
        ][self as usize]
    }

    /// Status glyph size and inset (`bottom / end`).
    fn status(self) -> (f32, f32) {
        [
            (4., 0.),
            (6., 0.),
            (6., 1.),
            (6., 1.),
            (8., 1.),
            (10., 1.),
            (12., 1.),
            (12., 0.),
            (14., -2.),
        ][self as usize]
    }

    fn stack_overlap(self) -> f32 {
        [4., 4., 4., 6., 6., 8., 10., 10., 12.][self as usize]
    }

    fn spread_gap(self) -> f32 {
        [
            spacing::XX_SMALL,
            spacing::XX_SMALL,
            spacing::X_SMALL,
            spacing::MEDIUM,
            spacing::MEDIUM,
            spacing::LARGE,
            spacing::LARGE,
            spacing::X_LARGE,
            spacing::X_LARGE,
        ][self as usize]
    }

    fn one_letter(self) -> bool {
        matches!(self, Self::XXXSmall | Self::XXSmall | Self::XSmall)
    }
}

/// Global tint for icon / initials avatars (web `color`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarColor {
    /// `neutral300` / `neutral850` (default)
    #[default]
    Neutral,
    /// Blue
    Blue,
    /// Green
    Green,
    /// Lime
    Lime,
    /// Magenta
    Magenta,
    /// Orange
    Orange,
    /// Purple
    Purple,
    /// Red
    Red,
    /// Teal
    Teal,
    /// Yellow
    Yellow,
}

impl AvatarColor {
    /// All colors.
    pub const ALL: [AvatarColor; 10] = [
        Self::Neutral,
        Self::Blue,
        Self::Green,
        Self::Lime,
        Self::Magenta,
        Self::Orange,
        Self::Purple,
        Self::Red,
        Self::Teal,
        Self::Yellow,
    ];

    /// `(shell background, glyph / initials ink)` — `avatar.shell.background.*`
    /// (`*100`) and `avatar.icon.*` (`*1000`).
    pub fn colors(self) -> (Hsla, Hsla) {
        use palette::*;
        let (bg, fg) = match self {
            Self::Neutral => (NEUTRAL_300, NEUTRAL_850),
            Self::Blue => (BLUE_100, BLUE_1000),
            Self::Green => (GREEN_100, GREEN_1000),
            Self::Lime => (LIME_100, LIME_1000),
            Self::Magenta => (MAGENTA_100, MAGENTA_1000),
            Self::Orange => (ORANGE_100, ORANGE_1000),
            Self::Purple => (PURPLE_100, PURPLE_1000),
            Self::Red => (RED_100, RED_1000),
            Self::Teal => (TEAL_100, TEAL_1000),
            Self::Yellow => (YELLOW_100, YELLOW_1000),
        };
        (to_hsla(bg), to_hsla(fg))
    }
}

/// Presence status (Web Kit `Avatar/Status`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Presence {
    /// `green500` dot.
    Online,
    /// `yellow300` crescent.
    Away,
    /// `red500` dot with a white bar.
    Unavailable,
    /// `neutral300` dot.
    Offline,
}

/// Avatar layout (web `layout`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarLayout {
    /// Initials from the name.
    Initials,
    /// Person glyph.
    Icon,
}

/// A user / entity picture with initials or icon fallback.
#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
    image: Option<ImageSource>,
    size: AvatarSize,
    color: AvatarColor,
    layout: Option<AvatarLayout>,
    icon: IconName,
    presence: Option<Presence>,
}

impl Avatar {
    /// Avatar for `name` (initials fallback; empty name → person icon).
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            image: None,
            size: AvatarSize::Medium,
            color: AvatarColor::Neutral,
            layout: None,
            icon: IconName::Person,
            presence: None,
        }
    }

    /// Picture (layout `image`).
    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }

    /// Size.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Tint for icon / initials layouts.
    pub fn color(mut self, color: AvatarColor) -> Self {
        self.color = color;
        self
    }

    /// Force a layout.
    pub fn layout(mut self, layout: AvatarLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Glyph for the icon layout (default `Person`).
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self.layout = Some(AvatarLayout::Icon);
        self
    }

    /// Status glyph at the bottom-end corner (web `showStatus` + `status`).
    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }
}

fn initials(name: &str, max: usize) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(max)
        .collect::<String>()
        .to_uppercase()
}

fn status_glyph(status: Presence, size: f32) -> gpui::AnyElement {
    let k = size / 14.;
    match status {
        Presence::Online | Presence::Offline => div()
            .size(px(size))
            .rounded_full()
            .bg(to_hsla(if status == Presence::Online {
                palette::GREEN_500
            } else {
                palette::NEUTRAL_300
            }))
            .into_any_element(),
        Presence::Unavailable => div()
            .size(px(size))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(to_hsla(palette::RED_500))
            .child(
                div()
                    .w(px(8.33 * k))
                    .h(px((2.33 * k).max(1.)))
                    .rounded_full()
                    .bg(to_hsla(palette::NEUTRAL_WHITE)),
            )
            .into_any_element(),
        Presence::Away => canvas(
            |_, _, _| {},
            move |b, _, window, _| {
                let o = b.origin;
                let p = |x: f32, y: f32| point(o.x + px(x * k), o.y + px(y * k));
                let mut path = PathBuilder::fill();
                path.move_to(p(8.92, 0.));
                path.arc_to(p(7., 7.) - o, px(0.), true, true, p(0., 8.92));
                path.arc_to(p(7.15, 7.15) - o, px(0.), false, false, p(8.92, 0.));
                path.close();
                if let Ok(path) = path.build() {
                    window.paint_path(path, to_hsla(palette::YELLOW_300));
                }
            },
        )
        .size(px(size))
        .into_any_element(),
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let s = self.size.px();
        let layout = self.layout.unwrap_or(if self.name.trim().is_empty() {
            AvatarLayout::Icon
        } else {
            AvatarLayout::Initials
        });
        let max = if self.layout == Some(AvatarLayout::Initials) || !self.size.one_letter() {
            2
        } else {
            1
        };
        let text = initials(&self.name, max);
        let (bg, fg) = self.color.colors();
        let showing_image = self.image.is_some();
        let (bg, border) = if showing_image {
            (AvatarColor::Neutral.colors().0, theme.colors.border_subtle)
        } else if layout == AvatarLayout::Initials {
            (bg, fg)
        } else {
            (bg, theme.colors.border_subtle)
        };
        let typo = if self.color == AvatarColor::Neutral {
            self.size.initials_neutral()
        } else {
            self.size.initials_accent()
        };
        let (dot, inset) = self.size.status();
        div()
            .relative()
            .flex_none()
            .size(px(s))
            .child(
                div()
                    .size_full()
                    .rounded_full()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(bg)
                    .text_color(fg)
                    .text_style_token(typo)
                    .border_1()
                    .border_color(border)
                    .map(|this| match self.image {
                        Some(src) => this.child(
                            img(src)
                                .size_full()
                                .rounded_full()
                                .object_fit(ObjectFit::Cover),
                        ),
                        None if layout == AvatarLayout::Icon || text.is_empty() => this.child(
                            Icon::new(self.icon)
                                .size(IconSize::Custom(px(self.size.icon())))
                                .color(fg),
                        ),
                        None => this.child(text),
                    }),
            )
            .when_some(self.presence, |this, status| {
                this.child(
                    div()
                        .absolute()
                        .right(px(inset))
                        .bottom(px(inset))
                        .child(status_glyph(status, dot)),
                )
            })
    }
}

/// Group layout (web `AvatarGroup layout`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AvatarsLayout {
    /// Overlapping (default).
    #[default]
    Stack,
    /// Spaced out.
    Spread,
}

/// A row of avatars with a `+N` count bubble — web `Avatars`:
/// `stack` overlaps each avatar by `--avatar-stack-overlap` (4–12px by size;
/// md = 6) with later avatars painted on top (ascending `z-index`) and the
/// same overlap as trailing padding; `spread` uses the size's gap (md = 10).
/// No extra ring — each avatar keeps its 1px `borderSubtle` shell. The count
/// bubble is a thin `borderDefault` circle on `backgroundPrimary`.
#[derive(IntoElement)]
pub struct Avatars {
    avatars: Vec<Avatar>,
    size: AvatarSize,
    layout: AvatarsLayout,
    max: usize,
    total: Option<usize>,
}

impl Avatars {
    /// Group of initials avatars for these names.
    pub fn new(names: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self::from_avatars(names.into_iter().map(Avatar::new))
    }

    /// Group of prepared avatars (images, icons, colors). Their size is
    /// overridden by [`Self::size`].
    pub fn from_avatars(avatars: impl IntoIterator<Item = Avatar>) -> Self {
        Self {
            avatars: avatars.into_iter().collect(),
            size: AvatarSize::Medium,
            layout: AvatarsLayout::Stack,
            max: 4,
            total: None,
        }
    }

    /// Size.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Stack or spread.
    pub fn layout(mut self, layout: AvatarsLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Visible avatars before the `+N` bubble.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max.max(1);
        self
    }

    /// Total member count when it exceeds the avatars passed in (web `total`).
    pub fn total(mut self, total: usize) -> Self {
        self.total = Some(total);
        self
    }
}

impl RenderOnce for Avatars {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let c = theme.colors;
        let s = self.size.px();
        let total = self
            .total
            .unwrap_or(self.avatars.len())
            .max(self.avatars.len());
        let shown = self.avatars.len().min(self.max);
        let overflow = total.saturating_sub(shown);
        let stack = self.layout == AvatarsLayout::Stack;
        let overlap = self.size.stack_overlap();
        let size = self.size;
        div()
            .flex()
            .items_center()
            .when(stack, |d| d.pr(px(overlap)))
            .when(!stack, |d| d.gap(px(size.spread_gap())))
            .children(
                self.avatars
                    .into_iter()
                    .take(shown)
                    .enumerate()
                    .map(move |(i, a)| {
                        div()
                            .relative()
                            .flex_none()
                            .when(stack && i > 0, |d| d.ml(px(-overlap)))
                            .child(a.size(size))
                    }),
            )
            .when(overflow > 0, |this| {
                this.child(
                    div()
                        .when(stack, |d| d.ml(px(-overlap)))
                        .size(px(s))
                        .flex_none()
                        .rounded_full()
                        .border_1()
                        .border_color(c.border_default)
                        .bg(c.background_primary)
                        .text_color(c.foreground_secondary)
                        .text_style_token(size.count_text())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(format!("+{overflow}")),
                )
            })
    }
}
