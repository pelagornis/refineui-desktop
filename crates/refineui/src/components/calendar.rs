//! Calendar — web `Calendar` (month grid; single date or a range across two
//! months).
//!
//! Root: `roundedXXLarge`, thin `borderDefault`, `backgroundPrimary`,
//! `p-medium` (10px); header `pb-small`; body `gap-xxx-small`; weekday row
//! `py-xx-small`. 32px day cells, `roundedLarge`; hover / active plates and a
//! 0.96 press scale over `duration-normal` (emphasized). Range mode shows two
//! months (`gap-xx-large`) with `subtitleMd` captions; the endpoints are
//! brand cells on a `backgroundPrimaryHover` band that also fills the days
//! between.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{radius, size::component, spacing, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, scaled_plate, use_focus_handle,
    use_interaction,
};
use crate::{
    ActiveTheme, Button, ButtonVariant, ControlSize, IconName, IconSize, Select, SelectAppearance,
    StyledExt,
};

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// A calendar date (proleptic Gregorian).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Year.
    pub year: i32,
    /// Month 1..=12.
    pub month: u32,
    /// Day 1..=31.
    pub day: u32,
}

impl Date {
    /// New date (not validated beyond clamping the day).
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            day: day.clamp(1, days_in_month(year, month)),
        }
    }

    /// Day of week, 0 = Sunday.
    pub fn weekday(self) -> u32 {
        // Sakamoto's algorithm.
        const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let y = if self.month < 3 {
            self.year - 1
        } else {
            self.year
        };
        ((y + y / 4 - y / 100 + y / 400 + T[(self.month - 1) as usize] + self.day as i32)
            .rem_euclid(7)) as u32
    }

    /// First day of the month `delta` months away.
    pub fn add_months(self, delta: i32) -> Self {
        let m0 = self.year * 12 + self.month as i32 - 1 + delta;
        Date::new(m0.div_euclid(12), (m0.rem_euclid(12) + 1) as u32, 1)
    }
}

/// Days in a month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Selection mode (web `CalendarMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarMode {
    /// One date.
    #[default]
    Single,
    /// A start and end date over two months.
    Range,
}

/// Range callback: `(start, end)`; `end` is `None` until the second pick.
pub type RangeHandler = Rc<dyn Fn(Option<Date>, Option<Date>, &mut Window, &mut App)>;

/// Next `(start, end)` after picking `d` (web `onRangeChange` rules): a
/// first pick (or a pick after a complete range) starts over; a second pick
/// before the start swaps the ends.
pub fn next_range(start: Option<Date>, end: Option<Date>, d: Date) -> (Option<Date>, Option<Date>) {
    match (start, end) {
        (Some(s), None) if d >= s => (Some(s), Some(d)),
        (Some(s), None) => (Some(d), Some(s)),
        _ => (Some(d), None),
    }
}

/// Month view with previous/next navigation. Uncontrolled month, controlled selection.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    mode: CalendarMode,
    selected: Option<Date>,
    range: (Option<Date>, Option<Date>),
    today: Option<Date>,
    initial_month: Date,
    on_select: Option<Rc<dyn Fn(&Date, &mut Window, &mut App)>>,
    on_range: Option<RangeHandler>,
    bare: bool,
}

impl Calendar {
    /// Calendar initially showing `month`.
    pub fn new(id: impl Into<ElementId>, month: Date) -> Self {
        Self {
            id: id.into(),
            mode: CalendarMode::Single,
            selected: None,
            range: (None, None),
            today: None,
            initial_month: Date::new(month.year, month.month, 1),
            on_select: None,
            on_range: None,
            bare: false,
        }
    }

    /// Selected date.
    pub fn selected(mut self, d: Option<Date>) -> Self {
        self.selected = d;
        self
    }

    /// Range mode with the current `start` / `end` (web `mode="range"`).
    pub fn range(mut self, start: Option<Date>, end: Option<Date>) -> Self {
        self.mode = CalendarMode::Range;
        self.range = (start, end);
        self
    }

    /// Highlight "today".
    pub fn today(mut self, d: Date) -> Self {
        self.today = Some(d);
        self
    }

