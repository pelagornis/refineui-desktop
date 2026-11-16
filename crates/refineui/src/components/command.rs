//! Command — web `Command` (`components/Command/style.ts`): a command palette
//! with a search field and a filtered, grouped results list.
//!
//! Detached layout (`gap-small`): a [`crate::SearchField`] over a separate
//! results panel — hairline `borderDefault`, `backgroundPrimary`, `shadow2`,
//! radius by size (`sm` 8 / `md` 12 / `lg` 16px), at most 320px tall, then it
//! scrolls (edge-softened ScrollArea). List padding `px-medium` /
//! `py-small`, `gap-xxx-small`. Group headings `captionLg` tertiary
//! (`px-x-small` / `py-xxx-small`); dividers 1px `borderDefault` in a
//! `p-x-small` wrap. Items: `roundedLarge`, `px-x-small`, `gap-x-small`,
//! row text and `py` by size (`bodySm`/4, `bodyMd`/8, `bodyLg`/10), icons
//! 16 / 20 / 24px, shortcut `bodyXs` tertiary. The selected item is marked by
//! a sliding indicator (`backgroundSurfaceHover`, 200ms emphasized); pressed
//! items fill `backgroundSurfaceActive` and scale to 0.96. Empty state:
//! `bodyXs` tertiary, `py-xx-large`, fades in. ↑ / ↓ move the selection,
//! Enter runs it, the pointer selects on hover.

use gpui::{
    AnyElement, Context, ElementId, Entity, EventEmitter, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, div, prelude::*, px,
};
use refineui_tokens::{radius, spacing, stroke, typography::role};

use crate::components::sliding_highlight::sliding_plate;
use crate::motion::{
    Motion, TrackInteraction, animate, animate_from, child_id, press_scale_to, probe, scaled_plate,
    use_interaction, use_layout_probe,
};
use crate::{
    ActiveTheme, ControlSize, Elevation, Icon, IconName, IconSize, InputEvent, ScrollArea,
    SearchField, StyledExt, TextInput, scroll_edge_item,
};

/// Results cap (`min(60vh, foundationSize3200)`).
pub const COMMAND_LIST_MAX_HEIGHT: f32 = 320.;

/// One runnable entry.
#[derive(Clone)]
pub struct CommandItem {
    value: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    keywords: Vec<SharedString>,
    disabled: bool,
}

impl CommandItem {
    /// Item whose value is its label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            icon: None,
            shortcut: None,
            keywords: Vec::new(),
            disabled: false,
        }
    }

    /// Stable value reported on select (defaults to the label).
    pub fn value(mut self, v: impl Into<SharedString>) -> Self {
        self.value = v.into();
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Shortcut hint.
    pub fn shortcut(mut self, s: impl Into<SharedString>) -> Self {
        self.shortcut = Some(s.into());
        self
    }

    /// Extra search terms.
    pub fn keywords(mut self, k: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.keywords = k.into_iter().map(Into::into).collect();
        self
    }

    /// Disabled (shown, not selectable).
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }

    /// cmdk-style match: every query word appears in the label or a keyword.
    pub fn matches(&self, query: &str) -> bool {
        let hay = std::iter::once(self.label.to_lowercase())
            .chain(self.keywords.iter().map(|k| k.to_lowercase()))
            .collect::<Vec<_>>()
            .join(" ");
        query
            .split_whitespace()
            .all(|w| hay.contains(&w.to_lowercase()))
    }
}

/// A headed group of items (web `CommandGroup`); `None` heading for none.
#[derive(Clone)]
pub struct CommandGroup {
    heading: Option<SharedString>,
    items: Vec<CommandItem>,
}

impl CommandGroup {
    /// New group.
    pub fn new(heading: impl Into<SharedString>) -> Self {
        Self {
            heading: Some(heading.into()),
            items: Vec::new(),
        }
    }

    /// Group without a heading.
    pub fn untitled() -> Self {
        Self {
            heading: None,
            items: Vec::new(),
        }
    }

    /// Add an item.
    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }
}

/// Emitted by [`Command`].
#[derive(Clone, Debug, PartialEq)]
pub enum CommandEvent {
    /// An item was run (Enter or click); carries its value.
    Select(SharedString),
}

/// The command palette (an entity: create with `cx.new(|cx| Command::new(window, cx))`).
pub struct Command {
    search: Entity<TextInput>,
    groups: Vec<CommandGroup>,
    query: SharedString,
    selected: Option<SharedString>,
    size: ControlSize,
    empty: SharedString,
    _sub: Subscription,
}

impl EventEmitter<CommandEvent> for Command {}

