//! ShortcutRecorder — a field that records a keyboard shortcut (desktop
//! addition, for preferences and keymap editors).
//!
//! Click (or focus and press Enter/Space) to start recording; the next
//! non-modifier keystroke with its modifiers becomes the shortcut. While
//! recording the field shows the held modifiers live. `Escape` cancels,
//! `Backspace`/`Delete` clears. The value is GPUI key syntax with the
//! platform primary modifier written as `secondary` (`secondary-shift-p`),
//! so it round-trips through [`crate::interaction::keymap::KeybindingManager`]; it is drawn with
//! the active platform's labels as [`crate::Kbd`] keycaps.
//!
//! With `chords(true)` a second keystroke pressed within
//! [`CHORD_TIMEOUT`] of the first is recorded as a chord (`ctrl-k ctrl-s`);
//! otherwise the first keystroke is committed when the timeout passes.
//!
//! A conflict with another binding in the installed [`crate::interaction::keymap::Keymap`] is
//! shown under the field (`conflict_check(true)`, on by default); the
//! binding for [`ShortcutRecorder::action`] itself is ignored.

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent, Modifiers,
    ModifiersChangedEvent, ParentElement, Render, SharedString, Styled, Window, div, prelude::*,
    px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::platform::Modifier;
use crate::{ActiveTheme, Icon, IconName, IconSize, Kbd, StyledExt};

/// How long a chord-enabled recorder waits for the second keystroke.
pub const CHORD_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1000);

/// Emitted when the recorded shortcut changes (`None` = cleared).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutChanged(pub Option<String>);

/// Shortcut recorder field (entity).
pub struct ShortcutRecorder {
    focus: FocusHandle,
    keys: Option<String>,
    recording: bool,
    held: Modifiers,
    placeholder: SharedString,
    action: Option<String>,
    conflict_check: bool,
    disabled: bool,
    chords: bool,
    pending: Option<String>,
    generation: u64,
}

impl EventEmitter<ShortcutChanged> for ShortcutRecorder {}

impl Focusable for ShortcutRecorder {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn mods_string(m: &Modifiers, primary: Modifier) -> Vec<&'static str> {
    let mut v = Vec::new();
    let (cmd_is_primary, ctrl_is_primary) = match primary {
        Modifier::Cmd => (true, false),
        Modifier::Ctrl => (false, true),
    };
    if m.control {
        v.push(if ctrl_is_primary { "secondary" } else { "ctrl" });
    }
    if m.alt {
        v.push("alt");
    }
    if m.shift {
        v.push("shift");
    }
    if m.platform {
        v.push(if cmd_is_primary { "secondary" } else { "cmd" });
    }
    v
}

/// GPUI key syntax for a keystroke, with the platform primary modifier
/// written as `secondary` (`secondary-shift-p`).
pub fn shortcut_keys(modifiers: &Modifiers, key: &str, primary: Modifier) -> String {
    let mut parts = mods_string(modifiers, primary);
    parts.push(key);
    parts.join("-").to_lowercase()
}

