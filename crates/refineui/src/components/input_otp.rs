//! InputOTP — web `InputOTP`: a row of independent soft square cells.
//!
//! Cells (web `inputOtpStyles.slot`): `backgroundSurfaceHover` plate with a
//! transparent thin border, `controlHeight` square (36 / 44 / 52, default
//! `lg` = 52) with `roundedLarge / XLarge / XXLarge`, digits in the Foundation
//! title scale (`subTitle1 / title3 / title2` — lg is 28/36 semibold),
//! `sizeSmall` (8px) gap. Hover lightens to
//! `backgroundSurfaceActive`, the active cell shows a thin `borderDefault`
//! (both over `duration-fast`, emphasized). Empty cells pulse like Skeleton,
//! staggered by `index × duration-fast`; the pulse stops when filled, focused,
//! disabled, or under reduced motion.
//!
//! Each cell is focusable like the web's separate `<input>`s: clicking a cell
//! puts the caret there, typing writes at the caret and advances, Backspace
//! clears the cell (or the previous one when empty) and steps back, ←/→ move
//! between cells, ⌘V / Ctrl-V pastes from the caret. The focused cell shows
//! the thin `borderDefault`; an empty focused cell shows a 1px caret in
//! `foregroundPrimary`.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{motion::duration, radius, spacing, stroke, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, skeleton_pulse_opacity, use_focus_handle,
    use_interaction,
};
use crate::{ActiveTheme, ControlSize, StyledExt};

type ChangeHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// One-time-code input.
#[derive(IntoElement)]
pub struct InputOtp {
    id: ElementId,
    length: usize,
    size: ControlSize,
    disabled: bool,
    numeric: bool,
    default_value: SharedString,
    on_change: Option<ChangeHandler>,
    on_complete: Option<ChangeHandler>,
}

/// Web `InputOTP` default size (`lg`).
pub(crate) const OTP_DEFAULT_SIZE: ControlSize = ControlSize::Large;
/// Gap between slots (`sizeSmall`).
pub(crate) const OTP_GAP: f32 = spacing::SMALL;

/// Slot `(side, radius, digit style)`: a `controlHeight` square (36 / 44 / 52
/// at web density) with `roundedLarge / XLarge / XXLarge`.
pub(crate) fn otp_slot(
    density: refineui_tokens::size::Density,
    size: ControlSize,
) -> (f32, f32, refineui_tokens::typography::TextStyle) {
    let side = density.field(size).height;
    match size {
        ControlSize::Small => (side, radius::LARGE, role::SUBTITLE_LG),
        ControlSize::Medium => (side, radius::X_LARGE, role::TITLE_SM),
        ControlSize::Large => (side, radius::XX_LARGE, role::TITLE_MD),
    }
}

impl InputOtp {
    /// Six-cell numeric code input.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            length: 6,
            size: OTP_DEFAULT_SIZE,
            disabled: false,
            numeric: true,
            default_value: SharedString::default(),
            on_change: None,
            on_complete: None,
        }
    }

    /// Number of cells (web `maxLength`).
    pub fn length(mut self, n: usize) -> Self {
        self.length = n.max(1);
        self
    }

    /// `sm | md | lg` (web default `lg`).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Accept letters as well as digits.
    pub fn alphanumeric(mut self) -> Self {
        self.numeric = false;
        self
    }

    /// Initial value.
    pub fn default_value(mut self, v: impl Into<SharedString>) -> Self {
        self.default_value = v.into();
        self
    }

    /// Called on every change.
    pub fn on_change(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Called once every cell is filled.
    pub fn on_complete(
        mut self,
        f: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_complete = Some(Rc::new(f));
        self
    }
}

struct OtpState {
    value: String,
    /// Focused cell (web: which `<input>` has focus).
    cursor: usize,
}

impl RenderOnce for InputOtp {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let len = self.length;
        let initial: String = self.default_value.chars().take(len).collect();
        let start = initial.chars().count().min(len - 1);
        let state = window.use_keyed_state(child_id(&self.id, "otp"), cx, move |_, _| OtpState {
            value: initial,
            cursor: start,
        });
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = !self.disabled && focus.is_focused(window);
        let value: Vec<char> = state.read(cx).value.chars().collect();
        let active = state.read(cx).cursor.min(len - 1);
        let (cell, corner, text) = otp_slot(theme.density, self.size);
        let pulse = !self.disabled && !crate::motion::reduced_motion(cx);