impl Command {
    /// New palette with a "Type a command or search…" field.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| SearchField::new(cx).placeholder("Type a command or search…"));
        let sub = cx.subscribe(&search, |this: &mut Self, _, e: &InputEvent, cx| match e {
            InputEvent::Change(q) => {
                this.query = q.clone();
                this.ensure_selection();
                cx.notify();
            }
            InputEvent::Submit(_) => {
                if let Some(v) = this.selected.clone() {
                    cx.emit(CommandEvent::Select(v));
                }
            }
        });
        Self {
            search,
            groups: Vec::new(),
            query: SharedString::default(),
            selected: None,
            size: ControlSize::Medium,
            empty: "No results found.".into(),
            _sub: sub,
        }
    }

    /// Add a group.
    pub fn group(mut self, g: CommandGroup) -> Self {
        self.groups.push(g);
        self.ensure_selection();
        self
    }

    /// Replace every group (e.g. to refresh shortcut labels) and keep the
    /// selection valid.
    pub fn set_groups(&mut self, groups: Vec<CommandGroup>, cx: &mut Context<Self>) {
        self.groups = groups;
        self.ensure_selection();
        cx.notify();
    }

    /// Clear the search query.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.search.update(cx, |s, cx| s.set_text("", cx));
        self.query = SharedString::default();
        self.selected = None;
        self.ensure_selection();
        cx.notify();
    }

    /// Row size (web `size`, default `md`).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Empty-state text.
    pub fn empty_text(mut self, t: impl Into<SharedString>) -> Self {
        self.empty = t.into();
        self
    }

    /// The search field entity (to focus it, read the query…).
    pub fn search(&self) -> &Entity<TextInput> {
        &self.search
    }

    /// Currently selected item value.
    pub fn selected(&self) -> Option<&SharedString> {
        self.selected.as_ref()
    }

    fn visible(&self) -> Vec<(Option<SharedString>, Vec<CommandItem>)> {
        let q = self.query.trim().to_string();
        self.groups
            .iter()
            .map(|g| {
                (
                    g.heading.clone(),
                    g.items
                        .iter()
                        .filter(|i| q.is_empty() || i.matches(&q))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, items)| !items.is_empty())
            .collect()
    }

    fn enabled_values(&self) -> Vec<SharedString> {
        self.visible()
            .into_iter()
            .flat_map(|(_, items)| items)
            .filter(|i| !i.disabled)
            .map(|i| i.value)
            .collect()
    }

    fn ensure_selection(&mut self) {
        let values = self.enabled_values();
        if self.selected.as_ref().is_none_or(|s| !values.contains(s)) {
            self.selected = values.first().cloned();
        }
    }

    fn step(&mut self, delta: isize) {
        let values = self.enabled_values();
        if values.is_empty() {
            return;
        }
        let ix = self
            .selected
            .as_ref()
            .and_then(|s| values.iter().position(|v| v == s))
            .map(|i| i as isize + delta)
            .unwrap_or(0)
            .rem_euclid(values.len() as isize);
        self.selected = Some(values[ix as usize].clone());
    }
}

