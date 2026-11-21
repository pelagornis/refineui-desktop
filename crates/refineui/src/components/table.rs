//! Table — web `Table` (header, rows, hover, selection, striping).

use std::rc::Rc;

use gpui::{
    AnyElement, App, DefiniteLength, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
    relative,
};

use refineui_foundation::{radius, size::component, spacing, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, probe, use_interaction, use_layout_probe,
};
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// Column alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    /// Left (default).
    #[default]
    Start,
    /// Centre.
    Center,
    /// Right (numbers).
    End,
}

/// Column definition.
#[derive(Clone)]
pub struct Column {
    title: SharedString,
    width: Option<DefiniteLength>,
    align: Align,
    sorted: Option<bool>,
}

impl Column {
    /// Flexible column.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            width: None,
            align: Align::Start,
            sorted: None,
        }
    }

    /// Fixed width (px).
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(px(w).into());
        self
    }

    /// Fractional width (`0.0..=1.0` of the table).
    pub fn fraction(mut self, f: f32) -> Self {
        self.width = Some(relative(f.clamp(0., 1.)));
        self
    }

    /// Alignment.
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }

    /// Show a sort indicator (`true` = ascending).
    pub fn sorted(mut self, ascending: bool) -> Self {
        self.sorted = Some(ascending);
        self
    }
}

/// Data table. Cells are any element; use [`Table::text_rows`] for strings.
#[derive(IntoElement)]
pub struct Table {
    id: ElementId,
    columns: Vec<Column>,
    rows: Vec<Vec<AnyElement>>,
    selected: Option<usize>,
    striped: bool,
    on_select: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
    on_sort: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Table {
    /// New table.
    pub fn new(id: impl Into<ElementId>, columns: impl IntoIterator<Item = Column>) -> Self {
        Self {
            id: id.into(),
            columns: columns.into_iter().collect(),
            rows: Vec::new(),
            selected: None,
            striped: false,
            on_select: None,
            on_sort: None,
        }
    }

    /// Append a row of elements.
    pub fn row(mut self, cells: impl IntoIterator<Item = AnyElement>) -> Self {
        self.rows.push(cells.into_iter().collect());
        self
    }

    /// Append rows of strings.
    pub fn text_rows<R, S>(mut self, rows: impl IntoIterator<Item = R>) -> Self
    where
        R: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        for r in rows {
            self.rows.push(
                r.into_iter()
                    .map(|s| div().child(s.into()).into_any_element())
                    .collect(),
            );
        }
        self
    }

    /// Highlight a row.
    pub fn selected(mut self, ix: Option<usize>) -> Self {
        self.selected = ix;
        self
    }

    /// Zebra striping.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Row click.
    pub fn on_select(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// Header click (column index).
    pub fn on_sort(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_sort = Some(Rc::new(f));
        self
    }
}

fn cell(col: &Column) -> gpui::Div {
    let d = div()
        .flex()
        .items_center()
        .px(px(spacing::MEDIUM))
        .whitespace_nowrap()
        .overflow_hidden();
    let d = match col.width {
        Some(w) => d.flex_none().w(w),
        None => d.flex_1().min_w_0(),
    };
    match col.align {
        Align::Start => d,
        Align::Center => d.justify_center(),
        Align::End => d.justify_end(),
    }
}

impl RenderOnce for Table {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let parent = self.id.clone();
        let columns = self.columns.clone();
        let mut rows = Vec::with_capacity(self.rows.len());
        let host = use_layout_probe(&child_id(&self.id, "highlight"), window, cx);
        let mut plate_target = None;
        for (rix, cells) in self.rows.into_iter().enumerate() {
            let selected = self.selected == Some(rix);
            let id = ElementId::NamedChild(Box::new(parent.clone()), format!("row-{rix}").into());
            let (state, i) = use_interaction(&id, window, cx);
            let clear = c.background_primary.opacity(0.);
            if i.hovered && !selected {
                plate_target = Some(rix);
            }
            // Hover is the sliding plate (web `tableStyles.highlight`);
            // selected rows keep their own fill.
            let bg = if selected {
                c.background_surface_selected
            } else if self.striped && rix % 2 == 1 {
                c.background_primary_hover
            } else {
                clear
            };
            let bg = animate(child_id(&id, "bg"), bg, Motion::FAST, window, cx);
            let handler = self.on_select.clone();
            let n = cells.len();
            rows.push(
                div()
                    .id(id)
                    .relative()
                    .child(probe(&host, Some(rix)))
                    .flex()
                    .h(px(component::TABLE_ROW_HEIGHT + 2. * theme.row_inset()))
                    .rounded(px(radius::LARGE))
                    .bg(bg)
                    .text_color(c.foreground_primary)
                    .track_interaction(&state)
                    .when_some(handler, |d, h| {
                        d.cursor_action().on_click(move |_, w, cx| h(&rix, w, cx))
                    })
                    .children(
                        cells
                            .into_iter()
                            .zip(columns.iter())
                            .map(|(el, col)| cell(col).child(el)),
                    )
                    .when(n < columns.len(), |d| {
                        d.children(columns[n..].iter().map(cell))
                    }),
            );
        }
        // Web `Table`: separated rows (4px spacing), header rail on
        // `surfaceActive` with `roundedLarge` ends and a thin `borderSubtle`
        // inset bottom edge, `bodySm` medium tertiary; cells `bodySm` primary,
        // 36 / 44px heights, `sizeMedium` side padding. Rows: hover
        // `surfaceHover`, selected `surfaceSelected`, rounded ends.
        div()
            .id(self.id)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(spacing::XX_SMALL))
            .px(px(spacing::SMALL))
            .pb(px(spacing::SMALL))
            .text_style_token(role::BODY_SM)
            .child(
                div()
                    .flex()
                    .h(px(component::TABLE_HEAD_HEIGHT + 2. * theme.row_inset()))
                    .rounded(px(radius::LARGE))
                    .overflow_hidden()
                    .bg(c.background_surface_active)
                    .border_b_1()
                    .border_color(c.border_subtle)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(c.foreground_tertiary)
                    .children(columns.iter().enumerate().map(|(ix, col)| {
                        let on_sort = self.on_sort.clone();
                        cell(col)
                            .gap(px(spacing::XX_SMALL))
                            .child(col.title.clone())
                            .when_some(col.sorted, |d, asc| {
                                d.child(
                                    Icon::new(if asc {
                                        IconName::ChevronUp
                                    } else {
                                        IconName::ChevronDown
                                    })
                                    .size(IconSize::XXSmall)
                                    .color(c.foreground_tertiary),
                                )
                            })
                            .when_some(on_sort, |d, h| {
                                d.child(
                                    div()
                                        .id(ElementId::NamedChild(
                                            Box::new(parent.clone()),
                                            format!("sort-{ix}").into(),
                                        ))
                                        .absolute()
                                        .size_full()
                                        .cursor_action()
                                        .on_click(move |_, w, cx| h(&ix, w, cx)),
                                )
                                .relative()
                            })
                    })),
            )
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XX_SMALL))
                    .child(probe(&host, None))
                    .child(crate::components::sliding_highlight::sliding_plate(
                        &child_id(&parent, "highlight"),
                        &host,
                        plate_target,
                        c.background_surface_hover,
                        radius::LARGE,
                        window,
                        cx,
                    ))
                    .children(rows),
            )
    }
}
