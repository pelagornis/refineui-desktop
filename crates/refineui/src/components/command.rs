//! Command — web `Command` (`components/Command/style.ts`): a searchable
//! command list with a search field and a filtered, grouped results list.
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
//!
//! Web API parity (`Command`, `CommandDialog`, `CommandItem`): items carry a
//! stable `value`, `keywords`, start/end icons, a `shortcut` and an
//! `on_select`; the root takes `should_filter` / `filter` (web
//! `(value, search, keywords) -> score`), emits the search query
//! ([`CommandEvent::QueryChange`], web `onValueChange`) and the highlighted
//! item ([`CommandEvent::Highlight`], web `onSelectedValueChange`).
//! [`CommandDialog`] is the web `CommandDialog`: the same `Command` (opaque
//! panel, no material) inside a [`crate::Dialog`] whose panel is transparent,
//! so the field and the results panel float on the scrim; it defaults the
//! command to `lg`. `size` also sizes the search field, and the list caps at
//! `min(60% of the window, 320px)`.
//!
//! Matching is fuzzy (ordered subsequence, scored for word starts and runs);
//! matched label characters are highlighted, items sort by score within
//! their group and groups by their best item.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, ElementId, Entity, EventEmitter, FontWeight, HighlightStyle,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, StyledText, Subscription, Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::components::sliding_highlight::sliding_plate;
use crate::motion::{
    Motion, TrackInteraction, animate, animate_from, child_id, press_scale_to, probe, scaled_plate,
    use_interaction, use_layout_probe,
};
use crate::{
    ActiveTheme, ControlSize, Elevation, Icon, IconName, IconSize, InputEvent, ResultVariant,
    ResultView, ScrollArea, SearchField, StyledExt, TextInput, scroll_edge_item,
};

/// Results cap (`min(60vh, foundationSize3200)`).
pub const COMMAND_LIST_MAX_HEIGHT: f32 = 320.;

/// One runnable entry.
#[derive(Clone)]
pub struct CommandItem {
    value: SharedString,
    label: SharedString,
    subtitle: Option<SharedString>,
    icon: Option<IconName>,
    end_icon: Option<IconName>,
    shortcut: Option<SharedString>,
    keywords: Vec<SharedString>,
    disabled: bool,
    /// GPUI action dispatched when the item runs (desktop addition).
    action: Option<SharedString>,
    on_select: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App)>>,
}

/// A scored match: higher is better; `positions` are matched label char indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandMatch {
    /// Match quality (0 = hidden).
    pub score: f32,
    /// Char indices of the label that matched (for highlighting).
    pub positions: Vec<usize>,
}

/// Fuzzy subsequence match of `query` in `text` (case-insensitive).
///
/// Every query char must appear in order. Scores reward word-start hits,
/// consecutive runs and an early first hit; returns `None` on no match.
pub fn fuzzy_match(query: &str, text: &str) -> Option<CommandMatch> {
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if q.is_empty() {
        return Some(CommandMatch {
            score: 1.,
            positions: Vec::new(),
        });
    }
    let t: Vec<char> = text.chars().collect();
    let lower: Vec<char> = t
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let word_start = |i: usize| {
        i == 0 || !t[i - 1].is_alphanumeric() || (t[i].is_uppercase() && t[i - 1].is_lowercase())
    };
    // Greedy forward pass that prefers word starts: for each query char take
    // the next word-start occurrence if one exists before the next plain one
    // would force a miss, else the next occurrence.
    let mut positions = Vec::with_capacity(q.len());
    let mut from = 0;
    for (k, qc) in q.iter().enumerate() {
        let plain = (from..t.len()).find(|i| lower[*i] == *qc)?;
        let prefer = (plain..t.len()).find(|i| lower[*i] == *qc && word_start(*i));
        let pick = match prefer {
            // Only jump ahead if the rest of the query still fits after it.
            Some(ws) if ws != plain && subsequence(&q[k + 1..], &lower[ws + 1..]) => ws,
            _ => plain,
        };
        positions.push(pick);
        from = pick + 1;
    }
    let mut score = 0.;
    for (n, &i) in positions.iter().enumerate() {
        score += 1.;
        if word_start(i) {
            score += 2.;
        }
        if n > 0 && positions[n - 1] + 1 == i {
            score += 1.5;
        }
    }
    score += 2. / (1. + positions[0] as f32);
    // Shorter labels win ties.
    score -= t.len() as f32 * 0.01;
    Some(CommandMatch {
        score: score.max(0.01),
        positions,
    })
}

