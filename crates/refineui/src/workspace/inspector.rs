//! Inspector (brief §6 Composition): the property sheet of the current
//! selection, usually in the right dock.
//!
//! Collapsible [`InspectorSection`]s (Panel headers) of label / value
//! rows: labels are Body 3 `foregroundSecondary` in a fixed column
//! (96px), values take the rest; rows are at least `row_height` tall.
//! With no sections the inspector shows an empty state ("No selection" by
//! default). Long sheets scroll.

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, px,
};
use refineui_foundation::{spacing, stroke, typography::role};

use super::panel::Panel;
use crate::motion::child_id;
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// Width of the inspector label column, px.
pub const INSPECTOR_LABEL_WIDTH: f32 = 96.;

/// A titled group of property rows.
pub struct InspectorSection {
    title: SharedString,
    rows: Vec<(Option<SharedString>, AnyElement)>,
    collapsed: Option<bool>,
}

impl InspectorSection {
    /// Section titled `title`.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
            collapsed: None,
        }
    }

    /// A `label` / value row.
    pub fn row(mut self, label: impl Into<SharedString>, value: impl IntoElement) -> Self {
        self.rows
            .push((Some(label.into()), value.into_any_element()));
        self
    }

    /// A full-width row (no label).
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.rows.push((None, e.into_any_element()));
        self
    }

    /// Start collapsed.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = Some(collapsed);
        self
    }
}

/// The property sheet. See the module docs.
#[derive(IntoElement)]
pub struct Inspector {
    id: ElementId,
    header: Option<AnyElement>,
    sections: Vec<InspectorSection>,
    empty: SharedString,
}

impl Inspector {
    /// Empty inspector.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            header: None,
            sections: Vec::new(),
            empty: "No selection".into(),
        }
    }

    /// Element above the sections (selection name, kind badge).
    pub fn header(mut self, e: impl IntoElement) -> Self {
        self.header = Some(e.into_any_element());
        self
    }

    /// Add a section.
    pub fn section(mut self, s: InspectorSection) -> Self {
        self.sections.push(s);
        self
    }

    /// Empty-state text.
    pub fn empty_text(mut self, t: impl Into<SharedString>) -> Self {
        self.empty = t.into();
        self
    }
}

impl RenderOnce for Inspector {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let c = theme.colors;
        let m = theme.desktop();
        if self.sections.is_empty() {
            return div()
                .id(self.id)
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(spacing::SMALL))
                .p(px(spacing::LARGE))
                .child(
                    Icon::new(IconName::Info)
                        .size(IconSize::Medium)
                        .color(c.foreground_tertiary),
                )
                .child(
                    div()
                        .text_style_token(role::BODY_SM)
                        .text_color(c.foreground_tertiary)
                        .child(self.empty),
                );
        }
        let id = self.id.clone();
        div()
            .id(self.id)
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .children(self.header.map(|h| {
                div()
                    .flex_none()
                    .px(px(m.panel_padding))
                    .py(px(spacing::SMALL))
                    .border_b(px(stroke::HAIRLINE))
                    .border_color(c.border_subtle)
                    .child(h)
            }))
            .children(self.sections.into_iter().enumerate().map(|(ix, s)| {
                let p = Panel::new(
                    ElementId::NamedChild(
                        Box::new(child_id(&id, "section")),
                        ix.to_string().into(),
                    ),
                    s.title,
                )
                .collapsible(true)
                .fill(false)
                .default_collapsed(s.collapsed.unwrap_or(false))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .px(px(m.panel_padding))
                        .pb(px(spacing::SMALL))
                        .children(s.rows.into_iter().map(|(label, value)| {
                            div()
                                .min_h(px(m.row_height))
                                .flex()
                                .items_center()
                                .gap(px(spacing::SMALL))
                                .children(label.map(|l| {
                                    div()
                                        .flex_none()
                                        .w(px(INSPECTOR_LABEL_WIDTH))
                                        .truncate()
                                        .text_style_token(role::BODY_SM)
                                        .text_color(c.foreground_secondary)
                                        .child(l)
                                }))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .text_style_token(role::BODY_SM)
                                        .text_color(c.foreground_primary)
                                        .child(value),
                                )
                        })),
                );
                div()
                    .flex_none()
                    .when(ix > 0, |d| {
                        d.border_t(px(stroke::HAIRLINE))
                            .border_color(c.border_subtle)
                    })
                    .child(p)
            }))
    }
}
