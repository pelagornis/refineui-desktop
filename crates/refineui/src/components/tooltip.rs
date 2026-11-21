//! Tooltip — web `Tooltip`. Attach with GPUI's `.tooltip(Tooltip::text("…"))`.

use gpui::{
    AnyView, App, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::*, px,
};

use gpui::AnimationExt;
use refineui_foundation::{radius, size::component, spacing, stroke, typography::role};

use crate::components::floating::{Beak, BeakAlign, BeakSide};
use crate::motion::Motion;
use crate::{ActiveTheme, Elevation, Kbd, StyledExt};

/// Tooltip content view.
pub struct Tooltip {
    text: SharedString,
    shortcut: Option<SharedString>,
}

impl Tooltip {
    /// Builder usable directly with `.tooltip(...)` on any stateful element.
    pub fn text(
        text: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let text = text.into();
        move |_, cx| {
            cx.new(|_| Tooltip {
                text: text.clone(),
                shortcut: None,
            })
            .into()
        }
    }

    /// Tooltip with a keyboard shortcut hint, e.g. `Tooltip::with_shortcut("Save", "⌘S")`.
    pub fn with_shortcut(
        text: impl Into<SharedString>,
        shortcut: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let text = text.into();
        let shortcut = shortcut.into();
        move |_, cx| {
            cx.new(|_| Tooltip {
                text: text.clone(),
                shortcut: Some(shortcut.clone()),
            })
            .into()
        }
    }

    /// Tooltip for a control bound to a GPUI action: the shortcut comes from
    /// the installed [`crate::interaction::keymap::Keymap`] in the platform's
    /// notation (`⇧⌘P` / `Ctrl+Shift+P`); no hint when unbound.
    pub fn for_action(
        text: impl Into<SharedString>,
        action: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let text = text.into();
        let action = action.into();
        move |_, cx| {
            let shortcut = crate::interaction::keymap::shortcut_for(&action, cx).map(Into::into);
            cx.new(|_| Tooltip {
                text: text.clone(),
                shortcut,
            })
            .into()
        }
    }

    /// Render a static tooltip bubble (used by docs / screenshots).
    pub fn bubble(text: impl Into<SharedString>, cx: &App) -> gpui::Div {
        bubble(text.into(), None, None, cx)
    }
}

fn bubble(
    text: SharedString,
    shortcut: Option<SharedString>,
    beak: Option<Beak>,
    cx: &App,
) -> gpui::Div {
    let theme = cx.theme();
    let c = theme.colors;
    // Web `Tooltip`: transparent hairline-border panel (`roundedMedium`,
    // `bodyXs`, px `sizeMedium` / py `sizeSmall`) over a `FloatingSurface`
    // that fills + strokes the panel and its beak as one outline.
    // With materials active the panel is the Tooltip material (thin, real
    // blur) and the outline only strokes it and fills the beak.
    let (chrome, shadows) = crate::components::floating::floating_chrome(
        crate::SurfaceKind::Tooltip,
        radius::MEDIUM,
        beak,
        theme.floating_fill(),
        c.border_default,
        theme.shadow(Elevation::FLOATING),
        cx,
    );
    div()
        .relative()
        .max_w(px(component::TOOLTIP_MAX_WIDTH))
        .flex()
        .flex_col()
        .rounded(px(radius::MEDIUM))
        .shadow(shadows)
        .children(chrome)
        .text_color(c.foreground_primary)
        .font_family(theme.font_family.clone())
        .text_style_token(role::BODY_XS)
        .whitespace_nowrap()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::X_SMALL))
                .px(px(spacing::MEDIUM + stroke::HAIRLINE))
                .py(px(spacing::SMALL + stroke::HAIRLINE))
                .child(text)
                .when_some(shortcut, |this, s| this.child(Kbd::new(s))),
        )
}

/// A tooltip bubble with its 16×8 beak (2px rounded tip) inset `sizeMedium`
/// from the start edge — web default `align="Start"`. `BeakSide::Bottom` is a
/// tooltip shown above its trigger; `Top` one shown below. Used by docs /
/// screenshots.
pub fn tooltip_with_beak(text: impl Into<SharedString>, side: BeakSide, cx: &App) -> gpui::Div {
    tooltip_with_beak_aligned(text, side, BeakAlign::Start, cx)
}

/// [`tooltip_with_beak`] with an explicit beak alignment.
pub fn tooltip_with_beak_aligned(
    text: impl Into<SharedString>,
    side: BeakSide,
    align: BeakAlign,
    cx: &App,
) -> gpui::Div {
    bubble(text.into(), None, Some(Beak::tooltip(side, align)), cx)
}

/// GPUI places a hover tooltip 1px below-right of the cursor and flips it
/// above when it would overflow the window. The beak follows that flip and
/// sits on the facing edge at the start inset.
fn live_tooltip_beak(window: &Window) -> Beak {
    let mouse = window.mouse_position();
    let view = window.viewport_size();
    let room_below = f32::from(view.height) - f32::from(mouse.y);
    if room_below < 48. && f32::from(mouse.y) > 48. {
        Beak::tooltip(BeakSide::Bottom, BeakAlign::Start)
    } else {
        Beak::tooltip(BeakSide::Top, BeakAlign::Start)
    }
}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // GPUI shows tooltips next to the pointer; fade in like the web
        // frosted-panel enter. The beak is part of one closed outline with
        // the body (same fill and hairline) and follows a bottom flip.
        let beak = live_tooltip_beak(window);
        let pad = component::TOOLTIP_BEAK_HEIGHT + spacing::XX_SMALL;
        let below = beak.side == BeakSide::Top;
        div()
            .pl(px(4.))
            .when(below, |d| d.pt(px(pad)))
            .when(!below, |d| d.pb(px(pad)))
            .child(bubble(
                self.text.clone(),
                self.shortcut.clone(),
                Some(beak),
                cx,
            ))
            .with_animation(
                "refineui-tooltip-enter",
                // Web `useFloatingPresence`: fade + 6px slide away from the
                // trigger over `duration-fast` (emphasized).
                Motion::new(
                    crate::motion::duration::FAST,
                    crate::motion::easing::EMPHASIZED,
                )
                .animation(),
                move |d, t| {
                    let inset = pad - spacing::X_SMALL * (1. - t);
                    let d = d.opacity(t);
                    if below {
                        d.pt(px(inset))
                    } else {
                        d.pb(px(inset))
                    }
                },
            )
    }
}
