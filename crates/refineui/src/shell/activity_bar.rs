//! Activity Bar / Navigation Rail — the vertical strip of top-level
//! destinations at the window edge (brief §5 Application). Rebuilt from the
//! old Side Dock with RefineUI's own look and free customization.
//!
//! **One component, no variants.** The look is the former Rail preset,
//! rebuilt on RefineUI tokens: a `surfaceSunken` column (density width) with
//! a hairline `borderSubtle` inner edge; tiles are circles at rest that
//! morph to `roundedXLarge` squares on hover / selection; the selected tile
//! sits on `backgroundBrandSubtle` with a `foregroundBrand` icon and a
//! `backgroundBrand` edge bar (8 unread / ~40% hover / ~80% selected of the
//! tile); hover is `backgroundPrimaryHover`; badges are brand pills.
//!
//! Items can show an external [`ActivityItem::image`] (URL or asset) and a
//! count or dot [`ActivityItem::badge`]. [`ActivityItem::folder`] groups
//! items in a Toolbar-style tube: a closed folder shows a 2×2 stacked
//! preview of its members (and a count badge when there are more than four
//! and the item has no badge of its own); an open folder shows the
//! FolderOpen icon, and the members sit inside the same rounded plate,
//! which grows and shrinks with accordion height motion (`ACCORDION_PANEL`
//! / `ACCORDION_CONTENT`).
//!
//! Slots: `header` / `footer` elements, pinned `footer_item`s, a custom
//! face per item ([`ActivityBar::render_item`]) and drag-to-reorder
//! ([`ActivityBar::on_reorder`]).
//!
//! **Keyboard** — when the bar has focus: ↑ / ↓ move a focus ring (open
//! folder children are in the order), Home / End jump, Enter / Space
//! activate and also toggle a focused folder; ⌥↑ / ⌥↓ move the focused
//! main item when reordering is on (folder children are skipped).
//! **Pointer** — click activates; items never act on hover alone; tooltips
//! (with the item's action shortcut) appear when labels are hidden.

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, Hsla, ImageSource, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Window, anchored, div, img, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::components::floating::{BeakAlign, BeakSide};
use crate::interaction::dnd::{DropEdge, drop_edge, insertion_index};
use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, presence, probe, use_interaction,
    use_layout_probe,
};
use crate::{
    ActiveTheme, DragPreview, DropIndicator, Icon, IconName, IconSize, ScrollArea, StyledExt,
};

/// Item corner shape.
#[allow(dead_code)] // internal render knobs; the public bar has one look
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ItemShape {
    /// Rounded square with this radius.
    Rounded(f32),
    /// Sharp square.
    Square,
    /// Circle.
    Circle,
    /// Circle at rest, `roundedXXLarge` square when hovered / selected
    /// (the Rail preset).
    Morph,
}

/// Selection indicator.
#[allow(dead_code)] // internal render knobs; the public bar has one look
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndicatorStyle {
    /// No indicator (pair with a selected colour).
    None,
    /// Bar on the docked edge (grows on hover / select; a short mark for
    /// unread items).
    Bar,
    /// Dot under the icon (selected: ink, unread: muted).
    Dot,
    /// Filled plate behind the selected item.
    Plate,
}

/// How item names show.
#[allow(dead_code)] // internal render knobs; the public bar has one look
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelMode {
    /// Tooltip beside the item (icon-only bar).
    Tooltip,
    /// Caption under each icon (navigation rail).
    Below,
    /// Label beside the icon in a wider bar (expanded).
    Beside,
}

/// Which window edge the bar is docked to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarSide {
    /// Leading (left) edge.
    #[default]
    Leading,
    /// Trailing (right) edge.
    Trailing,
}

/// The complete look of an [`ActivityBar`]; `None` colours resolve from the
/// theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ActivityBarStyle {
    /// Bar width (`None` = density `activity_bar_width`).
    pub width: Option<f32>,
    /// Width in [`LabelMode::Beside`].
    pub expanded_width: f32,
    /// Item size (`None` = density `activity_item`).
    pub item_size: Option<f32>,
    /// Gap between items.
    pub gap: f32,
    /// Item shape.
    pub shape: ItemShape,
    /// Selection indicator.
    pub indicator: IndicatorStyle,
    /// Fill selected items with `selected` even when the indicator is not
    /// `Plate`.
    pub fill_selected: bool,
    /// Label mode.
    pub labels: LabelMode,
    /// Icon size.
    pub icon: IconSize,
    /// Hairline on the inner edge.
    pub border: bool,
    /// Bar background.
    pub background: Option<Hsla>,
    /// Item fill at rest.
    pub item: Option<Hsla>,
    /// Item ink at rest.
    pub ink: Option<Hsla>,
    /// Hover fill.
    pub hover: Option<Hsla>,
    /// Selected fill.
    pub selected: Option<Hsla>,
    /// Selected ink.
    pub selected_ink: Option<Hsla>,
    /// Indicator colour.
    pub indicator_color: Option<Hsla>,
    /// Badge fill.
    pub badge: Option<Hsla>,
    /// Badge ink.
    pub badge_ink: Option<Hsla>,
}

impl ActivityBarStyle {
    /// The one Activity Bar look (an improved Rail). See the module docs.
    pub(crate) const fn unified() -> Self {
        Self {
            width: None,
            expanded_width: 220.,
            item_size: None,
            gap: spacing::XX_SMALL,
            shape: ItemShape::Morph,
            indicator: IndicatorStyle::Bar,
            fill_selected: true,
            labels: LabelMode::Tooltip,
            icon: IconSize::Small,
            border: true,
            background: None,
            item: None,
            ink: None,
            hover: None,
            selected: None,
            selected_ink: None,
            indicator_color: None,
            badge: None,
            badge_ink: None,
        }
    }
}