fn subsequence(q: &[char], t: &[char]) -> bool {
    let mut it = t.iter();
    q.iter().all(|c| it.any(|x| x == c))
}

/// Web `CommandFilter`: `(value, search, keywords) -> score` (0 hides).
pub type CommandFilter = Rc<dyn Fn(&str, &str, &[SharedString]) -> f32>;

impl CommandItem {
    /// Item whose value is its label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            subtitle: None,
            icon: None,
            end_icon: None,
            shortcut: None,
            keywords: Vec::new(),
            disabled: false,
            action: None,
            on_select: None,
        }
    }

    /// Secondary line under the label (desktop addition).
    pub fn subtitle(mut self, s: impl Into<SharedString>) -> Self {
        self.subtitle = Some(s.into());
        self
    }

    /// Trailing icon (web `endIcon`), e.g. a chevron.
    pub fn end_icon(mut self, icon: IconName) -> Self {
        self.end_icon = Some(icon);
        self
    }

    /// Runs when the item is chosen (web `onSelect`), in addition to
    /// [`CommandEvent::Select`].
    pub fn on_select(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// The item's value.
    pub fn value_str(&self) -> &SharedString {
        &self.value
    }

    /// The item's label.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Scores this item: the label fuzzily (with highlight positions), else
    /// keywords (substring of every query word, no highlight).
    pub fn score(&self, query: &str) -> Option<CommandMatch> {
        let q = query.trim();
        if q.is_empty() {
            return Some(CommandMatch {
                score: 1.,
                positions: Vec::new(),
            });
        }
        if let Some(m) = fuzzy_match(q, &self.label) {
            return Some(m);
        }
        let hay = std::iter::once(&self.label)
            .chain(self.keywords.iter())
            .map(|k| k.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        q.split_whitespace()
            .all(|w| hay.contains(&w.to_lowercase()))
            .then(|| CommandMatch {
                score: 0.5,
                positions: Vec::new(),
            })
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

    /// Dispatch the GPUI action named `name` (e.g. `"editor::Save"`) when
    /// this item is run; `name` is also its `value`. Without an explicit
    /// [`Self::shortcut`] the row shows the keymap shortcut bound to the
    /// action. Desktop addition (the web item has only `onSelect`).
    pub fn action(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        self.value = name.clone();
        self.action = Some(name);
        self
    }

    /// Does the item match `query` (fuzzy label or keywords)?
    pub fn matches(&self, query: &str) -> bool {
        self.score(query).is_some()
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
    /// The search query changed (web `onValueChange`).
    QueryChange(SharedString),
    /// The highlighted item changed (web `onSelectedValueChange`).
    Highlight(SharedString),
}

/// Web `size` default: `md`, or `lg` inside a `CommandDialog`.
fn resolve_size(explicit: Option<ControlSize>, in_dialog: bool) -> ControlSize {
    explicit.unwrap_or(if in_dialog {
        ControlSize::Large
    } else {
        ControlSize::Medium
    })
}

/// Web `min(60vh, 320px)` list cap for a window `viewport_h` tall.
fn list_max_height(viewport_h: f32) -> f32 {
    COMMAND_LIST_MAX_HEIGHT.min(viewport_h * 0.6)
}

/// The command list (an entity: create with `cx.new(|cx| Command::new(window, cx))`).
pub struct Command {
    search: Entity<TextInput>,
    groups: Vec<CommandGroup>,
    query: SharedString,
    selected: Option<SharedString>,
    /// Explicit `size`; `None` follows the web default (`md`, `lg` inside a
    /// [`CommandDialog`]).
    size: Option<ControlSize>,
    /// Rendered by a [`CommandDialog`] (web `commandProps.size ?? "lg"`).
    in_dialog: bool,
    /// Size last pushed to the search field (web passes `size` to `SearchField`).
    search_size: Option<ControlSize>,
    empty: SharedString,
    should_filter: bool,
    filter: Option<CommandFilter>,
    hints: bool,
    /// Enter pressed in the field: run on the next render (needs `Window`).
    pending_select: Option<SharedString>,
    _sub: Subscription,
}

impl EventEmitter<CommandEvent> for Command {}

impl Command {
    /// New command list with a "Type a command or search…" field.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| SearchField::new(cx).placeholder("Type a command or search…"));
        let sub = cx.subscribe(&search, |this: &mut Self, _, e: &InputEvent, cx| match e {
            InputEvent::Change(q) => {
                this.query = q.clone();
                // Typing re-ranks; keep the best match highlighted like cmdk.
                this.selected = None;
                this.ensure_selection();
                cx.emit(CommandEvent::QueryChange(q.clone()));
                cx.notify();
            }
            InputEvent::Submit(_) => {
                if let Some(v) = this.selected.clone() {
                    this.pending_select = Some(v);
                    cx.notify();
                }
            }
        });
        Self {
            search,
            groups: Vec::new(),
            query: SharedString::default(),
            selected: None,
            size: None,
            in_dialog: false,
            search_size: None,
            empty: "No results found.".into(),
            should_filter: true,
            filter: None,
            hints: false,
            pending_select: None,
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

    /// Size of the search field, rows, icons and list radius (web `size`;
    /// default `md`, or `lg` inside a [`CommandDialog`]).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    /// The size this command renders at.
    pub fn effective_size(&self) -> ControlSize {
        resolve_size(self.size, self.in_dialog)
    }

    pub(crate) fn set_in_dialog(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.in_dialog != on {
            self.in_dialog = on;
            cx.notify();
        }
    }

    /// Filter items by the query (web `shouldFilter`, default `true`).
    pub fn should_filter(mut self, on: bool) -> Self {
        self.should_filter = on;
        self
    }

    /// Custom scorer (web `filter`): `(value, search, keywords) -> score`,
    /// 0 hides the item. Disables label highlighting.
    pub fn filter(mut self, f: impl Fn(&str, &str, &[SharedString]) -> f32 + 'static) -> Self {
        self.filter = Some(Rc::new(f));
        self
    }

    /// Search placeholder.
    pub fn placeholder(self, p: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let p = p.into();
        self.search.update(cx, |s, cx| {
            s.set_placeholder(p, cx);
        });
        self
    }

    /// Show the keyboard hint row under the results.
    pub fn hints(mut self, on: bool) -> Self {
        self.hints = on;
        self
    }

    /// The current query.
    pub fn query(&self) -> &SharedString {
        &self.query
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

    /// Visible groups with each item's match, sorted by score while searching.
    fn visible(&self) -> Vec<(Option<SharedString>, Vec<(CommandItem, CommandMatch)>)> {
        let q = self.query.trim().to_string();
        let mut groups: Vec<_> = self
            .groups
            .iter()
            .map(|g| {
                let mut items: Vec<(CommandItem, CommandMatch)> = g
                    .items
                    .iter()
                    .filter_map(|i| {
                        let m = if !self.should_filter || q.is_empty() {
                            Some(CommandMatch {
                                score: 1.,
                                positions: Vec::new(),
                            })
                        } else if let Some(f) = &self.filter {
                            let s = f(&i.value, &q, &i.keywords);
                            (s > 0.).then(|| CommandMatch {
                                score: s,
                                positions: Vec::new(),
                            })
                        } else {
                            i.score(&q)
                        };
                        m.map(|m| (i.clone(), m))
                    })
                    .collect();
                if self.should_filter && !q.is_empty() {
                    items.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
                }
                (g.heading.clone(), items)
            })
            .filter(|(_, items)| !items.is_empty())
            .collect();
        if self.should_filter && !q.is_empty() {
            let best = |items: &Vec<(CommandItem, CommandMatch)>| {
                items.first().map(|i| i.1.score).unwrap_or(0.)
            };
            groups.sort_by(|a, b| best(&b.1).total_cmp(&best(&a.1)));
        }
        groups
    }

    fn enabled_values(&self) -> Vec<SharedString> {
        self.visible()
            .into_iter()
            .flat_map(|(_, items)| items)
            .filter(|(i, _)| !i.disabled)
            .map(|(i, _)| i.value)
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

    fn item(&self, value: &SharedString) -> Option<&CommandItem> {
        self.groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| &i.value == value)
    }

    /// Runs an item: its `on_select`, then [`CommandEvent::Select`], then
    /// its [`CommandItem::action`] (dispatched from the focused element).
    pub fn select(&mut self, value: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = Some(value.clone());
        let (on_select, action) = match self.item(&value) {
            Some(i) => (i.on_select.clone(), i.action.clone()),
            None => (None, None),
        };
        if let Some(h) = on_select {
            h(&value, window, cx);
        }
        cx.emit(CommandEvent::Select(value));
        if let Some(a) = action.and_then(|a| cx.build_action(&a, None).ok()) {
            window.dispatch_action(a, cx);
        }
        cx.notify();
    }
}

/// Highlighted label: matched chars semibold in `foregroundBrand`.
fn highlighted(label: &SharedString, positions: &[usize], brand: gpui::Hsla) -> StyledText {
    if positions.is_empty() {
        return StyledText::new(label.clone());
    }
    let mut ranges = Vec::new();
    for (ci, (bi, ch)) in label.char_indices().enumerate() {
        if positions.contains(&ci) {
            ranges.push((
                bi..bi + ch.len_utf8(),
                HighlightStyle {
                    color: Some(brand),
                    font_weight: Some(FontWeight::SEMIBOLD),
                    ..Default::default()
                },
            ));
        }
    }
    StyledText::new(label.clone()).with_highlights(ranges)
}

impl Render for Command {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(v) = self.pending_select.take() {
            self.select(v, window, cx);
        }
        let theme = cx.theme().clone();
        let c = theme.colors;
        let id: ElementId = ("refineui-command", cx.entity_id()).into();
        let size = self.effective_size();
        if self.search_size != Some(size) {
            // Web `CommandInput` passes the command `size` to its SearchField.
            self.search_size = Some(size);
            self.search.update(cx, |s, cx| s.set_size(size, cx));
        }
        // Web: the list grows up to `min(60vh, 320px)`, then scrolls.
        let list_max_h = list_max_height(f32::from(window.viewport_size().height));
        let (text, py, icon) = match size {
            ControlSize::Small => (role::BODY_SM, spacing::XX_SMALL, 16.),
            ControlSize::Medium => (role::BODY_MD, spacing::SMALL, 20.),
            ControlSize::Large => (role::BODY_LG, spacing::MEDIUM, 24.),
        };
        let list_radius = match size {
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
            for (item, matched) in items {
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
                let shortcut = item.shortcut.clone().or_else(|| {
                    item.action
                        .as_ref()
                        .and_then(|a| crate::interaction::keymap::shortcut_for(a, cx))
                        .map(SharedString::from)
                });
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
                            .flex()
                            .flex_col()
                            .child(highlighted(
                                &item.label,
                                &matched.positions,
                                c.foreground_brand,
                            ))
                            .when_some(item.subtitle.clone(), |d, s| {
                                d.child(
                                    div()
                                        .text_style_token(role::BODY_XS)
                                        .text_color(c.foreground_tertiary)
                                        .truncate()
                                        .child(s),
                                )
                            }),
                    )
                    .when_some(shortcut, |d, s| {
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
                    .when_some(item.end_icon, |d, ic| {
                        d.child(
                            Icon::new(ic)
                                .size(IconSize::Custom(px(icon * 0.8)))
                                .color(c.foreground_tertiary),
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
                                            t.selected = Some(v.clone());
                                            cx.emit(CommandEvent::Highlight(v));
                                            cx.notify();
                                        }
                                    })
                                    .ok();
                            })
                            .on_click(move |_, window, cx| {
                                let v = value.clone();
                                this.update(cx, |t, cx| t.select(v, window, cx)).ok();
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
        let panel_shadow = theme.shadow(Elevation::SURFACE);
        // Web `listShell`: an opaque `backgroundPrimary` panel with a hairline
        // `borderDefault` and `shadow-2`, inline and inside a CommandDialog.
        let plate = crate::material::Backing {
            fill: c.background_primary,
            border: c.border_default,
            shadows: panel_shadow,
            layer: None,
        };
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
                    if let Some(v) = t.selected.clone() {
                        cx.emit(CommandEvent::Highlight(v));
                    }
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
                    .border_color(plate.border)
                    .bg(plate.fill)
                    .shadow(plate.shadows)
                    .relative()
                    .children(plate.layer)
                    .map(|d| {
                        if empty {
                            // Web `CommandEmpty`: the empty state lives inside
                            // the list plate. The search field stays a sibling
                            // above this plate (already pinned / not scrolling).
                            d.child(
                                div().opacity(empty_fade).w_full().child(
                                    ResultView::new(ResultVariant::Empty)
                                        .compact(true)
                                        .title(self.empty.clone()),
                                ),
                            )
                        } else {
                            d.child(
                                ScrollArea::new(child_id(&id, "scroll"))
                                    .max_height(px(list_max_h))
                                    .child(list_inner),
                            )
                        }
                    })
                    .when(self.hints, |d| {
                        let hint = |k: &'static str, label: &'static str| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::XX_SMALL))
                                .child(crate::Kbd::new(k))
                                .child(label)
                        };
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::MEDIUM))
                                .px(px(spacing::MEDIUM))
                                .py(px(spacing::X_SMALL))
                                .border_t(px(stroke::THIN))
                                .border_color(c.border_subtle)
                                .bg(c.background_surface)
                                .text_style_token(role::CAPTION_LG)
                                .text_color(c.foreground_tertiary)
                                .child(hint("↑↓", "Navigate"))
                                .child(hint("↵", "Run"))
                                .child(hint("esc", "Close")),
                        )
                    }),
            )
    }
}

