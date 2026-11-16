//! Spinner — web `Spinner` (`xs … xxl`).

use std::sync::OnceLock;
use std::time::Instant;

use gpui::{
    App, Hsla, IntoElement, ParentElement, PathBuilder, RenderOnce, SharedString, Styled, Window,
    canvas, div, point, prelude::*, px,
};
use refineui_tokens::{motion::duration, size::component, spacing, typography::role};

use crate::{ActiveTheme, StyledExt};

/// Spinner sizes (`spinnerSize*` tokens: 16 / 20 / 24 / 28 / 32 / 48).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerSize {
    /// 16px
    XSmall,
    /// 20px
    Small,
    /// 24px (default)
    #[default]
    Medium,
    /// 28px
    Large,
    /// 32px
    XLarge,
    /// 48px
    XXLarge,
}

impl SpinnerSize {
    /// All sizes.
    pub const ALL: [SpinnerSize; 6] = [
        Self::XSmall,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::XLarge,
        Self::XXLarge,
    ];

    /// Diameter in px (16 / 20 / 24 / 28 / 32 / 48).
    pub fn px(self) -> f32 {
        component::SPINNER_SIZES[self as usize]
    }
}

/// An indeterminate progress ring.
/// Indeterminate loading ring: `brandSubtle` ring with a `brand` top
/// quarter, rotating once per `duration-spinner` (800ms, linear).
#[derive(IntoElement)]
pub struct Spinner {
    size: SpinnerSize,
    color: Option<Hsla>,
    label: Option<SharedString>,
}

impl Spinner {
    /// Medium spinner in the brand color.
    pub fn new() -> Self {
        Self {
            size: SpinnerSize::Medium,
            color: None,
            label: None,
        }
    }

    /// Size.
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Arc color.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Text beside the ring (`foregroundPrimary`, size-matched typography).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

fn epoch() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let ix = self.size as usize;
        let size = component::SPINNER_SIZES[ix];
        let ring = component::SPINNER_RING_WIDTHS[ix];
        let arc = self.color.unwrap_or(theme.colors.background_brand);
        let track = if self.color.is_some() {
            arc.opacity(0.2)
        } else {
            theme.colors.background_brand_subtle
        };
        let reduced = crate::motion::reduced_motion(cx);
        let typo = [
            role::CAPTION_LG,
            role::BODY_MD,
            role::BODY_LG,
            role::SUBTITLE_MD,
            role::SUBTITLE_LG,
            role::TITLE_SM,
        ][ix];
        let ring_el = canvas(
            |_, _, _| {},
            move |b, _, window, _| {
                let turn = if reduced {
                    0.
                } else {
                    let ms = epoch().elapsed().as_millis() as u64 % duration::SPINNER;
                    window.request_animation_frame();
                    ms as f32 / duration::SPINNER as f32
                };
                let r = size / 2. - ring / 2.;
                let center = point(b.origin.x + px(size / 2.), b.origin.y + px(size / 2.));
                let at = |deg: f32| {
                    let a = deg.to_radians();
                    point(center.x + px(r * a.cos()), center.y + px(r * a.sin()))
                };
                let radii = point(px(r), px(r));
                let mut t = PathBuilder::stroke(px(ring));
                t.move_to(at(0.));
                t.arc_to(radii, px(0.), false, true, at(180.));
                t.arc_to(radii, px(0.), false, true, at(360.));
                if let Ok(p) = t.build() {
                    window.paint_path(p, track);
                }
                // CSS `border-top` on a circle: the quarter between the
                // top-left and top-right diagonals.
                let start = -135. + turn * 360.;
                let mut a = PathBuilder::stroke(px(ring));
                a.move_to(at(start));
                a.arc_to(radii, px(0.), false, true, at(start + 90.));
                if let Ok(p) = a.build() {
                    window.paint_path(p, arc);
                }
            },
        )
        .flex_none()
        .size(px(size));
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(spacing::X_SMALL))
            .child(ring_el)
            .when_some(self.label, |d, l| {
                d.child(
                    div()
                        .text_style_token(typo)
                        .text_color(theme.colors.foreground_primary)
                        .child(l),
                )
            })
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}
