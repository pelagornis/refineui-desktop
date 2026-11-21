//! Stepper — web `Stepper` (horizontal multi-step progress rail).
//!
//! Indicators share the Calendar day cell (32px, `roundedLarge`, thin border,
//! `captionLg` medium): complete / current = `brand` fill with a filled
//! checkmark; upcoming = `borderDefault` outline with the step number.
//! Colors transition over `duration-medium` (emphasized). Rails are 2px
//! `borderDefault` tracks whose `brand` fill sweeps in over `duration-panel`.
//! Titles `labelMd` medium, descriptions `captionMd`.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px, relative,
};
use refineui_foundation::{
    motion::{duration, easing},
    radius,
    size::component,
    spacing,
    typography::role,
};

use crate::components::progress_stepper::StepState;
use crate::motion::{
    Motion, TrackInteraction, animate, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, Icon, IconName, IconSize, IconStyle, StyledExt};

struct Step {
    title: SharedString,
    description: Option<SharedString>,
}

/// Multi-step progress with numbered indicators.
#[derive(IntoElement)]
pub struct Stepper {
    id: ElementId,
    steps: Vec<Step>,
    current: usize,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Stepper {
    /// New stepper with `current` (0-based) active.
    pub fn new(id: impl Into<ElementId>, current: usize) -> Self {
        Self {
            id: id.into(),
            steps: Vec::new(),
            current,
            on_change: None,
        }
    }

    /// Add a step.
    pub fn step(mut self, title: impl Into<SharedString>) -> Self {
        self.steps.push(Step {
            title: title.into(),
            description: None,
        });
        self
    }

    /// Add a step with a description.
    pub fn step_with_description(
        mut self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.steps.push(Step {
            title: title.into(),
            description: Some(description.into()),
        });
        self
    }

    /// Make steps clickable.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

const MEDIUM_EMPHASIZED: Motion = Motion::new(duration::MEDIUM, easing::EMPHASIZED);
const RAIL: Motion = Motion::new(duration::PANEL, easing::EMPHASIZED);

impl RenderOnce for Stepper {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let n = self.steps.len();
        let day = component::CALENDAR_DAY;
        let rail_h = component::PROGRESS_TRACK_SM;
        let show_ring = focus_visible(cx);
        let mut cols = Vec::with_capacity(n);
        for (ix, step) in self.steps.into_iter().enumerate() {
            let state = StepState::of(ix, self.current);
            let id = ElementId::NamedChild(Box::new(self.id.clone()), format!("step-{ix}").into());
            let (istate, i) = use_interaction(&id, window, cx);
            let focus = use_focus_handle(&id, 0, window, cx);
            let interactive = self.on_change.is_some();
            let hovered = interactive && i.hovered;
            let (bg, border, fg) = match state {
                StepState::Complete | StepState::Current => (
                    c.background_brand,
                    c.background_brand,
                    c.foreground_inversed,
                ),
                StepState::Upcoming if interactive && i.pressed => (
                    c.background_primary_active,
                    c.border_strong,
                    c.foreground_primary,
                ),
                StepState::Upcoming if hovered => (
                    c.background_primary_hover,
                    c.border_strong,
                    c.foreground_primary_hover,
                ),
                StepState::Upcoming => (
                    c.background_primary.opacity(0.),
                    c.border_default,
                    c.foreground_secondary,
                ),
            };
            let key = |s: &str| ElementId::NamedChild(Box::new(id.clone()), s.to_string().into());
            let bg = animate(key("bg"), bg, MEDIUM_EMPHASIZED, window, cx);
            let border = animate(key("bd"), border, MEDIUM_EMPHASIZED, window, cx);
            let fg = animate(key("fg"), fg, MEDIUM_EMPHASIZED, window, cx);
            let title_fg = match state {
                StepState::Complete => c.foreground_primary,
                StepState::Current => c.foreground_brand,
                StepState::Upcoming if hovered => c.foreground_primary,
                StepState::Upcoming => c.foreground_secondary,
            };
            let title_fg = animate(key("tfg"), title_fg, Motion::FAST, window, cx);
            let desc_fg = if state == StepState::Upcoming {
                c.foreground_tertiary
            } else {
                c.foreground_secondary
            };
            let focused = show_ring && focus.is_focused(window);
            let handler = self.on_change.clone();
            cols.push(
                div()
                    .id(id.clone())
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(spacing::SMALL))
                    .rounded(px(radius::MEDIUM))
                    .when_some(handler, |d, h| {
                        d.cursor_action()
                            .track_focus(&focus)
                            .track_interaction(&istate)
                            .on_click(move |_, w, cx| h(&ix, w, cx))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(day))
                            .rounded(px(radius::LARGE))
                            .border_1()
                            .border_color(border)
                            .bg(bg)
                            .text_color(fg)
                            .text_style_token(role::CAPTION_LG)
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .when(focused, |d| d.shadow(theme.focus_ring()))
                            .map(|d| {
                                if state == StepState::Upcoming {
                                    d.child((ix + 1).to_string())
                                } else {
                                    d.child(
                                        Icon::new(IconName::Checkmark)
                                            .style(IconStyle::Filled)
                                            .size(IconSize::XSmall)
                                            .color(fg),
                                    )
                                }
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(spacing::XXX_SMALL))
                            .text_center()
                            .px(px(spacing::XX_SMALL))
                            .child(
                                div()
                                    .text_style_token(role::LABEL_MD)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(title_fg)
                                    .child(step.title),
                            )
                            .when_some(step.description, |d, t| {
                                d.child(
                                    div()
                                        .text_style_token(role::CAPTION_MD)
                                        .text_color(desc_fg)
                                        .child(t),
                                )
                            }),
                    ),
            );
            if ix + 1 < n {
                // Web `stepper-separator`: an equal flex column between steps;
                // its rail spans `-50% + half-marker` on both sides so one
                // straight track meets both indicator edges, and a single
                // brand fill sweeps left → right (`scale-x`, `duration-panel`
                // emphasized) when the step before it completes.
                let fill = animate(
                    key("sep"),
                    if ix < self.current { 1.0f32 } else { 0. },
                    RAIL,
                    window,
                    cx,
                );
                cols.push(
                    div()
                        .id(key("separator"))
                        .relative()
                        .flex_1()
                        .min_w(px(24.))
                        .h(px(day))
                        .child(
                            div()
                                .absolute()
                                .top(px((day - rail_h) / 2.))
                                .left(relative(-0.5))
                                .right(relative(-0.5))
                                .h(px(rail_h))
                                .px(px(day / 2.))
                                .child(
                                    div()
                                        .size_full()
                                        .overflow_hidden()
                                        .bg(c.border_default)
                                        .child(
                                            div().h_full().w(relative(fill)).bg(c.background_brand),
                                        ),
                                ),
                        ),
                );
            }
        }
        div()
            .id(self.id)
            .flex()
            .w_full()
            .items_start()
            .children(cols)
    }
}
