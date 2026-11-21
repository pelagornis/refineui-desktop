//! Chart — web `Chart` (`components/Chart`): a bar or (monotone) line chart
//! with a y-axis, grid, x labels, a hover cursor and tooltip.
//!
//! Geometry follows the web viewBox (800 × 320, `aspect 5/2`): plot padding
//! top 10 / bottom 4 / x 10, bars fill half their band with a 12px
//! (`roundedXLarge`) top radius, a monotone (Fritsch–Carlson) line with a
//! 2px stroke, an area that fades from 50% to 0, and 4px points with a 2px
//! `backgroundPrimary` ring. The scale max is the web `niceMax` (1/2/5/10 ×
//! 10ⁿ). Grid lines are 1px `borderSubtle` at 50% (the baseline
//! `borderDefault`, full). Axis labels `captionSm` tertiary (x: `mt-small`,
//! 24px tall; y: min 40px, `pe-x-small`); the hovered x label turns
//! primary. Bars grow from the baseline and the line draws in over
//! `duration-panel` (emphasized). Tooltip: `roundedMedium`, thin
//! `borderDefault`, `surfacePopover`, `px-medium` / `py-small`.

use gpui::{
    App, Bounds, ElementId, Hsla, IntoElement, MouseMoveEvent, ParentElement, PathBuilder, Pixels,
    Point, RenderOnce, SharedString, Styled, Window, canvas, div, linear_color_stop,
    linear_gradient, point, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::motion::{Motion, animate, animate_from, child_id, probe, use_layout_probe};
use crate::{ActiveTheme, Elevation, StyledExt};

const VB_W: f32 = 800.;
const VB_H: f32 = 320.;
const PAD_TOP: f32 = 10.;
const PAD_BOTTOM: f32 = 4.;
const PAD_X: f32 = 10.;
const BAR_FILL: f32 = 0.5;

/// Series color (web `ChartSeriesColor` → `background*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChartSeries {
    /// `backgroundBrand`.
    #[default]
    Brand,
    /// `backgroundInfo`.
    Info,
    /// `backgroundSuccess`.
    Success,
    /// `backgroundWarning`.
    Warning,
    /// `backgroundError`.
    Error,
    /// `backgroundDiscovery`.
    Discovery,
}

/// Bar or line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChartKind {
    /// Vertical bars (web `ChartBars`).
    #[default]
    Bar,
    /// Monotone line (web `ChartLine`).
    Line,
}

/// Web `niceMax`: round the scale maximum up to 1 / 2 / 5 / 10 × 10ⁿ.
pub fn nice_max(raw: f32) -> f32 {
    if raw <= 0. {
        return 1.;
    }
    let mag = 10f32.powf(raw.log10().floor());
    let n = raw / mag;
    let unit = if n <= 1. {
        1.
    } else if n <= 2. {
        2.
    } else if n <= 5. {
        5.
    } else {
        10.
    };
    unit * mag
}

/// Web `formatAxisTick`.
pub fn format_tick(v: f32) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

/// Fritsch–Carlson tangents for monotone cubic interpolation (web `monotoneLinePath`).
pub fn monotone_tangents(pts: &[(f32, f32)]) -> Vec<f32> {
    let n = pts.len();
    if n < 2 {
        return vec![0.; n];
    }
    let mut dx = vec![0.; n - 1];
    let mut m = vec![0.; n - 1];
    for i in 0..n - 1 {
        dx[i] = pts[i + 1].0 - pts[i].0;
        let dy = pts[i + 1].1 - pts[i].1;
        m[i] = if dx[i] != 0. { dy / dx[i] } else { 0. };
    }
    let mut d = vec![0.; n];
    d[0] = m[0];
    for i in 1..n - 1 {
        d[i] = if m[i - 1] * m[i] <= 0. {
            0.
        } else {
            let span = dx[i - 1] + dx[i];
            if span != 0. {
                3. * span / ((span + dx[i]) / m[i - 1] + (span + dx[i - 1]) / m[i])
            } else {
                0.
            }
        };
    }
    d[n - 1] = m[n - 2];
    d
}

/// A small chart.
#[derive(IntoElement)]
pub struct Chart {
    id: ElementId,
    kind: ChartKind,
    values: Vec<f32>,
    labels: Vec<SharedString>,
    series: ChartSeries,
    lines: usize,
    max: Option<f32>,
    area: bool,
    points: bool,
}