#[derive(Clone)]
enum Face {
    Icon(IconName),
    Image(ImageSource),
    /// Members and whether the folder starts open.
    Folder(Vec<ActivityItem>, bool),
}

/// Badge on an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemBadge {
    /// Count (`99+` above 99).
    Count(u32),
    /// Small dot.
    Dot,
}

/// One destination.
#[derive(Clone)]
pub struct ActivityItem {
    id: SharedString,
    label: SharedString,
    face: Face,
    badge: Option<ItemBadge>,
    unread: bool,
    disabled: bool,
    action: Option<SharedString>,
}

impl ActivityItem {
    fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, face: Face) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            face,
            badge: None,
            unread: false,
            disabled: false,
            action: None,
        }
    }

    /// Icon item.
    pub fn icon(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: IconName,
    ) -> Self {
        Self::new(id, label, Face::Icon(icon))
    }

    /// Picture item from an external image source (URL, asset, or bytes).
    pub fn image(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        src: impl Into<ImageSource>,
    ) -> Self {
        Self::new(id, label, Face::Image(src.into()))
    }

    /// Folder of items. Clicking the tile expands or collapses the members
    /// inside a shared Toolbar-style tube. Closed, the tile shows a 2×2
    /// stacked preview of the members; open, it shows the FolderOpen icon.
    pub fn folder(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        members: impl IntoIterator<Item = ActivityItem>,
    ) -> Self {
        Self::new(
            id,
            label,
            Face::Folder(members.into_iter().collect(), false),
        )
    }

    /// Open this folder on first show (folders only).
    pub fn default_open(mut self, open: bool) -> Self {
        if let Face::Folder(_, default_open) = &mut self.face {
            *default_open = open;
        }
        self
    }

    /// Count badge (`0` hides it).
    pub fn badge(mut self, count: u32) -> Self {
        self.badge = (count > 0).then_some(ItemBadge::Count(count));
        self
    }

    /// Dot badge.
    pub fn dot(mut self) -> Self {
        self.badge = Some(ItemBadge::Dot);
        self
    }

    /// Unread mark (indicator-dependent: short bar / muted dot).
    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    /// Disabled (skipped by the keyboard, no activation).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The GPUI action this item mirrors — its shortcut shows in the
    /// tooltip.
    pub fn action(mut self, action: impl Into<SharedString>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Id.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Label.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Icon, for icon items.
    pub fn icon_name(&self) -> Option<IconName> {
        match self.face {
            Face::Icon(i) => Some(i),
            _ => None,
        }
    }
}

/// State passed to a custom item renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActivityItemState {
    /// Selected.
    pub selected: bool,
    /// Pointer over it.
    pub hovered: bool,
    /// Keyboard cursor on it (focus-visible).
    pub focused: bool,
    /// Item size (px).
    pub size: f32,
    /// Ink colour the bar would use.
    pub ink: Hsla,
}

type SelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type ReorderHandler = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;
type ItemRenderer =
    Rc<dyn Fn(&ActivityItem, ActivityItemState, &mut Window, &mut App) -> AnyElement>;

enum Entry {
    Item(ActivityItem),
    Divider,
}

/// The bar. See the module docs.
#[derive(IntoElement)]
pub struct ActivityBar {
    id: ElementId,
    side: BarSide,
    style: ActivityBarStyle,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
    entries: Vec<Entry>,
    footer_items: Vec<ActivityItem>,
    selected: Option<SharedString>,
    on_select: Option<SelectHandler>,
    on_reorder: Option<ReorderHandler>,
    render_item: Option<ItemRenderer>,
    fill: bool,
}

