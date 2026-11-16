//! Field — web `Field` (label, control, hint, error).

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*, px,
};

use refineui_tokens::{spacing, typography::role};

use crate::{ActiveTheme, ControlSize, Label, StyledExt};

/// Form row wrapping any control.
#[derive(IntoElement)]
pub struct Field {
    label: Option<SharedString>,
    hint: Option<SharedString>,
    error: Option<SharedString>,
    required: bool,
    size: ControlSize,
    control: Option<AnyElement>,
}

impl Field {
    /// New field.
    pub fn new() -> Self {
        Self {
            label: None,
            hint: None,
            error: None,
            required: false,
            size: ControlSize::Large,
            control: None,
        }
    }

    /// Label above the control.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Helper text below the control.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Error message (replaces the hint).
    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Optional error.
    pub fn error_opt(mut self, error: Option<SharedString>) -> Self {
        self.error = error;
        self
    }

    /// Required marker.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Label size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The control (TextInput entity, Select, Checkbox, …).
    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.control = Some(control.into_any_element());
        self
    }
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Field {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        // Web `Field`: column, `sizeXXSmall` gap; hint / error in `captionSm`
        // (`foregroundTertiary` / `foregroundError`).
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::XX_SMALL))
            .w_full()
            .when_some(self.label, |d, l| {
                d.child(Label::new(l).size(self.size).required(self.required))
            })
            .children(self.control)
            .map(|d| match (self.error, self.hint) {
                (Some(err), _) => d.child(
                    div()
                        .text_style_token(role::CAPTION_SM)
                        .text_color(c.foreground_error)
                        .child(err),
                ),
                (None, Some(hint)) => d.child(
                    div()
                        .text_style_token(role::CAPTION_SM)
                        .text_color(c.foreground_tertiary)
                        .child(hint),
                ),
                (None, None) => d,
            })
    }
}
