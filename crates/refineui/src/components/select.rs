//! Select — web `Select` (`filled | ghost | plain`, `sm | md | lg`).
//! Parts: trigger, value, content, group / label, separator, item, and a
//! leading check item indicator (web `SelectItemIndicator`).
//! Keyboard: Space/Enter/↓/↑ opens, ↑/↓ move (skipping disabled items),
//! Home/End jump, typeahead by first letter, Enter/Space picks, Escape closes.
//!
//! Placement is always anchored to the trigger: open below when there is
//! room (after `collision_padding` and `side_offset`), flip above when
//! there is not, and shrink the panel to the remaining space with an
//! internal [`ScrollArea`] (the selected item is scrolled into view). The
//! popup never covers the trigger.

use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, KeyDownEvent, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, anchored, div, prelude::*, px,
};

use gpui::{CursorStyle, InteractiveElement, Transformation};
use refineui_foundation::{radius, size::icon, spacing, stroke, typography::role};

use crate::components::plate::row_plate;
use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, presence, press_scale,
    probe, scaled_plate, use_interaction, use_layout_probe,
};
use crate::{
    ActiveTheme, CollisionOptions, ControlSize, FloatingLayout, FloatingSide, Icon, IconName,
    IconSize, PlacementRect, PlacementViewport, ScrollArea, StyledExt, place_popper,
    scroll_edge_item,
};

/// Trigger appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectAppearance {
    /// Hairline-bordered field on `backgroundPrimary` (default).
    #[default]
    Filled,
    /// Borderless; hover / open shows the `backgroundPrimaryHover` plate.
    Ghost,
    /// Text and chevron only.
    Plain,
}

#[derive(Default)]
struct SelectState {
    open: bool,
    highlighted: Option<usize>,
    /// Set when the menu opens: scroll the selected / highlighted row into
    /// view on the next frame.
    align_scroll: bool,
    /// Geometry captured while open. The exit fade reuses it so the panel
    /// fades in place instead of jumping to re-align with a newly picked item.
    frozen: Option<SelectLayout>,
    /// Last highlighted index we scrolled into view, so arrow keys move the
    /// viewport without fighting a later pointer scroll.
    last_scrolled_hl: Option<usize>,
}

/// One selectable row (web `SelectItem`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    /// Stable value (web `SelectItem` `value`).
    pub value: SharedString,
    /// Visible label (web `SelectItem` children / `SelectItemText`).
    pub label: SharedString,
    /// When true the row is not highlighted or picked (web `disabled`).
    pub disabled: bool,
    /// Typeahead string; defaults to [`Self::label`] (web `textValue`).
    pub text_value: Option<SharedString>,
}

impl SelectOption {
    /// An enabled option whose value and label are `label`.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            disabled: false,
            text_value: None,
        }
    }

    /// An option with a distinct `value` and `label`.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Disabled option.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Typeahead text (defaults to the label).
    pub fn text_value(mut self, text: impl Into<SharedString>) -> Self {
        self.text_value = Some(text.into());
        self
    }

    fn needle(&self) -> &str {
        self.text_value
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(self.label.as_ref())
    }
}

impl From<&str> for SelectOption {
    fn from(label: &str) -> Self {
        Self::new(SharedString::from(label.to_owned()))
    }
}

impl From<String> for SelectOption {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

impl From<SharedString> for SelectOption {
    fn from(label: SharedString) -> Self {
        Self::new(label)
    }
}

/// A row in the Select popup: an item, a group label, or a separator
/// (web `SelectItem` / `SelectLabel` / `SelectSeparator`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectEntry {
    /// Selectable option.
    Item(SelectOption),
    /// Group heading (web `SelectLabel`).
    Label(SharedString),
    /// Visual divider (web `SelectSeparator`).
    Separator,
}

/// Placement of an open Select popup. Alias of [`FloatingLayout`]; see that
/// type for the mapping to `--refineui-select-content-available-height`,
/// `--refineui-select-trigger-height`, and
/// `--refineui-select-content-transform-origin`.
pub type SelectLayout = FloatingLayout;

/// Web `CONTENT_MARGIN` (`foundationSize100`).
const CONTENT_MARGIN: f32 = 10.0;
/// Item row: `p-x-small` (6) × 2 + `bodyMd` line (20).
const ITEM_HEIGHT: f32 = 32.0;
/// Group `gap-px` between rows.
const ITEM_GAP: f32 = 1.0;
/// Group label: `py-xxx-small` × 2 + `captionLg` line (16).
const LABEL_HEIGHT: f32 = spacing::XXX_SMALL * 2. + 16.;
/// Separator: `my-x-small` × 2 + 1px rule.
const SEPARATOR_HEIGHT: f32 = spacing::X_SMALL * 2. + 1.;
/// Content shell inset: hairline border + `p-xx-small`.
const SHELL_INSET: f32 = stroke::HAIRLINE + spacing::XX_SMALL;
fn entry_height(entry: &SelectEntry) -> f32 {
    match entry {
        SelectEntry::Item(_) => ITEM_HEIGHT,
        SelectEntry::Label(_) => LABEL_HEIGHT,
        SelectEntry::Separator => SEPARATOR_HEIGHT,
    }
}

