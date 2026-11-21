//! Preferences window layout (desktop addition): a section sidebar and a
//! content pane of grouped label/control rows.
//!
//! ```text
//! ┌──────────────┬──────────────────────────────────────┐
//! │ ⚙ General    │ General                              │
//! │ ◐ Appearance │ How the app starts and saves.        │
//! │ ⌨ Shortcuts  │ ┌──────────────────────────────────┐ │
//! │ …            │ │ Label            [control]       │ │
//! │              │ │ description                      │ │
//! │              │ ├──────────────────────────────────┤ │
//! │              │ │ …                                │ │
//! └──────────────┴──────────────────────────────────────┘
//! ```
//!
//! The sidebar is window chrome (`Sidebar` material when active, otherwise
//! `surfaceSunken`); groups are `backgroundPrimary` cards with hairline row
//! separators. It fills its parent; put it in a [`crate::WindowFrame`] or a
//! dedicated preferences window.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::motion::child_id;
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// One entry in the [`PreferencesWindow`] sidebar.
#[derive(Clone)]
pub struct PreferencesSection {
    label: SharedString,
    icon: IconName,
    description: Option<SharedString>,
}

impl PreferencesSection {
    /// Section with a label and glyph.
    pub fn new(label: impl Into<SharedString>, icon: IconName) -> Self {
        Self {
            label: label.into(),
            icon,
            description: None,
        }
    }

    /// Line under the section title in the content pane.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }
}

type SelectHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// Sidebar + content preferences layout.
#[derive(IntoElement)]
pub struct PreferencesWindow {
    id: ElementId,
    sections: Vec<PreferencesSection>,
    selected: usize,
    on_select: Option<SelectHandler>,
    content: Vec<AnyElement>,
    sidebar_width: Pixels,
    footer: Option<AnyElement>,
}

impl PreferencesWindow {
    /// Empty layout.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            sections: Vec::new(),
            selected: 0,
            on_select: None,
            content: Vec::new(),
            sidebar_width: px(208.),
            footer: None,
        }
    }

    /// Add a sidebar section.
    pub fn section(mut self, s: PreferencesSection) -> Self {
        self.sections.push(s);
        self
    }

    /// Selected section index.
    pub fn selected(mut self, ix: usize) -> Self {
        self.selected = ix;
        self
    }

    /// Section click.
    pub fn on_select(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// Content of the selected section (usually [`PreferencesGroup`]s).
    pub fn child(mut self, el: impl IntoElement) -> Self {
        self.content.push(el.into_any_element());
        self
    }

    /// Sidebar width (default 208px).
    pub fn sidebar_width(mut self, w: impl Into<Pixels>) -> Self {
        self.sidebar_width = w.into();
        self
    }

    /// Element pinned at the bottom of the sidebar (account, version).
    pub fn footer(mut self, el: impl IntoElement) -> Self {
        self.footer = Some(el.into_any_element());
        self
    }
}

