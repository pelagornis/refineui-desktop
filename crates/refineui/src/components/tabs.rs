//! Tabs — web `Tabs` / `TabsList` / `TabsTrigger` / `TabsContent`.
//!
//! Same usage as the web: tabs are keyed by a string `value`; the root is
//! uncontrolled with `default_value` or controlled with `value` +
//! `on_value_change`; each `TabsContent` renders only while its value is
//! selected, `gap-medium` below the list.
//!
//! ```ignore
//! Tabs::new("t")
//!     .default_value("overview")
//!     .trigger(TabsTrigger::new("overview", "Overview").icon(IconName::Home))
//!     .trigger(TabsTrigger::new("issues", "Issues").count(12))
//!     .content(TabsContent::new("overview", overview_panel()))
//!     .content(TabsContent::new("issues", issues_panel()))
//! ```
//!
//! Keyboard (web parity): Left/Up and Right/Down move focus *and* selection
//! across enabled triggers (wrapping), Home/End jump to the ends.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, px,
};

use gpui::CursorStyle;
use refineui_foundation::{spacing, stroke, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, probe, use_interaction,
    use_layout_probe,
};
use crate::{ActiveTheme, Badge, BadgeVariant, Icon, IconName, IconSize, StyledExt};

/// One trigger in the list (web `TabsTrigger`).
#[derive(Clone)]
pub struct TabsTrigger {
    value: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    count: Option<usize>,
    disabled: bool,
}

impl TabsTrigger {
    /// Trigger for `value` showing `label`.
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
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

    /// Disabled (skipped by keyboard navigation).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A panel shown while its `value` is selected (web `TabsContent`).
pub struct TabsContent {
    value: SharedString,
    child: AnyElement,
}

impl TabsContent {
    /// Panel for `value`.
    pub fn new(value: impl Into<SharedString>, child: impl IntoElement) -> Self {
        Self {
            value: value.into(),
            child: child.into_any_element(),
        }
    }
}

/// Web `Tabs` root: a `TabsList` of triggers plus optional content panels.
///
/// `captionLg` triggers over a 1px `borderDefault` rule; a 2px brand underline
/// slides to the selected tab (transform + width over `duration-medium`,
/// emphasized). Unselected tabs are `foregroundTertiary`, hover
/// `foregroundPrimaryHover`; hovering the selected tab tints the line `brandHover`.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    triggers: Vec<TabsTrigger>,
    contents: Vec<TabsContent>,
    value: Option<SharedString>,
    default_value: Option<SharedString>,
    on_value_change: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App)>>,
}

impl Tabs {
    /// Empty tab root.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            triggers: Vec::new(),
            contents: Vec::new(),
            value: None,
            default_value: None,
            on_value_change: None,
        }
    }

    /// Adds a trigger to the list.
    pub fn trigger(mut self, trigger: TabsTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Adds several triggers.
    pub fn triggers(mut self, triggers: impl IntoIterator<Item = TabsTrigger>) -> Self {
        self.triggers.extend(triggers);
        self
    }

    /// Adds a content panel.
    pub fn content(mut self, content: TabsContent) -> Self {
        self.contents.push(content);
        self
    }

    /// Controlled selected value.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Initial value when uncontrolled (defaults to the first enabled trigger).
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Called with the newly selected value.
    pub fn on_value_change(
        mut self,
        f: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_value_change = Some(Rc::new(f));
        self
    }
}