fn list_full_height(entries: &[SelectEntry]) -> f32 {
    let mut h = SHELL_INSET * 2.;
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            h += ITEM_GAP;
        }
        h += entry_height(e);
    }
    h.max(SHELL_INSET * 2. + ITEM_HEIGHT)
}

fn item_offset_top(entries: &[SelectEntry], item_ix: usize) -> f32 {
    let mut y = SHELL_INSET;
    let mut seen = 0usize;
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            y += ITEM_GAP;
        }
        if matches!(e, SelectEntry::Item(_)) {
            if seen == item_ix {
                return y;
            }
            seen += 1;
        }
        y += entry_height(e);
    }
    y
}

fn items(entries: &[SelectEntry]) -> impl Iterator<Item = (usize, &SelectOption)> {
    entries
        .iter()
        .filter_map(|e| match e {
            SelectEntry::Item(o) => Some(o),
            _ => None,
        })
        .enumerate()
}

fn item_count(entries: &[SelectEntry]) -> usize {
    entries
        .iter()
        .filter(|e| matches!(e, SelectEntry::Item(_)))
        .count()
}

fn item_at(entries: &[SelectEntry], ix: usize) -> Option<&SelectOption> {
    items(entries).find_map(|(i, o)| (i == ix).then_some(o))
}

/// Next enabled item index in `dir` (+1 / −1), clamped (web does not wrap).
fn step_enabled(entries: &[SelectEntry], from: usize, dir: isize) -> usize {
    let opts: Vec<(usize, bool)> = items(entries).map(|(i, o)| (i, o.disabled)).collect();
    if opts.is_empty() {
        return from;
    }
    let mut i = from as isize;
    loop {
        i += dir;
        if i < 0 {
            return opts
                .iter()
                .find(|(_, d)| !*d)
                .map(|(ix, _)| *ix)
                .unwrap_or(from);
        }
        if i as usize >= opts.len() {
            return opts
                .iter()
                .rev()
                .find(|(_, d)| !*d)
                .map(|(ix, _)| *ix)
                .unwrap_or(from);
        }
        if !opts[i as usize].1 {
            return opts[i as usize].0;
        }
    }
}

fn first_enabled(entries: &[SelectEntry]) -> Option<usize> {
    items(entries).find_map(|(i, o)| (!o.disabled).then_some(i))
}

fn last_enabled(entries: &[SelectEntry]) -> Option<usize> {
    items(entries)
        .filter(|(_, o)| !o.disabled)
        .map(|(i, _)| i)
        .last()
}

/// Web typeahead: first character, search after `from`, then wrap.
fn typeahead(entries: &[SelectEntry], from: Option<usize>, key: &str) -> Option<usize> {
    let needle = key.to_lowercase();
    if needle.chars().count() != 1 {
        return None;
    }
    let opts: Vec<(usize, String, bool)> = items(entries)
        .map(|(i, o)| (i, o.needle().to_lowercase(), o.disabled))
        .collect();
    if opts.is_empty() {
        return None;
    }
    let start = from.map(|f| f + 1).unwrap_or(0);
    opts.iter()
        .skip(start)
        .chain(opts.iter())
        .find(|(_, label, disabled)| !disabled && label.starts_with(&needle))
        .map(|(i, _, _)| *i)
}

/// Collision-aware placement for an open Select, in window coordinates.
///
/// Always [`place_popper`] with preferred side [`FloatingSide::Bottom`]:
/// detect room below the trigger, flip above when that room is short, and
/// shrink to the remaining space. The panel never covers the trigger.
fn select_layout(
    trigger: gpui::Bounds<gpui::Pixels>,
    entries: &[SelectEntry],
    viewport: gpui::Size<gpui::Pixels>,
    opts: CollisionOptions,
) -> SelectLayout {
    let trigger = PlacementRect::from_bounds(trigger);
    let viewport = PlacementViewport::from_size(viewport);
    place_popper(
        trigger,
        viewport,
        trigger.width,
        list_full_height(entries),
        FloatingSide::Bottom,
        opts,
    )
}

/// Keep the highlighted row inside the visible viewport when the user
/// arrows through a list taller than the open panel.
fn ensure_item_visible(
    scroll: &gpui::ScrollHandle,
    entries: &[SelectEntry],
    index: usize,
    viewport_h: f32,
) {
    if viewport_h <= 0. {
        return;
    }
    let pad = spacing::XX_SMALL;
    let item_top = item_offset_top(entries, index);
    let item_bottom = item_top + ITEM_HEIGHT;
    let current = -f32::from(scroll.offset().y);
    let mut next = current;
    if item_top < current + pad {
        next = (item_top - pad).max(0.);
    } else if item_bottom > current + viewport_h - pad {
        next = (item_bottom + pad - viewport_h).max(0.);
    }
    if (next - current).abs() > 0.5 {
        scroll.set_offset(gpui::point(px(0.), px(-next)));
    }
}

