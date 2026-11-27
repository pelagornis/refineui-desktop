//! DateInput — web `DateInput` (`components/DateInput/style.ts`): an Input-
//! styled trigger that opens a [`crate::Calendar`] in an end-aligned popover.
//!
//! The trigger is the Input shell (`formControlInputSizeClass`, default `lg`)
//! with the value (`Nov 10, 2026`) or placeholder and a trailing 20px
//! `calendar` icon in `foregroundSecondary`. Without `full_width` the
//! trigger hugs its text; with it, it fills the parent (put it in a
//! [`crate::Field`] for a label). The popover opens below, end-aligned, and
//! hugs the calendar (no padding, calendar drawn without its own border /
//! fill). `range` mode opens the two-month range calendar and shows
//! `start – end` (`…` until the end is picked; a reversed range is
//! swapped). Like the web, picking a date keeps the calendar open; a click
//! outside (or on the trigger) closes it.

use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use crate::components::menu::popup_state;
use crate::components::picker::{PickerField, picker_field, picker_popup};
use crate::motion::child_id;
use crate::{Calendar, ControlSize, Date, IconName, RangeHandler};

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `Nov 10, 2026` (web `Intl.DateTimeFormat(…, { year, month: "short", day })` in `en`).
pub fn format_date(d: Date) -> String {
    format!(
        "{} {}, {}",
        MONTHS_SHORT[(d.month - 1) as usize],
        d.day,
        d.year
    )
}

/// A date (or date range) field.
#[derive(IntoElement)]
pub struct DateInput {
    id: ElementId,
    range_mode: bool,
    value: Option<Date>,
    range: (Option<Date>, Option<Date>),
    month: Option<Date>,
    placeholder: Option<SharedString>,
    size: ControlSize,
    error: bool,
    success: bool,
    disabled: bool,
    full_width: bool,
    week_starts_on: u32,
    on_change: Option<Rc<dyn Fn(&Date, &mut Window, &mut App)>>,
    on_range: Option<RangeHandler>,
}

impl DateInput {
    /// Single-date input.
    pub fn new(id: impl Into<ElementId>, value: Option<Date>) -> Self {
        Self {
            id: id.into(),
            range_mode: false,
            value,
            range: (None, None),
            month: None,
            placeholder: None,
            size: ControlSize::Large,
            error: false,
            success: false,
            disabled: false,
            full_width: false,
            week_starts_on: 0,
            on_change: None,
            on_range: None,
        }
    }

    /// Range input (web `mode="range"`).
    pub fn range(id: impl Into<ElementId>, start: Option<Date>, end: Option<Date>) -> Self {
        let mut this = Self::new(id, None);
        this.range_mode = true;
        this.range = (start, end);
        this
    }

    /// Month shown when nothing is selected (defaults to the value).
    pub fn month(mut self, month: Date) -> Self {
        self.month = Some(month);
        self
    }

    /// Placeholder (`Select date` / `Select date range`).
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = Some(p.into());
        self
    }

    /// Field size (default `Large`, like the web).
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

    /// First weekday column (web `weekStartsOn`): 0 = Sunday (default), 1 = Monday.
    pub fn week_starts_on(mut self, day: u32) -> Self {
        self.week_starts_on = day % 7;
        self
    }

    /// Picked date (single mode).
    pub fn on_change(mut self, f: impl Fn(&Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Range change (range mode).
    pub fn on_range_change(
        mut self,
        f: impl Fn(Option<Date>, Option<Date>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_range = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for DateInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = popup_state(&self.id, window, cx);
        let is_open = *open.read(cx);
        let (start, end) = match self.range {
            (Some(s), Some(e)) if e < s => (Some(e), Some(s)),
            r => r,
        };
        let text: Option<SharedString> = if self.range_mode {
            start.map(|s| {
                format!(
                    "{} – {}",
                    format_date(s),
                    end.map(format_date).unwrap_or_else(|| "…".into())
                )
                .into()
            })
        } else {
            self.value.map(|d| format_date(d).into())
        };
        let placeholder = self.placeholder.clone().unwrap_or_else(|| {
            if self.range_mode {
                "Select date range".into()
            } else {
                "Select date".into()
            }
        });
        let trigger = picker_field(
            PickerField {
                id: child_id(&self.id, "field"),
                size: self.size,
                text,
                placeholder,
                icon: IconName::Calendar,
                error: self.error,
                success: self.success,
                disabled: self.disabled,
                open: is_open,
                full_width: self.full_width,
                content: None,
            },
            window,
            cx,
        );
        let month = self
            .month
            .or(if self.range_mode { start } else { self.value })
            .unwrap_or(Date::new(2026, 1, 1));
        // The web keeps the popover open after a pick; it closes on an
        // outside click.
        let calendar = Calendar::new(child_id(&self.id, "calendar"), month)
            .week_starts_on(self.week_starts_on)
            .bare(true);
        let calendar = if self.range_mode {
            let on_range = self.on_range.clone();
            calendar
                .range(start, end)
                .on_range_change(move |s, e, w, cx| {
                    if let Some(h) = &on_range {
                        h(s, e, w, cx);
                    }
                })
        } else {
            let on_change = self.on_change.clone();
            calendar.selected(self.value).on_select(move |d, w, cx| {
                if let Some(h) = &on_change {
                    h(d, w, cx);
                }
            })
        };
        let panel = (!self.disabled).then(|| calendar.into_any_element());
        if self.disabled && is_open {
            open.update(cx, |o, _| *o = false);
        }
        picker_popup(
            &self.id,
            &open,
            trigger.into_any_element(),
            panel,
            0.,
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
    fn formats_like_intl_short() {
        assert_eq!(format_date(Date::new(2026, 11, 10)), "Nov 10, 2026");
    }
}