    /// Date click.
    pub fn on_select(mut self, f: impl Fn(&Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// Range change (range mode), see [`next_range`].
    pub fn on_range_change(
        mut self,
        f: impl Fn(Option<Date>, Option<Date>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_range = Some(Rc::new(f));
        self
    }

    /// Drop the root border / fill (web `calendarContent` inside DateInput).
    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }
}

impl Calendar {
    fn month_rows(
        &self,
        month: Date,
        panel: usize,
        month_state: &gpui::Entity<Date>,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<gpui::Div> {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let range_mode = self.mode == CalendarMode::Range;
        let first_wd = month.weekday();
        let dim = days_in_month(month.year, month.month);
        let prev_month = month.add_months(-1);
        let next_month = month.add_months(1);
        let prev_dim = days_in_month(prev_month.year, prev_month.month);
        let day = px(component::CALENDAR_DAY);
        let show_ring = focus_visible(cx);

        // Leading / trailing days from the neighbouring months fill whole weeks.
        let mut grid: Vec<(Date, bool)> = Vec::new();
        for k in (0..first_wd).rev() {
            grid.push((
                Date::new(prev_month.year, prev_month.month, prev_dim - k),
                false,
            ));
        }
        for d in 1..=dim {
            grid.push((Date::new(month.year, month.month, d), true));
        }
        let mut k = 1;
        while !grid.len().is_multiple_of(7) {
            grid.push((Date::new(next_month.year, next_month.month, k), false));
            k += 1;
        }

        let (r_start, r_end) = self.range;
        let mut cells = Vec::with_capacity(grid.len());
        for (ix, (date, current)) in grid.into_iter().enumerate() {
            // Range mode leaves other-month cells empty (web `dayPlaceholder`).
            if range_mode && !current {
                cells.push(div().size(day).flex_none().into_any_element());
                continue;
            }
            let is_start = range_mode && r_start == Some(date);
            let is_end = range_mode && r_end == Some(date);
            let in_middle = range_mode
                && matches!((r_start, r_end), (Some(s), Some(e)) if date > s && date < e);
            let is_sel = if range_mode {
                is_start || is_end
            } else {
                self.selected == Some(date)
            };
            let is_today = self.today == Some(date);
            let id =
                ElementId::NamedChild(Box::new(self.id.clone()), format!("d{panel}-{ix}").into());
            let (state, i) = use_interaction(&id, window, cx);
            let focus = use_focus_handle(&id, 0, window, cx);
            let clear = c.background_primary.opacity(0.);
            let (bg, fg) = if is_sel {
                (c.background_brand, c.foreground_inversed)
            } else if i.pressed {
                (c.background_primary_active, c.foreground_primary)
            } else if i.hovered {
                (c.background_primary_hover, c.foreground_primary_hover)
            } else if in_middle {
                (c.background_primary_hover, c.foreground_primary)
            } else if current {
                (clear, c.foreground_primary)
            } else {
                (clear, c.foreground_disabled)
            };
            // Web day: `duration-normal` emphasized, `active: scale(0.96)`.
            let m = Motion::new(
                crate::motion::duration::NORMAL,
                crate::motion::easing::EMPHASIZED,
            );
            let bg = animate(child_id(&id, "bg"), bg, m, window, cx);
            let fg = animate(child_id(&id, "fg"), fg, m, window, cx);
            let scale = crate::motion::press_scale_to(
                child_id(&id, "press"),
                i.pressed,
                crate::motion::scale::BUTTON_PRESS,
                window,
                cx,
            );
            let focused = show_ring && focus.is_focused(window);
            let handler = self.on_select.clone();
            let on_range = self.on_range.clone();
            let nav = month_state.clone();
            let complete = r_start.is_some() && r_end.is_some() && r_start != r_end;
            // Endpoint band (web `dayWrapEndpoint`): the hover fill behind the
            // brand cell, rounded on the outer side only.
            let band = (is_start || is_end) && complete;
            let r = px(radius::LARGE);
            cells.push(
                div()
                    .size(day)
                    .flex_none()
                    .when(band, |d| {
                        d.bg(c.background_primary_hover)
                            .when(is_start, |d| d.rounded_tl(r).rounded_bl(r))
                            .when(is_end, |d| d.rounded_tr(r).rounded_br(r))
                    })
                    .when(in_middle, |d| d.bg(c.background_primary_hover))
                    .child(
                        div()
                            .id(id)
                            .relative()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(fg)
                            .child(
                                scaled_plate(scale)
                                    .rounded(px(if in_middle && !i.hovered {
                                        0.
                                    } else {
                                        radius::LARGE * scale
                                    }))
                                    .bg(bg),
                            )
                            .when(is_today && !is_sel, |d| {
                                d.font_weight(gpui::FontWeight::SEMIBOLD)
                            })
                            .when(focused, |d| d.rounded(r).shadow(theme.focus_ring()))
                            .cursor_action()
                            .track_focus(&focus)
                            .track_interaction(&state)
                            .on_click(move |_, w, cx| {
                                if !current {
                                    nav.update(cx, |m, cx| {
                                        *m = Date::new(date.year, date.month, 1);
                                        cx.notify();
                                    });
                                }
                                if let Some(h) = &on_range {
                                    let (s, e) = next_range(r_start, r_end, date);
                                    h(s, e, w, cx);
                                }
                                if let Some(h) = &handler {
                                    h(&date, w, cx)
                                }
                            })
                            .child(div().relative().child(date.day.to_string())),
                    )
                    .into_any_element(),
            );
        }
        let mut week_rows = Vec::new();
        let mut it = cells.into_iter().peekable();
        while it.peek().is_some() {
            week_rows.push(div().flex().children(it.by_ref().take(7)));
        }
        week_rows
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let initial = self.initial_month;
        let month_state =
            window.use_keyed_state(child_id(&self.id, "month"), cx, move |_, _| initial);
        let month = *month_state.read(cx);
        let day = px(component::CALENDAR_DAY);

        let nav_button = |name: &'static str, icon: IconName, delta: i32| {
            let state = month_state.clone();
            Button::icon_only(child_id(&self.id, name), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Small)
                .icon_size(IconSize::Small)
                .on_click(move |_, _, cx| {
                    state.update(cx, |m, cx| {
                        *m = m.add_months(delta);
                        cx.notify();
                    })
                })
        };
        let weekdays = || {
            div().flex().children(WEEKDAYS.into_iter().map(|w| {
                div()
                    .w(day)
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .py(px(spacing::XX_SMALL))
                    .text_color(c.foreground_primary)
                    .child(w)
            }))
        };
        let body = |rows: Vec<gpui::Div>| {
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::XXX_SMALL))
                .text_style_token(role::CAPTION_LG)
                .child(weekdays())
                .children(rows)
        };
        let root = div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .w_auto()
            .p(px(spacing::MEDIUM))
            .rounded(px(radius::XX_LARGE))
            .when(!self.bare, |d| {
                d.border_1()
                    .border_color(c.border_default)
                    .bg(c.background_primary)
            })
            .text_color(c.foreground_primary);

        if self.mode == CalendarMode::Range {
            let second = month.add_months(1);
            let rows0 = self.month_rows(month, 0, &month_state, window, cx);
            let rows1 = self.month_rows(second, 1, &month_state, window, cx);
            let caption = |m: Date| {
                div()
                    .flex_1()
                    .min_w_0()
                    .text_center()
                    .text_style_token(role::SUBTITLE_MD)
                    .text_color(c.foreground_primary)
                    .child(format!("{} {}", MONTHS[(m.month - 1) as usize], m.year))
            };
            let spacer = || {
                div()
                    .flex_none()
                    .size(px(theme.button_metrics(ControlSize::Small).height))
            };
            return root.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(spacing::XX_LARGE))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .w_full()
                                    .items_center()
                                    .pb(px(spacing::SMALL))
                                    .child(nav_button("prev", IconName::ChevronLeft, -1))
                                    .child(caption(month))
                                    .child(spacer()),
                            )
                            .child(body(rows0)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .w_full()
                                    .items_center()
                                    .pb(px(spacing::SMALL))
                                    .child(spacer())
                                    .child(caption(second))
                                    .child(nav_button("next", IconName::ChevronRight, 1)),
                            )
                            .child(body(rows1)),
                    ),
            );
        }

        let rows = self.month_rows(month, 0, &month_state, window, cx);
        let set_month = month_state.clone();
        let set_year = month_state.clone();
        let years: Vec<i32> = (month.year - 10..=month.year + 10).collect();
        let year_ix = years.iter().position(|y| *y == month.year);
        let years_for_change = years.clone();
        root.min_w(px(component::CALENDAR_MIN_WIDTH))
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .pb(px(spacing::SMALL))
                    .child(nav_button("prev", IconName::ChevronLeft, -1))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(
                                Select::new(
                                    child_id(&self.id, "month-select"),
                                    MONTHS.iter().map(|m| SharedString::from(&m[..3])),
                                    Some((month.month - 1) as usize),
                                )
                                .appearance(SelectAppearance::Ghost)
                                .size(ControlSize::Small)
                                .on_change(move |ix, _, cx| {
                                    set_month.update(cx, |m, cx| {
                                        *m = Date::new(m.year, *ix as u32 + 1, 1);
                                        cx.notify();
                                    })
                                }),
                            )
                            .child(
                                Select::new(
                                    child_id(&self.id, "year-select"),
                                    years.iter().map(|y| SharedString::from(y.to_string())),
                                    year_ix,
                                )
                                .appearance(SelectAppearance::Ghost)
                                .size(ControlSize::Small)
                                .on_change(move |ix, _, cx| {
                                    let y = years_for_change[*ix];
                                    set_year.update(cx, |m, cx| {
                                        *m = Date::new(y, m.month, 1);
                                        cx.notify();
                                    })
                                }),
                            ),
                    )
                    .child(nav_button("next", IconName::ChevronRight, 1)),
            )
            .child(body(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekday_and_months() {
        assert_eq!(Date::new(2026, 11, 10).weekday(), 2); // Tuesday
        assert_eq!(Date::new(2024, 1, 1).weekday(), 1); // Monday
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(Date::new(2026, 1, 5).add_months(-1), Date::new(2025, 12, 1));
    }

    #[test]
    fn range_picks_follow_the_web_rules() {
        let (a, b) = (Date::new(2026, 11, 3), Date::new(2026, 11, 9));
        assert_eq!(next_range(None, None, a), (Some(a), None));
        assert_eq!(next_range(Some(a), None, b), (Some(a), Some(b)));
        assert_eq!(next_range(Some(b), None, a), (Some(a), Some(b)));
        assert_eq!(next_range(Some(a), Some(b), b), (Some(b), None));
    }
}