/// Single-choice dropdown.
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    entries: Vec<SelectEntry>,
    selected: Option<usize>,
    placeholder: SharedString,
    size: ControlSize,
    appearance: SelectAppearance,
    disabled: bool,
    width: Option<gpui::Pixels>,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
    /// Open on first mount (stories / screenshots). After that the
    /// element's own open state takes over.
    default_open: bool,
    collision_padding: f32,
    side_offset: f32,
}

impl Select {
    /// New select.
    pub fn new(
        id: impl Into<ElementId>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        selected: Option<usize>,
    ) -> Self {
        Self {
            id: id.into(),
            entries: options
                .into_iter()
                .map(|o| SelectEntry::Item(SelectOption::from(o.into())))
                .collect(),
            selected,
            placeholder: "Select…".into(),
            size: ControlSize::Medium,
            appearance: SelectAppearance::Filled,
            disabled: false,
            width: None,
            on_change: None,
            default_open: false,
            collision_padding: CONTENT_MARGIN,
            side_offset: crate::DEFAULT_SIDE_OFFSET,
        }
    }

    /// Placeholder when nothing is selected.
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// Size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Trigger appearance.
    pub fn appearance(mut self, a: SelectAppearance) -> Self {
        self.appearance = a;
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fixed width (default: fill parent).
    pub fn width(mut self, w: gpui::Pixels) -> Self {
        self.width = Some(w);
        self
    }

    /// Start with the list open (first frame only; used by the gallery
    /// screenshot of the open popup).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Viewport-edge padding used when flipping and clamping the popup
    /// (web `collisionPadding`, default 10).
    pub fn collision_padding(mut self, padding: f32) -> Self {
        self.collision_padding = padding.max(0.);
        self
    }

    /// Gap between the trigger and the popup (web `sideOffset`, default 4).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset.max(0.);
        self
    }

    /// Called with the picked item index (skipping labels and separators).
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    /// Replace the list with structured entries (items, group labels,
    /// separators) — web `SelectContent` children.
    pub fn entries(mut self, entries: impl IntoIterator<Item = SelectEntry>) -> Self {
        self.entries = entries.into_iter().collect();
        self
    }

    /// Append a group label and its items (web `SelectGroup` + `SelectLabel`).
    pub fn group(
        mut self,
        label: impl Into<SharedString>,
        options: impl IntoIterator<Item = impl Into<SelectOption>>,
    ) -> Self {
        self.entries.push(SelectEntry::Label(label.into()));
        self.entries
            .extend(options.into_iter().map(|o| SelectEntry::Item(o.into())));
        self
    }

    /// Append a divider (web `SelectSeparator`).
    pub fn separator(mut self) -> Self {
        self.entries.push(SelectEntry::Separator);
        self
    }

    /// Append a single item (web `SelectItem`).
    pub fn item(mut self, option: impl Into<SelectOption>) -> Self {
        self.entries.push(SelectEntry::Item(option.into()));
        self
    }