impl RenderOnce for PreferencesWindow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let (side_fill, side_layer) =
            crate::material::chrome(crate::SurfaceKind::Sidebar, c.surface_sunken, cx);
        let current = self.sections.get(self.selected).cloned();
        let on_select = self.on_select.clone();
        let rows = self.sections.iter().enumerate().map(|(ix, s)| {
            let selected = ix == self.selected;
            let f = on_select.clone();
            div()
                .id(ElementId::NamedInteger("pref-section".into(), ix as u64))
                .flex()
                .items_center()
                .gap(px(spacing::SMALL))
                .h(px(32.))
                .px(px(spacing::SMALL))
                .rounded(px(radius::MEDIUM))
                .text_style_token(role::BODY_MD)
                .when(selected, |d| {
                    d.bg(c.background_surface_selected)
                        .text_color(c.foreground_primary)
                })
                .when(!selected, |d| {
                    d.text_color(c.foreground_secondary)
                        .hover(|s| s.bg(c.background_surface_hover))
                })
                .cursor_action()
                .when_some(f, |d, f| d.on_click(move |_, w, cx| f(&ix, w, cx)))
                .child(
                    Icon::new(s.icon)
                        .size(IconSize::Custom(px(16.)))
                        .color(if selected {
                            c.foreground_brand
                        } else {
                            c.foreground_secondary
                        }),
                )
                .child(s.label.clone())
        });
        div()
            .id(self.id.clone())
            .size_full()
            .min_h_0()
            .flex()
            .bg(c.background_primary)
            .child(
                div()
                    .relative()
                    .flex_none()
                    .w(self.sidebar_width)
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(side_fill)
                    .children(side_layer)
                    .border_r(px(stroke::HAIRLINE))
                    .border_color(c.border_subtle)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .p(px(spacing::SMALL))
                            .children(rows),
                    )
                    .children(self.footer.map(|f| div().p(px(spacing::SMALL)).child(f))),
            )
            .child(
                div()
                    .id(child_id(&self.id, "content"))
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .max_w(px(640.))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::LARGE))
                            .px(px(spacing::XX_LARGE))
                            .py(px(spacing::X_LARGE))
                            .when_some(current, |d, s| {
                                d.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(spacing::XX_SMALL))
                                        .child(
                                            div()
                                                .text_style_token(role::TITLE_SM)
                                                .text_color(c.foreground_primary)
                                                .child(s.label),
                                        )
                                        .when_some(s.description, |d, t| {
                                            d.child(
                                                div()
                                                    .text_style_token(role::BODY_SM)
                                                    .text_color(c.foreground_secondary)
                                                    .child(t),
                                            )
                                        }),
                                )
                            })
                            .children(self.content),
                    ),
            )
    }
}

/// A titled card of [`PreferencesRow`]s.
#[derive(IntoElement)]
pub struct PreferencesGroup {
    title: Option<SharedString>,
    description: Option<SharedString>,
    rows: Vec<AnyElement>,
}

impl PreferencesGroup {
    /// Untitled group.
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            rows: Vec::new(),
        }
    }

    /// Group heading.
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Text under the card.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Add a row (usually a [`PreferencesRow`]).
    pub fn row(mut self, r: impl IntoElement) -> Self {
        self.rows.push(r.into_any_element());
        self
    }
}

impl Default for PreferencesGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for PreferencesGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let n = self.rows.len();
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::X_SMALL))
            .when_some(self.title, |d, t| {
                d.child(
                    div()
                        .px(px(spacing::XX_SMALL))
                        .text_style_token(role::SUBTITLE_MD)
                        .text_color(c.foreground_primary)
                        .child(t),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .rounded(px(radius::X_LARGE))
                    .border(px(stroke::HAIRLINE))
                    .border_color(c.border_subtle)
                    .bg(c.background_surface)
                    .overflow_hidden()
                    .children(self.rows.into_iter().enumerate().map(|(i, r)| {
                        div()
                            .when(i + 1 < n, |d| {
                                d.border_b(px(stroke::HAIRLINE))
                                    .border_color(c.border_subtle)
                            })
                            .child(r)
                    })),
            )
            .when_some(self.description, |d, t| {
                d.child(
                    div()
                        .px(px(spacing::XX_SMALL))
                        .text_style_token(role::CAPTION_MD)
                        .text_color(c.foreground_tertiary)
                        .child(t),
                )
            })
    }
}

/// Label (+ description) on the left, control on the right.
#[derive(IntoElement)]
pub struct PreferencesRow {
    label: SharedString,
    description: Option<SharedString>,
    control: Option<AnyElement>,
}

impl PreferencesRow {
    /// Row with a label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            control: None,
        }
    }

    /// Secondary line under the label.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Trailing control (Switch, Select, ShortcutRecorder, …).
    pub fn control(mut self, el: impl IntoElement) -> Self {
        self.control = Some(el.into_any_element());
        self
    }
}

impl RenderOnce for PreferencesRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        div()
            .flex()
            .items_center()
            .gap(px(spacing::LARGE))
            .min_h(px(48.))
            .px(px(spacing::MEDIUM))
            .py(px(spacing::SMALL))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_style_token(role::BODY_MD)
                            .text_color(c.foreground_primary)
                            .child(self.label),
                    )
                    .when_some(self.description, |d, t| {
                        d.child(
                            div()
                                .text_style_token(role::CAPTION_MD)
                                .text_color(c.foreground_secondary)
                                .child(t),
                        )
                    }),
            )
            .children(self.control.map(|c| div().flex_none().child(c)))
    }
}
