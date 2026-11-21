//! TimeInput — web `TimeInput` (`components/TimeInput/style.ts`): an Input-
//! styled trigger with a trailing `clock` icon that opens an Apple-style
//! wheel picker in an end-aligned popover.
//!
//! Panel (`p-large` popover, `gap-medium`): columns side by side
//! (`gap-xx-small`) over one shared selection band — `roundedLarge`,
//! `backgroundSurfaceHover`, 32px tall, vertically centered. Each column is
//! 80px wide and 140px tall (`timeInputWheelHeight`); rows are 32px,
//! `body1`, tabular, `foregroundSecondary` (selected: semibold
//! `foregroundPrimary`), and fade out toward both edges like the web mask
//! (`transparent → black 35% … 65% → transparent`). Scrolling settles on the
//! centered row after 120ms and selects it; clicking a row centers it.
//! `hour_cycle_12h` adds an AM/PM column; values stay `HH:mm` (24h).
//!
//! **Native entry** (like `<input type="time">`): the field itself holds
//! editable `HH`, `MM` (and `AM/PM`) segments. Click or Tab focuses a segment;
//! typing digits fills it and auto-advances once the segment is unambiguous
//! (`1` waits for a second hour digit, `3` becomes `03` and advances);
//! Up/Down step the segment (minutes by `minute_step`, wrapping); Left/Right
//! move between segments; Backspace clears the segment (`--`); `a` / `p` set
//! the period. The clock icon still opens the wheel picker.

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::components::menu::popup_state;
use crate::components::picker::{PickerField, picker_field, picker_popup};
use crate::motion::{Motion, animate, child_id};
use crate::{ActiveTheme, ControlSize, IconName, StyledExt};
use gpui::{FocusHandle, KeyDownEvent, MouseButton};

/// Row height (`foundationSize320`).
pub const TIME_ROW: f32 = 32.;
/// Wheel height (`timeInputWheelHeight` = `foundationSize1400`).
pub const TIME_WHEEL_HEIGHT: f32 = 140.;
/// Column width (`foundationSize800`).
pub const TIME_COLUMN_WIDTH: f32 = 80.;
/// Web `SCROLL_SETTLE_MS`.
const SETTLE: Duration = Duration::from_millis(120);

/// A 24-hour time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    /// 0..=23.
    pub hour: u32,
    /// 0..=59.
    pub minute: u32,
}

impl Time {
    /// New time (clamped).
    pub fn new(hour: u32, minute: u32) -> Self {
        Self {
            hour: hour.min(23),
            minute: minute.min(59),
        }
    }

    /// Parse `HH:mm`.
    pub fn parse(s: &str) -> Option<Self> {
        let (h, m) = s.split_once(':')?;
        Some(Self::new(h.trim().parse().ok()?, m.trim().parse().ok()?))
    }

