//! Window root helper and global key bindings.

use gpui::{App, Div, KeyBinding, Styled, div, prelude::*};

use crate::ActiveTheme;

#[allow(missing_docs)]
mod actions {
    gpui::actions!(refineui, [FocusNext, FocusPrevious]);
}
pub use actions::{FocusNext, FocusPrevious};

pub(crate) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", FocusNext, None),
        KeyBinding::new("shift-tab", FocusPrevious, None),
    ]);
    crate::shell::ApplicationShell::bind_keys(cx);
    crate::shell::MenuBar::bind_keys(cx);
    crate::CommandPalette::bind_keys(cx);
}

pub(crate) fn set_focus_visible(visible: bool, window: &mut gpui::Window, cx: &mut App) {
    if crate::motion::focus_visible(cx) != visible {
        cx.set_global(crate::motion::FocusVisible(visible));
        window.refresh();
    }
}

/// A full-size root `div` for a window: theme background, text color, font,
/// Tab / Shift-Tab focus traversal, and click-outside blur. Use it as the outermost element of
/// your root view.
pub fn root(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .size_full()
        .bg(theme.colors.background_primary)
        .text_color(theme.colors.foreground_primary)
        .font_family(theme.font_family.clone())
        .text_size(gpui::px(refineui_tokens::typography::role::BODY_MD.size))
        .line_height(gpui::px(refineui_tokens::typography::role::BODY_MD.line_height))
        .on_action(|_: &FocusNext, window, _| window.focus_next())
        .on_action(|_: &FocusPrevious, window, _| window.focus_prev())
        // `:focus-visible` — rings follow keyboard navigation, not clicks.
        .capture_any_mouse_down(|_, window, cx| {
            set_focus_visible(false, window, cx);
            // Web click-outside blur: a press clears focus first; focusable
            // targets (inputs, triggers, buttons, dialog panels) take it back in
            // the bubble phase, so only presses on non-focusable areas blur.
            window.blur();
        })
        .capture_key_down(|event, window, cx| {
            let k = event.keystroke.key.as_str();
            if matches!(
                k,
                "tab" | "up" | "down" | "left" | "right" | "home" | "end" | "space" | "enter" | "f6"
                    | "f10"
            ) && !event.keystroke.modifiers.platform
            {
                set_focus_visible(true, window, cx);
            }
        })
}