        let mut cells = Vec::with_capacity(len);
        for ix in 0..len {
            let cid = ElementId::NamedChild(Box::new(self.id.clone()), format!("cell-{ix}").into());
            let (istate, i) = use_interaction(&cid, window, cx);
            let ch = value.get(ix).copied();
            let is_active = focused && ix == active;
            let bg = if self.disabled {
                c.background_surface_disabled
            } else if i.hovered && !is_active {
                c.background_surface_active
            } else {
                c.background_surface_hover
            };
            let border = if self.disabled {
                c.border_disabled
            } else if is_active {
                c.border_default
            } else {
                gpui::transparent_black()
            };
            let bg = animate(
                child_id(&cid, "bg"),
                bg,
                Motion::new(duration::FAST, refineui_tokens::motion::easing::EMPHASIZED),
                window,
                cx,
            );
            let border = animate(
                child_id(&cid, "bd"),
                border,
                Motion::new(duration::FAST, refineui_tokens::motion::easing::EMPHASIZED),
                window,
                cx,
            );
            let fh = focus.clone();
            let st_click = state.clone();
            let el = div()
                .id(cid.clone())
                .relative()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(cell))
                .rounded(px(corner))
                .border(px(stroke::THIN))
                .border_color(border)
                .bg(bg)
                .text_style_token(text)
                .text_color(if self.disabled {
                    c.foreground_disabled
                } else {
                    c.foreground_primary
                })
                .when(self.disabled, |d| d.cursor_not_allowed())
                .when(!self.disabled, |d| {
                    d.cursor_text()
                        .track_interaction(&istate)
                        .on_click(move |_, w, cx| {
                            st_click.update(cx, |s, cx| {
                                s.cursor = ix;
                                cx.notify();
                            });
                            fh.focus(w);
                        })
                })
                .when_some(ch, |d, ch| d.child(ch.to_string()))
                .when(ch.is_none() && is_active, |d| {
                    // Caret (`caret-color: foregroundPrimary`, native 1px bar
                    // spanning the digit line box).
                    d.child(
                        div()
                            .w(px(stroke::THIN))
                            .h(px(text.size * 1.15))
                            .bg(c.foreground_primary)
                            .with_animation(
                                child_id(&cid, "caret"),
                                Animation::new(Duration::from_millis(1060)).repeat(),
                                |d, t| d.opacity(if t < 0.5 { 1. } else { 0. }),
                            ),
                    )
                });
            let pulsing = pulse && ch.is_none() && !is_active;
            cells.push(if pulsing {
                // Stagger: delay of `ix × duration-fast`, expressed as a phase
                // offset into the shared skeleton loop.
                let phase = (ix as f32 * duration::FAST as f32) / duration::SKELETON as f32;
                el.with_animation(
                    child_id(&cid, "pulse"),
                    crate::motion::skeleton_pulse(),
                    move |d, t| d.opacity(skeleton_pulse_opacity((t + phase).fract())),
                )
                .into_any_element()
            } else {
                el.into_any_element()
            });
        }

        let st = state.clone();
        let on_change = self.on_change.clone();
        let on_complete = self.on_complete.clone();
        let numeric = self.numeric;
        let disabled = self.disabled;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(OTP_GAP))
            .font_family(theme.font_family.clone())
            .when(!disabled, |d| d.track_focus(&focus))
            .on_key_down(move |e: &gpui::KeyDownEvent, w, cx| {
                if disabled {
                    return;
                }
                let ks = &e.keystroke;
                let (cur_value, at) = {
                    let s = st.read(cx);
                    (s.value.clone(), s.cursor.min(len - 1))
                };
                let chars: Vec<char> = cur_value.chars().collect();
                let accept = |ch: char| {
                    if numeric {
                        ch.is_ascii_digit()
                    } else {
                        ch.is_ascii_alphanumeric()
                    }
                };
                let head = |n: usize| chars.iter().take(n).collect::<String>();
                let tail = |n: usize| chars.iter().skip(n).collect::<String>();
                // Web `setCharAt` / `clearAt` / `pasteAt` / `focusAt`.
                let (next, cursor) = if ks.key == "backspace" {
                    if at < chars.len() {
                        (head(at) + &tail(at + 1), at)
                    } else if at > 0 {
                        (head(at - 1) + &tail(at), at - 1)
                    } else {
                        (cur_value.clone(), at)
                    }
                } else if ks.key == "left" {
                    (cur_value.clone(), at.saturating_sub(1))
                } else if ks.key == "right" {
                    (cur_value.clone(), (at + 1).min(len - 1))
                } else if (ks.modifiers.platform || ks.modifiers.control) && ks.key == "v" {
                    let chunk: String = cx
                        .read_from_clipboard()
                        .and_then(|c| c.text())
                        .unwrap_or_default()
                        .chars()
                        .filter(|c| accept(*c))
                        .collect();
                    if chunk.is_empty() {
                        return;
                    }
                    let next: String = (head(at) + &chunk).chars().take(len).collect();
                    let n = next.chars().count();
                    (next, n.min(len - 1))
                } else if let Some(ch) = ks
                    .key_char
                    .as_deref()
                    .and_then(|s| s.chars().next())
                    .filter(|c| accept(*c))
                {
                    let at = at.min(chars.len());
                    let mut next = head(at);
                    next.push(ch);
                    (next, (at + 1).min(len - 1))
                } else {
                    return;
                };
                cx.stop_propagation();
                if cursor != at {
                    st.update(cx, |s, cx| {
                        s.cursor = cursor;
                        cx.notify();
                    });
                }
                if next == st.read(cx).value {
                    return;
                }
                st.update(cx, |s, cx| {
                    s.value = next.clone();
                    cx.notify();
                });
                let v: SharedString = next.clone().into();
                if let Some(f) = &on_change {
                    f(&v, w, cx);
                }
                if next.chars().count() == len
                    && let Some(f) = &on_complete
                {
                    f(&v, w, cx);
                }
            })
            .children(cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use refineui_tokens::size::Density;

    /// Pins web `InputOTP` slots so the size cannot regress silently.
    #[test]
    fn otp_slots_match_web_sizes() {
        assert_eq!(OTP_DEFAULT_SIZE, ControlSize::Large);
        assert_eq!(OTP_GAP, 8.);
        let rows = [
            (ControlSize::Small, 36., 8.),
            (ControlSize::Medium, 44., 12.),
            (ControlSize::Large, 52., 16.),
        ];
        for (size, side, r) in rows {
            let (s, rr, _) = otp_slot(Density::Comfortable, size);
            assert_eq!((s, rr), (side, r), "{size:?}");
        }
    }
}