/// Next enabled index from `from` by `delta` (wrapping); `None` if none enabled.
pub(crate) fn roving(enabled: &[bool], from: usize, delta: i32) -> Option<usize> {
    let n = enabled.len() as i32;
    (1..=n)
        .map(|k| (from as i32 + delta * k).rem_euclid(n) as usize)
        .find(|i| enabled[*i])
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let parent = self.id.clone();
        // Uncontrolled value lives in keyed state; `value` overrides it.
        let first_enabled = self
            .triggers
            .iter()
            .find(|t| !t.disabled)
            .map(|t| t.value.clone())
            .unwrap_or_default();
        let initial = self.default_value.clone().unwrap_or(first_enabled);
        let inner = window.use_keyed_state(child_id(&self.id, "value"), cx, |_, _| initial);
        let current = self.value.clone().unwrap_or_else(|| inner.read(cx).clone());
        let selected_ix = self
            .triggers
            .iter()
            .position(|t| t.value == current)
            .unwrap_or(0);
        let select: Rc<dyn Fn(SharedString, &mut Window, &mut App)> = {
            let inner = inner.clone();
            let cb = self.on_value_change.clone();
            Rc::new(move |v: SharedString, w: &mut Window, cx: &mut App| {
                inner.update(cx, |s, cx| {
                    *s = v.clone();
                    cx.notify();
                });
                if let Some(cb) = &cb {
                    cb(&v, w, cx);
                }
            })
        };
        let probe_state = use_layout_probe(&self.id, window, cx);
        let measured = probe_state.read(cx).relative(selected_ix);
        let sel_id = ElementId::NamedChild(
            Box::new(child_id(&self.id, "tab")),
            format!("{selected_ix}").into(),
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
        let enabled: Vec<bool> = self.triggers.iter().map(|t| !t.disabled).collect();
        let values: Vec<SharedString> = self.triggers.iter().map(|t| t.value.clone()).collect();
        let handles: Vec<FocusHandle> = (0..self.triggers.len())
            .map(|ix| {
                window
                    .use_keyed_state(
                        ElementId::NamedChild(
                            Box::new(child_id(&parent, "focus")),
                            format!("{ix}").into(),
                        ),
                        cx,
                        |_, cx| cx.focus_handle(),
                    )
                    .read(cx)
                    .clone()
            })
            .collect();
        for (ix, tab) in self.triggers.into_iter().enumerate() {
            let selected = ix == selected_ix;
            let item_id =
                ElementId::NamedChild(Box::new(child_id(&parent, "tab")), format!("{ix}").into());
            let (state, i) = use_interaction(&item_id, window, cx);
            let handler = (!tab.disabled).then(|| {
                let select = select.clone();
                let v = tab.value.clone();
                move |w: &mut Window, cx: &mut App| select(v.clone(), w, cx)
            });
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
                            .track_focus(&handles[ix].clone().tab_index(if selected { 0 } else { -1 }).tab_stop(selected))
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
                    .when_some(handler, |d, h| d.on_click(move |_, w, cx| h(w, cx))),
            );
        }
        let key_select = select.clone();
        let list = div()
            .id(child_id(&self.id, "list"))
            .relative()
            .flex()
            .w_full()
            .on_key_down(move |ev: &KeyDownEvent, window, cx| {
                let target = match ev.keystroke.key.as_str() {
                    "right" | "down" => roving(&enabled, selected_ix, 1),
                    "left" | "up" => roving(&enabled, selected_ix, -1),
                    "home" => enabled.iter().position(|e| *e),
                    "end" => enabled.iter().rposition(|e| *e),
                    _ => return,
                };
                cx.stop_propagation();
                if let Some(t) = target {
                    window.focus(&handles[t]);
                    key_select(values[t].clone(), window, cx);
                }
            })
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
            });
        let panel = self
            .contents
            .into_iter()
            .find(|c| c.value == current)
            .map(|c| c.child);
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap(px(spacing::MEDIUM))
            .child(list)
            .children(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::roving;

    #[test]
    fn roving_skips_disabled_and_wraps() {
        let e = [true, false, true, true];
        assert_eq!(roving(&e, 0, 1), Some(2));
        assert_eq!(roving(&e, 3, 1), Some(0));
        assert_eq!(roving(&e, 0, -1), Some(3));
        assert_eq!(roving(&[false, false], 0, 1), None);
    }
}
