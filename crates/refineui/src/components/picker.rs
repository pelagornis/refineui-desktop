//! Shared pieces of the field-style pickers (DateInput, TimeInput): the
//! trigger (web `inputStyles` + `formControlInputSizeClass`) and the
//! end-aligned popover that opens below it.

use gpui::{
    AnyElement, App, ElementId, Entity, MouseButton, ParentElement, Styled, Window, anchored, div,
    prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke};

use crate::components::floating::{Beak, BeakAlign, BeakSide, floating_gap, floating_surface};
use crate::motion::{Motion, animate, child_id, presence, probe, use_layout_probe};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, StyledExt};

/// Web popover / tooltip enter: opacity + a `sizeXSmall` (6px) slide from the
/// trigger side over `duration-fast`, emphasized (`popoverStyles.motion`).
pub(crate) const FLOAT_ENTER: Motion = Motion::new(
    crate::motion::duration::FAST,
    crate::motion::easing::EMPHASIZED,
);

/// Trigger options.
pub(crate) struct PickerField {
    pub id: ElementId,
    pub size: ControlSize,
    pub text: Option<gpui::SharedString>,
    pub placeholder: gpui::SharedString,
    pub icon: IconName,
    pub error: bool,
    pub success: bool,
    pub disabled: bool,
    pub open: bool,
    pub full_width: bool,
    /// Replaces the value/placeholder text (TimeInput's editable segments).
    pub content: Option<AnyElement>,
}

/// The trigger button: Input shell (border default → hover → focus halo while
/// open), value or placeholder text, trailing 20px icon in `foregroundSecondary`.
pub(crate) fn picker_field(
    f: PickerField,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Stateful<gpui::Div> {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let m = theme.field_metrics(f.size);
    let hovered = window.use_keyed_state(child_id(&f.id, "hover"), cx, |_, _| false);
    let is_hover = *hovered.read(cx) && !f.disabled;
    let (border, halo) = if f.disabled {
        (c.border_disabled, false)
    } else if f.error {
        (c.border_error, false)
    } else if f.success {
        (c.border_success, false)
    } else if f.open {
        (c.border_focus, true)
    } else if is_hover {
        (c.border_hover, false)
    } else {
        (c.border_default, false)
    };
    let border = animate(child_id(&f.id, "border"), border, Motion::FAST, window, cx);
    let halo_a = animate(
        child_id(&f.id, "halo"),
        if halo { 1.0f32 } else { 0.0 },
        Motion::FAST,
        window,
        cx,
    );
    let hover_state = hovered.clone();
    div()
        .id(f.id)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(spacing::SMALL))
        .when(f.full_width, |d| d.w_full())
        .min_h(px(m.height))
        .px(px(m.padding_x))
        .py(px(m.padding_y))
        .rounded(px(m.radius))
        .border(px(stroke::THIN))
        .border_color(border)
        .bg(if f.disabled {
            c.background_surface_disabled
        } else {
            c.background_primary
        })
        .when(halo_a > 0.01, |d| {
            d.shadow(theme.ring(c.border_focus.opacity(halo_a), stroke::HAIRLINE))
        })
        .text_style_token(m.text)
        .text_color(if f.disabled {
            c.foreground_disabled
        } else {
            c.foreground_primary
        })
        .when(!f.disabled, |d| d.cursor_action())
        .on_hover(move |h, _, cx| {
            hover_state.update(cx, |s, cx| {
                *s = *h;
                cx.notify();
            })
        })
        .child(match f.content {
            Some(content) => div().flex_1().min_w_0().child(content),
            None => div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .when(f.text.is_none(), |d| d.text_color(c.foreground_placeholder))
                .child(f.text.unwrap_or(f.placeholder)),
        })
        .child(
            Icon::new(f.icon)
                .size(IconSize::Small)
                .color(c.foreground_secondary),
        )
}

/// Trigger + end-aligned popover (web `PopoverContent placement="bottom"
/// align="end" fitContent`). `open` is the keyed open flag; `panel` is
/// rendered only while mounted, with `pad` inner padding.
pub(crate) fn picker_popup(
    id: &ElementId,
    open: &Entity<bool>,
    trigger: AnyElement,
    panel: Option<AnyElement>,
    pad: f32,
    full_width: bool,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Stateful<gpui::Div> {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let is_open = *open.read(cx);
    let p = presence(
        child_id(id, "presence"),
        is_open,
        FLOAT_ENTER,
        FLOAT_ENTER,
        window,
        cx,
    );
    let measure = use_layout_probe(id, window, cx);
    let (offset_x, measured) = {
        let m = measure.read(cx);
        match (m.container, m.items.first().copied().flatten()) {
            (Some(t), Some(panel)) => (t.size.width - panel.size.width, true),
            _ => (px(0.), false),
        }
    };
    let toggle = open.clone();
    let close_out = open.clone();
    let gap = floating_gap(refineui_foundation::size::component::POPOVER_BEAK_HEIGHT);
    // Slide from the trigger side: -6px → 0 while entering.
    let slide = px(-spacing::X_SMALL * (1. - p.progress));
    div()
        .id(id.clone())
        .relative()
        .flex_none()
        .when(full_width, |d| d.w_full())
        .child(
            div()
                .id("trigger")
                .relative()
                .when(full_width, |d| d.w_full())
                .child(probe(&measure, None))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    toggle.update(cx, |o, cx| {
                        *o = !*o;
                        cx.notify();
                    })
                })
                .child(trigger),
        )
        .when(p.mounted, |d| {
            let Some(body) = panel else { return d };
            d.child(
                crate::layer(
                    anchored()
                        .offset(gpui::point(offset_x, px(0.)))
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .id("picker-wrap")
                                .opacity(if measured { p.progress } else { 0. })
                                .mt(gap + slide)
                                .when(p.open, |d| {
                                    d.on_mouse_down_out(move |_, w, cx| {
                                        let close_out = close_out.clone();
                                        crate::layer::close_on_outside_press(w, cx, move |_, cx| {
                                            close_out.update(cx, |o, cx| {
                                                *o = false;
                                                cx.notify();
                                            })
                                        })
                                    })
                                })
                                .child(
                                    div()
                                        .id("picker-panel")
                                        .occlude()
                                        .relative()
                                        .p(px(pad + stroke::HAIRLINE))
                                        .rounded(px(radius::XX_LARGE))
                                        .shadow(theme.shadow(crate::Elevation::FLOATING))
                                        .font_family(theme.font_family.clone())
                                        .text_color(c.foreground_primary)
                                        .child(floating_surface(
                                            radius::XX_LARGE,
                                            Some(Beak::popover(BeakSide::Top, BeakAlign::End)),
                                            theme.floating_fill(),
                                            c.border_default,
                                        ))
                                        .child(probe(&measure, Some(0)))
                                        .child(body),
                                ),
                        ),
                )
                .priority(1),
            )
        })
}
