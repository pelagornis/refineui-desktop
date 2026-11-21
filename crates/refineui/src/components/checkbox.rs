//! Checkbox — web `Checkbox` (`default | circular`, indeterminate).

use std::rc::Rc;

use gpui::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, size::component, spacing, stroke, typography::role};

use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle,
    use_interaction,
};
use crate::{ActiveTheme, Icon, IconName, IconSize, IconStyle, StyledExt};

/// Tri-state check value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckState {
    /// Unchecked (default).
    #[default]
    Unchecked,
    /// Checked.
    Checked,
    /// Mixed.
    Indeterminate,
}

impl From<bool> for CheckState {
    fn from(b: bool) -> Self {
        if b { Self::Checked } else { Self::Unchecked }
    }
}

impl CheckState {
    /// `true` when checked.
    pub fn is_checked(self) -> bool {
        self == Self::Checked
    }
}

/// Change callback: receives the *new* checked value.
pub type ToggleHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// Checkbox shape (web `variant`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckboxVariant {
    /// `roundedLarge` square (default).
    #[default]
    Default,
    /// Circle.
    Circular,
}

/// A checkbox with optional label. Toggles on click, Enter or Space.
///
/// States follow `refineui.css`: unchecked hover → `borderSubtle`, pressed →
/// `borderStrong`; checked → brand / brandHover / brandActive; keyboard focus →
/// 2px `borderStrong` (checked: `brandActive` fill + `borderFocus`). Colors
/// transition over `duration-fast`.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    state: CheckState,
    label: Option<SharedString>,
    description: Option<SharedString>,
    disabled: bool,
    variant: CheckboxVariant,
    on_change: Option<ToggleHandler>,
}

impl Checkbox {
    /// New checkbox.
    pub fn new(id: impl Into<ElementId>, state: impl Into<CheckState>) -> Self {
        Self {
            id: id.into(),
            state: state.into(),
            label: None,
            description: None,
            disabled: false,
            variant: CheckboxVariant::Default,
            on_change: None,
        }
    }

    /// Label (`labelMd`).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Helper text below the label (`captionMd`).
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Shape variant.
    pub fn variant(mut self, variant: CheckboxVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Round indicator (web `variant="circular"`).
    pub fn circular(mut self, circular: bool) -> Self {
        self.variant = if circular {
            CheckboxVariant::Circular
        } else {
            CheckboxVariant::Default
        };
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let on = self.state != CheckState::Unchecked;
        let (state, i) = use_interaction(&self.id, window, cx);
        let i = if self.disabled {
            Interaction::default()
        } else {
            i
        };
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus.is_focused(window) && focus_visible(cx) && !i.pressed;

        let (bg, border, border_w) = if self.disabled {
            if on {
                (
                    c.background_surface_selected,
                    c.background_surface_selected,
                    1.0,
                )
            } else {
                (c.background_primary_hover, c.border_disabled, 1.0)
            }
        } else if focused {
            if on {
                (c.background_brand_active, c.border_focus, stroke::THICK)
            } else {
                (c.background_primary, c.border_strong, stroke::THICK)
            }
        } else if on {
            let fill = i.pick(
                c.background_brand,
                c.background_brand_hover,
                c.background_brand_active,
            );
            (fill, fill, 1.0)
        } else {
            (
                c.background_primary,
                i.pick(c.border_default, c.border_subtle, c.border_strong),
                1.0,
            )
        };
        let bg = animate(child_id(&self.id, "bg"), bg, Motion::FAST_EASE, window, cx);
        let border = animate(
            child_id(&self.id, "bd"),
            border,
            Motion::FAST_EASE,
            window,
            cx,
        );
        let border_w = animate(
            child_id(&self.id, "bw"),
            border_w,
            Motion::FAST_EASE,
            window,
            cx,
        );
        let mark = animate(
            child_id(&self.id, "mark"),
            if on { 1.0 } else { 0.0 },
            Motion::FAST,
            window,
            cx,
        );
        let check_fg = if self.disabled {
            c.foreground_secondary
        } else {
            c.foreground_inversed
        };

        let size = px(component::CHECKBOX);
        let r = match self.variant {
            CheckboxVariant::Default => px(radius::LARGE),
            CheckboxVariant::Circular => px(radius::CIRCLE),
        };
        let next = !self.state.is_checked();
        let handler = self.on_change.clone();
        let label_fg = if self.disabled {
            c.foreground_disabled
        } else {
            c.foreground_primary
        };
        let has_desc = self.description.is_some();
        let indicator = self.state;

        div()
            .id(self.id.clone())
            .flex()
            .when(has_desc, |d| d.items_start())
            .when(!has_desc, |d| d.items_center())
            .gap(px(spacing::X_SMALL))
            .when(!self.disabled, |d| {
                d.cursor_action()
                    .track_focus(&focus)
                    .track_interaction(&state)
                    .when_some(handler, |d, h| d.on_click(move |_, w, cx| h(&next, w, cx)))
            })
            .when(self.disabled, |d| {
                d.cursor(CursorStyle::OperationNotAllowed)
            })
            .child(
                div()
                    .when(has_desc, |d| d.mt(px(spacing::XXX_SMALL)))
                    .flex_none()
                    .size(size)
                    .rounded(r)
                    .border(px(border_w))
                    .border_color(border)
                    .bg(bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .when(mark > 0.001, |d| {
                        d.child(div().opacity(mark).map(|d| {
                            match indicator {
                                CheckState::Indeterminate => d.child(
                                    div()
                                        .w(px(spacing::MEDIUM))
                                        .h(px(stroke::THICK))
                                        .rounded_full()
                                        .bg(check_fg),
                                ),
                                _ => d.child(
                                    Icon::new(IconName::Checkmark)
                                        .style(IconStyle::Filled)
                                        .size(IconSize::Small)
                                        .color(check_fg),
                                ),
                            }
                        }))
                    }),
            )
            .when(self.label.is_some() || has_desc, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .gap(px(spacing::X_SMALL))
                        .when_some(self.label, |d, l| {
                            d.child(
                                div()
                                    .text_style_token(role::LABEL_MD)
                                    .text_color(label_fg)
                                    .child(l),
                            )
                        })
                        .when_some(self.description, |d, t| {
                            d.child(
                                div()
                                    .text_style_token(role::CAPTION_MD)
                                    .text_color(if self.disabled {
                                        c.foreground_disabled
                                    } else {
                                        c.foreground_secondary
                                    })
                                    .child(t),
                            )
                        }),
                )
            })
    }
}