/// Web `CommandDialog`: a [`Command`] inside a [`crate::Dialog`] whose panel
/// is transparent (the search field and results panel float on the scrim).
/// The command looks exactly like an inline one (same opaque results panel,
/// search field, rows and tokens), at `lg` unless [`Command::size`] was set.
/// The only dialog chrome is the modal scrim: `surfaceOverlay` with the
/// web's 2px `overlayScrim` backdrop blur when materials are active.
///
/// It is only a composition, like the web: [`crate::Dialog`] (`bare`,
/// `size` default `Large`) around the command. Controlled like `Dialog`:
/// keep rendering it and pass `open`; close it in `on_close` (scrim click or
/// Escape). The Dialog focuses the search field on open (web: first
/// focusable in the panel) and gives focus back on close; each opening
/// starts with an empty query (the web unmounts the closed dialog's
/// Command). Items with [`CommandItem::action`] dispatch their action; close
/// the dialog on [`CommandEvent::Select`] for ⇧⌘P-style use.
#[derive(IntoElement)]
pub struct CommandDialog {
    id: ElementId,
    command: Entity<Command>,
    open: bool,
    size: crate::DialogSize,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    inline: bool,
}

impl CommandDialog {
    /// Dialog around `command`.
    pub fn new(id: impl Into<ElementId>, command: Entity<Command>) -> Self {
        Self {
            id: id.into(),
            command,
            open: true,
            size: crate::DialogSize::Large,
            on_close: None,
            inline: false,
        }
    }