impl Chart {
    /// Bar chart of `values` with x `labels`.
    pub fn bars(
        id: impl Into<ElementId>,
        values: impl IntoIterator<Item = f32>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: ChartKind::Bar,
            values: values.into_iter().collect(),
            labels: labels.into_iter().map(Into::into).collect(),
            series: ChartSeries::Brand,
            lines: 5,
            max: None,
            area: true,
            points: true,
        }
    }

    /// Line chart of `values` with x `labels`.
    pub fn line(
        id: impl Into<ElementId>,
        values: impl IntoIterator<Item = f32>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let mut c = Self::bars(id, values, labels);
        c.kind = ChartKind::Line;
        c
    }

    /// Series color.
    pub fn series(mut self, s: ChartSeries) -> Self {
        self.series = s;
        self
    }

    /// Grid / tick count (default 5).
    pub fn lines(mut self, n: usize) -> Self {
        self.lines = n.max(2);
        self
    }

    /// Fixed scale max (default: `nice_max` of the values).
    pub fn max(mut self, m: f32) -> Self {
        self.max = Some(m);
        self
    }

    /// Line: fill the area under the line (default on).
    pub fn area(mut self, on: bool) -> Self {
        self.area = on;
        self
    }

    /// Line: draw points (default on).
    pub fn points(mut self, on: bool) -> Self {
        self.points = on;
        self
    }
}

fn series_color(s: ChartSeries, cx: &App) -> Hsla {
    let c = &cx.theme().colors;
    match s {
        ChartSeries::Brand => c.background_brand,
        ChartSeries::Info => c.background_info,
        ChartSeries::Success => c.background_success,
        ChartSeries::Warning => c.background_warning,
        ChartSeries::Error => c.background_error,
        ChartSeries::Discovery => c.background_discovery,
    }
}