impl ShortcutRecorder {
    /// Empty recorder.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            keys: None,
            recording: false,
            held: Modifiers::default(),
            placeholder: "Record Shortcut".into(),
            action: None,
            conflict_check: true,
            disabled: false,
            chords: false,
            pending: None,
            generation: 0,
        }
    }

    /// Accept two-keystroke chords (`ctrl-k ctrl-s`); default off.
    pub fn chords(mut self, on: bool) -> Self {
        self.chords = on;
        self
    }

    /// Initial shortcut (GPUI key syntax).
    pub fn with_keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    /// Text when no shortcut is set.
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// The action this shortcut is for; its own binding is not a conflict.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Show conflicts with the installed keymap (default on).
    pub fn conflict_check(mut self, on: bool) -> Self {
        self.conflict_check = on;
        self
    }

    /// Disable.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }

    /// Current shortcut (GPUI key syntax).
    pub fn keys(&self) -> Option<&str> {
        self.keys.as_deref()
    }

    /// Whether the field is waiting for a keystroke.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Set the shortcut (emits [`ShortcutChanged`] when it changes).
    pub fn set_keys(&mut self, keys: Option<String>, cx: &mut Context<Self>) {
        if self.keys != keys {
            self.keys = keys.clone();
            cx.emit(ShortcutChanged(keys));
        }
        cx.notify();
    }

    /// Start (or stop) recording.
    pub fn set_recording(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.recording = on && !self.disabled;
        self.held = Modifiers::default();
        self.pending = None;
        self.generation += 1;
        if self.recording {
            window.focus(&self.focus);
        }
        cx.notify();
    }

    /// Action already bound to the current shortcut in the installed
    /// keymap (other than [`Self::action`]).
    pub fn conflict(&self, cx: &App) -> Option<String> {
        let keys = self.keys.as_ref()?;
        let os = crate::platform::chrome_os(cx);
        let km = cx.try_global::<crate::interaction::keymap::Keymap>()?;
        let want = crate::interaction::keymap::normalize(keys, os);
        km.0.bindings()
            .iter()
            .filter(|b| b.os.is_none_or(|o| o == os))
            .filter(|b| Some(&b.action) != self.action.as_ref())
            .find(|b| crate::interaction::keymap::normalize(&b.keys, os) == want)
            .map(|b| b.action.clone())
    }

    fn key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let ks = &ev.keystroke;
        let bare = !ks.modifiers.modified();
        if !self.recording {
            if bare && (ks.key == "enter" || ks.key == "space") {
                self.set_recording(true, window, cx);
                cx.stop_propagation();
            } else if bare && (ks.key == "backspace" || ks.key == "delete") {
                self.set_keys(None, cx);
                cx.stop_propagation();
            }
            return;
        }
        cx.stop_propagation();
        if bare && ks.key == "escape" {
            self.set_recording(false, window, cx);
            return;
        }
        if bare && (ks.key == "backspace" || ks.key == "delete") {
            self.set_keys(None, cx);
            self.set_recording(false, window, cx);
            return;
        }
        let primary = crate::platform::active(cx).primary_modifier();
        let stroke = shortcut_keys(&ks.modifiers, &ks.key, primary);
        if let Some(first) = self.pending.take() {
            self.set_keys(Some(format!("{first} {stroke}")), cx);
            self.set_recording(false, window, cx);
            return;
        }
        if self.chords {
            // Wait for an optional second keystroke.
            self.pending = Some(stroke);
            self.held = Modifiers::default();
            self.generation += 1;
            let generation = self.generation;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(CHORD_TIMEOUT).await;
                this.update(cx, |this, cx| {
                    if this.generation == generation && this.recording {
                        if let Some(first) = this.pending.take() {
                            this.set_keys(Some(first), cx);
                        }
                        this.recording = false;
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
            cx.notify();
            return;
        }
        self.set_keys(Some(stroke), cx);
        self.set_recording(false, window, cx);
    }
}

impl Render for ShortcutRecorder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.recording && !self.focus.is_focused(window) {
            // Focus moved away: stop recording (a pending first stroke of a
            // chord is kept as the shortcut).
            self.recording = false;
            if let Some(first) = self.pending.take() {
                self.keys = Some(first.clone());
                cx.emit(ShortcutChanged(Some(first)));
            }
        }
        let theme = cx.theme();
        let c = theme.colors;
        let adapter = crate::platform::active(cx);
        let focused = self.focus.is_focused(window);
        let conflict = if self.conflict_check && !self.recording && !self.disabled {
            self.conflict(cx)
        } else {
            None
        };
        let border = if conflict.is_some() {
            c.border_error
        } else if self.recording {
            c.border_focus
        } else {
            c.border_default
        };
        let label: Option<String> = if self.recording {
            let primary = adapter.primary_modifier();
            let held = mods_string(&self.held, primary);
            let held_label = (!held.is_empty()).then(|| {
                // Label of the modifiers alone (key part left blank).
                let s = adapter.shortcut_label(&format!("{}-", held.join("-")));
                s.trim_end_matches('+').to_string()
            });
            match (&self.pending, held_label) {
                (Some(p), Some(h)) => Some(format!("{} {h}", adapter.shortcut_label(p))),
                (Some(p), None) => Some(adapter.shortcut_label(p)),
                (None, h) => h,
            }
        } else {
            self.keys.as_ref().map(|k| adapter.shortcut_label(k))
        };
        // One keycap per stroke on macOS (`⇧⌘P`), one per key elsewhere
        // (`Ctrl` `Shift` `P`); chords are separated by a gap.
        let mac = adapter.os() == crate::platform::Os::MacOs;
        let strokes: Vec<Vec<String>> = match &label {
            Some(l) => l
                .split_whitespace()
                .map(|stroke| {
                    if mac {
                        vec![stroke.to_string()]
                    } else {
                        stroke.split('+').map(str::to_string).collect()
                    }
                })
                .collect(),
            None => Vec::new(),
        };
        let waiting = self.recording && self.pending.is_some();
        let has_keys = self.keys.is_some() && !self.recording;
        let fg_hint = if self.recording {
            c.foreground_brand
        } else {
            c.foreground_tertiary
        };
        let hint: SharedString = if self.recording {
            "Type Shortcut…".into()
        } else {
            self.placeholder.clone()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::XX_SMALL))
            .child(
                div()
                    .id("shortcut-recorder")
                    .track_focus(&self.focus)
                    .key_context("ShortcutRecorder")
                    .on_key_down(cx.listener(Self::key_down))
                    .on_modifiers_changed(cx.listener(|this, ev: &ModifiersChangedEvent, _, cx| {
                        if this.recording {
                            this.held = ev.modifiers;
                            cx.notify();
                        }
                    }))
                    .when(!self.disabled, |d| {
                        d.cursor_action()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let on = !this.recording;
                                this.set_recording(on, window, cx);
                            }))
                    })
                    .h(px(32.))
                    .min_w(px(160.))
                    .px(px(spacing::SMALL))
                    .flex()
                    .items_center()
                    .gap(px(spacing::X_SMALL))
                    .rounded(px(radius::MEDIUM))
                    .border(px(stroke::THIN))
                    .border_color(border)
                    .bg(if self.recording {
                        c.background_brand_subtle
                    } else {
                        c.background_surface
                    })
                    .when(focused && !self.recording, |d| {
                        d.shadow(theme.focus_ring_color(c.border_focus))
                    })
                    .when(self.disabled, |d| d.opacity(0.5))
                    .child(
                        Icon::new(IconName::Keyboard)
                            .size(IconSize::Custom(px(16.)))
                            .color(if self.recording {
                                c.foreground_brand
                            } else {
                                c.foreground_secondary
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(spacing::XXX_SMALL))
                            .when(strokes.is_empty(), |d| {
                                d.child(
                                    div()
                                        .text_style_token(role::BODY_SM)
                                        .text_color(fg_hint)
                                        .child(hint),
                                )
                            })
                            .children(strokes.into_iter().enumerate().map(|(i, caps)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::XXX_SMALL))
                                    .when(i > 0, |d| d.ml(px(spacing::X_SMALL)))
                                    .children(caps.into_iter().map(Kbd::new))
                            }))
                            .when(waiting, |d| {
                                d.child(
                                    div()
                                        .ml(px(spacing::XX_SMALL))
                                        .text_style_token(role::BODY_SM)
                                        .text_color(c.foreground_brand)
                                        .child("…"),
                                )
                            }),
                    )
                    .when(has_keys && !self.disabled, |d| {
                        d.child(
                            div()
                                .id("shortcut-recorder-clear")
                                .cursor_action()
                                .rounded(px(radius::SMALL))
                                .p(px(2.))
                                .hover(|s| s.bg(c.background_surface_hover))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.set_keys(None, cx);
                                }))
                                .child(
                                    Icon::new(IconName::Dismiss)
                                        .size(IconSize::Custom(px(12.)))
                                        .color(c.foreground_tertiary),
                                ),
                        )
                    }),
            )
            .when_some(conflict, |d, action| {
                d.child(
                    div()
                        .text_style_token(role::CAPTION_MD)
                        .text_color(c.foreground_error)
                        .child(format!("Already used by {action}")),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_use_secondary_for_the_primary_modifier() {
        let m = Modifiers {
            platform: true,
            shift: true,
            ..Default::default()
        };
        assert_eq!(shortcut_keys(&m, "P", Modifier::Cmd), "shift-secondary-p");
        let m = Modifiers {
            control: true,
            ..Default::default()
        };
        assert_eq!(shortcut_keys(&m, "k", Modifier::Ctrl), "secondary-k");
        assert_eq!(shortcut_keys(&m, "k", Modifier::Cmd), "ctrl-k");
    }
}
