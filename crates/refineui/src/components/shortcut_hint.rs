//! ShortcutHint — a keyboard shortcut in the platform's notation.
//!
//! Give it raw GPUI keys ([`ShortcutHint::keys`], `"secondary-shift-p"`) or
//! an action name ([`ShortcutHint::action`], looked up in the installed
//! [`crate::interaction::keymap::Keymap`]). It renders `⇧⌘P` on macOS and
//! `Ctrl+Shift+P` on Windows / Linux (following
//! [`crate::platform::chrome_os`]):
//!
//! * [`ShortcutStyle::Keycaps`] (default) — [`crate::Kbd`] caps: one cap
//!   per stroke on macOS (`⇧⌘P`), one per key elsewhere (`Ctrl` `Shift`
//!   `P`), chords separated by a gap.
//! * [`ShortcutStyle::Text`] — plain `bodyXs` tertiary text, the style of
//!   menu rows and status-bar hints.
//!
//! Nothing renders when an action has no binding.

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};
use refineui_foundation::{spacing, typography::role};

use crate::platform::Os;
use crate::{ActiveTheme, Kbd, StyledExt};

/// How a [`ShortcutHint`] draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShortcutStyle {
    /// Keycaps ([`crate::Kbd`]).
    #[default]
    Keycaps,
    /// Plain tertiary text.
    Text,
}

#[derive(Clone)]
enum Source {
    Keys(SharedString),
    Action(SharedString),
}

/// A platform-formatted shortcut. See the module docs.
#[derive(IntoElement, Clone)]
pub struct ShortcutHint {
    source: Source,
    style: ShortcutStyle,
    inverted: bool,
    os: Option<Os>,
}

impl ShortcutHint {
    /// Raw keys in GPUI syntax (`secondary-` = ⌘ on macOS, Ctrl elsewhere).
    pub fn keys(keys: impl Into<SharedString>) -> Self {
        Self {
            source: Source::Keys(keys.into()),
            style: ShortcutStyle::Keycaps,
            inverted: false,
            os: None,
        }
    }

    /// The shortcut bound to a GPUI action in the installed keymap.
    pub fn action(name: impl Into<SharedString>) -> Self {
        Self {
            source: Source::Action(name.into()),
            style: ShortcutStyle::Keycaps,
            inverted: false,
            os: None,
        }
    }

    /// Keycaps or text.
    pub fn style(mut self, style: ShortcutStyle) -> Self {
        self.style = style;
        self
    }

    /// Keycaps for dark surfaces (tooltips).
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    /// Format for this OS instead of [`crate::platform::chrome_os`].
    pub fn os(mut self, os: Os) -> Self {
        self.os = Some(os);
        self
    }

    /// The label this hint shows, or `None` when the action is unbound.
    pub fn label(&self, cx: &App) -> Option<String> {
        let os = self.os.unwrap_or_else(|| crate::platform::chrome_os(cx));
        let keys = match &self.source {
            Source::Keys(k) => k.to_string(),
            Source::Action(a) => crate::interaction::keymap::keys_for_action(a, cx)?,
        };
        Some(crate::platform::for_os(os).shortcut_label(&keys))
    }
}

/// Split a platform label into keycaps: per chord, one cap per stroke on
/// macOS (`⇧⌘P`), one per `+`-joined key elsewhere.
pub fn shortcut_caps(label: &str, os: Os) -> Vec<Vec<String>> {
    label
        .split(' ')
        .filter(|c| !c.is_empty())
        .map(|chord| match os {
            Os::MacOs => vec![chord.to_string()],
            _ => {
                // `Ctrl++` → ["Ctrl", "+"].
                let mut parts: Vec<String> = Vec::new();
                let mut rest = chord;
                while let Some(i) = rest.find('+') {
                    if i == 0 {
                        break;
                    }
                    parts.push(rest[..i].to_string());
                    rest = &rest[i + 1..];
                }
                if !rest.is_empty() {
                    parts.push(rest.to_string());
                }
                parts
            }
        })
        .collect()
}

impl RenderOnce for ShortcutHint {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(label) = self.label(cx) else {
            return div();
        };
        let c = cx.theme().colors;
        match self.style {
            ShortcutStyle::Text => div()
                .flex_none()
                .whitespace_nowrap()
                .text_style_token(role::BODY_XS)
                .text_color(if self.inverted {
                    c.foreground_inversed.opacity(0.75)
                } else {
                    c.foreground_tertiary
                })
                .child(label),
            ShortcutStyle::Keycaps => {
                let os = self.os.unwrap_or_else(|| crate::platform::chrome_os(cx));
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(spacing::X_SMALL))
                    .children(shortcut_caps(&label, os).into_iter().map(|chord| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::XXX_SMALL))
                            .children(
                                chord
                                    .into_iter()
                                    .map(|k| Kbd::new(k).inverted(self.inverted)),
                            )
                    }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_split_per_platform() {
        assert_eq!(shortcut_caps("⇧⌘P", Os::MacOs), vec![vec!["⇧⌘P"]]);
        assert_eq!(
            shortcut_caps("Ctrl+Shift+P", Os::Windows),
            vec![vec!["Ctrl", "Shift", "P"]]
        );
        assert_eq!(
            shortcut_caps("Ctrl+K Ctrl+S", Os::Linux),
            vec![vec!["Ctrl", "K"], vec!["Ctrl", "S"]]
        );
        assert_eq!(
            shortcut_caps("Ctrl++", Os::Windows),
            vec![vec!["Ctrl", "+"]]
        );
    }
}