impl ActivityBar {
    /// New bar (collapsed).
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            side: BarSide::Leading,
            style: ActivityBarStyle::unified(),
            header: None,
            footer: None,
            entries: Vec::new(),
            footer_items: Vec::new(),
            selected: None,
            on_select: None,
            on_reorder: None,
            render_item: None,
            fill: true,
        }
    }

    /// Docked edge.
    pub fn side(mut self, side: BarSide) -> Self {
        self.side = side;
        self
    }

    /// Element above the items (logo, workspace switcher).
    pub fn header(mut self, e: impl IntoElement) -> Self {
        self.header = Some(e.into_any_element());
        self
    }

    /// Element below the pinned footer items (account, sync state).
    pub fn footer(mut self, e: impl IntoElement) -> Self {
        self.footer = Some(e.into_any_element());
        self
    }

    /// Add an item.
    pub fn item(mut self, item: ActivityItem) -> Self {
        self.entries.push(Entry::Item(item));
        self
    }

    /// Add items.
    pub fn items(mut self, items: impl IntoIterator<Item = ActivityItem>) -> Self {
        self.entries.extend(items.into_iter().map(Entry::Item));
        self
    }

    /// Divider between groups.
    pub fn divider(mut self) -> Self {
        self.entries.push(Entry::Divider);
        self
    }

    /// Item pinned to the bottom (Settings, Accounts).
    pub fn footer_item(mut self, item: ActivityItem) -> Self {
        self.footer_items.push(item);
        self
    }

    /// Selected item id.
    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    /// Activation handler.
    pub fn on_select(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// Enable drag-to-reorder of the main items: `f(from, to)` with
    /// indices into the main item list (dividers excluded; `to` is the
    /// final index after removal, as [`crate::interaction::dnd::reorder`]).
    pub fn on_reorder(mut self, f: impl Fn(usize, usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_reorder = Some(Rc::new(f));
        self
    }

    /// Draw every item face yourself (the bar keeps hit-testing, focus,
    /// indicator and tooltip).
    pub fn render_item(
        mut self,
        f: impl Fn(&ActivityItem, ActivityItemState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.render_item = Some(Rc::new(f));
        self
    }

    /// Fill the parent's height (default) or hug the content.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }
}

fn badge_text(n: u32) -> SharedString {
    if n > 99 {
        "99+".into()
    } else {
        n.to_string().into()
    }
}

/// Edge-bar heights `(unread, hover, selected)` for an item of `size`.
pub fn bar_heights(size: f32) -> (f32, f32, f32) {
    (8., (size * 0.42).round(), (size * 0.83).round())
}

#[derive(Clone)]
struct Resolved {
    bar: Hsla,
    item: Hsla,
    ink: Hsla,
    hover: Hsla,
    selected: Hsla,
    selected_ink: Hsla,
    indicator: Hsla,
    badge: Hsla,
    badge_ink: Hsla,
    size: f32,
    width: f32,
}

fn resolve(s: &ActivityBarStyle, cx: &App) -> Resolved {
    let t = cx.theme();
    let c = t.colors;
    let d = t.density.desktop();
    let size = s.item_size.unwrap_or(d.activity_item);
    let width = match s.labels {
        LabelMode::Beside => s.expanded_width,
        _ => s.width.unwrap_or(d.activity_bar_width),
    };
    Resolved {
        bar: s.background.unwrap_or(c.surface_sunken),
        item: s.item.unwrap_or(gpui::transparent_black()),
        ink: s.ink.unwrap_or(c.foreground_secondary),
        hover: s.hover.unwrap_or(c.background_primary_hover),
        selected: s.selected.unwrap_or(c.background_brand_subtle),
        selected_ink: s.selected_ink.unwrap_or(c.foreground_brand),
        indicator: s.indicator_color.unwrap_or(c.background_brand),
        badge: s.badge.unwrap_or(c.background_brand),
        badge_ink: s.badge_ink.unwrap_or(c.foreground_on_brand),
        size,
        width,
    }
}

/// Payload of an activity-item drag.
#[derive(Clone)]
pub struct ActivityDrag {
    bar: ElementId,
    from: usize,
    label: SharedString,
    icon: Option<IconName>,
}

impl Render for ActivityDrag {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let mut p = DragPreview::new(self.label.clone());
        if let Some(i) = self.icon {
            p = p.icon(i);
        }
        p.element(cx)
    }
}

struct Ctx<'a> {
    bar: &'a ElementId,
    side: BarSide,
    style: &'a ActivityBarStyle,
    r: Resolved,
    selected: Option<&'a SharedString>,
    on_select: Option<SelectHandler>,
    on_reorder: Option<ReorderHandler>,
    render_item: Option<ItemRenderer>,
    cursor: Option<SharedString>,
    focused: bool,
    cursor_state: gpui::Entity<Option<SharedString>>,
    drop: gpui::Entity<Option<(usize, DropEdge)>>,
    folder_open: HashSet<SharedString>,
    on_folder_toggle: Rc<dyn Fn(&SharedString, &mut App)>,
}