    /// `HH:mm`.
    pub fn format(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// `hh:mm AM` (web `formatDisplay` for `12h`).
    pub fn format_12h(self) -> String {
        let h = if self.hour.is_multiple_of(12) {
            12
        } else {
            self.hour % 12
        };
        let p = if self.hour >= 12 { "PM" } else { "AM" };
        format!("{h:02}:{:02} {p}", self.minute)
    }
}

/// Opacity of a wheel row whose center is `y` px from the top of a `h`-tall
/// column — web mask `linear-gradient(transparent, black 35%, black 65%,
/// transparent)`.
pub fn wheel_fade(y: f32, h: f32) -> f32 {
    let t = (y / h).clamp(0., 1.);
    if t < 0.35 {
        t / 0.35
    } else if t > 0.65 {
        (1. - t) / 0.35
    } else {
        1.
    }
}

#[derive(Default)]
struct Wheel {
    last: f32,
    moved_at: Option<Instant>,
    synced: Option<usize>,
}

type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// One wheel column. `selected` is the index into `labels`.
fn column(
    id: ElementId,
    labels: Vec<SharedString>,
    selected: usize,
    on_pick: Pick,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let c = cx.theme().colors;
    let handle = window
        .use_keyed_state(child_id(&id, "scroll"), cx, |_, _| ScrollHandle::new())
        .read(cx)
        .clone();
    let wheel = window.use_keyed_state(child_id(&id, "wheel"), cx, |_, _| Wheel::default());
    let n = labels.len();
    let pad = (TIME_WHEEL_HEIGHT - TIME_ROW) / 2.;
    // Center the selected row (on open, and when the value changes elsewhere).
    if wheel.read(cx).synced != Some(selected) && wheel.read(cx).moved_at.is_none() {
        handle.set_offset(gpui::point(px(0.), px(-(selected as f32) * TIME_ROW)));
        let off = -(selected as f32) * TIME_ROW;
        wheel.update(cx, |w, _| {
            w.synced = Some(selected);
            w.last = off;
        });
    }
    let offset = f32::from(handle.offset().y);
    let now = Instant::now();
    // Settle: once the wheel has been still for 120ms, snap and select.
    let mut pick = None;
    wheel.update(cx, |w, _| {
        if (offset - w.last).abs() > 0.5 {
            w.last = offset;
            w.moved_at = Some(now);
        }
    });
    if let Some(t) = wheel.read(cx).moved_at {
        if now.duration_since(t) >= SETTLE {
            let ix = ((-offset / TIME_ROW).round().max(0.) as usize).min(n.saturating_sub(1));
            handle.set_offset(gpui::point(px(0.), px(-(ix as f32) * TIME_ROW)));
            wheel.update(cx, |w, _| {
                w.moved_at = None;
                w.last = -(ix as f32) * TIME_ROW;
                w.synced = Some(ix);
            });
            if ix != selected {
                pick = Some(ix);
            }
        }
        window.request_animation_frame();
    }
    if let Some(ix) = pick {
        on_pick(ix, window, cx);
    }
    let rows: Vec<_> = labels
        .into_iter()
        .enumerate()
        .map(|(ix, label)| {
            let center = pad + ix as f32 * TIME_ROW + TIME_ROW / 2. + offset;
            let fade = wheel_fade(center, TIME_WHEEL_HEIGHT);
            let is_sel = ix == selected;
            let pick = on_pick.clone();
            let handle = handle.clone();
            let wheel = wheel.clone();
            let row_id = ElementId::NamedChild(Box::new(id.clone()), format!("r{ix}").into());
            let fg = animate(
                child_id(&row_id, "fg"),
                if is_sel {
                    c.foreground_primary
                } else {
                    c.foreground_secondary
                },
                Motion::FAST,
                window,
                cx,
            );
            div()
                .id(row_id)
                .flex()
                .flex_none()
                .w_full()
                .h(px(TIME_ROW))
                .items_center()
                .justify_center()
                .px(px(spacing::SMALL))
                .text_color(fg)
                .when(is_sel, |d| d.font_weight(gpui::FontWeight::SEMIBOLD))
                .opacity(fade)
                .cursor_action()
                .hover(|s| s.text_color(c.foreground_primary))
                .on_click(move |_, w, cx| {
                    handle.set_offset(gpui::point(px(0.), px(-(ix as f32) * TIME_ROW)));
                    wheel.update(cx, |s, _| {
                        s.synced = Some(ix);
                        s.last = -(ix as f32) * TIME_ROW;
                        s.moved_at = None;
                    });
                    pick(ix, w, cx);
                })
                .child(label)
        })
        .collect();
    div()
        .id(id)
        .relative()
        .flex()
        .flex_col()
        .flex_none()
        .w(px(TIME_COLUMN_WIDTH))
        .h(px(TIME_WHEEL_HEIGHT))
        .overflow_y_scroll()
        .track_scroll(&handle)
        .child(div().flex_none().h(px(pad)))
        .children(rows)
        .child(div().flex_none().h(px(pad)))
        .into_any_element()
}

/// Editable segment of the native entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimeSegment {
    Hour,
    Minute,
    Period,
}

/// Native-entry state: active segment, a pending first digit, blank segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimeEntry {
    pub time: Time,
    pub seg: TimeSegment,
    pub pending: Option<u32>,
    /// `[hour, minute]` cleared with Backspace (shown as `--`).
    pub blank: [bool; 2],
}

impl TimeEntry {
    pub(crate) fn new(value: Option<Time>) -> Self {
        Self {
            time: value.unwrap_or_default(),
            seg: TimeSegment::Hour,
            pending: None,
            blank: [value.is_none(); 2],
        }
    }