    /// Open state (web `open`).
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Dialog width (web `size`).
    pub fn size(mut self, size: crate::DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Close request (web `onOpenChange(false)`).
    pub fn on_close(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    /// Render in flow (docs / previews).
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

impl RenderOnce for CommandDialog {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Web: `<Dialog size="lg"><DialogContent transparent, unscrolled>
        // <Command size={commandProps.size ?? "lg"}>`. Everything else
        // (scrim, Escape / outside click, focus into the panel and back) is
        // the Dialog's own behaviour.
        self.command.update(cx, |c, cx| c.set_in_dialog(true, cx));
        let search = self.command.read(cx).search().clone();
        let search = gpui::Focusable::focus_handle(search.read(cx), cx);
        let command = self.command.clone();
        let mut d = crate::Dialog::new(self.id, "Command")
            .bare(true)
            .size(self.size)
            .open(self.open)
            .inline(self.inline)
            .initial_focus(search)
            .on_open(move |_, cx| command.update(cx, |c, cx| c.reset(cx)))
            .child(self.command);
        if let Some(h) = self.on_close {
            d = d.on_close(move |w, cx| h(w, cx));
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_defaults_follow_the_web() {
        assert_eq!(resolve_size(None, false), ControlSize::Medium);
        assert_eq!(resolve_size(None, true), ControlSize::Large);
        assert_eq!(
            resolve_size(Some(ControlSize::Small), true),
            ControlSize::Small
        );
        assert_eq!(list_max_height(1000.), COMMAND_LIST_MAX_HEIGHT);
        assert!((list_max_height(400.) - 240.).abs() < 0.01);
    }

    #[test]
    fn matches_every_word_in_label_or_keywords() {
        let i = CommandItem::new("Open settings").keywords(["preferences"]);
        assert!(i.matches("open"));
        assert!(i.matches("SETTINGS open"));
        assert!(i.matches("pref"));
        assert!(!i.matches("close"));
    }

    #[test]
    fn action_sets_value_and_dispatch_target() {
        let i = CommandItem::new("Save").action("editor::Save");
        assert_eq!(i.value_str().as_ref(), "editor::Save");
        assert_eq!(i.action.as_ref().map(|a| a.as_ref()), Some("editor::Save"));
        assert_eq!(i.label().as_ref(), "Save");
        assert!(CommandItem::new("Open").action.is_none());
    }

    #[test]
    fn fuzzy_prefers_word_starts_and_reports_positions() {
        let m = fuzzy_match("tsb", "Toggle Side Bar").unwrap();
        assert_eq!(m.positions, vec![0, 7, 12]);
        let a = fuzzy_match("set", "Settings").unwrap();
        let b = fuzzy_match("set", "Reset view").unwrap();
        assert!(a.score > b.score);
        assert!(fuzzy_match("xyz", "Settings").is_none());
        assert!(fuzzy_match("ofi", "Open File").is_some());
    }
}
