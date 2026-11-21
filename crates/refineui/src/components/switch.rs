//! Switch — web `Switch`.

use gpui::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle,
    use_interaction,
};
use crate::{ActiveTheme, StyledExt, ToggleHandler};

/// Web `Toggle` is RefineUI's name for the switch (`Switch` re-exports it).
pub type Toggle = Switch;

/// An on/off switch (web `Toggle` / `Switch`). Toggles on click, Enter or Space.
///
/// 44×24 track, 20px thumb with `shadow-2`; the thumb slides `switchThumb` (20px)
/// and the track color transitions over `duration-medium`.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    on: bool,
    label: Option<SharedString>,
    disabled: bool,
    on_change: Option<ToggleHandler>,
}

impl Switch {
    /// New switch.
    pub fn new(id: impl Into<ElementId>, on: bool) -> Self {
        Self {
            id: id.into(),
            on,
            label: None,
            disabled: false,
            on_change: None,
        }
    }

    /// Label shown to the right.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(f));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use refineui_foundation::size::component::{
            SWITCH_HEIGHT, SWITCH_PADDING, SWITCH_THUMB, SWITCH_WIDTH,
        };
        let theme = cx.theme().clone();
        let c = theme.colors;
        let (state, i) = use_interaction(&self.id, window, cx);
        let i = if self.disabled {
            Interaction::default()
        } else {
            i
        };
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus.is_focused(window) && focus_visible(cx);
        // `componentColorTokens.switch` + the hover / active rules in refineui.css.
        let track = match (self.disabled, self.on) {
            (true, true) => c.background_brand_subtle,
            (true, false) => c.background_surface_disabled,
            (false, true) => i.pick(
                c.background_brand,
                c.background_brand_hover,
                c.background_brand_active,
            ),
            (false, false) => i.pick(
                c.background_primary_active,
                c.background_surface_selected,
                c.background_brand_subtle,
            ),
        };
        let thumb = match (self.disabled, self.on) {
            (true, false) => c.background_brand_subtle,
            _ => c.background_primary,
        };
        // `transition: background-color / transform var(--duration-medium)`.
        let track = animate(
            child_id(&self.id, "track"),
            track,
            Motion::MEDIUM,
            window,
            cx,
        );
        let thumb = animate(
            child_id(&self.id, "thumb"),
            thumb,
            Motion::MEDIUM,
            window,
            cx,
        );
        let x = animate(
            child_id(&self.id, "x"),
            if self.on { SWITCH_THUMB } else { 0.0 },
            Motion::MEDIUM,
            window,
            cx,
        );
        let ring = theme.focus_ring_color(c.border_focus);
        let next = !self.on;
        let handler = self.on_change.clone();
        let label_fg = if self.disabled {
            c.foreground_disabled
        } else {
            c.foreground_primary
        };
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(spacing::SMALL))
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
                    .relative()
                    .flex_none()
                    .w(px(SWITCH_WIDTH))
                    .h(px(SWITCH_HEIGHT))
                    .rounded(px(radius::X_LARGE))
                    .bg(track)
                    .when(focused, |d| d.shadow(ring))
                    .child(
                        div()
                            .absolute()
                            .top(px(SWITCH_PADDING))
                            .left(px(SWITCH_PADDING + x))
                            .size(px(SWITCH_THUMB))
                            .rounded(px(radius::X_LARGE))
                            .bg(thumb)
                            .when(!self.disabled, |d| {
                                d.shadow(theme.shadow(crate::Elevation::SURFACE))
                            }),
                    ),
            )
            .when_some(self.label, |d, l| {
                d.child(
                    div()
                        .text_style_token(role::LABEL_MD)
                        .text_color(label_fg)
                        .child(l),
                )
            })
    }
}
