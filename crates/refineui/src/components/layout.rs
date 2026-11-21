//! Layout helpers — web `Layout` (`Box`, `Stack`, `Grid`, `Container`,
//! `Spacer`) as thin `div` presets. Gaps and paddings take the spacing
//! tokens (`refineui_foundation::spacing::*`) like the web `sizeXxx` props.

use gpui::{Div, Hsla, Styled, div, px};

/// Horizontal flex row, items centred, `gap` px (web `Stack direction="row" align="center"`).
pub fn h_stack(gap: f32) -> Div {
    div().flex().flex_row().items_center().gap(px(gap))
}

/// Vertical flex column, `gap` px (web `Stack`).
pub fn v_stack(gap: f32) -> Div {
    div().flex().flex_col().gap(px(gap))
}

/// Flexible spacer that pushes siblings apart (web `Spacer`).
pub fn spacer() -> Div {
    div().flex_1()
}

/// Equal-column CSS grid (web `Grid columns gap`). Use
/// `.col_span(n)` on children for web `GridItem span`.
pub fn grid(columns: u16, gap: f32) -> Div {
    div().grid().grid_cols(columns.max(1)).gap(px(gap)).w_full()
}

#[cfg(feature = "web-extras")]
/// Full-width column with horizontal padding (web `Container`, default
/// `sizeXXLarge` = 24px).
pub fn container() -> Div {
    container_with(refineui_foundation::spacing::XX_LARGE)
}

#[cfg(feature = "web-extras")]
/// [`container`] with a custom horizontal padding.
pub fn container_with(padding_x: f32) -> Div {
    div().w_full().mx_auto().px(px(padding_x))
}

/// Token box (web `Box padding background radius border borderColor`).
pub fn token_box(
    padding: f32,
    background: Option<Hsla>,
    radius: f32,
    border: Option<(f32, Hsla)>,
) -> Div {
    let mut d = div().p(px(padding)).rounded(px(radius));
    if let Some(bg) = background {
        d = d.bg(bg);
    }
    if let Some((w, c)) = border {
        d = d.border(px(w)).border_color(c);
    }
    d
}