    /// Stretch to the parent's width (web `fullWidth`). Already the default
    /// when [`Self::width`] is not set.
    pub fn full_width(mut self, on: bool) -> Self {
        if on {
            self.width = None;
        }
        self
    }
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let default_open = self.default_open;
        let state = window.use_keyed_state(
            ElementId::NamedChild(Box::new(self.id.clone()), "state".into()),
            cx,
            |_, _| SelectState {
                open: default_open,
                align_scroll: default_open,
                ..SelectState::default()
            },
        );
        let (open, highlighted) = {
            let s = state.read(cx);
            (s.open, s.highlighted)
        };
        let p = presence(
            child_id(&self.id, "presence"),
            open,
            Motion::FROSTED,
            Motion::FROSTED,
            window,
            cx,
        );
        let probe_state = use_layout_probe(&self.id, window, cx);
        let trigger_width = probe_state.read(cx).container.map(|b| b.size.width);
        let label = self
            .selected
            .and_then(|i| item_at(&self.entries, i).map(|o| o.label.clone()));
        let has_value = label.is_some();
        let n = item_count(&self.entries);
        let selected = self.selected;
        let m = theme.select_metrics(self.size);
        let trigger_id = child_id(&self.id, "trigger");
        let (tstate, ti) = use_interaction(&trigger_id, window, cx);
        let ti = if self.disabled {
            Interaction::default()
        } else {
            ti
        };
        // refineui.css select-trigger: hover / active / open plate.
        let plate = match self.appearance {
            SelectAppearance::Plain => gpui::transparent_black(),
            SelectAppearance::Filled if self.disabled => c.background_surface_disabled,
            _ if self.disabled => gpui::transparent_black(),
            SelectAppearance::Filled => {
                if ti.pressed {
                    c.background_primary_active
                } else if ti.hovered || open {
                    c.background_primary_hover
                } else {
                    c.background_primary
                }
            }
            SelectAppearance::Ghost => {
                if ti.pressed {
                    c.background_primary_active
                } else if ti.hovered || open {
                    c.background_primary_hover
                } else {
                    gpui::transparent_black()
                }
            }
        };
        let fg = if self.disabled {
            c.foreground_disabled
        } else if !has_value {
            c.foreground_placeholder
        } else if ti.hovered && !ti.pressed && !open {
            c.foreground_primary_hover
        } else {
            c.foreground_primary
        };
        let plate = animate(child_id(&self.id, "plate"), plate, Motion::FAST, window, cx);
        // `select-trigger:active` / `[data-pressed]` → press scale.
        let t_scale = press_scale(child_id(&self.id, "press"), ti.pressed, window, cx);
        let trigger_border = match self.appearance {
            SelectAppearance::Filled if self.disabled => Some(c.border_disabled),
            SelectAppearance::Filled => Some(c.border_default),
            _ => None,
        };
        let list_id = child_id(&self.id, "list");
        let scroll = ScrollArea::handle(list_id.clone(), window, cx);
        let collision = CollisionOptions {
            collision_padding: self.collision_padding,
            side_offset: self.side_offset,
        };
        let geometry = probe_state
            .read(cx)
            .container
            .map(|t| select_layout(t, &self.entries, window.viewport_size(), collision));
        // Freeze the panel geometry during the exit so picking an item does not
        // re-align (move) the fading panel to the new selection.
        let geometry = if p.open {
            if geometry.is_some() && state.read(cx).frozen != geometry {
                state.update(cx, |s, _| s.frozen = geometry);
            }
            geometry
        } else {
            state.read(cx).frozen.or(geometry)
        };
        if p.mounted && !crate::vibrancy::native_frost_active(cx) {
            if state.read(cx).align_scroll
                && let Some(_layout) = geometry
            {
                if let Some(ix) = self.selected.or(highlighted) {
                    let vh = f32::from(scroll.bounds().size.height);
                    if vh > 0. {
                        ensure_item_visible(&scroll, &self.entries, ix, vh);
                    }
                }
                state.update(cx, |s, _| {
                    s.align_scroll = false;
                    s.last_scrolled_hl = highlighted.or(self.selected);
                });
            } else if p.open
                && let Some(ix) = highlighted
                && state.read(cx).last_scrolled_hl != Some(ix)
            {
                let vh = f32::from(scroll.bounds().size.height);
                if vh > 0. {
                    ensure_item_visible(&scroll, &self.entries, ix, vh);
                    state.update(cx, |s, _| s.last_scrolled_hl = Some(ix));
                }
            }
        }
        let fg = animate(child_id(&self.id, "fg"), fg, Motion::FAST, window, cx);
        let chevron = animate(
            child_id(&self.id, "chevron"),
            if open { 1.0 } else { 0.0 },
            Motion::FAST,
            window,
            cx,
        );
        let appearance = self.appearance;
        let ring = theme.focus_ring();
        let show_focus = focus_visible(cx);

        let toggle = state.clone();
        let keys = state.clone();
        let close_out = state.clone();
        let native_close = state.clone();
        let on_change_key = self.on_change.clone();
        let key_entries = self.entries.clone();

        let native = crate::vibrancy::native_frost_active(cx);
        let host = use_layout_probe(&child_id(&self.id, "highlight"), window, cx);
        let (options, slide_plate) = if p.mounted && !native {
            let (rows, plate) = select_rows(
                &self.id,
                self.entries.clone(),
                selected,
                highlighted,
                &state,
                self.on_change.clone(),
                window,
                cx,
            );
            (rows, Some(plate))
        } else {
            (Vec::new(), None)
        };
        if native {
            let bounds = geometry
                .map(|l| gpui::Bounds::new(l.origin(), gpui::size(px(l.width), px(l.height))));
            let list = self.entries.clone();
            let list_state = state.clone();
            let still = state.clone();
            let close_state = state.clone();
            let on_change = self.on_change.clone();
            let parent = self.id.clone();
            crate::vibrancy::drive_native_popup(
                &self.id,
                open,
                bounds,
                move || crate::vibrancy::NativePopupSpec {
                    radius: radius::XX_LARGE,
                    close: Rc::new(move |cx| {
                        close_state.update(cx, |s, cx| {
                            if s.open {
                                s.open = false;
                                cx.notify();
                            }
                        })
                    }),
                    still_open: Rc::new(move |cx| still.read(cx).open),
                    content: Rc::new(move |window, cx| {
                        let layout = geometry.unwrap_or(SelectLayout {
                            x: 0.,
                            y: 0.,
                            width: 0.,
                            height: 0.,
                            available_height: 0.,
                            trigger_height: 0.,
                            transform_origin: crate::TransformOrigin { x: 0., y: 0. },
                            side: FloatingSide::Bottom,
                        });
                        native_select_list(
                            &parent,
                            list.clone(),
                            selected,
                            &list_state,
                            on_change.clone(),
                            layout.width,
                            layout.height,
                            window,
                            cx,
                        )
                    }),
                    close_on_deactivate: true,
                    report_size: None,
                    popover: false,
                },
                window,
                cx,
            );
        }