fn face(
    item: &ActivityItem,
    size: f32,
    ink: Hsla,
    radius_px: f32,
    icon: IconSize,
    folder_open_t: f32,
    _cx: &App,
) -> AnyElement {
    match &item.face {
        Face::Icon(i) => Icon::new(*i).size(icon).color(ink).into_any_element(),
        Face::Image(src) => img(src.clone())
            .size(px(size))
            .rounded(px(radius_px))
            .into_any_element(),
        Face::Folder(members, _) => {
            // Open: FolderOpen glyph. Closed: 2×2 stacked preview of up to
            // four members (empty slots keep the grid stable). An empty
            // folder falls back to the closed Folder glyph.
            if folder_open_t > 0.5 || members.is_empty() {
                let glyph = if folder_open_t > 0.5 {
                    IconName::FolderOpen
                } else {
                    IconName::Folder
                };
                Icon::new(glyph).size(icon).color(ink).into_any_element()
            } else {
                folder_preview(members, size, ink)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Main(usize),
    Footer,
    /// Member of an open folder; in ↑/↓ order, not in ⌥↑/⌥↓ reorder.
    Child,
}

/// Badge shown on a tile: the item's own badge, or a folder member count
/// when there are more than four members and no user badge.
fn displayed_badge(item: &ActivityItem) -> Option<ItemBadge> {
    if item.badge.is_some() {
        return item.badge;
    }
    match &item.face {
        Face::Folder(members, _) if members.len() > 4 => {
            Some(ItemBadge::Count(members.len() as u32))
        }
        _ => None,
    }
}

fn folder_preview_cell(member: &ActivityItem, cell: f32, ink: Hsla) -> AnyElement {
    let inner = cell.max(1.);
    let icon_sz = (inner * 0.78).max(8.);
    let slot = div()
        .size(px(inner))
        .rounded(px((inner * 0.28).min(radius::SMALL)))
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center();
    match &member.face {
        Face::Icon(name) => slot
            .child(
                Icon::new(*name)
                    .size(IconSize::Custom(px(icon_sz)))
                    .color(ink),
            )
            .into_any_element(),
        Face::Image(src) => slot
            .child(img(src.clone()).size(px(inner)))
            .into_any_element(),
        Face::Folder(_, _) => slot
            .child(
                Icon::new(IconName::Folder)
                    .size(IconSize::Custom(px(icon_sz)))
                    .color(ink),
            )
            .into_any_element(),
    }
}

fn folder_preview(members: &[ActivityItem], size: f32, ink: Hsla) -> AnyElement {
    let pad = spacing::XXX_SMALL;
    let gap = 2.0_f32;
    let inner = (size - pad * 2. - gap).max(1.);
    let cell = inner / 2.;
    let slot = |i: usize| {
        members
            .get(i)
            .map(|m| folder_preview_cell(m, cell, ink))
            .unwrap_or_else(|| div().size(px(cell.max(1.))).into_any_element())
    };
    div()
        .flex()
        .flex_col()
        .gap(px(gap))
        .p(px(pad))
        .size(px(size))
        .child(div().flex().gap(px(gap)).child(slot(0)).child(slot(1)))
        .child(div().flex().gap(px(gap)).child(slot(2)).child(slot(3)))
        .into_any_element()
}

/// ToolbarGroup-like tube around a folder tile and its accordion panel.
fn folder_tube(
    item_size: f32,
    labels: LabelMode,
    border: Hsla,
    fill: Hsla,
    tile: AnyElement,
    panel: Option<AnyElement>,
) -> AnyElement {
    let pad = spacing::XXX_SMALL;
    div()
        .flex()
        .flex_col()
        .items_center()
        .p(px(pad))
        .rounded(px(radius::XX_LARGE + pad))
        .border(px(stroke::THIN))
        .border_color(border)
        .bg(fill)
        .when(labels == LabelMode::Beside, |d| d.w_full())
        .when(labels != LabelMode::Beside, |d| {
            d.w(px(item_size + pad * 2.))
        })
        .child(tile)
        .children(panel)
        .into_any_element()
}

#[derive(Clone, Debug, PartialEq)]
struct NavEntry {
    id: SharedString,
    disabled: bool,
    /// Index in the main item row; `None` for folder children and footer items.
    main_ix: Option<usize>,
    is_folder: bool,
}

fn nav_order(
    entries: &[Entry],
    footer_items: &[ActivityItem],
    open: &HashSet<SharedString>,
) -> Vec<NavEntry> {
    let mut order = Vec::new();
    let mut main_ix = 0usize;
    for e in entries {
        let Entry::Item(i) = e else { continue };
        let is_folder = matches!(i.face, Face::Folder(_, _));
        order.push(NavEntry {
            id: i.id.clone(),
            disabled: i.disabled,
            main_ix: Some(main_ix),
            is_folder,
        });
        if is_folder
            && open.contains(&i.id)
            && let Face::Folder(members, _) = &i.face
        {
            for m in members {
                order.push(NavEntry {
                    id: m.id.clone(),
                    disabled: m.disabled,
                    main_ix: None,
                    is_folder: matches!(m.face, Face::Folder(_, _)),
                });
            }
        }
        main_ix += 1;
    }
    for i in footer_items {
        order.push(NavEntry {
            id: i.id.clone(),
            disabled: i.disabled,
            main_ix: None,
            is_folder: matches!(i.face, Face::Folder(_, _)),
        });
    }
    order
}

fn tile(
    _ix: usize,
    item: ActivityItem,
    role: Role,
    t: &Ctx,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let r = &t.r;
    let s = t.style;
    let iid = ElementId::NamedChild(Box::new(t.bar.clone()), format!("item-{}", item.id).into());
    let (istate, i) = use_interaction(&iid, window, cx);
    let active = t.selected == Some(&item.id);
    let hovered = i.hovered && !item.disabled;
    let size = r.size;
    let beside = s.labels == LabelMode::Beside;
    let below = s.labels == LabelMode::Below;
    let fill_sel = active && (s.indicator == IndicatorStyle::Plate || s.fill_selected);
    let (bg, ink) = match (&item.face, fill_sel, hovered) {
        (Face::Image(_), _, _) | (Face::Folder(_, _), _, _) => (
            if hovered && !matches!(item.face, Face::Image(_)) {
                c.background_primary_hover
            } else {
                r.item
            },
            r.ink,
        ),
        (_, true, _) => (r.selected, r.selected_ink),
        (_, false, true) => (
            r.hover,
            if s.shape == ItemShape::Morph {
                r.selected_ink
            } else {
                c.foreground_primary
            },
        ),
        _ => (r.item, if active { r.selected_ink } else { r.ink }),
    };
    let ink = if item.disabled {
        c.foreground_disabled
    } else {
        ink
    };
    let bg = animate(child_id(&iid, "bg"), bg, Motion::FAST, window, cx);
    let ink = animate(child_id(&iid, "ink"), ink, Motion::FAST, window, cx);
    let square = matches!(item.face, Face::Folder(_, _)) || active || hovered;
    let radius_target = match s.shape {
        ItemShape::Rounded(rr) => rr,
        ItemShape::Square => 0.,
        ItemShape::Circle => size / 2.,
        ItemShape::Morph => {
            if square {
                radius::XX_LARGE
            } else {
                size / 2.
            }
        }
    };
    let rad = animate(
        child_id(&iid, "radius"),
        radius_target,
        Motion::FAST,
        window,
        cx,
    );
    let (unread_h, hover_h, sel_h) = bar_heights(size);
    let bar_target = if s.indicator != IndicatorStyle::Bar {
        0.
    } else if active {
        sel_h
    } else if hovered {
        hover_h
    } else if item.unread {
        unread_h
    } else {
        0.
    };
    let bar_h = animate(
        child_id(&iid, "bar"),
        bar_target,
        Motion::INDICATOR,
        window,
        cx,
    );
    let bounds_probe = use_layout_probe(&iid, window, cx);
    let bounds = bounds_probe.read(cx).container;
    let show_tip = s.labels == LabelMode::Tooltip && hovered;
    let tip = presence(
        child_id(&iid, "tip"),
        show_tip,
        Motion::new(
            crate::motion::duration::FAST,
            crate::motion::easing::EMPHASIZED,
        ),
        Motion::FAST,
        window,
        cx,
    );
    let focus_ring = t.focused && t.cursor.as_ref() == Some(&item.id) && focus_visible(cx);
    let state = ActivityItemState {
        selected: active,
        hovered,
        focused: focus_ring,
        size,
        ink,
    };
    let is_folder = matches!(item.face, Face::Folder(_, _));
    let folder_is_open = is_folder && t.folder_open.contains(&item.id);
    let folder_open_t = if is_folder {
        animate(
            child_id(&iid, "folder-open"),
            if folder_is_open { 1.0f32 } else { 0.0 },
            Motion::CHEVRON,
            window,
            cx,
        )
    } else {
        0.
    };
    let face_el = match &t.render_item {
        Some(f) => f(&item, state, window, cx),
        None => face(&item, size, ink, rad, s.icon, folder_open_t, cx),
    };

    let bar = div()
        .absolute()
        .top(px((size - bar_h) / 2.))
        .w(px(if s.shape == ItemShape::Morph { 4. } else { 3. }))
        .h(px(bar_h.max(0.)))
        .bg(r.indicator)
        .when(bar_h < 0.5, |d| d.invisible())
        .map(|d| match t.side {
            BarSide::Leading => d.left_0().rounded_r(px(radius::SMALL)),
            BarSide::Trailing => d.right_0().rounded_l(px(radius::SMALL)),
        });

    let dot = (s.indicator == IndicatorStyle::Dot && (active || item.unread)).then(|| {
        div()
            .absolute()
            .bottom(px(2.))
            .left(px(size / 2. - 2.))
            .size(px(4.))
            .rounded_full()
            .bg(if active {
                r.indicator
            } else {
                c.foreground_tertiary
            })
    });

    // Over the icon's corner, or inline at the row end when labels sit
    // beside the icons.
    let badge = displayed_badge(&item).map(|b| {
        let base = div()
            .rounded_full()
            .bg(r.badge)
            .when(!beside, |d| {
                d.absolute().border(px(stroke::THICK)).border_color(r.bar)
            })
            .when(beside, |d| d.flex_none());
        match b {
            ItemBadge::Dot if beside => base.size(px(8.)).mr(px(spacing::X_SMALL)),
            ItemBadge::Dot => base.right(px(-1.)).top(px(-1.)).size(px(10.)),
            ItemBadge::Count(n) => base
                .when(!beside, |d| {
                    d.right(px(-4.))
                        .bottom(px(-4.))
                        .min_w(px(16. + 2. * stroke::THICK))
                        .h(px(16. + 2. * stroke::THICK))
                        .px(px(3. + stroke::THICK))
                })
                .when(beside, |d| {
                    d.min_w(px(18.))
                        .h(px(18.))
                        .px(px(5.))
                        .mr(px(spacing::XX_SMALL))
                })
                .flex()
                .items_center()
                .justify_center()
                .text_color(r.badge_ink)
                .text_size(px(10.))
                .line_height(px(14.))
                .font_weight(gpui::FontWeight::BOLD)
                .child(badge_text(n)),
        }
    });
    let (corner_badge, row_badge) = if beside { (None, badge) } else { (badge, None) };

    let tooltip = (tip.mounted).then_some(bounds).flatten().map(|b| {
        let gap = px(spacing::MEDIUM);
        let (x, beak) = match t.side {
            BarSide::Leading => (b.right() + gap, BeakSide::Left),
            BarSide::Trailing => (b.left() - gap, BeakSide::Right),
        };
        let text: SharedString = match item
            .action
            .as_ref()
            .and_then(|a| crate::interaction::keymap::shortcut_for(a, cx))
        {
            Some(k) => format!("{}  {}", item.label, k).into(),
            None => item.label.clone(),
        };
        let bubble = crate::components::tooltip::tooltip_with_beak_aligned(
            text,
            beak,
            BeakAlign::Center,
            cx,
        );
        let slide = px(spacing::X_SMALL * (1. - tip.progress));
        crate::layer(
            anchored()
                .position(gpui::point(x, b.top()))
                .map(|a| match t.side {
                    BarSide::Leading => a,
                    BarSide::Trailing => a.anchor(gpui::Corner::TopRight),
                })
                .child(
                    div()
                        .h(px(size))
                        .flex()
                        .items_center()
                        .opacity(tip.progress)
                        .map(|d| match t.side {
                            BarSide::Leading => d.ml(slide),
                            BarSide::Trailing => d.mr(slide),
                        })
                        .child(bubble),
                ),
        )
        .priority(crate::foundation::LayerOrder::Tooltip.priority())
        .into_any_element()
    });

    let on_select = t.on_select.clone();
    let on_folder_toggle = t.on_folder_toggle.clone();
    let cursor = t.cursor_state.clone();
    let id = item.id.clone();
    let disabled = item.disabled;
    let label_el = match s.labels {
        LabelMode::Beside => Some(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_style_token(role::BODY_SM)
                .text_color(ink)
                .when(active, |d| d.font_weight(gpui::FontWeight::MEDIUM))
                .child(item.label.clone())
                .into_any_element(),
        ),
        _ => None,
    };
    let caption = below.then(|| {
        div()
            .w_full()
            .flex()
            .justify_center()
            .text_style_token(role::CAPTION_SM)
            .text_color(if active {
                c.foreground_primary
            } else {
                c.foreground_secondary
            })
            .when(active, |d| d.font_weight(gpui::FontWeight::MEDIUM))
            .child(div().truncate().child(item.label.clone()))
    });

    let face_box = div()
        .relative()
        .flex_none()
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .when(!beside, |d| d.rounded(px(rad)).bg(bg))
        .child(face_el)
        .children(dot)
        .children(corner_badge);

    let drop_state = t.drop.clone();
    let drop_now = *t.drop.read(cx);
    let drop_line = match (role, drop_now) {
        (Role::Main(m), Some((target, edge))) if target == m => Some(edge),
        _ => None,
    };
    let reorder = t.on_reorder.clone();
    let bar_id = t.bar.clone();
    let tile_el = div()
        .id(iid)
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .when(beside, |d| {
            d.w_full()
                .h(px(size))
                .gap(px(spacing::X_SMALL))
                .px(px(spacing::XX_SMALL))
                .rounded(px(match s.shape {
                    ItemShape::Circle | ItemShape::Morph => size / 2.,
                    ItemShape::Square => 0.,
                    ItemShape::Rounded(rr) => rr,
                }))
                .bg(bg)
        })
        .when(below, |d| {
            d.flex_col()
                .gap(px(spacing::XXX_SMALL))
                .w(px(r.width - 2. * spacing::XX_SMALL))
                .py(px(spacing::XX_SMALL))
                .rounded(px(radius::LARGE))
        })
        .when(!beside && !below, |d| d.justify_center())
        .when(focus_ring, |d| d.shadow(theme.focus_ring()))
        .when(!disabled, |d| d.cursor_action())
        .when(disabled, |d| {
            d.cursor_role(crate::foundation::CursorRole::NotAllowed)
        })
        .track_interaction(&istate)
        .on_click(move |_, w, cx| {
            if disabled {
                return;
            }
            cursor.update(cx, |c, _| *c = Some(id.clone()));
            if is_folder {
                on_folder_toggle(&id, cx);
            }
            if let Some(h) = &on_select {
                h(&id, w, cx);
            }
        })
        .child(probe(&bounds_probe, None))
        .child(face_box)
        .children(label_el)
        .children(row_badge)
        .children(caption);

    let tile_el = match (role, reorder) {
        (Role::Main(m), Some(reorder)) => {
            let payload = ActivityDrag {
                bar: bar_id.clone(),
                from: m,
                label: item.label.clone(),
                icon: item.icon_name(),
            };
            let over = drop_state.clone();
            let clear = drop_state.clone();
            let bar_for_move = bar_id.clone();
            let bar_for_drop = bar_id.clone();
            tile_el
                .on_drag(payload, |p, _, _, cx| cx.new(|_| p.clone()))
                .on_drag_move::<ActivityDrag>(move |e, _, cx| {
                    if e.drag(cx).bar != bar_for_move {
                        return;
                    }
                    let b = e.bounds;
                    if !b.contains(&e.event.position) {
                        return;
                    }
                    let edge = drop_edge(
                        f32::from(e.event.position.y - b.top()),
                        f32::from(b.size.height),
                        false,
                    );
                    over.update(cx, |d, cx| {
                        if *d != Some((m, edge)) {
                            *d = Some((m, edge));
                            cx.notify();
                        }
                    });
                })
                .on_drop(move |p: &ActivityDrag, w, cx| {
                    if p.bar != bar_for_drop {
                        return;
                    }
                    let edge = clear.read(cx).map(|(_, e)| e).unwrap_or(DropEdge::Before);
                    clear.update(cx, |d, cx| {
                        *d = None;
                        cx.notify();
                    });
                    let to = insertion_index(m, edge, usize::MAX);
                    let to = if to > p.from { to - 1 } else { to };
                    if to != p.from {
                        reorder(p.from, to, w, cx);
                    }
                })
        }
        _ => tile_el,
    };

    let ind_w = r.width;
    div()
        .relative()
        .w(px(ind_w))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .when(beside, |d| d.px(px(spacing::X_SMALL)))
        .when(drop_line == Some(DropEdge::Before), |d| {
            d.child(
                div()
                    .absolute()
                    .top(px(-4.))
                    .left_0()
                    .right_0()
                    .px(px(spacing::XX_SMALL))
                    .child(DropIndicator::between_rows()),
            )
        })
        .child(bar)
        .child(tile_el)
        .when(drop_line == Some(DropEdge::After), |d| {
            d.child(
                div()
                    .absolute()
                    .bottom(px(-4.))
                    .left_0()
                    .right_0()
                    .px(px(spacing::XX_SMALL))
                    .child(DropIndicator::between_rows()),
            )
        })
        .children(tooltip)
        .into_any_element()
}

fn divider(width: f32, cx: &App) -> AnyElement {
    let c = cx.theme().colors;
    div()
        .w(px(width))
        .flex_none()
        .flex()
        .justify_center()
        .py(px(spacing::XX_SMALL))
        .child(
            div()
                .w(px((width * 0.45).min(32.)))
                .h(px(stroke::THIN))
                .rounded_full()
                .bg(c.border_default),
        )
        .into_any_element()
}

impl RenderOnce for ActivityBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let r = resolve(&self.style, cx);
        let width = animate(
            child_id(&self.id, "width"),
            r.width,
            Motion::MEDIUM,
            window,
            cx,
        );
        let focus: FocusHandle = window
            .use_keyed_state(child_id(&self.id, "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let cursor_state =
            window.use_keyed_state(
                child_id(&self.id, "cursor"),
                cx,
                |_, _| None::<SharedString>,
            );
        let drop = window.use_keyed_state(child_id(&self.id, "drop"), cx, |_, _| {
            None::<(usize, DropEdge)>
        });
        let focused = focus.is_focused(window);
        // Clear a stale drop marker once no drag is active.
        if drop.read(cx).is_some() && !cx.has_active_drag() {
            drop.update(cx, |d, _| *d = None);
        }

        let defaults: HashSet<SharedString> = self
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Item(i) => match &i.face {
                    Face::Folder(_, true) => Some(i.id.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let folders =
            window.use_keyed_state(child_id(&self.id, "folders"), cx, move |_, _| defaults);
        let folder_entity = folders.clone();
        let on_folder_toggle: Rc<dyn Fn(&SharedString, &mut App)> = Rc::new(move |id, cx| {
            folder_entity.update(cx, |set, cx| {
                if !set.remove(id) {
                    set.insert(id.clone());
                }
                cx.notify();
            });
        });
        let folder_open = folders.read(cx).clone();
        let order = nav_order(&self.entries, &self.footer_items, &folder_open);
        let cursor = cursor_state
            .read(cx)
            .clone()
            .or_else(|| self.selected.clone());

        let t = Ctx {
            bar: &self.id,
            side: self.side,
            style: &self.style,
            r: Resolved { width, ..r.clone() },
            selected: self.selected.as_ref(),
            on_select: self.on_select.clone(),
            on_reorder: self.on_reorder.clone(),
            render_item: self.render_item.clone(),
            cursor,
            focused,
            cursor_state: cursor_state.clone(),
            drop: drop.clone(),
            folder_open,
            on_folder_toggle,
        };
        let mut ix = 0usize;
        let mut main_ix = 0usize;
        let mut list: Vec<AnyElement> = Vec::new();
        for e in self.entries {
            match e {
                Entry::Item(item) => {
                    let members = match &item.face {
                        Face::Folder(m, _) => Some((item.id.clone(), m.clone())),
                        _ => None,
                    };
                    let open = members
                        .as_ref()
                        .is_some_and(|(id, _)| t.folder_open.contains(id));
                    let tile_el = tile(ix, item, Role::Main(main_ix), &t, window, cx);
                    ix += 1;
                    main_ix += 1;
                    if let Some((fid, members)) = members {
                        let panel_id = ElementId::NamedChild(
                            Box::new(self.id.clone()),
                            format!("folder-{fid}").into(),
                        );
                        let panel = crate::components::disclosure::collapse_panel(
                            &panel_id,
                            open,
                            window,
                            cx,
                            |w, cx| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(t.style.gap))
                                    .children(
                                        members
                                            .into_iter()
                                            .map(|m| tile(0, m, Role::Child, &t, w, cx)),
                                    )
                                    .into_any_element()
                            },
                        );
                        list.push(folder_tube(
                            t.r.size,
                            t.style.labels,
                            c.border_default,
                            c.background_brand_subtle,
                            tile_el,
                            panel,
                        ));
                    } else {
                        list.push(tile_el);
                    }
                }
                Entry::Divider => list.push(divider(width, cx)),
            }
        }
        let mut footer_list: Vec<AnyElement> = Vec::new();
        for item in self.footer_items {
            footer_list.push(tile(ix, item, Role::Footer, &t, window, cx));
            ix += 1;
        }

        let keys = cursor_state.clone();
        let on_select = self.on_select.clone();
        let on_reorder = self.on_reorder.clone();
        let on_folder_toggle_kb = t.on_folder_toggle.clone();
        let n = order.len();
        let order_kb = order.clone();
        let selected_id = self.selected.clone();
        let edge_border = self.style.border;
        div()
            .id(self.id.clone())
            .key_context("ActivityBar")
            .track_focus(&focus)
            .tab_index(0)
            .on_key_down(move |e: &KeyDownEvent, w, cx| {
                if n == 0 {
                    return;
                }
                let cur_id = keys.read(cx).clone().or_else(|| selected_id.clone());
                let cur = cur_id
                    .as_ref()
                    .and_then(|id| order_kb.iter().position(|o| &o.id == id));
                let enabled = |i: usize| !order_kb[i].disabled;
                let k = e.keystroke.key.as_str();
                let alt = e.keystroke.modifiers.alt;
                if alt && (k == "up" || k == "down") {
                    if let (Some(c), Some(h)) = (cur, &on_reorder)
                        && let Some(from) = order_kb[c].main_ix
                    {
                        // Walk neighboring main-row items only; folder children
                        // and footer items are not part of the reorder list.
                        let to = if k == "up" {
                            order_kb[..c]
                                .iter()
                                .rev()
                                .find_map(|o| o.main_ix)
                                .unwrap_or(from)
                        } else {
                            order_kb[c + 1..]
                                .iter()
                                .find_map(|o| o.main_ix)
                                .unwrap_or(from)
                        };
                        if to != from {
                            cx.stop_propagation();
                            h(from, to, w, cx);
                            keys.update(cx, |kk, cx| {
                                *kk = Some(order_kb[c].id.clone());
                                cx.notify();
                            });
                        }
                    }
                    return;
                }
                let next = match k {
                    "down" => {
                        let start = cur.map(|c| c + 1).unwrap_or(0);
                        (start..n).find(|&i| enabled(i)).or(cur)
                    }
                    "up" => match cur {
                        None => (0..n).rev().find(|&i| enabled(i)),
                        Some(c) => (0..c).rev().find(|&i| enabled(i)).or(Some(c)),
                    },
                    "home" => (0..n).find(|&i| enabled(i)),
                    "end" => (0..n).rev().find(|&i| enabled(i)),
                    "enter" | "space" => {
                        if let Some(c) = cur
                            && enabled(c)
                        {
                            cx.stop_propagation();
                            if order_kb[c].is_folder {
                                on_folder_toggle_kb(&order_kb[c].id, cx);
                            }
                            if let Some(h) = &on_select {
                                h(&order_kb[c].id, w, cx);
                            }
                        }
                        return;
                    }
                    _ => return,
                };
                cx.stop_propagation();
                keys.update(cx, |kk, cx| {
                    *kk = next.map(|i| order_kb[i].id.clone());
                    cx.notify();
                });
            })
            .flex()
            .flex_col()
            .flex_none()
            .w(px(width))
            .when(self.fill, |d| d.h_full())
            .map(|d| {
                let (fill, layer) = crate::material::chrome(crate::SurfaceKind::Sidebar, r.bar, cx);
                d.relative().bg(fill).children(layer)
            })
            .when(edge_border, |d| match self.side {
                BarSide::Leading => d
                    .border_r(px(stroke::HAIRLINE))
                    .border_color(c.border_subtle),
                BarSide::Trailing => d
                    .border_l(px(stroke::HAIRLINE))
                    .border_color(c.border_subtle),
            })
            .font_family(theme.font_family.clone())
            .text_color(c.foreground_primary)
            .pt(px(spacing::X_SMALL))
            .children(self.header.map(|h| {
                div()
                    .flex_none()
                    .flex()
                    .justify_center()
                    .pb(px(spacing::X_SMALL))
                    .child(h)
            }))
            .child(
                div().flex_1().min_h_0().child(
                    ScrollArea::new(child_id(&self.id, "scroll"))
                        .edge_blur(false)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(self.style.gap))
                                .pt(px(spacing::XX_SMALL))
                                .pb(px(spacing::X_SMALL))
                                .children(list),
                        ),
                ),
            )
            .when(!footer_list.is_empty(), |d| {
                d.child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(self.style.gap))
                        .pb(px(spacing::X_SMALL))
                        .children(footer_list),
                )
            })
            .children(self.footer.map(|f| {
                div()
                    .flex_none()
                    .flex()
                    .justify_center()
                    .pb(px(spacing::X_SMALL))
                    .child(f)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::Entry;
    use super::*;

    #[test]
    fn badges_and_folder_default_open() {
        assert_eq!(badge_text(7), "7");
        assert_eq!(badge_text(120), "99+");
        let open = ActivityItem::folder(
            "games",
            "Games",
            [ActivityItem::icon("a", "A", IconName::Bug)],
        )
        .default_open(true);
        assert!(matches!(open.face, Face::Folder(_, true)));
        assert!(open.badge.is_none());
        assert!(displayed_badge(&open).is_none());

        let many: Vec<_> = (0..5)
            .map(|i| ActivityItem::icon(format!("m{i}"), format!("M{i}"), IconName::Bug))
            .collect();
        let stacked = ActivityItem::folder("apps", "Apps", many);
        assert_eq!(displayed_badge(&stacked), Some(ItemBadge::Count(5)));
        let labelled = stacked.clone().badge(2);
        assert_eq!(displayed_badge(&labelled), Some(ItemBadge::Count(2)));
    }

    #[test]
    fn nav_order_includes_open_folder_children_not_in_reorder() {
        let entries = vec![
            Entry::Item(ActivityItem::icon("a", "A", IconName::Document)),
            Entry::Item(
                ActivityItem::folder(
                    "folder",
                    "Folder",
                    [
                        ActivityItem::icon("c1", "C1", IconName::Search),
                        ActivityItem::icon("c2", "C2", IconName::Bug),
                    ],
                )
                .default_open(true),
            ),
            Entry::Item(ActivityItem::icon("b", "B", IconName::Cube)),
        ];
        let footer = vec![ActivityItem::icon(
            "settings",
            "Settings",
            IconName::Settings,
        )];
        let closed = nav_order(&entries, &footer, &HashSet::new());
        assert_eq!(
            closed.iter().map(|e| e.id.as_ref()).collect::<Vec<_>>(),
            ["a", "folder", "b", "settings"]
        );
        assert_eq!(
            closed.iter().map(|e| e.main_ix).collect::<Vec<_>>(),
            [Some(0), Some(1), Some(2), None]
        );

        let mut open = HashSet::new();
        open.insert("folder".into());
        let opened = nav_order(&entries, &footer, &open);
        assert_eq!(
            opened.iter().map(|e| e.id.as_ref()).collect::<Vec<_>>(),
            ["a", "folder", "c1", "c2", "b", "settings"]
        );
        assert!(opened[0].main_ix == Some(0) && !opened[0].is_folder);
        assert!(opened[1].is_folder && opened[1].main_ix == Some(1));
        assert!(opened[2].main_ix.is_none() && !opened[2].is_folder);
        assert!(opened[3].main_ix.is_none());
        assert_eq!(opened[4].main_ix, Some(2));
        assert!(opened[5].main_ix.is_none());
    }

    #[test]
    fn unified_style_is_an_improved_rail() {
        let r = ActivityBarStyle::unified();
        assert_eq!(r.shape, ItemShape::Morph);
        assert_eq!(r.indicator, IndicatorStyle::Bar);
        assert_eq!(r.labels, LabelMode::Tooltip);
        assert_eq!(r.expanded_width, 220.);
        assert_eq!(bar_heights(48.), (8., 20., 40.));
    }
}
