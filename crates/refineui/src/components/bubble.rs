//! Bubble — web `Bubble` / `BubbleGroup` / `BubbleReactions`
//! (`components/Bubble/style.ts`): a chat message surface.
//!
//! Content: `bodyMd`, `roundedXXLarge` (16px), thin transparent border,
//! `px-large` / `py-medium` (16 / 10px), at most 80% of the conversation
//! column (ghost: full width). Variants: `default` brand fill with inversed
//! text, `secondary` / `muted` `surfaceSelected`, `tinted` brandSubtle with a
//! `borderStrong` stroke and brand text, `outline` surface + `borderStrong`,
//! `ghost` bare text, `destructive` error fill.
//!
//! In a [`BubbleGroup`] (`gap-xx-small`, 4px) consecutive bubbles flatten
//! only their sender-side corners to `roundedLarge` (8px). Reactions sit in
//! a pill (`roundedCircle`, thin `borderDefault`, `backgroundPrimary`,
//! `px-x-small` / `py-xx-small`, shadow2) overlapping the top or bottom edge
//! by 3/4 of its height, inset `sizeMedium` from the inline edge; the bubble
//! reserves `sizeXXLarge` (24px) for it.

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*, px, relative,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::{ActiveTheme, Elevation, StyledExt};

/// Bubble fill (web `BubbleVariant`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleVariant {
    /// Brand fill, inversed text (outgoing).
    #[default]
    Default,
    /// `surfaceSelected` fill (incoming).
    Secondary,
    /// Same fill as secondary.
    Muted,
    /// Brand-subtle fill, `borderStrong` stroke, brand text.
    Tinted,
    /// Surface fill, `borderStrong` stroke.
    Outline,
    /// No surface (assistant prose).
    Ghost,
    /// Error fill.
    Destructive,
}

impl BubbleVariant {
    /// All variants.
    pub const ALL: [BubbleVariant; 7] = [
        Self::Default,
        Self::Secondary,
        Self::Muted,
        Self::Tinted,
        Self::Outline,
        Self::Ghost,
        Self::Destructive,
    ];

    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Secondary => "Secondary",
            Self::Muted => "Muted",
            Self::Tinted => "Tinted",
            Self::Outline => "Outline",
            Self::Ghost => "Ghost",
            Self::Destructive => "Destructive",
        }
    }
}

/// Sender side (web `BubbleAlign`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleAlign {
    /// Incoming (left).
    #[default]
    Start,
    /// Outgoing (right).
    End,
}

/// Edge the reactions pill overlaps (web `BubbleReactionsSide`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BubbleReactionsSide {
    /// Above the bubble.
    Top,
    /// Below the bubble (default).
    #[default]
    Bottom,
}

/// Position of a bubble inside a [`BubbleGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Run {
    #[default]
    Only,
    First,
    Middle,
    Last,
}

/// One chat bubble.
#[derive(IntoElement)]
pub struct Bubble {
    variant: BubbleVariant,
    align: BubbleAlign,
    content: Vec<AnyElement>,
    reactions: Vec<SharedString>,
    reactions_side: BubbleReactionsSide,
    run: Run,
}

impl Bubble {
    /// Text bubble.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self::empty().child(div().child(text.into()))
    }

    /// Bubble without content; add with [`Self::child`].
    pub fn empty() -> Self {
        Self {
            variant: BubbleVariant::Default,
            align: BubbleAlign::Start,
            content: Vec::new(),
            reactions: Vec::new(),
            reactions_side: BubbleReactionsSide::Bottom,
            run: Run::Only,
        }
    }

    /// Add content.
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content.push(e.into_any_element());
        self
    }

    /// Fill variant.
    pub fn variant(mut self, v: BubbleVariant) -> Self {
        self.variant = v;
        self
    }

    /// Sender side.
    pub fn align(mut self, a: BubbleAlign) -> Self {
        self.align = a;
        self
    }

    /// Reactions pill (emoji / counts).
    pub fn reactions(mut self, r: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.reactions = r.into_iter().map(Into::into).collect();
        self
    }

    /// Which edge the reactions overlap.
    pub fn reactions_side(mut self, side: BubbleReactionsSide) -> Self {
        self.reactions_side = side;
        self
    }
}

/// Corner radii `(top_start, top_end, bottom_end, bottom_start)` for a bubble
/// in a run (web `bubbleStyles.group` connected-corner rules).
fn corners(align: BubbleAlign, run: Run) -> (f32, f32, f32, f32) {
    let (big, small) = (radius::XX_LARGE, radius::LARGE);
    let (top, bottom) = match run {
        Run::Only => (big, big),
        Run::First => (big, small),
        Run::Middle => (small, small),
        Run::Last => (small, big),
    };
    match align {
        // Sender edge = start (left).
        BubbleAlign::Start => (top, big, big, bottom),
        BubbleAlign::End => (big, top, bottom, big),
    }
}