        div()
            .id(self.id)
            .relative()
            .map(|d| match self.width {
                Some(w) => d.w(w),
                None => d.w_full(),
            })
            .child(
                div()
                    .id(trigger_id)
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::X_SMALL))
                    .map(|d| match appearance {
                        // Hairline border is painted on the plate; the box
                        // reserves it as padding.
                        SelectAppearance::Filled => d
                            .min_h(px(m.height))
                            .px(px(spacing::MEDIUM + stroke::HAIRLINE))
                            .py(px(spacing::X_SMALL + stroke::HAIRLINE))
                            .rounded(px(radius::LARGE)),
                        SelectAppearance::Ghost => d
                            .min_h(px(m.height))
                            .px(px(spacing::X_SMALL))
                            .py(px(spacing::XX_SMALL))
                            .rounded(px(radius::LARGE)),
                        SelectAppearance::Plain => d,
                    })
                    .child(
                        scaled_plate(t_scale)
                            .rounded(px(radius::LARGE * t_scale))
                            .bg(plate)
                            .when_some(trigger_border, |d, b| {
                                d.border(px(stroke::HAIRLINE)).border_color(b)
                            }),
                    )
                    .text_style_token(m.text)
                    .text_color(fg)
                    .child(probe(&probe_state, None))
                    .when(!self.disabled, |d| {
                        d.cursor_action()
                            .track_interaction(&tstate)
                            .tab_index(0)
                            .when(show_focus, |d| d.focus(move |s| s.shadow(ring)))
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |s, cx| {
                                    s.open = !s.open;
                                    s.highlighted = None;
                                    s.align_scroll = s.open;
                                    cx.notify();
                                })
                            })
                            .on_key_down(move |e: &KeyDownEvent, w, cx| {
                                let key = e.keystroke.key.as_str();
                                let mods = e.keystroke.modifiers;
                                let (is_open, hl) = {
                                    let s = keys.read(cx);
                                    (s.open, s.highlighted)
                                };
                                match key {
                                    "down" | "up" if n > 0 => {
                                        cx.stop_propagation();
                                        keys.update(cx, |s, cx| {
                                            if !s.open {
                                                s.open = true;
                                                s.align_scroll = true;
                                                s.highlighted = selected
                                                    .filter(|i| {
                                                        item_at(&key_entries, *i)
                                                            .is_some_and(|o| !o.disabled)
                                                    })
                                                    .or_else(|| first_enabled(&key_entries));
                                            } else {
                                                let cur = hl.or(selected).unwrap_or(0);
                                                let dir = if key == "down" { 1 } else { -1 };
                                                s.highlighted =
                                                    Some(step_enabled(&key_entries, cur, dir));
                                            }
                                            cx.notify();
                                        });
                                    }
                                    "home" if is_open => {
                                        cx.stop_propagation();
                                        if let Some(ix) = first_enabled(&key_entries) {
                                            keys.update(cx, |s, cx| {
                                                s.highlighted = Some(ix);
                                                cx.notify();
                                            });
                                        }
                                    }
                                    "end" if is_open => {
                                        cx.stop_propagation();
                                        if let Some(ix) = last_enabled(&key_entries) {
                                            keys.update(cx, |s, cx| {
                                                s.highlighted = Some(ix);
                                                cx.notify();
                                            });
                                        }
                                    }
                                    "enter" | "space" => {
                                        cx.stop_propagation();
                                        let pick = is_open
                                            && hl.is_some_and(|ix| {
                                                item_at(&key_entries, ix)
                                                    .is_some_and(|o| !o.disabled)
                                            });
                                        if pick && let (Some(ix), Some(h)) = (hl, &on_change_key) {
                                            h(&ix, w, cx);
                                        }
                                        keys.update(cx, |s, cx| {
                                            if pick || !is_open {
                                                s.open = !is_open;
                                                s.align_scroll = !is_open;
                                                s.highlighted = if is_open {
                                                    None
                                                } else {
                                                    selected
                                                        .filter(|i| {
                                                            item_at(&key_entries, *i)
                                                                .is_some_and(|o| !o.disabled)
                                                        })
                                                        .or_else(|| first_enabled(&key_entries))
                                                };
                                            }
                                            cx.notify();
                                        });
                                    }
                                    "escape" if is_open => {
                                        cx.stop_propagation();
                                        keys.update(cx, |s, cx| {
                                            s.open = false;
                                            cx.notify();
                                        });
                                    }
                                    _ if is_open
                                        && !mods.control
                                        && !mods.alt
                                        && !mods.platform
                                        && key.chars().count() == 1 =>
                                    {
                                        if let Some(ix) =
                                            typeahead(&key_entries, hl.or(selected), key)
                                        {
                                            cx.stop_propagation();
                                            keys.update(cx, |s, cx| {
                                                s.highlighted = Some(ix);
                                                cx.notify();
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            })
                    })
                    .when(self.disabled, |d| {
                        d.cursor(CursorStyle::OperationNotAllowed)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(label.unwrap_or(self.placeholder)),
                    )
                    .child(
                        Icon::new(IconName::ChevronDown)
                            .size(IconSize::XSmall)
                            .color(fg)
                            .transform(Transformation::rotate(gpui::radians(
                                chevron * std::f32::consts::PI,
                            ))),
                    ),
            )
            .when(native && open, |d| {
                // Clicks in the pop-up window never reach this window.
                d.on_mouse_down_out(move |_, _, cx| {
                    native_close.update(cx, |s, cx| {
                        s.open = false;
                        cx.notify();
                    })
                })
            })
            .when(p.mounted && !native, |d| {
                d.child(
                    crate::layer(
                        anchored()
                            .when_some(geometry, |a, layout| {
                                // Slide a few px out of the trigger as the
                                // popup enters, along `transform_origin`.
                                let mut y = layout.y;
                                if p.progress < 0.999 {
                                    let slide = (1.0 - p.progress) * 6.0;
                                    y += match layout.side {
                                        FloatingSide::Bottom => -slide,
                                        FloatingSide::Top => slide,
                                    };
                                }
                                a.position(gpui::point(px(layout.x), px(y)))
                            })
                            .child(
                                // Outer box: the frosted plate (a material
                                // when active) — it must not scroll, so the
                                // list scrolls in an inner box.
                                crate::material::frosted_plate(
                                    div()
                                        // Hidden until the trigger has been measured.
                                        .opacity(if geometry.is_some() { p.progress } else { 0. })
                                        .when_some(trigger_width, |d, w| d.min_w(w))
                                        .when_some(geometry, |d, layout| {
                                            d.min_w(px(layout.width)).h(px(layout.height))
                                        })
                                        .flex()
                                        .flex_col()
                                        .font_family(theme.font_family.clone())
                                        .when(p.open, |d| {
                                            d.occlude().on_mouse_down_out(move |_, _, cx| {
                                                close_out.update(cx, |s, cx| {
                                                    s.open = false;
                                                    cx.notify();
                                                })
                                            })
                                        }),
                                    crate::SurfaceKind::Menu,
                                    cx,
                                )
                                .child(
                                    ScrollArea::new(list_id).child(
                                        div()
                                            .p(px(spacing::XX_SMALL))
                                            .flex()
                                            .flex_col()
                                            .gap(px(1.))
                                            .child(
                                                // Non-scrolling content box: the
                                                // hover plate is positioned
                                                // against it so it scrolls with
                                                // the rows.
                                                div()
                                                    .relative()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(1.))
                                                    .child(probe(&host, None))
                                                    .children(slide_plate)
                                                    .children(options),
                                            ),
                                    ),
                                ),
                            ),
                    )
                    .priority(1),
                )
            })
    }
}

