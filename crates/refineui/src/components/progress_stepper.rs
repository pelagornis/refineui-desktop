//! ProgressStepper — web `ProgressStepper` (multi-step completion indicator).
//!
//! A 16px `brandSubtle` pill track with a `brand` fill, 8px pips inset 8px
//! from each end (`foregroundOnBrand` for complete / current, `borderDefault`
//! upcoming) and `labelSm` labels under the track (first label start-aligned,
//! last end-aligned, the rest centered). The fill is a pill whose round tip
//! sits just past the current pip; its width moves over `duration-panel` with
//! the emphasized curve, pips and labels recolor over `duration-fast`.

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{motion::easing::EMPHASIZED, opacity, spacing, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, probe, use_focus_handle,
    use_interaction, use_layout_probe,
};
use crate::{ActiveTheme, StyledExt};

/// Step state relative to the current step (web `ProgressStepperState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StepState {
    /// Before the current step.
    Complete,
    /// The current step.
    Current,
    /// After the current step.
    Upcoming,
}

impl StepState {
    pub(crate) fn of(ix: usize, current: usize) -> Self {
        if ix < current {
            Self::Complete
        } else if ix == current {
            Self::Current
        } else {
            Self::Upcoming
        }
    }
}

type StepHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// Compact pill-track step progress (web `ProgressStepper`).
///
/// ```ignore
/// ProgressStepper::new("checkout", ["Cart", "Shipping", "Payment", "Review"], self.step)
///     .on_change(cx.listener(|this, ix, _, cx| { this.step = *ix; cx.notify(); }));
/// ```
#[derive(IntoElement)]
pub struct ProgressStepper {
    id: ElementId,
    steps: Vec<SharedString>,
    current: usize,
    disabled: HashSet<usize>,
    on_change: Option<StepHandler>,
}

impl ProgressStepper {
    /// Steps with `current` (0-based) active.
    pub fn new(
        id: impl Into<ElementId>,
        steps: impl IntoIterator<Item = impl Into<SharedString>>,
        current: usize,
    ) -> Self {
        Self {
            id: id.into(),
            steps: steps.into_iter().map(Into::into).collect(),
            current,
            disabled: HashSet::new(),
            on_change: None,
        }
    }

    /// Make steps clickable (web `onValueChange`); Enter / Space activate the
    /// focused step.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Disable step `ix` (web `ProgressStepperItem disabled`): half opacity,
    /// not clickable.
    pub fn disabled_step(mut self, ix: usize) -> Self {
        self.disabled.insert(ix);
        self
    }
}

/// `--refineui-progress-stepper-track` (`foundationSize160`).
const TRACK: f32 = 16.;
/// `--refineui-progress-stepper-pip` (`foundationSize80`).
const PIP: f32 = 8.;
/// `--refineui-progress-stepper-inset` (`foundationSize80`).
const INSET: f32 = 8.;

/// Web `pipLeftForIndex`.
fn pip_left(width: f32, ix: f32, count: usize) -> f32 {
    if count <= 1 {
        INSET
    } else {
        INSET + ix * (width - 2. * INSET - PIP) / (count - 1) as f32
    }
}

/// Web `fillWidthForStep` (with a fractional, animated index).
fn fill_width(width: f32, ix: f32, count: usize) -> f32 {
    if count <= 1 || ix >= (count - 1) as f32 {
        width
    } else {
        (pip_left(width, ix, count) + PIP + TRACK / 2.).min(width)
    }
}