impl RenderOnce for Bubble {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let clear = gpui::transparent_black();
        let (bg, fg, border) = match self.variant {
            BubbleVariant::Default => (c.background_brand, c.foreground_inversed, clear),
            BubbleVariant::Secondary | BubbleVariant::Muted => {
                (c.background_surface_selected, c.foreground_primary, clear)
            }
            BubbleVariant::Tinted => (
                c.background_brand_subtle,
                c.foreground_brand,
                c.border_strong,
            ),
            BubbleVariant::Outline => (c.background_surface, c.foreground_primary, c.border_strong),
            BubbleVariant::Ghost => (clear, c.foreground_primary, clear),
            BubbleVariant::Destructive => (c.background_error, c.foreground_on_error, clear),
        };
        let ghost = self.variant == BubbleVariant::Ghost;
        let (ts, te, be, bs) = corners(self.align, self.run);
        let end = self.align == BubbleAlign::End;
        let has_reactions = !self.reactions.is_empty();
        let top = self.reactions_side == BubbleReactionsSide::Top;
        // Pill height: py 4 + bodyMd line 20 + 2 × thin border.
        let pill_h = spacing::XX_SMALL * 2. + role::BODY_MD.line_height + stroke::THIN * 2.;
        let overlap = px(-pill_h * 0.75);
        let content = div()
            .text_style_token(role::BODY_MD)
            .text_color(fg)
            .min_w_0()
            .max_w_full()
            .overflow_hidden()
            .when(!ghost, |d| {
                d.px(px(spacing::LARGE))
                    .py(px(spacing::MEDIUM))
                    .bg(bg)
                    .border(px(stroke::THIN))
                    .border_color(border)
                    .map(|d| {
                        // Physical corners from logical ones (LTR).
                        let (tl, tr, br, bl) = (ts, te, be, bs);
                        d.rounded_tl(px(tl))
                            .rounded_tr(px(tr))
                            .rounded_br(px(br))
                            .rounded_bl(px(bl))
                    })
            })
            .children(self.content);
        let pill = has_reactions.then(|| {
            div()
                .absolute()
                .when(top, |d| d.top(overlap))
                .when(!top, |d| d.bottom(overlap))
                .when(end, |d| d.right(px(spacing::MEDIUM)))
                .when(!end, |d| d.left(px(spacing::MEDIUM)))
                .flex()
                .items_center()
                .gap(px(spacing::XX_SMALL))
                .px(px(spacing::X_SMALL))
                .py(px(spacing::XX_SMALL))
                .rounded_full()
                .border(px(stroke::THIN))
                .border_color(c.border_default)
                .bg(c.background_primary)
                .shadow(theme.shadow(Elevation::SURFACE))
                .text_style_token(role::BODY_MD)
                .text_color(c.foreground_primary)
                .children(self.reactions.into_iter().map(|r| div().child(r)))
        });
        div()
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .when(end, |d| d.items_end())
            .when(!end, |d| d.items_start())
            .when(has_reactions && !top, |d| d.mb(px(spacing::XX_LARGE)))
            .when(has_reactions && top, |d| d.mt(px(spacing::XX_LARGE)))
            .child(
                div()
                    .relative()
                    .min_w_0()
                    .when(!ghost, |d| d.max_w(relative(0.8)))
                    .when(ghost, |d| d.max_w_full())
                    .child(content)
                    .children(pill),
            )
    }
}

/// Consecutive bubbles from one sender (web `BubbleGroup`).
#[derive(IntoElement)]
pub struct BubbleGroup {
    bubbles: Vec<Bubble>,
}

impl BubbleGroup {
    /// New group.
    pub fn new(bubbles: impl IntoIterator<Item = Bubble>) -> Self {
        Self {
            bubbles: bubbles.into_iter().collect(),
        }
    }
}

impl RenderOnce for BubbleGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let n = self.bubbles.len();
        div()
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap(px(spacing::XX_SMALL))
            .children(self.bubbles.into_iter().enumerate().map(move |(i, mut b)| {
                b.run = match (n, i) {
                    (1, _) => Run::Only,
                    (_, 0) => Run::First,
                    (_, i) if i == n - 1 => Run::Last,
                    _ => Run::Middle,
                };
                b
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_flatten_only_the_sender_edge() {
        // Incoming first bubble: bottom-start flattened, end side keeps 16.
        assert_eq!(corners(BubbleAlign::Start, Run::First), (16., 16., 16., 8.));
        assert_eq!(corners(BubbleAlign::Start, Run::Middle), (8., 16., 16., 8.));
        assert_eq!(corners(BubbleAlign::End, Run::Last), (16., 8., 16., 16.));
        assert_eq!(corners(BubbleAlign::End, Run::Only), (16., 16., 16., 16.));
    }
}