impl RenderOnce for Chart {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let n = self.values.len().max(1);
        let peak = self.values.iter().fold(0f32, |a, v| a.max(v.max(0.)));
        let scale_max = self.max.filter(|m| *m > 0.).unwrap_or(nice_max(peak));
        let ticks: Vec<f32> = (0..self.lines)
            .map(|i| scale_max / (self.lines - 1) as f32 * i as f32)
            .collect();
        let plot = use_layout_probe(&self.id, window, cx);
        let plot_bounds = plot.read(cx).container;
        let hover = window.use_keyed_state(child_id(&self.id, "hover"), cx, |_, _| None::<usize>);
        let hovered = *hover.read(cx);
        let grow = animate_from(
            child_id(&self.id, "enter"),
            0.0f32,
            1.0,
            Motion::new(
                crate::motion::duration::PANEL,
                crate::motion::easing::EMPHASIZED,
            ),
            window,
            cx,
        );
        let color = series_color(self.series, cx);
        let kind = self.kind;
        let values = self.values.clone();
        let (area, points) = (self.area, self.points);
        let (grid, base, bg) = (c.border_subtle, c.border_default, c.background_primary);
        let cursor = c.background_surface_hover;
        let cursor_a = animate(
            child_id(&self.id, "cursor"),
            if hovered.is_some() { 1.0f32 } else { 0.0 },
            Motion::FAST,
            window,
            cx,
        );
        let ticks_paint = ticks.clone();
        let paint = move |b: Bounds<Pixels>, window: &mut Window| {
            let sx = f32::from(b.size.width) / VB_W;
            let sy = f32::from(b.size.height) / VB_H;
            let at = |x: f32, y: f32| -> Point<Pixels> {
                point(b.origin.x + px(x * sx), b.origin.y + px(y * sy))
            };
            let plot_h = VB_H - PAD_TOP - PAD_BOTTOM;
            let y_of = |v: f32| PAD_TOP + plot_h - (v.max(0.) / scale_max) * plot_h;
            let band = (VB_W - PAD_X * 2.) / n as f32;
            // Hover cursor band (bars) / rule (line).
            if let Some(ix) = hovered
                && kind == ChartKind::Bar
            {
                let x = PAD_X + ix as f32 * band;
                window.paint_quad(
                    gpui::fill(
                        Bounds::from_corners(at(x, PAD_TOP), at(x + band, VB_H - PAD_BOTTOM)),
                        cursor.opacity(cursor.a * cursor_a),
                    )
                    .corner_radii(px(radius::LARGE)),
                );
            }
            for (i, t) in ticks_paint.iter().enumerate() {
                let y = y_of(*t);
                let baseline = i == 0;
                let col = if baseline {
                    base
                } else {
                    grid.opacity(grid.a * 0.5)
                };
                window.paint_quad(gpui::fill(
                    Bounds::new(
                        at(PAD_X, y),
                        gpui::size(px((VB_W - PAD_X * 2.) * sx), px(stroke::THIN)),
                    ),
                    col,
                ));
            }
            match kind {
                ChartKind::Bar => {
                    let w = band * BAR_FILL;
                    for (i, v) in values.iter().enumerate() {
                        let x = PAD_X + i as f32 * band + (band - w) / 2.;
                        let top = y_of(*v);
                        let h = (VB_H - PAD_BOTTOM - top) * grow;
                        if h <= 0. {
                            continue;
                        }
                        let y = VB_H - PAD_BOTTOM - h;
                        let r = (radius::X_LARGE / sx.max(0.01)).min(w / 2.).min(h);
                        let mut p = PathBuilder::fill();
                        p.move_to(at(x, y + h));
                        p.line_to(at(x, y + r));
                        p.curve_to(at(x + r, y), at(x, y));
                        p.line_to(at(x + w - r, y));
                        p.curve_to(at(x + w, y + r), at(x + w, y));
                        p.line_to(at(x + w, y + h));
                        p.close();
                        let dim = hovered.is_some_and(|h| h != i);
                        if let Ok(path) = p.build() {
                            window.paint_path(
                                path,
                                if dim {
                                    color.opacity(color.a * 0.5)
                                } else {
                                    color
                                },
                            );
                        }
                    }
                }
                ChartKind::Line => {
                    let step = if n == 1 {
                        0.
                    } else {
                        (VB_W - PAD_X * 2.) / (n - 1) as f32
                    };
                    let pts: Vec<(f32, f32)> = values
                        .iter()
                        .enumerate()
                        .map(|(i, v)| (PAD_X + step * i as f32, y_of(*v)))
                        .collect();
                    if pts.is_empty() {
                        return;
                    }
                    let d = monotone_tangents(&pts);
                    // Draw-in: only the first `grow` of the x range.
                    let reveal = PAD_X + (VB_W - PAD_X * 2.) * grow;
                    let curve = |p: &mut PathBuilder| {
                        p.move_to(at(pts[0].0, pts[0].1));
                        for i in 0..pts.len().saturating_sub(1) {
                            let (x1, y1) = pts[i];
                            let (x2, y2) = pts[i + 1];
                            if x1 >= reveal {
                                break;
                            }
                            let h = x2 - x1;
                            p.cubic_bezier_to(
                                at(x2, y2),
                                at(x1 + h / 3., y1 + d[i] * h / 3.),
                                at(x2 - h / 3., y2 - d[i + 1] * h / 3.),
                            );
                        }
                    };
                    let last_x = pts
                        .iter()
                        .map(|p| p.0)
                        .rfind(|x| *x <= reveal + 0.1)
                        .unwrap_or(pts[0].0);
                    if area {
                        let mut p = PathBuilder::fill();
                        curve(&mut p);
                        p.line_to(at(last_x, VB_H - PAD_BOTTOM));
                        p.line_to(at(pts[0].0, VB_H - PAD_BOTTOM));
                        p.close();
                        if let Ok(path) = p.build() {
                            window.paint_path(
                                path,
                                linear_gradient(
                                    180.,
                                    linear_color_stop(color.opacity(0.5), 0.),
                                    linear_color_stop(color.opacity(0.), 1.),
                                ),
                            );
                        }
                    }
                    let mut s = PathBuilder::stroke(px(stroke::THICK));
                    curve(&mut s);
                    if let Ok(path) = s.build() {
                        window.paint_path(path, color);
                    }
                    if points {
                        for (i, (x, y)) in pts.iter().enumerate() {
                            if *x > reveal + 0.1 {
                                continue;
                            }
                            let r = if hovered == Some(i) { 6. } else { 4. };
                            let center = at(*x, *y);
                            let outer = r + 2.;
                            window.paint_quad(
                                gpui::fill(
                                    Bounds::new(
                                        point(center.x - px(outer), center.y - px(outer)),
                                        gpui::size(px(outer * 2.), px(outer * 2.)),
                                    ),
                                    bg,
                                )
                                .corner_radii(px(outer)),
                            );
                            window.paint_quad(
                                gpui::fill(
                                    Bounds::new(
                                        point(center.x - px(r), center.y - px(r)),
                                        gpui::size(px(r * 2.), px(r * 2.)),
                                    ),
                                    color,
                                )
                                .corner_radii(px(r)),
                            );
                        }
                    }
                }
            }
        };
        let width = plot_bounds.map(|b| f32::from(b.size.width)).unwrap_or(0.);
        let height = width * VB_H / VB_W;
        let sx = width / VB_W;
        let sy = height / VB_H;
        let plot_h = VB_H - PAD_TOP - PAD_BOTTOM;
        let label_x = |i: usize| -> f32 {
            match kind {
                ChartKind::Bar => {
                    let band = (VB_W - PAD_X * 2.) / n as f32;
                    (PAD_X + band * (i as f32 + 0.5)) * sx
                }
                ChartKind::Line => {
                    let step = if n == 1 {
                        0.
                    } else {
                        (VB_W - PAD_X * 2.) / (n - 1) as f32
                    };
                    (PAD_X + step * i as f32) * sx
                }
            }
        };
        let hover_move = hover.clone();
        let hover_leave = hover.clone();
        let label_w = 80.;
        let tooltip = hovered.and_then(|i| {
            let v = *self.values.get(i)?;
            let label = self.labels.get(i).cloned().unwrap_or_default();
            let x = label_x(i);
            let y = (PAD_TOP + plot_h - (v.max(0.) / scale_max) * plot_h) * sy;
            let flip = x > width * 0.75;
            Some(
                div()
                    .absolute()
                    .top(px((y - 8.).max(0.)))
                    .when(!flip, |d| d.left(px(x + 12.)))
                    .when(flip, |d| d.right(px(width - x + 12.)))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XXX_SMALL))
                    .rounded(px(radius::MEDIUM))
                    .border(px(stroke::THIN))
                    .border_color(c.border_default)
                    .bg(c.surface_popover)
                    .px(px(spacing::MEDIUM))
                    .py(px(spacing::SMALL))
                    .shadow(theme.shadow(Elevation::FLOATING))
                    .opacity(cursor_a)
                    .child(
                        div()
                            .text_style_token(role::CAPTION_MD)
                            .text_color(c.foreground_secondary)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_style_token(role::BODY_SM)
                            .text_color(c.foreground_primary)
                            .child(format_tick(v)),
                    ),
            )
        });
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap(px(spacing::MEDIUM))
            .child(
                div()
                    .flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(spacing::MEDIUM))
                    // Y axis.
                    .child(
                        div()
                            .relative()
                            .flex_none()
                            .min_w(px(40.))
                            .pr(px(spacing::X_SMALL))
                            .h(px(height))
                            .children(ticks.iter().map(|t| {
                                let y = (PAD_TOP + plot_h - (t / scale_max) * plot_h) * sy;
                                div()
                                    .absolute()
                                    .right(px(spacing::X_SMALL))
                                    .top(px(y - role::CAPTION_SM.line_height / 2.))
                                    .text_style_token(role::CAPTION_SM)
                                    .text_color(c.foreground_tertiary)
                                    .child(format_tick(*t))
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .id(child_id(&self.id, "plot"))
                                    .relative()
                                    .w_full()
                                    .h(px(height))
                                    .child(probe(&plot, None))
                                    .child(canvas(|_, _, _| {}, move |b, _, w, _| paint(b, w)).size_full())
                                    .on_mouse_move(move |e: &MouseMoveEvent, _, cx| {
                                        let Some(b) = plot_bounds else { return };
                                        let x = f32::from(e.position.x - b.origin.x) / (f32::from(b.size.width) / VB_W);
                                        let ix = match kind {
                                            ChartKind::Bar => ((x - PAD_X) / ((VB_W - PAD_X * 2.) / n as f32)).floor(),
                                            ChartKind::Line => {
                                                let step = if n == 1 { 1. } else { (VB_W - PAD_X * 2.) / (n - 1) as f32 };
                                                ((x - PAD_X) / step).round()
                                            }
                                        }
                                        .clamp(0., (n - 1) as f32) as usize;
                                        hover_move.update(cx, |h, cx| {
                                            if *h != Some(ix) {
                                                *h = Some(ix);
                                                cx.notify();
                                            }
                                        });
                                    })
                                    .on_hover(move |inside, _, cx| {
                                        if !*inside {
                                            hover_leave.update(cx, |h, cx| {
                                                *h = None;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .children(tooltip),
                            )
                            // X labels.
                            .child(
                                div()
                                    .relative()
                                    .w_full()
                                    .mt(px(spacing::SMALL))
                                    .h(px(24.))
                                    .children(self.labels.iter().enumerate().map(|(i, l)| {
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left(px(label_x(i) - label_w / 2.))
                                            .w(px(label_w))
                                            .flex()
                                            .justify_center()
                                            .text_style_token(role::CAPTION_SM)
                                            .text_color(if hovered == Some(i) {
                                                c.foreground_primary
                                            } else {
                                                c.foreground_tertiary
                                            })
                                            .child(l.clone())
                                    })),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_max_rounds_like_the_web() {
        assert_eq!(nice_max(305.), 500.);
        assert_eq!(nice_max(186.), 200.);
        assert_eq!(nice_max(0.), 1.);
        assert_eq!(format_tick(125.), "125");
        assert_eq!(format_tick(12.5), "12.5");
    }

    #[test]
    fn monotone_tangents_flatten_at_extrema() {
        let d = monotone_tangents(&[(0., 0.), (1., 2.), (2., 0.)]);
        assert_eq!(d[1], 0.);
    }
}