    fn segments(twelve: bool) -> &'static [TimeSegment] {
        if twelve {
            &[TimeSegment::Hour, TimeSegment::Minute, TimeSegment::Period]
        } else {
            &[TimeSegment::Hour, TimeSegment::Minute]
        }
    }

    /// The committed value, if every segment is filled.
    pub(crate) fn value(&self) -> Option<Time> {
        (!self.blank[0] && !self.blank[1]).then_some(self.time)
    }

    /// Moves focus by `delta` segments; returns false at either end.
    pub(crate) fn step_segment(&mut self, delta: i32, twelve: bool) -> bool {
        let segs = Self::segments(twelve);
        let ix = segs.iter().position(|s| *s == self.seg).unwrap_or(0) as i32 + delta;
        self.pending = None;
        if ix < 0 || ix >= segs.len() as i32 {
            return false;
        }
        self.seg = segs[ix as usize];
        true
    }

    /// Types a digit into the active segment (native auto-advance rules).
    pub(crate) fn type_digit(&mut self, d: u32, twelve: bool) {
        match self.seg {
            TimeSegment::Hour => {
                let pm = self.time.hour >= 12;
                let (max_first, max) = if twelve { (1, 12) } else { (2, 23) };
                let shown = match self.pending.take() {
                    Some(first) => {
                        let v = first * 10 + d;
                        if v > max || (twelve && v == 0) {
                            d.max(u32::from(twelve))
                        } else {
                            v
                        }
                    }
                    None if d > max_first => d,
                    None => {
                        self.pending = Some(d);
                        d.max(u32::from(twelve && d == 0))
                    }
                };
                self.time.hour = if twelve {
                    shown % 12 + if pm { 12 } else { 0 }
                } else {
                    shown
                };
                self.blank[0] = false;
                if self.pending.is_none() {
                    self.step_segment(1, twelve);
                }
            }
            TimeSegment::Minute => {
                let m = match self.pending.take() {
                    Some(first) => first * 10 + d,
                    None if d > 5 => d,
                    None => {
                        self.pending = Some(d);
                        d
                    }
                };
                self.time.minute = m;
                self.blank[1] = false;
                if self.pending.is_none() {
                    self.step_segment(1, twelve);
                }
            }
            TimeSegment::Period => {}
        }
    }

    /// Up (+1) / Down (-1) on the active segment, wrapping.
    pub(crate) fn nudge(&mut self, dir: i32, minute_step: u32) {
        self.pending = None;
        match self.seg {
            TimeSegment::Hour => {
                self.time.hour = if self.blank[0] {
                    if dir > 0 { 0 } else { 23 }
                } else {
                    (self.time.hour as i32 + dir).rem_euclid(24) as u32
                };
                self.blank[0] = false;
            }
            TimeSegment::Minute => {
                let step = minute_step.max(1) as i32;
                self.time.minute = if self.blank[1] {
                    if dir > 0 { 0 } else { (60 - step) as u32 }
                } else {
                    let snapped = self.time.minute as i32 / step * step;
                    (snapped + dir * step).rem_euclid(60) as u32
                };
                self.blank[1] = false;
            }
            TimeSegment::Period => self.time.hour = (self.time.hour + 12) % 24,
        }
    }

    /// Backspace: drop the pending digit, else clear the segment.
    pub(crate) fn backspace(&mut self) {
        if self.pending.take().is_some() {
            return;
        }
        match self.seg {
            TimeSegment::Hour => self.blank[0] = true,
            TimeSegment::Minute => self.blank[1] = true,
            TimeSegment::Period => {}
        }
    }

    /// `a` / `p` on the period segment.
    pub(crate) fn set_period(&mut self, pm: bool) {
        let h = self.time.hour % 12;
        self.time.hour = if pm { h + 12 } else { h };
    }

    /// Display text for a segment.
    pub(crate) fn text(&self, seg: TimeSegment, twelve: bool) -> String {
        match seg {
            TimeSegment::Hour if self.blank[0] => "--".into(),
            TimeSegment::Minute if self.blank[1] => "--".into(),
            TimeSegment::Hour => {
                let h = self.time.hour;
                let h = if twelve {
                    if h.is_multiple_of(12) { 12 } else { h % 12 }
                } else {
                    h
                };
                if self.pending.is_some() && self.seg == TimeSegment::Hour {
                    format!("{h:>2}").replace(' ', "0")
                } else {
                    format!("{h:02}")
                }
            }
            TimeSegment::Minute => format!("{:02}", self.time.minute),
            TimeSegment::Period => if self.time.hour >= 12 { "PM" } else { "AM" }.into(),
        }
    }
}