type OnChange = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// The option rows (and the sliding hover plate) of an open select,
/// rendered in whichever window shows the list.
#[allow(clippy::too_many_arguments)]
fn select_rows(
    parent: &ElementId,
    entries: Vec<SelectEntry>,
    selected: Option<usize>,
    highlighted: Option<usize>,
    state: &gpui::Entity<SelectState>,
    on_change: Option<OnChange>,
    window: &mut Window,
    cx: &mut App,
) -> (Vec<gpui::AnyElement>, gpui::AnyElement) {
    let c = cx.theme().colors;
    let parent = parent.clone();
    let parent = &parent;
    let mut options = Vec::new();
    let mut plate_target = None;
    let host = use_layout_probe(&child_id(parent, "highlight"), window, cx);
    {
        let mut item_ix = 0usize;
        for (row_ix, entry) in entries.into_iter().enumerate() {
            match entry {
                SelectEntry::Label(text) => {
                    let id = ElementId::NamedChild(
                        Box::new(parent.clone()),
                        format!("label-{row_ix}").into(),
                    );
                    options.push(
                        div()
                            .id(id)
                            .flex_none()
                            .px(px(spacing::X_SMALL))
                            .py(px(spacing::XXX_SMALL))
                            .text_color(c.foreground_tertiary)
                            .text_style_token(role::CAPTION_LG)
                            .child(text)
                            .into_any_element(),
                    );
                }
                SelectEntry::Separator => {
                    let id = ElementId::NamedChild(
                        Box::new(parent.clone()),
                        format!("sep-{row_ix}").into(),
                    );
                    options.push(
                        div()
                            .id(id)
                            .flex_none()
                            .w_full()
                            .h(px(SEPARATOR_HEIGHT))
                            .flex()
                            .items_center()
                            .child(div().w_full().h(px(1.)).bg(c.border_default))
                            .into_any_element(),
                    );
                }
                SelectEntry::Item(opt) => {
                    let ix = item_ix;
                    item_ix += 1;
                    let disabled = opt.disabled;
                    let item_id =
                        ElementId::NamedChild(Box::new(parent.clone()), format!("opt-{ix}").into());
                    let (istate, mut ii) = use_interaction(&item_id, window, cx);
                    if !disabled && (Some(ix) == highlighted || ii.hovered) {
                        plate_target = Some(ix);
                    }
                    // The sliding plate carries hover; rows keep only their
                    // selected / pressed fills (web `[data-sliding-highlight]`).
                    if !ii.pressed {
                        ii.hovered = false;
                    }
                    let is_sel = Some(ix) == selected;
                    let (bg, ifg) = row_plate(ii, is_sel, disabled, c.foreground_primary, cx);
                    let bg = animate(child_id(&item_id, "bg"), bg, Motion::FAST, window, cx);
                    let scale = press_scale(
                        child_id(&item_id, "press"),
                        ii.pressed && !disabled,
                        window,
                        cx,
                    );
                    let handler = on_change.clone();
                    let st = state.clone();
                    options.push(
                        scroll_edge_item(
                            child_id(&item_id, "edge"),
                            div()
                                .id(item_id)
                                .relative()
                                .flex()
                                .flex_none()
                                .items_center()
                                .gap(px(spacing::X_SMALL))
                                .p(px(spacing::X_SMALL))
                                .rounded(px(radius::LARGE))
                                .child(
                                    scaled_plate(scale)
                                        .rounded(px(radius::LARGE * scale))
                                        .bg(bg),
                                )
                                .text_color(ifg)
                                .text_style_token(role::BODY_MD)
                                .map(|d| {
                                    if disabled {
                                        d.cursor(CursorStyle::OperationNotAllowed)
                                    } else {
                                        d.cursor_action()
                                    }
                                })
                                .track_interaction(&istate)
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .when(!disabled, |d| {
                                    d.on_click(move |_, w, cx| {
                                        if let Some(h) = &handler {
                                            h(&ix, w, cx);
                                        }
                                        st.update(cx, |s, cx| {
                                            s.open = false;
                                            cx.notify();
                                        });
                                    })
                                })
                                .child(
                                    div()
                                        .flex_none()
                                        .size(px(icon::X_SMALL))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .when(is_sel, |d| {
                                            d.child(
                                                Icon::new(IconName::Checkmark)
                                                    .size(IconSize::XSmall)
                                                    .color(ifg),
                                            )
                                        }),
                                )
                                .child(div().flex_1().min_w_0().truncate().child(opt.label))
                                .child(probe(&host, Some(ix))),
                        )
                        .into_any_element(),
                    );
                }
            }
        }
    }
    let slide_plate = {
        crate::components::sliding_highlight::sliding_plate(
            &child_id(parent, "highlight"),
            &host,
            plate_target,
            c.background_primary_hover,
            radius::LARGE,
            window,
            cx,
        )
    };
    (options, slide_plate.into_any_element())
}