impl RenderOnce for ProgressStepper {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let n = self.steps.len().max(1);
        let current = self.current.min(n - 1);
        let measure = use_layout_probe(&child_id(&self.id, "probe"), window, cx);
        let width = measure.read(cx).container.map(|b| f32::from(b.size.width));
        // Web fill: `transition-[width] duration-panel ease-emphasized`.
        let pos = animate(
            child_id(&self.id, "pos"),
            current as f32,
            Motion::new(refineui_foundation::motion::duration::PANEL, EMPHASIZED),
            window,
            cx,
        );
        let fill = width.map(|w| fill_width(w, pos, n));
        let interactive = self.on_change.is_some();
        let show_focus = focus_visible(cx);
        let ring = theme.focus_ring_color(c.border_focus);
        let last = n - 1;
        let mut pips = Vec::new();
        let mut items = Vec::new();
        for (ix, label) in self.steps.into_iter().enumerate() {
            let state = StepState::of(ix, current);
            let disabled = self.disabled.contains(&ix);
            let can_activate = interactive && !disabled;
            let id = ElementId::NamedChild(Box::new(self.id.clone()), format!("s-{ix}").into());
            let (istate, i) = use_interaction(&id, window, cx);
            let focus = use_focus_handle(&id, 0, window, cx);
            let focused = can_activate && focus.is_focused(window);
            let hovered = can_activate && i.hovered;
            // `hover:[marker][upcoming] → foregroundSecondary`, label likewise.
            let pip_bg = match state {
                StepState::Upcoming if hovered => c.foreground_secondary,
                StepState::Upcoming => c.border_default,
                _ => c.foreground_on_brand,
            };
            let pip_bg = animate(child_id(&id, "pip"), pip_bg, Motion::FAST, window, cx);
            let fg = match state {
                StepState::Complete => c.foreground_primary,
                StepState::Current => c.foreground_brand,
                StepState::Upcoming if hovered => c.foreground_secondary,
                StepState::Upcoming => c.foreground_tertiary,
            };
            let fg = animate(child_id(&id, "fg"), fg, Motion::FAST, window, cx);
            if let Some(w) = width {
                let ring = ring.clone();
                pips.push(
                    div()
                        .absolute()
                        .top(px((TRACK - PIP) / 2.))
                        .left(px(pip_left(w, ix as f32, n)))
                        .size(px(PIP))
                        .rounded_full()
                        .bg(pip_bg)
                        .when(disabled, |d| d.opacity(opacity::DISABLED))
                        // `focus-visible` ring sits on the marker.
                        .when(show_focus && focused, move |d| d.shadow(ring)),
                );
            }
            let handler = self.on_change.clone().filter(|_| can_activate);
            let key_handler = handler.clone();
            items.push(
                div()
                    .id(id)
                    .flex_1()
                    .min_w_0()
                    .pt(px(TRACK + spacing::X_SMALL))
                    .text_style_token(role::LABEL_SM)
                    .text_color(fg)
                    .truncate()
                    .when(state == StepState::Current, |d| {
                        d.font_weight(gpui::FontWeight::MEDIUM)
                    })
                    .map(|d| {
                        if ix == 0 && n > 1 {
                            d.pl(px(INSET)).text_left()
                        } else if ix == last && n > 1 {
                            d.pr(px(INSET)).text_right()
                        } else {
                            d.text_center()
                        }
                    })
                    .when(disabled, |d| {
                        d.opacity(opacity::DISABLED)
                            .cursor(gpui::CursorStyle::OperationNotAllowed)
                    })
                    .when_some(handler, |d, h| {
                        d.cursor_action()
                            .track_interaction(&istate)
                            .track_focus(&focus)
                            .on_click(move |_, w, cx| h(&ix, w, cx))
                    })
                    .when_some(key_handler, |d, h| {
                        d.on_key_down(move |e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                cx.stop_propagation();
                                h(&ix, w, cx);
                            }
                        })
                    })
                    .child(label),
            );
        }
        div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .child(probe(&measure, None))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(px(TRACK))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(c.background_brand_subtle)
                    .when_some(fill, |d, f| {
                        d.child(
                            div()
                                .h_full()
                                .w(px(f))
                                .rounded_full()
                                .bg(c.background_brand),
                        )
                    }),
            )
            .children(pips)
            .child(
                div()
                    .relative()
                    .flex()
                    .w_full()
                    .items_start()
                    .children(items),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pips_sit_inside_insets() {
        assert_eq!(pip_left(400., 0., 4), INSET);
        assert_eq!(pip_left(400., 3., 4), 400. - INSET - PIP);
    }

    #[test]
    fn fill_tip_passes_current_pip() {
        let w = 400.;
        assert_eq!(fill_width(w, 0., 4), INSET + PIP + TRACK / 2.);
        assert_eq!(fill_width(w, 3., 4), w);
        assert_eq!(fill_width(w, 0., 1), w);
    }

    #[test]
    fn states() {
        assert_eq!(StepState::of(0, 1), StepState::Complete);
        assert_eq!(StepState::of(1, 1), StepState::Current);
        assert_eq!(StepState::of(2, 1), StepState::Upcoming);
    }
}
