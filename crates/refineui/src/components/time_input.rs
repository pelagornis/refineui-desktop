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

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{radius, spacing, typography::role};

use crate::components::menu::popup_state;
use crate::components::picker::{PickerField, picker_field, picker_popup};
use crate::motion::{Motion, animate, child_id};
use crate::{ActiveTheme, ControlSize, IconName, StyledExt};

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

/// A time field with a wheel picker.
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
        let trigger = picker_field(
            PickerField {
                id: child_id(&self.id, "field"),
                size: self.size,
                text,
                placeholder: self.placeholder.clone(),
                icon: IconName::Clock,
                error: self.error,
                success: self.success,
                disabled: self.disabled,
                open: is_open,
                full_width: self.full_width,
            },
            window,
            cx,
        );
        let commit = {
            let on_change = self.on_change.clone();
            let draft_state = draft_state.clone();
            Rc::new(move |t: Time, w: &mut Window, cx: &mut App| {
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
    fn wheel_fade_matches_the_mask() {
        assert_eq!(wheel_fade(70., 140.), 1.);
        assert_eq!(wheel_fade(0., 140.), 0.);
        assert!((wheel_fade(24.5, 140.) - 0.5).abs() < 0.01);
    }
}