impl Render for Command {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let id: ElementId = ("refineui-command", cx.entity_id()).into();
        let (text, py, icon) = match self.size {
            ControlSize::Small => (role::BODY_SM, spacing::XX_SMALL, 16.),
            ControlSize::Medium => (role::BODY_MD, spacing::SMALL, 20.),
            ControlSize::Large => (role::BODY_LG, spacing::MEDIUM, 24.),
        };
        let list_radius = match self.size {
            ControlSize::Small => radius::LARGE,
            ControlSize::Medium => radius::X_LARGE,
            ControlSize::Large => radius::XX_LARGE,
        };
        let host = use_layout_probe(&child_id(&id, "list"), window, cx);
        let groups = self.visible();
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut selected_row = None;
        let mut slot = 0usize;
        let n_groups = groups.len();
        for (gi, (heading, items)) in groups.into_iter().enumerate() {
            let mut group = div().flex().flex_col().gap(px(spacing::XXX_SMALL));
            if let Some(h) = heading {
                group = group.child(
                    div()
                        .px(px(spacing::X_SMALL))
                        .py(px(spacing::XXX_SMALL))
                        .text_style_token(role::CAPTION_LG)
                        .text_color(c.foreground_tertiary)
                        .child(h),
                );
            }
            for item in items {
                let item_id: ElementId =
                    ElementId::NamedChild(Box::new(id.clone()), item.value.clone());
                let (state, i) = use_interaction(&item_id, window, cx);
                let is_sel = self.selected.as_ref() == Some(&item.value);
                if is_sel {
                    selected_row = Some(slot);
                }
                let fg = if item.disabled {
                    c.foreground_disabled
                } else if is_sel || i.hovered {
                    c.foreground_primary_hover
                } else {
                    c.foreground_primary
                };
                let m = Motion::new(
                    crate::motion::duration::NORMAL,
                    crate::motion::easing::EMPHASIZED,
                );
                let fg = animate(child_id(&item_id, "fg"), fg, m, window, cx);
                let bg = animate(
                    child_id(&item_id, "bg"),
                    if i.pressed && !item.disabled {
                        c.background_surface_active
                    } else {
                        gpui::transparent_black()
                    },
                    m,
                    window,
                    cx,
                );
                let scale = press_scale_to(
                    child_id(&item_id, "press"),
                    i.pressed && !item.disabled,
                    crate::motion::scale::BUTTON_PRESS,
                    window,
                    cx,
                );
                let value = item.value.clone();
                let hover_value = item.value.clone();
                let disabled = item.disabled;
                let this = cx.entity().downgrade();
                let this_hover = this.clone();
                let row = div()
                    .id(item_id.clone())
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(spacing::X_SMALL))
                    .px(px(spacing::X_SMALL))
                    .py(px(py + theme.row_inset()))
                    .rounded(px(radius::LARGE))
                    .text_style_token(text)
                    .text_color(fg)
                    .child(
                        scaled_plate(scale)
                            .rounded(px(radius::LARGE * scale))
                            .bg(bg),
                    )
                    .child(probe(&host, Some(slot)))
                    .when_some(item.icon, |d, ic| {
                        d.child(Icon::new(ic).size(IconSize::Custom(px(icon))).color(fg))
                    })
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .child(item.label.clone()),
                    )
                    .when_some(item.shortcut.clone(), |d, s| {
                        d.child(
                            div()
                                .relative()
                                .ml_auto()
                                .flex_none()
                                .text_style_token(role::BODY_XS)
                                .text_color(c.foreground_tertiary)
                                .child(s),
                        )
                    })
                    .when(!disabled, |d| {
                        d.cursor_action()
                            // `track_interaction` already owns this element's
                            // `on_hover` (GPUI allows one per element and
                            // debug-asserts on a second), so pointer
                            // selection follows the web `onPointerMove`.
                            .track_interaction(&state)
                            .on_mouse_move(move |_, _, cx| {
                                let v = hover_value.clone();
                                this_hover
                                    .update(cx, |t, cx| {
                                        if t.selected.as_ref() != Some(&v) {
                                            t.selected = Some(v);
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                            })
                            .on_click(move |_, _, cx| {
                                let v = value.clone();
                                this.update(cx, |t, cx| {
                                    t.selected = Some(v.clone());
                                    cx.emit(CommandEvent::Select(v));
                                    cx.notify();
                                })
                                .ok();
                            })
                    })
                    .when(disabled, |d| d.cursor_not_allowed());
                group = group.child(scroll_edge_item(child_id(&item_id, "edge"), row));
                slot += 1;
            }
            rows.push(group.into_any_element());
            if gi + 1 < n_groups {
                rows.push(
                    div()
                        .flex()
                        .items_center()
                        .w_full()
                        .p(px(spacing::X_SMALL))
                        .child(div().w_full().h(px(stroke::THIN)).bg(c.border_default))
                        .into_any_element(),
                );
            }
        }
        let empty = rows.is_empty();
        let plate = sliding_plate(
            &child_id(&id, "indicator"),
            &host,
            selected_row,
            c.background_surface_hover,
            radius::LARGE,
            window,
            cx,
        );
        let empty_fade = animate_from(
            child_id(&id, if empty { "empty-in" } else { "empty-out" }),
            0.0f32,
            1.0,
            Motion::FROSTED,
            window,
            cx,
        );
        let list_inner = div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(spacing::XXX_SMALL))
            .px(px(spacing::MEDIUM))
            .py(px(spacing::SMALL))
            .child(probe(&host, None))
            .child(plate)
            .children(rows);
        let this = cx.entity().downgrade();
        div()
            .id(id.clone())
            .flex()
            .flex_col()
            .w_full()
            .gap(px(spacing::SMALL))
            .font_family(theme.font_family.clone())
            .text_color(c.foreground_primary)
            .on_key_down(move |e: &KeyDownEvent, _, cx| {
                let d = match e.keystroke.key.as_str() {
                    "down" => 1,
                    "up" => -1,
                    _ => return,
                };
                this.update(cx, |t, cx| {
                    t.step(d);
                    cx.notify();
                })
                .ok();
                cx.stop_propagation();
            })
            .child(self.search.clone())
            .child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .rounded(px(list_radius))
                    .border(px(stroke::HAIRLINE))
                    .border_color(c.border_default)
                    .bg(c.background_primary)
                    .shadow(theme.shadow(Elevation::SURFACE))
                    .map(|d| {
                        if empty {
                            d.child(
                                div()
                                    .opacity(empty_fade)
                                    .px(px(spacing::MEDIUM))
                                    .py(px(spacing::XX_LARGE))
                                    .text_center()
                                    .text_style_token(role::BODY_XS)
                                    .text_color(c.foreground_tertiary)
                                    .child(self.empty.clone()),
                            )
                        } else {
                            d.child(
                                ScrollArea::new(child_id(&id, "scroll"))
                                    .max_height(px(COMMAND_LIST_MAX_HEIGHT))
                                    .child(list_inner),
                            )
                        }
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_every_word_in_label_or_keywords() {
        let i = CommandItem::new("Open settings").keywords(["preferences"]);
        assert!(i.matches("open"));
        assert!(i.matches("SETTINGS open"));
        assert!(i.matches("pref"));
        assert!(!i.matches("close"));
    }
}
