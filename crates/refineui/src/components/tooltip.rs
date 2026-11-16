//! Tooltip — web `Tooltip`. Attach with GPUI's `.tooltip(Tooltip::text("…"))`.

use gpui::{
    AnyView, App, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::*, px,
};

use gpui::AnimationExt;
use refineui_tokens::{radius, size::component, spacing, stroke, typography::role};

use crate::components::floating::{Beak, BeakAlign, BeakSide, floating_surface};
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
    div()
        .relative()
        .max_w(px(component::TOOLTIP_MAX_WIDTH))
        .flex()
        .items_center()
        .gap(px(spacing::X_SMALL))
        .px(px(spacing::MEDIUM + stroke::HAIRLINE))
        .py(px(spacing::SMALL + stroke::HAIRLINE))
        .rounded(px(radius::MEDIUM))
        .shadow(theme.shadow(Elevation::FLOATING))
        .child(floating_surface(
            radius::MEDIUM,
            beak,
            theme.floating_fill(),
            c.border_default,
        ))
        .text_color(c.foreground_primary)
        .font_family(theme.font_family.clone())
        .text_style_token(role::BODY_XS)
        .whitespace_nowrap()
        .child(text)
        .when_some(shortcut, |this, s| this.child(Kbd::new(s)))
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

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // GPUI shows tooltips next to the pointer; fade in like the web
        // frosted-panel enter (`duration-normal`, standard).
        // GPUI places tooltips just below-right of the pointer, so the beak
        // sits on the top edge at the start inset, pointing at the cursor.
        div()
            .pl(px(4.))
            .pt(px(component::TOOLTIP_BEAK_HEIGHT + spacing::XX_SMALL))
            .child(bubble(
                self.text.clone(),
                self.shortcut.clone(),
                Some(Beak::tooltip(BeakSide::Top, BeakAlign::Start)),
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
                |d, t| {
                    d.opacity(t)
                        .pt(px(component::TOOLTIP_BEAK_HEIGHT + spacing::XX_SMALL
                            - spacing::X_SMALL * (1. - t)))
                },
            )
    }
}