/// A time field with native segment entry and a wheel picker.
#[derive(IntoElement)]
pub struct TimeInput {
    id: ElementId,
    value: Option<Time>,
    placeholder: SharedString,
    minute_step: u32,
    twelve_hour: bool,
    size: ControlSize,
    error: bool,
    success: bool,
    disabled: bool,
    full_width: bool,
    on_change: Option<Rc<dyn Fn(&Time, &mut Window, &mut App)>>,
}

impl TimeInput {
    /// New time input.
    pub fn new(id: impl Into<ElementId>, value: Option<Time>) -> Self {
        Self {
            id: id.into(),
            value,
            placeholder: "Select time".into(),
            minute_step: 1,
            twelve_hour: false,
            size: ControlSize::Large,
            error: false,
            success: false,
            disabled: false,
            full_width: false,
            on_change: None,
        }
    }

    /// Placeholder (default `Select time`).
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// Minute increments (1, 5, 10, 15 or 30).
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = step.clamp(1, 30);
        self
    }

    /// 12-hour picker with an AM/PM column (web `hourCycle="12h"`).
    pub fn twelve_hour(mut self, on: bool) -> Self {
        self.twelve_hour = on;
        self
    }

    /// Field size (default `Large`).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Error border.
    pub fn error(mut self, e: bool) -> Self {
        self.error = e;
        self
    }

    /// Success border.
    pub fn success(mut self, s: bool) -> Self {
        self.success = s;
        self
    }

    /// Disabled.
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }

    /// Stretch to the parent's width.
    pub fn full_width(mut self, f: bool) -> Self {
        self.full_width = f;
        self
    }

    /// Called whenever a wheel settles on a new value.
    pub fn on_change(mut self, f: impl Fn(&Time, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for TimeInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let open = popup_state(&self.id, window, cx);
        let is_open = *open.read(cx);
        // Draft follows the value; commits go straight to `on_change` (web `commit`).
        let draft_state = window.use_keyed_state(child_id(&self.id, "draft"), cx, |_, _| {
            self.value.unwrap_or_default()
        });
        if let Some(v) = self.value
            && *draft_state.read(cx) != v
        {
            draft_state.update(cx, |d, _| *d = v);
        }
        let draft = *draft_state.read(cx);
        let text = self.value.map(|v| {
            SharedString::from(if self.twelve_hour {
                v.format_12h()
            } else {
                v.format()
            })
        });
        let focus = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let entry_state = window.use_keyed_state(child_id(&self.id, "entry"), cx, |_, _| {
            TimeEntry::new(self.value)
        });
        // External value changes win over the in-progress entry.
        {
            let e = *entry_state.read(cx);
            if self.value.is_some() && e.value() != self.value && e.pending.is_none() {
                entry_state.update(cx, |e, _| {
                    let seg = e.seg;
                    *e = TimeEntry::new(self.value);
                    e.seg = seg;
                });
            }
        }
        let focused = focus.is_focused(window);
        let content = segments_row(
            &self.id,
            &entry_state,
            &focus,
            focused,
            self.twelve_hour,
            self.minute_step,
            self.disabled,
            self.on_change.clone(),
            cx,
        );
        let _ = text;
        let trigger = picker_field(
            PickerField {
                id: child_id(&self.id, "field"),
                size: self.size,
                text: None,
                placeholder: self.placeholder.clone(),
                icon: IconName::Clock,
                error: self.error,
                success: self.success,
                disabled: self.disabled,
                open: is_open || focused,
                full_width: self.full_width,
                content: Some(content),
            },
            window,
            cx,
        );
        let commit = {
            let on_change = self.on_change.clone();
            let draft_state = draft_state.clone();
            let entry_state = entry_state.clone();
            Rc::new(move |t: Time, w: &mut Window, cx: &mut App| {
                entry_state.update(cx, |e, _| {
                    e.time = t;
                    e.blank = [false; 2];
                    e.pending = None;
                });
                draft_state.update(cx, |d, cx| {
                    *d = t;
                    cx.notify();
                });
                if let Some(h) = &on_change {
                    h(&t, w, cx);
                }
            })
        };
        let panel = Some(()).map(|_| {
            let step = self.minute_step;
            let minutes: Vec<u32> = (0..60).step_by(step as usize).collect();
            let mut cols: Vec<AnyElement> = Vec::new();
            let twelve = self.twelve_hour;
            if twelve {
                let commit = commit.clone();
                let pm = draft.hour >= 12;
                cols.push(column(
                    child_id(&self.id, "period"),
                    vec!["AM".into(), "PM".into()],
                    pm as usize,
                    Rc::new(move |ix, w, cx| {
                        let h = draft.hour % 12 + if ix == 1 { 12 } else { 0 };
                        commit(Time::new(h, draft.minute), w, cx)
                    }),
                    window,
                    cx,
                ));
            }
            let hours: Vec<u32> = if twelve {
                (1..=12).collect()
            } else {
                (0..24).collect()
            };
            let shown_hour = if twelve {
                if draft.hour.is_multiple_of(12) {
                    12
                } else {
                    draft.hour % 12
                }
            } else {
                draft.hour
            };
            let hour_ix = hours.iter().position(|h| *h == shown_hour).unwrap_or(0);
            {
                let commit = commit.clone();
                let hours2 = hours.clone();
                cols.push(column(
                    child_id(&self.id, "hour"),
                    hours
                        .iter()
                        .map(|h| SharedString::from(format!("{h:02}")))
                        .collect(),
                    hour_ix,
                    Rc::new(move |ix, w, cx| {
                        let h = hours2[ix];
                        let h = if twelve {
                            h % 12 + if draft.hour >= 12 { 12 } else { 0 }
                        } else {
                            h
                        };
                        commit(Time::new(h, draft.minute), w, cx)
                    }),
                    window,
                    cx,
                ));
            }
            cols.push(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .text_style_token(role::BODY_LG)
                    .text_color(c.foreground_primary)
                    .child(":")
                    .into_any_element(),
            );
            let minute_ix = minutes.iter().position(|m| *m >= draft.minute).unwrap_or(0);
            {
                let commit = commit.clone();
                let minutes2 = minutes.clone();
                cols.push(column(
                    child_id(&self.id, "minute"),
                    minutes
                        .iter()
                        .map(|m| SharedString::from(format!("{m:02}")))
                        .collect(),
                    minute_ix,
                    Rc::new(move |ix, w, cx| commit(Time::new(draft.hour, minutes2[ix]), w, cx)),
                    window,
                    cx,
                ));
            }
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::MEDIUM))
                .child(
                    div()
                        .relative()
                        .flex()
                        .justify_center()
                        .gap(px(spacing::XX_SMALL))
                        .text_style_token(role::BODY_LG)
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px((TIME_WHEEL_HEIGHT - TIME_ROW) / 2.))
                                .h(px(TIME_ROW))
                                .rounded(px(radius::LARGE))
                                .bg(c.background_surface_hover),
                        )
                        .children(cols),
                )
                .into_any_element()
        });
        picker_popup(
            &self.id,
            &open,
            trigger.into_any_element(),
            if self.disabled { None } else { panel },
            spacing::LARGE,
            self.full_width,
            window,
            cx,
        )
    }
}

