//! Skeleton — web `Skeleton` (pulsing placeholder).

use gpui::{AnimationExt, App, IntoElement, Pixels, RenderOnce, Styled, Window, div, px};

use crate::ActiveTheme;
use crate::motion::{skeleton_pulse, skeleton_pulse_opacity};

/// Shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkeletonShape {
    /// Rounded rectangle (default).
    #[default]
    Rect,
    /// Circle (avatar placeholder).
    Circle,
    /// Single text line.
    Text,
}

/// Loading placeholder.
#[derive(IntoElement)]
pub struct Skeleton {
    shape: SkeletonShape,
    width: Option<Pixels>,
    height: Option<Pixels>,
    animate: bool,
}

impl Skeleton {
    /// Default rectangle (full width, 20px tall).
    pub fn new() -> Self {
        Self {
            shape: SkeletonShape::Rect,
            width: None,
            height: None,
            animate: true,
        }
    }

    /// Shape.
    pub fn shape(mut self, shape: SkeletonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Width.
    pub fn width(mut self, w: Pixels) -> Self {
        self.width = Some(w);
        self
    }

    /// Height.
    pub fn height(mut self, h: Pixels) -> Self {
        self.height = Some(h);
        self
    }

    /// Disable the pulse (e.g. for reduced motion or screenshots).
    pub fn animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Web: `surfaceSelected`, `roundedLarge` (rect) / circle; opacity pulses
        // 1 → 0.5 → 1 over `duration-skeleton` (1500ms, ease-in-out), infinite.
        let color = cx.theme().colors.background_surface_selected;
        let h = self.height.unwrap_or(match self.shape {
            SkeletonShape::Text => px(12.),
            _ => px(refineui_tokens::size::component::SKELETON_HEIGHT),
        });
        let el = div().flex_none().h(h).bg(color);
        let el = match (self.shape, self.width) {
            (SkeletonShape::Circle, w) => el.w(w.unwrap_or(h)).rounded_full(),
            (SkeletonShape::Text, Some(w)) => el.w(w).rounded_full(),
            (SkeletonShape::Text, None) => el.w_full().rounded_full(),
            (_, Some(w)) => el.w(w).rounded(px(refineui_tokens::radius::LARGE)),
            (_, None) => el.w_full().rounded(px(refineui_tokens::radius::LARGE)),
        };
        if self.animate && !crate::motion::reduced_motion(cx) {
            el.with_animation("refineui-skeleton", skeleton_pulse(), |d, t| {
                d.opacity(skeleton_pulse_opacity(t))
            })
            .into_any_element()
        } else {
            el.into_any_element()
        }
    }
}
