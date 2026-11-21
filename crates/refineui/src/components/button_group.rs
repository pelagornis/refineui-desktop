//! ButtonGroup — web `ButtonGroup` (`components/ButtonGroup/style.ts`).
//!
//! `attached` (default) wraps the buttons in one tube outline — the same
//! track as SegmentedControl: `gap-xxx-small` (2px), thin `borderDefault`,
//! `backgroundBrandSubtle`, `p-xxx-small` (2px) and an outer radius of Button
//! radius + padding (10 / 14 / 18px) so the corners stay concentric.
//! `attached(false)` only spaces the buttons (`gap-xx-small`, 4px).
//! `size` / `variant` cascade to every Button that does not set its own.

use gpui::{App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, prelude::*, px};
use refineui_foundation::{spacing, stroke};

use crate::{ActiveTheme, Axis, Button, ButtonVariant, ControlSize};

/// Outer tube radius for a size (`attachedRadius`): Button radius + 2px padding.
pub fn button_group_radius(size: ControlSize) -> f32 {
    match size {
        ControlSize::Small => 10.,
        ControlSize::Medium => 14.,
        ControlSize::Large => 18.,
    }
}

/// A row (or column) of related buttons.
#[derive(IntoElement)]
pub struct ButtonGroup {
    buttons: Vec<Button>,
    size: Option<ControlSize>,
    variant: Option<ButtonVariant>,
    orientation: Axis,
    attached: bool,
}

impl ButtonGroup {
    /// Empty, attached, horizontal group.
    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
            size: None,
            variant: None,
            orientation: Axis::Horizontal,
            attached: true,
        }
    }

    /// Add a button.
    pub fn button(mut self, button: Button) -> Self {
        self.buttons.push(button);
        self
    }

    /// Add several buttons.
    pub fn buttons(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
        self.buttons.extend(buttons);
        self
    }

    /// Size cascaded to buttons without their own (web `size`).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Variant cascaded to buttons without their own (web `variant`).
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Layout direction (web `orientation`).
    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        self
    }

    /// `true` (default): one tube outline; `false`: spaced buttons only.
    pub fn attached(mut self, attached: bool) -> Self {
        self.attached = attached;
        self
    }
}

impl Default for ButtonGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// The tube chrome shared with [`crate::Toolbar`] groups.
pub(crate) fn group_tube(size: ControlSize, vertical: bool, cx: &App) -> gpui::Div {
    let c = &cx.theme().colors;
    div()
        .flex()
        .flex_none()
        .when(vertical, |d| d.flex_col())
        .when(!vertical, |d| d.items_center())
        .gap(px(spacing::XXX_SMALL))
        .border(px(stroke::THIN))
        .border_color(c.border_default)
        .bg(c.background_brand_subtle)
        .p(px(spacing::XXX_SMALL))
        .rounded(px(
            cx.theme().density.button(size).radius + spacing::XXX_SMALL
        ))
}

/// The detached ButtonGroup row (`gap-xx-small`), for composites that wrap
/// each Button (e.g. [`crate::shell::MenuBar`]).
pub(crate) fn group_row(vertical: bool) -> gpui::Div {
    div()
        .flex()
        .flex_none()
        .when(vertical, |d| d.flex_col())
        .when(!vertical, |d| d.items_center())
        .gap(px(spacing::XX_SMALL))
}

impl RenderOnce for ButtonGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let vertical = self.orientation == Axis::Vertical;
        let (size, variant) = (self.size, self.variant);
        let buttons = self
            .buttons
            .into_iter()
            .map(move |b| b.inherit(size, variant).full_width(vertical));
        if self.attached {
            group_tube(size.unwrap_or_default(), vertical, cx).children(buttons)
        } else {
            group_row(vertical).children(buttons)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tube_radius_is_button_radius_plus_padding() {
        let d = refineui_foundation::size::Density::Comfortable;
        for s in ControlSize::ALL {
            assert_eq!(
                button_group_radius(s),
                d.button(s).radius + spacing::XXX_SMALL
            );
        }
    }
}