/// The list panel of a native-frost select (inside its pop-up window).
#[allow(clippy::too_many_arguments)]
fn native_select_list(
    parent: &ElementId,
    entries: Vec<SelectEntry>,
    selected: Option<usize>,
    state: &gpui::Entity<SelectState>,
    on_change: Option<OnChange>,
    min_width: f32,
    height: f32,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let theme = cx.theme().clone();
    let list_id = child_id(parent, "native-list");
    let scroll = ScrollArea::handle(list_id.clone(), window, cx);
    let highlighted = state.read(cx).highlighted;
    if state.read(cx).align_scroll {
        if let Some(ix) = selected.or(highlighted) {
            let vh = f32::from(scroll.bounds().size.height);
            if vh > 0. {
                ensure_item_visible(&scroll, &entries, ix, vh);
            }
        }
        state.update(cx, |s, _| {
            s.align_scroll = false;
            s.last_scrolled_hl = highlighted.or(selected);
        });
    } else if let Some(ix) = highlighted
        && state.read(cx).last_scrolled_hl != Some(ix)
    {
        let vh = f32::from(scroll.bounds().size.height);
        if vh > 0. {
            ensure_item_visible(&scroll, &entries, ix, vh);
            state.update(cx, |s, _| s.last_scrolled_hl = Some(ix));
        }
    }
    let (rows, plate) = select_rows(
        parent,
        entries,
        selected,
        highlighted,
        state,
        on_change,
        window,
        cx,
    );
    let host = use_layout_probe(&child_id(parent, "highlight"), window, cx);
    let list_h = height - 2. * stroke::HAIRLINE;
    ScrollArea::new(list_id)
        .max_height(px(list_h))
        .child(
            div()
                .min_w(px(min_width - 2. * stroke::HAIRLINE))
                .p(px(spacing::XX_SMALL))
                .font_family(theme.font_family.clone())
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(probe(&host, None))
                        .child(plate)
                        .children(rows),
                ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<SelectEntry> {
        vec![
            SelectEntry::Label("North America".into()),
            SelectEntry::Item(SelectOption::new("Eastern")),
            SelectEntry::Item(SelectOption::new("Central")),
            SelectEntry::Separator,
            SelectEntry::Label("Asia".into()),
            SelectEntry::Item(SelectOption::new("Tokyo")),
            SelectEntry::Item(SelectOption::new("Seoul").disabled(true)),
            SelectEntry::Item(SelectOption::new("Taipei")),
        ]
    }

    #[test]
    fn item_indices_skip_labels_and_separators() {
        let e = sample();
        assert_eq!(item_count(&e), 5);
        assert_eq!(item_at(&e, 0).unwrap().label.as_ref(), "Eastern");
        assert_eq!(item_at(&e, 2).unwrap().label.as_ref(), "Tokyo");
        assert!(item_at(&e, 3).unwrap().disabled);
        assert_eq!(first_enabled(&e), Some(0));
        assert_eq!(last_enabled(&e), Some(4));
    }

    #[test]
    fn arrows_skip_disabled_and_clamp() {
        let e = sample();
        // 0 Eastern, 1 Central, 2 Tokyo, 3 Seoul (disabled), 4 Taipei
        assert_eq!(step_enabled(&e, 2, 1), 4);
        assert_eq!(step_enabled(&e, 4, 1), 4);
        assert_eq!(step_enabled(&e, 4, -1), 2);
        assert_eq!(step_enabled(&e, 0, -1), 0);
    }

    #[test]
    fn typeahead_starts_with_and_wraps() {
        let e = sample();
        assert_eq!(typeahead(&e, None, "t"), Some(2)); // Tokyo
        assert_eq!(typeahead(&e, Some(2), "t"), Some(4)); // Taipei
        assert_eq!(typeahead(&e, Some(4), "t"), Some(2)); // wrap
        assert_eq!(typeahead(&e, None, "s"), None); // Seoul disabled
        assert_eq!(typeahead(&e, None, "ee"), None);
    }

    #[test]
    fn layout_heights_include_labels() {
        let e = sample();
        let expected = SHELL_INSET * 2.
            + LABEL_HEIGHT
            + ITEM_HEIGHT
            + ITEM_HEIGHT
            + SEPARATOR_HEIGHT
            + LABEL_HEIGHT
            + ITEM_HEIGHT
            + ITEM_HEIGHT
            + ITEM_HEIGHT
            + ITEM_GAP * 7.;
        assert!((list_full_height(&e) - expected).abs() < 0.01);
        let tokyo_top = item_offset_top(&e, 2);
        let expect_top = SHELL_INSET
            + LABEL_HEIGHT
            + ITEM_GAP
            + ITEM_HEIGHT
            + ITEM_GAP
            + ITEM_HEIGHT
            + ITEM_GAP
            + SEPARATOR_HEIGHT
            + ITEM_GAP
            + LABEL_HEIGHT
            + ITEM_GAP;
        assert!((tokyo_top - expect_top).abs() < 0.01);
    }

    fn layout_of(
        trigger: (f32, f32, f32, f32),
        viewport: (f32, f32),
        entries: &[SelectEntry],
        opts: CollisionOptions,
    ) -> SelectLayout {
        select_layout(
            gpui::Bounds::new(
                gpui::point(px(trigger.0), px(trigger.1)),
                gpui::size(px(trigger.2), px(trigger.3)),
            ),
            entries,
            gpui::size(px(viewport.0), px(viewport.1)),
            opts,
        )
    }

    fn overlaps_trigger(l: &SelectLayout, y: f32, h: f32) -> bool {
        let t_top = y;
        let t_bottom = y + h;
        let p_top = l.y;
        let p_bottom = l.y + l.height;
        p_top < t_bottom && p_bottom > t_top
    }

    #[test]
    fn layout_opens_below_the_trigger_and_never_covers_it() {
        let e = sample();
        let opts = CollisionOptions::default();
        let l = layout_of((40., 40., 200., 32.), (800., 600.), &e, opts);
        assert_eq!(l.side, FloatingSide::Bottom);
        assert_eq!(l.x, 40.);
        assert_eq!(l.width, 200.);
        assert_eq!(l.y, 40. + 32. + opts.side_offset);
        assert!(!overlaps_trigger(&l, 40., 32.));
    }

    #[test]
    fn layout_flips_above_when_there_is_no_room_below() {
        let e = sample();
        let opts = CollisionOptions::default();
        // Trigger near the bottom of an 600px viewport; the full list cannot fit below.
        let l = layout_of((40., 500., 200., 32.), (800., 600.), &e, opts);
        assert_eq!(l.side, FloatingSide::Top);
        assert!(l.y + l.height <= 500. - opts.side_offset + 0.01);
        assert!(!overlaps_trigger(&l, 500., 32.));
        assert!(l.y >= opts.collision_padding);
    }

    #[test]
    fn layout_height_shrinks_to_the_available_space() {
        let e = sample();
        let opts = CollisionOptions::default();
        // Short viewport so the full grouped list cannot fit above or below.
        let trigger = PlacementRect::new(40., 140., 200., 32.);
        let viewport = PlacementViewport::new(800., 300.);
        let (above, below) = crate::vertical_room(trigger, viewport, opts);
        assert!(
            list_full_height(&e) > above && list_full_height(&e) > below,
            "full={} above={} below={}",
            list_full_height(&e),
            above,
            below
        );
        let l = layout_of((40., 140., 200., 32.), (800., 300.), &e, opts);
        let expected_side = if below >= above {
            FloatingSide::Bottom
        } else {
            FloatingSide::Top
        };
        let expected_h = if expected_side == FloatingSide::Bottom {
            below
        } else {
            above
        };
        assert_eq!(l.side, expected_side);
        assert_eq!(l.available_height, expected_h);
        assert_eq!(l.height, expected_h);
        assert!(l.height < list_full_height(&e));
        assert!(!overlaps_trigger(&l, 140., 32.));
    }
}
