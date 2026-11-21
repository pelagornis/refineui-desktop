//! Radio and RadioGroup — web `Radio`.

use std::rc::Rc;

use gpui::{
    App, Axis as LayoutAxis, CursorStyle, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, px,
};

use refineui_foundation::{size::component, spacing, stroke, typography::role};

use crate::motion::{Motion, animate, child_id, focus_visible, use_focus_handle};
use crate::{ActiveTheme, StyledExt};

/// A single radio button. Usually rendered by [`RadioGroup`].
///
/// 20px circle with a 1px `borderDefault`; when checked the circle fills with
/// `foregroundPrimary` and a 10px `backgroundPrimary` dot scales in over
/// `duration-fast` ease-out. Keyboard focus draws a 2px `borderStrong` ring.
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    selected: bool,
    label: Option<SharedString>,
    description: Option<SharedString>,
    disabled: bool,
    on_select: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl Radio {
    /// New radio.
    pub fn new(id: impl Into<ElementId>, selected: bool) -> Self {
        Self {
            id: id.into(),
            selected,
            label: None,
            description: None,
            disabled: false,
            on_select: None,
        }
    }

    /// Label (`labelMd`).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Helper text under the label (`captionMd`).
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called when chosen.
    pub fn on_select(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Radio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus.is_focused(window) && focus_visible(cx);
        let (fill, border) = match (self.disabled, self.selected) {
            (true, true) => (c.foreground_disabled, c.foreground_disabled),
            (true, false) => (c.background_primary, c.border_disabled),
            (false, true) => (c.foreground_primary, c.foreground_primary),
            (false, false) => (c.background_primary, c.border_default),
        };
        let fill = animate(
            child_id(&self.id, "fill"),
            fill,
            Motion::FAST_EASE,
            window,
            cx,
        );
        let border = animate(
            child_id(&self.id, "bd"),
            border,
            Motion::FAST_EASE,
            window,
            cx,
        );
        let dot_scale = animate(
            child_id(&self.id, "dot"),
            if self.selected { 1.0 } else { 0.0 },
            Motion::FAST,
            window,
            cx,
        );
        let ring = cx.theme().ring(c.border_strong, stroke::THICK);
        let handler = self.on_select.clone();
        let label_fg = if self.disabled {
            c.foreground_disabled
        } else {
            c.foreground_primary
        };
        let has_desc = self.description.is_some();
        let dot = component::RADIO_DOT * dot_scale;
        div()
            .id(self.id)
            .flex()
            .when(has_desc, |d| d.items_start())
            .when(!has_desc, |d| d.items_center())
            .gap(px(spacing::X_SMALL))
            .when(!self.disabled, |d| {
                d.cursor_action()
                    .track_focus(&focus)
                    .when_some(handler, |d, h| d.on_click(move |_, w, cx| h(w, cx)))
            })
            .when(self.disabled, |d| {
                d.cursor(CursorStyle::OperationNotAllowed)
            })
            .child(
                div()
                    .when(has_desc, |d| d.mt(px(spacing::XXX_SMALL)))
                    .flex_none()
                    .size(px(component::CHECKBOX))
                    .rounded_full()
                    .border_1()
                    .border_color(border)
                    .bg(fill)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(focused, |d| d.shadow(ring))
                    .when(dot > 0.05, |d| {
                        d.child(div().size(px(dot)).rounded_full().bg(c.background_primary))
                    }),
            )
            .when(self.label.is_some() || has_desc, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
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

/// A set of mutually exclusive [`Radio`]s.
#[derive(IntoElement)]
pub struct RadioGroup {
    id: ElementId,
    options: Vec<SharedString>,
    selected: Option<usize>,
    axis: LayoutAxis,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl RadioGroup {
    /// New vertical group.
    pub fn new(
        id: impl Into<ElementId>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        selected: Option<usize>,
    ) -> Self {
        Self {
            id: id.into(),
            options: options.into_iter().map(Into::into).collect(),
            selected,
            axis: LayoutAxis::Vertical,
            disabled: false,
            on_change: None,
        }
    }

    /// Lay out horizontally.
    pub fn horizontal(mut self) -> Self {
        self.axis = LayoutAxis::Horizontal;
        self
    }

    /// Disable every option.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the chosen index.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let group_id = self.id.clone();
        div()
            .id(self.id)
            .flex()
            .gap(px(spacing::MEDIUM))
            .when(self.axis == LayoutAxis::Vertical, |d| d.flex_col())
            .when(self.axis == LayoutAxis::Horizontal, |d| {
                d.gap(px(spacing::LARGE))
            })
            .children(self.options.into_iter().enumerate().map(|(ix, label)| {
                let handler = self.on_change.clone();
                Radio::new(
                    ElementId::NamedChild(Box::new(group_id.clone()), format!("opt-{ix}").into()),
                    self.selected == Some(ix),
                )
                .label(label)
                .disabled(self.disabled)
                .when_some(handler, |r, h| r.on_select(move |w, cx| h(&ix, w, cx)))
            }))
    }
}
