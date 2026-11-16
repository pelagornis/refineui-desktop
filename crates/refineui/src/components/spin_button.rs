//! SpinButton — web `SpinButton` (numeric value with step buttons; ↑/↓ keys).

use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, KeyDownEvent, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use gpui::{CursorStyle, InteractiveElement, MouseButton};
use refineui_tokens::{size::component, spacing};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, IconStyle, StyledExt};

/// Numeric stepper.
#[derive(IntoElement)]
pub struct SpinButton {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    precision: usize,
    suffix: Option<SharedString>,
    size: ControlSize,
    disabled: bool,
    width: Option<gpui::Pixels>,
    on_change: Option<Rc<dyn Fn(&f64, &mut Window, &mut App)>>,
}

impl SpinButton {
    /// New stepper (0..=100, step 1).
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            min: 0.,
            max: 100.,
            step: 1.,
            precision: 0,
            suffix: None,
            size: ControlSize::Medium,
            disabled: false,
            width: None,
            on_change: None,
        }
    }

    /// Range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Step.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Decimal places shown.
    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    /// Unit suffix (e.g. "px").
    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fixed width (default 160px; the web field fills its container).
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(&f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for SpinButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let (min, max, step, value) = (self.min, self.max, self.step, self.value);
        let text = format!(
            "{:.*}{}",
            self.precision,
            value,
            self.suffix.clone().unwrap_or_default()
        );
        let m = theme.field_metrics(self.size);
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus.contains_focused(window, cx);
        // `:focus-within` → `borderFocus` (transition `duration-fast`).
        let border = if self.disabled {
            c.border_disabled
        } else if focused {
            c.border_focus
        } else {
            c.border_default
        };
        let border = animate(
            child_id(&self.id, "bd"),
            border,
            Motion::FAST_EASE,
            window,
            cx,
        );
        let mut steps = Vec::new();
        for (name, icon, delta, enabled) in [
            (
                "inc",
                IconName::ChevronUp,
                step,
                !self.disabled && value < max,
            ),
            (
                "dec",
                IconName::ChevronDown,
                -step,
                !self.disabled && value > min,
            ),
        ] {
            let id = child_id(&self.id, name);
            let (state, i) = use_interaction(&id, window, cx);
            let (bg, fg) = if enabled {
                (
                    i.pick(
                        c.background_primary,
                        c.background_surface_hover,
                        c.background_surface_active,
                    ),
                    i.pick(
                        c.foreground_secondary,
                        c.foreground_primary,
                        c.foreground_primary,
                    ),
                )
            } else {
                (c.background_surface_disabled, c.foreground_disabled)
            };
            let bg = animate(child_id(&id, "bg"), bg, Motion::FAST_EASE, window, cx);
            let fg = animate(child_id(&id, "fg"), fg, Motion::FAST_EASE, window, cx);
            let handler = self.on_change.clone();
            steps.push(
                div()
                    .id(id)
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .bg(bg)
                    .when(!enabled, |d| {
                        d.opacity(0.5).cursor(CursorStyle::OperationNotAllowed)
                    })
                    .when(enabled, |d| {
                        d.cursor_action()
                            .track_interaction(&state)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .when_some(handler, |d, h| {
                                d.on_click(move |_, w, cx| {
                                    h(&(value + delta).clamp(min, max), w, cx)
                                })
                            })
                    })
                    .child(
                        Icon::new(icon)
                            .style(IconStyle::Filled)
                            .size(IconSize::XSmall)
                            .color(fg),
                    ),
            );
        }
        let key_handler = self.on_change.clone();
        div()
            .id(self.id.clone())
            .flex()
            .h(px(m.height))
            .w(self.width.unwrap_or(px(160.)))
            .rounded(px(m.radius))
            .border_1()
            .border_color(border)
            .bg(if self.disabled {
                c.background_surface_disabled
            } else {
                c.background_primary
            })
            .overflow_hidden()
            .when(!self.disabled, |d| {
                d.track_focus(&focus)
                    .on_key_down(move |e: &KeyDownEvent, w, cx| {
                        let delta = match e.keystroke.key.as_str() {
                            "up" => step,
                            "down" => -step,
                            "pageup" => step * 10.,
                            "pagedown" => -step * 10.,
                            _ => return,
                        };
                        cx.stop_propagation();
                        if let Some(h) = &key_handler {
                            h(&(value + delta).clamp(min, max), w, cx);
                        }
                    })
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .pl(px(component::SPIN_FIELD_PADDING_START))
                    .pr(px(spacing::X_SMALL))
                    .text_style_token(m.text)
                    .text_color(if self.disabled {
                        c.foreground_disabled
                    } else {
                        c.foreground_primary
                    })
                    .child(text),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .h_full()
                    .w(px(component::SPIN_STEPPER_WIDTH))
                    .children(steps),
            )
    }
}