/// The editable `HH : MM [AM]` row inside the field.
fn segments_row(
    id: &ElementId,
    entry: &gpui::Entity<TimeEntry>,
    focus: &FocusHandle,
    focused: bool,
    twelve: bool,
    minute_step: u32,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&Time, &mut Window, &mut App)>>,
    cx: &App,
) -> AnyElement {
    let c = cx.theme().colors;
    let e = *entry.read(cx);
    let all_blank = e.blank == [true; 2];
    let seg_el = |seg: TimeSegment, cx: &App| {
        let active = focused && e.seg == seg;
        let entry = entry.clone();
        let focus = focus.clone();
        let blank = match seg {
            TimeSegment::Hour => e.blank[0],
            TimeSegment::Minute => e.blank[1],
            TimeSegment::Period => false,
        };
        let _ = cx;
        div()
            .id(child_id(
                id,
                match seg {
                    TimeSegment::Hour => "seg-hour",
                    TimeSegment::Minute => "seg-minute",
                    TimeSegment::Period => "seg-period",
                },
            ))
            .px(px(spacing::XX_SMALL))
            .rounded(px(radius::SMALL))
            .when(active, |d| {
                d.bg(c.background_brand_subtle)
                    .text_color(c.foreground_brand)
            })
            .when(!active && (blank || all_blank), |d| {
                d.text_color(c.foreground_placeholder)
            })
            .when(!disabled, |d| {
                d.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    window.focus(&focus);
                    entry.update(cx, |e, cx| {
                        e.seg = seg;
                        e.pending = None;
                        cx.notify();
                    });
                })
            })
            .child(e.text(seg, twelve))
    };
    let entry_k = entry.clone();
    div()
        .id(child_id(id, "segments"))
        .track_focus(focus)
        .flex()
        .items_center()
        .when(!disabled, |d| {
            d.on_key_down(move |ev: &KeyDownEvent, window, cx| {
                let key = ev.keystroke.key.as_str();
                let mut handled = true;
                let before = *entry_k.read(cx);
                let mut after = before;
                match key {
                    "up" => after.nudge(1, minute_step),
                    "down" => after.nudge(-1, minute_step),
                    "left" => handled = after.step_segment(-1, twelve),
                    "right" => handled = after.step_segment(1, twelve),
                    "tab" => {
                        handled = after
                            .step_segment(if ev.keystroke.modifiers.shift { -1 } else { 1 }, twelve)
                    }
                    "backspace" | "delete" => after.backspace(),
                    "a" if after.seg == TimeSegment::Period => after.set_period(false),
                    "p" if after.seg == TimeSegment::Period => after.set_period(true),
                    k if k.len() == 1 && k.as_bytes()[0].is_ascii_digit() => {
                        after.type_digit(u32::from(k.as_bytes()[0] - b'0'), twelve)
                    }
                    _ => handled = false,
                }
                if !handled {
                    return;
                }
                cx.stop_propagation();
                entry_k.update(cx, |e, cx| {
                    *e = after;
                    cx.notify();
                });
                if let (Some(v), Some(h)) = (after.value(), &on_change)
                    && before.value() != Some(v)
                {
                    h(&v, window, cx);
                }
            })
        })
        .child(seg_el(TimeSegment::Hour, cx))
        .child(div().child(":"))
        .child(seg_el(TimeSegment::Minute, cx))
        .when(twelve, |d| {
            d.child(div().w(px(spacing::XX_SMALL)))
                .child(seg_el(TimeSegment::Period, cx))
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_web_values() {
        let t = Time::parse("09:05").unwrap();
        assert_eq!(t.format(), "09:05");
        assert_eq!(Time::new(0, 30).format_12h(), "12:30 AM");
        assert_eq!(Time::new(13, 0).format_12h(), "01:00 PM");
    }

    #[test]
    fn native_entry_auto_advances() {
        let mut e = TimeEntry::new(None);
        e.type_digit(1, false); // waits: 1x could be 10-19
        assert_eq!((e.seg, e.pending), (TimeSegment::Hour, Some(1)));
        e.type_digit(5, false);
        assert_eq!((e.time.hour, e.seg), (15, TimeSegment::Minute));
        e.type_digit(7, false); // > 5: unambiguous
        assert_eq!(e.time.minute, 7);
        assert_eq!(e.value(), Some(Time::new(15, 7)));
        let mut e = TimeEntry::new(None);
        e.type_digit(3, false); // > 2: 03, advance
        assert_eq!((e.time.hour, e.seg), (3, TimeSegment::Minute));
        e.type_digit(4, false);
        e.type_digit(2, false);
        assert_eq!(e.time.minute, 42);
    }

    #[test]
    fn native_entry_arrows_and_backspace() {
        let mut e = TimeEntry::new(Some(Time::new(23, 58)));
        e.nudge(1, 1);
        assert_eq!(e.time.hour, 0);
        assert!(e.step_segment(1, false));
        e.nudge(1, 5); // snaps to the step then wraps
        assert_eq!(e.time.minute, 0);
        e.backspace();
        assert_eq!(e.value(), None);
        assert_eq!(e.text(TimeSegment::Minute, false), "--");
        assert!(!e.step_segment(1, false));
        let mut e = TimeEntry::new(Some(Time::new(9, 0)));
        e.seg = TimeSegment::Period;
        e.set_period(true);
        assert_eq!(e.time.hour, 21);
        e.seg = TimeSegment::Hour;
        e.type_digit(1, true);
        e.type_digit(1, true);
        assert_eq!(e.time.hour, 23); // 11 PM
    }

    #[test]
    fn wheel_fade_matches_the_mask() {
        assert_eq!(wheel_fade(70., 140.), 1.);
        assert_eq!(wheel_fade(0., 140.), 0.);
        assert!((wheel_fade(24.5, 140.) - 0.5).abs() < 0.01);
    }
}
