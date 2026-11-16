//! Tabs — web `Tabs` (tab list with an underline indicator).

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use gpui::CursorStyle;
use refineui_tokens::{spacing, stroke, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, probe, use_interaction,
    use_layout_probe,
};
use crate::{ActiveTheme, Badge, BadgeVariant, Icon, IconName, IconSize, StyledExt};

/// One tab.
#[derive(Clone)]
pub struct Tab {
    label: SharedString,
    icon: Option<IconName>,
    count: Option<usize>,
    disabled: bool,
}

impl Tab {
    /// New tab.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            count: None,
            disabled: false,
        }
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Trailing counter badge.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<T: Into<SharedString>> From<T> for Tab {
    fn from(label: T) -> Self {
        Tab::new(label)
    }
}

/// Horizontal tab list (web `Tabs`). Render the selected panel yourself below it.
///
/// `captionLg` triggers over a 1px `borderDefault` rule; a 2px brand underline
/// slides to the selected tab (transform + width over `duration-medium`,
/// emphasized). Unselected tabs are `foregroundTertiary`, hover
/// `foregroundPrimaryHover`; hovering the selected tab tints the line `brandHover`.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    tabs: Vec<Tab>,
    selected: usize,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Tabs {
    /// New tab list.
    pub fn new(
        id: impl Into<ElementId>,
        tabs: impl IntoIterator<Item = impl Into<Tab>>,
        selected: usize,
    ) -> Self {
        Self {
            id: id.into(),
            tabs: tabs.into_iter().map(Into::into).collect(),
            selected,
            on_change: None,
        }
    }

    /// Called with the chosen index.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let parent = self.id.clone();
        let probe_state = use_layout_probe(&self.id, window, cx);
        let measured = probe_state.read(cx).relative(self.selected);
        let sel_id = ElementId::NamedChild(
            Box::new(child_id(&self.id, "tab")),
            format!("{}", self.selected).into(),
        );
        let (_, sel_i) = use_interaction(&sel_id, window, cx);
        let line = if sel_i.hovered {
            c.background_brand_hover
        } else {
            c.background_brand
        };
        let line = animate(child_id(&self.id, "line"), line, Motion::FAST, window, cx);
        let indicator = measured.map(|b| {
            (
                animate(
                    child_id(&self.id, "ind-x"),
                    b.left(),
                    Motion::INDICATOR,
                    window,
                    cx,
                ),
                animate(
                    child_id(&self.id, "ind-w"),
                    b.size.width,
                    Motion::INDICATOR,
                    window,
                    cx,
                ),
            )
        });
        let ready = indicator.is_some();
        let show_focus = focus_visible(cx);
        let mut items = Vec::new();
        for (ix, tab) in self.tabs.into_iter().enumerate() {
            let selected = ix == self.selected;
            let item_id =
                ElementId::NamedChild(Box::new(child_id(&parent, "tab")), format!("{ix}").into());
            let (state, i) = use_interaction(&item_id, window, cx);
            let handler = self.on_change.clone().filter(|_| !tab.disabled);
            let fg = if tab.disabled {
                c.foreground_disabled
            } else if selected {
                c.foreground_primary
            } else if i.hovered {
                c.foreground_primary_hover
            } else {
                c.foreground_tertiary
            };
            let fg = animate(child_id(&item_id, "fg"), fg, Motion::FAST, window, cx);
            let ring = theme.focus_ring();
            items.push(
                div()
                    .id(item_id)
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .gap(px(spacing::X_SMALL))
                    .px(px(spacing::MEDIUM))
                    .py(px(spacing::X_SMALL + theme.tab_inset()))
                    .text_style_token(role::CAPTION_LG)
                    .whitespace_nowrap()
                    .text_color(fg)
                    .when(!tab.disabled, |d| {
                        d.cursor_action()
                            .track_interaction(&state)
                            .tab_index(0)
                            .when(show_focus, |d| d.focus(move |s| s.shadow(ring)))
                    })
                    .when(tab.disabled, |d| d.cursor(CursorStyle::OperationNotAllowed))
                    .when_some(tab.icon, |d, i| d.child(Icon::new(i).size(IconSize::XSmall).color(fg)))
                    .child(tab.label)
                    .when_some(tab.count, |d, n| {
                        d.child(Badge::new(n.to_string()).variant(BadgeVariant::Neutral))
                    })
                    // Pre-measure fallback: static underline under the selected tab.
                    .when(selected && !ready, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .h(px(stroke::THICK))
                                .bg(line),
                        )
                    })
                    .child(probe(&probe_state, Some(ix)))
                    .when_some(handler, |d, h| d.on_click(move |_, w, cx| h(&ix, w, cx))),
            );
        }
        div()
            .id(self.id)
            .relative()
            .flex()
            .w_full()
            .child(probe(&probe_state, None))
            // `::after` list rule — 1px `borderDefault` across the full width.
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(stroke::THIN))
                    .bg(c.border_default),
            )
            .children(items)
            .when_some(indicator, |d, (left, width)| {
                d.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left(left)
                        .w(width)
                        .h(px(stroke::THICK))
                        .bg(line),
                )
            })
    }
}
