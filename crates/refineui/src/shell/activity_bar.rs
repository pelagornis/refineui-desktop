//! Activity Bar — the strip of top-level destinations at a window edge
//! (brief §5 Application).
//!
//! **Look.** Desktop-tool style (VS Code, Zed, Xcode): every item is the
//! same `activity_item` (density) hit target with a `roundedLarge` plate and
//! a 20px icon. At rest the plate is clear and the icon is
//! `foregroundSecondary`; hover is `backgroundPrimaryHover`, pressed
//! `backgroundPrimaryActive` and selected `backgroundSurfaceSelected` with a
//! `foregroundPrimary` icon. The active item also gets an [`ActiveIndicator`]:
//! by default a 2px `backgroundBrand` line on the bar's edge (outer edge for a
//! vertical bar, bottom edge for a horizontal one). The bar is
//! `backgroundPrimary` (or the Sidebar chrome material) with a hairline
//! `borderSubtle` on its inner edge.
//!
//! **Badges** ([`ActivityItem::badge`] / [`ActivityItem::dot`]) sit on the
//! icon's top-right corner, fully inside the item, and wear a 2px ring in the
//! colour behind them (the plate when it shows, else the bar or the folder
//! group), so they never touch the bar's edge, the indicator or a group
//! outline. Corner badges are a compact pill (14px, 9px digits) that spells
//! out at most [`ActivityBar::corner_badge_max`] (default 9, so `9+`), keeping
//! the icon readable; the tooltip then carries the full count. With labels
//! beside the icons the badge moves to the end of the row at full size, and
//! counts above 99 read `99+`. [`ActivityItem::badge_variant`] picks
//! the colour (brand ink by default).
//!
//! **Labels** ([`ActivityLabels`]): `Tooltip` (icon-only; a RefineUI
//! [`crate::Tooltip`] opens beside the item, away from the docked edge, with
//! the item's action shortcut), `Below` (a caption under each icon) or
//! `Beside` (an expanded bar, [`ActivityBar::expanded`]); the width animates
//! when it changes.
//!
//! **Structure.** Items scroll (hidden scrollbar) between an optional
//! `header` and the pinned `footer_item`s / `footer`. [`ActivityBar::divider`]
//! and [`ActivityBar::section`] group items (a section shows its title when
//! labels are beside the icons). [`ActivityItem::folder`] groups items under
//! one item: open, the members sit in a recessed group plate that grows with
//! accordion motion; closed, the folder shows its icon and the sum of its
//! members' badges. A selected member also lights the folder's indicator.
//! [`BarSide::Top`] / [`BarSide::Bottom`] lay the bar out horizontally.
//!
//! **Keyboard** — when the bar has focus: ↑ / ↓ (← / → when horizontal) move
//! a focus ring, skipping disabled items (open folder members are in the
//! order); Home / End jump; Enter / Space activate and toggle a focused
//! folder; ⌥ + arrow moves the focused main item when reordering is on.
//! **Pointer** — click activates; nothing acts on hover alone. Disabled items
//! are dimmed (`foregroundDisabled`), not clickable and skipped by the keys.

use std::collections::HashSet;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FocusHandle, Hsla, ImageSource, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Window, div, img, prelude::*, px,
};
use refineui_foundation::{radius, size::icon, spacing, stroke, typography::role};

use crate::interaction::dnd::{DropEdge, drop_edge, insertion_index};
use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, mix, use_interaction,
};
use crate::platform::ScrollbarStyle;
use crate::{
    ActiveTheme, AnchorSide, BadgeVariant, DragPreview, DropIndicator, Icon, IconName, IconSize,
    ScrollArea, StyledExt, Tooltip,
};

/// Which window edge the bar is docked to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarSide {
    /// Leading (left) edge, vertical.
    #[default]
    Leading,
    /// Trailing (right) edge, vertical.
    Trailing,
    /// Top edge, horizontal (e.g. above a sidebar).
    Top,
    /// Bottom edge, horizontal.
    Bottom,
}

impl BarSide {
    /// Items run left to right.
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// Side a tooltip opens on: away from the docked edge.
    pub const fn tooltip_side(self) -> AnchorSide {
        match self {
            Self::Leading => AnchorSide::Right,
            Self::Trailing => AnchorSide::Left,
            Self::Top => AnchorSide::Bottom,
            Self::Bottom => AnchorSide::Top,
        }
    }
}

/// How item names show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityLabels {
    /// Icon-only bar; the name shows in a tooltip.
    #[default]
    Tooltip,
    /// Caption under each icon (navigation rail). Horizontal bars show the
    /// caption beside the icon instead.
    Below,
    /// Label beside the icon in a wider bar.
    Beside,
}

/// How the active item is marked (on top of its selected plate).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActiveIndicator {
    /// 2px brand line on the bar edge (default).
    #[default]
    Edge,
    /// The selected plate only.
    Plate,
}

/// Badge on an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemBadge {
    /// Count (`99+` above 99 in a row; corner badges cap at
    /// [`ActivityBar::corner_badge_max`]).
    Count(u32),
    /// Small dot.
    Dot,
}

impl ItemBadge {
    /// Text of a count badge (`None` for a dot).
    pub fn text(self) -> Option<SharedString> {
        self.label(ACTIVITY_ROW_BADGE_MAX)
    }

    /// Text of a count badge capped at `max` (`{max}+` above it).
    pub fn label(self, max: u32) -> Option<SharedString> {
        let max = max.max(1);
        match self {
            Self::Count(n) if n > max => Some(format!("{max}+").into()),
            Self::Count(n) => Some(n.to_string().into()),
            Self::Dot => None,
        }
    }

    /// Whether `label(max)` drops digits.
    pub fn is_capped(self, max: u32) -> bool {
        matches!(self, Self::Count(n) if n > max.max(1))
    }
}

#[derive(Clone)]
enum Face {
    Icon(IconName),
    Image(ImageSource),
    Folder {
        members: Vec<ActivityItem>,
        open: bool,
        icon: IconName,
    },
}

/// One destination.
#[derive(Clone)]
pub struct ActivityItem {
    id: SharedString,
    label: SharedString,
    tooltip: Option<SharedString>,
    face: Face,
    badge: Option<ItemBadge>,
    badge_variant: BadgeVariant,
    unread: bool,
    disabled: bool,
    action: Option<SharedString>,
}

impl ActivityItem {
    fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, face: Face) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tooltip: None,
            face,
            badge: None,
            badge_variant: BadgeVariant::Default,
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

    /// Picture item (workspace logo, account): an external image source
    /// (URL, asset or bytes) drawn as a 24px rounded square.
    pub fn image(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        src: impl Into<ImageSource>,
    ) -> Self {
        Self::new(id, label, Face::Image(src.into()))
    }

    /// Folder of items. Clicking it shows or hides the members in a group
    /// plate under it. Closed, it shows its icon ([`Self::folder_icon`],
    /// `Folder` by default) and the sum of its members' badges.
    pub fn folder(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        members: impl IntoIterator<Item = ActivityItem>,
    ) -> Self {
        Self::new(
            id,
            label,
            Face::Folder {
                members: members.into_iter().collect(),
                open: false,
                icon: IconName::Folder,
            },
        )
    }

    /// Icon of a folder (folders only; open folders show `FolderOpen` when
    /// this is the default `Folder`).
    pub fn folder_icon(mut self, glyph: IconName) -> Self {
        if let Face::Folder { icon, .. } = &mut self.face {
            *icon = glyph;
        }
        self
    }

    /// Open this folder on first show (folders only).
    pub fn default_open(mut self, open: bool) -> Self {
        if let Face::Folder { open: o, .. } = &mut self.face {
            *o = open;
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

    /// Badge colour: `Default` (brand ink, the default), `Neutral`,
    /// `Success`, `Warning` or `Danger` (`Outline` draws as `Neutral`).
    pub fn badge_variant(mut self, variant: BadgeVariant) -> Self {
        self.badge_variant = variant;
        self
    }

    /// Unread mark: a small muted dot on the bar edge (quieter than a
    /// badge).
    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    /// Disabled (dimmed, skipped by the keyboard, no activation).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Tooltip text (defaults to the label).
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
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

    /// Icon, for icon and folder items.
    pub fn icon_name(&self) -> Option<IconName> {
        match &self.face {
            Face::Icon(i) => Some(*i),
            Face::Folder { icon, .. } => Some(*icon),
            Face::Image(_) => None,
        }
    }

    /// Whether this is a folder.
    pub fn is_folder(&self) -> bool {
        matches!(self.face, Face::Folder { .. })
    }

    /// The badge drawn on the item: its own badge, or for a folder the sum
    /// of its members' count badges (a dot when members only have dots).
    pub fn displayed_badge(&self) -> Option<ItemBadge> {
        if self.badge.is_some() {
            return self.badge;
        }
        let Face::Folder { members, .. } = &self.face else {
            return None;
        };
        let mut sum = 0u32;
        let mut dot = false;
        for m in members {
            match m.displayed_badge() {
                Some(ItemBadge::Count(n)) => sum = sum.saturating_add(n),
                Some(ItemBadge::Dot) => dot = true,
                None => {}
            }
        }
        if sum > 0 {
            Some(ItemBadge::Count(sum))
        } else if dot {
            Some(ItemBadge::Dot)
        } else {
            None
        }
    }

    fn contains(&self, id: &SharedString) -> bool {
        match &self.face {
            Face::Folder { members, .. } => members.iter().any(|m| &m.id == id || m.contains(id)),
            _ => false,
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
    /// Pointer pressed on it.
    pub pressed: bool,
    /// Keyboard cursor on it (focus-visible).
    pub focused: bool,
    /// Disabled.
    pub disabled: bool,
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
    /// Group break; the title shows when labels are beside the icons.
    Section(Option<SharedString>),
}

/// The bar. See the module docs.
#[derive(IntoElement)]
pub struct ActivityBar {
    id: ElementId,
    side: BarSide,
    labels: ActivityLabels,
    indicator: ActiveIndicator,
    expanded_width: f32,
    border: bool,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
    entries: Vec<Entry>,
    footer_items: Vec<ActivityItem>,
    selected: Option<SharedString>,
    on_select: Option<SelectHandler>,
    on_reorder: Option<ReorderHandler>,
    render_item: Option<ItemRenderer>,
    fill: bool,
    corner_badge_max: u32,
}

/// Largest count a row badge (labels beside the icons) spells out.
pub const ACTIVITY_ROW_BADGE_MAX: u32 = 99;
/// Default largest count a corner badge spells out (icon-only and caption
/// bars); above it the badge reads `9+` and the tooltip has the count.
pub const ACTIVITY_CORNER_BADGE_MAX: u32 = 9;

/// Default width of an expanded ([`ActivityLabels::Beside`]) vertical bar.
pub const ACTIVITY_EXPANDED_WIDTH: f32 = 200.;
/// Width of a vertical bar with captions below the icons.
pub const ACTIVITY_CAPTION_WIDTH: f32 = 72.;
/// Length of the [`ActiveIndicator::Edge`] line (the icon size).
pub const ACTIVITY_INDICATOR_LENGTH: f32 = icon::SMALL;

impl ActivityBar {
    /// New icon-only bar on the leading edge.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            side: BarSide::Leading,
            labels: ActivityLabels::Tooltip,
            indicator: ActiveIndicator::Edge,
            expanded_width: ACTIVITY_EXPANDED_WIDTH,
            border: true,
            header: None,
            footer: None,
            entries: Vec::new(),
            footer_items: Vec::new(),
            selected: None,
            on_select: None,
            on_reorder: None,
            render_item: None,
            fill: true,
            corner_badge_max: ACTIVITY_CORNER_BADGE_MAX,
        }
    }

    /// Largest count a corner badge (icon-only and caption bars) spells out
    /// before it reads `{max}+` (default 9). Keep it to one or two digits so
    /// the badge stays on the icon's corner; the tooltip shows the full count.
    pub fn corner_badge_max(mut self, max: u32) -> Self {
        self.corner_badge_max = max.max(1);
        self
    }

    /// Docked edge ([`BarSide::Top`] / [`BarSide::Bottom`] are horizontal).
    pub fn side(mut self, side: BarSide) -> Self {
        self.side = side;
        self
    }

    /// How item names show.
    pub fn labels(mut self, labels: ActivityLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Expanded (labels beside the icons) or icon-only with tooltips.
    pub fn expanded(self, expanded: bool) -> Self {
        self.labels(if expanded {
            ActivityLabels::Beside
        } else {
            ActivityLabels::Tooltip
        })
    }

    /// Width of the expanded vertical bar (default
    /// [`ACTIVITY_EXPANDED_WIDTH`]).
    pub fn expanded_width(mut self, width: f32) -> Self {
        self.expanded_width = width;
        self
    }

    /// Active-item mark.
    pub fn indicator(mut self, indicator: ActiveIndicator) -> Self {
        self.indicator = indicator;
        self
    }

    /// Hairline on the inner edge (default on).
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    /// Element before the items (logo, workspace switcher).
    pub fn header(mut self, e: impl IntoElement) -> Self {
        self.header = Some(e.into_any_element());
        self
    }

    /// Element after the pinned footer items (account, sync state).
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
        self.entries.push(Entry::Section(None));
        self
    }

    /// Start a titled group: a divider in an icon-only bar, the title in an
    /// expanded one.
    pub fn section(mut self, title: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Section(Some(title.into())));
        self
    }

    /// Item pinned to the end (Settings, Accounts).
    pub fn footer_item(mut self, item: ActivityItem) -> Self {
        self.footer_items.push(item);
        self
    }

    /// Selected item id.
    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    /// Activation handler (click, Enter, Space).
    pub fn on_select(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// Enable drag-to-reorder of the main items: `f(from, to)` with
    /// indices into the main item list (sections excluded; `to` is the
    /// final index after removal, as [`crate::interaction::dnd::reorder`]).
    pub fn on_reorder(mut self, f: impl Fn(usize, usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_reorder = Some(Rc::new(f));
        self
    }

    /// Draw every item face yourself (the bar keeps the plate, hit-testing,
    /// focus, indicator, badge and tooltip).
    pub fn render_item(
        mut self,
        f: impl Fn(&ActivityItem, ActivityItemState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.render_item = Some(Rc::new(f));
        self
    }

    /// Fill the parent's cross size (default) or hug the content.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }
}

/// Effective label mode: horizontal bars have no room for a caption below.
fn effective_labels(side: BarSide, labels: ActivityLabels) -> ActivityLabels {
    if side.is_horizontal() && labels == ActivityLabels::Below {
        ActivityLabels::Beside
    } else {
        labels
    }
}

/// Bar thickness (width of a vertical bar, height of a horizontal one).
fn bar_thickness(side: BarSide, labels: ActivityLabels, expanded: f32, density: f32) -> f32 {
    if side.is_horizontal() {
        return density;
    }
    match labels {
        ActivityLabels::Tooltip => density,
        ActivityLabels::Below => ACTIVITY_CAPTION_WIDTH,
        ActivityLabels::Beside => expanded,
    }
}

/// Badge `(fill, ink)` for a variant.
fn badge_colors(v: BadgeVariant, c: &crate::ThemeColors) -> (Hsla, Hsla) {
    match v {
        BadgeVariant::Default => (c.background_brand, c.foreground_on_brand),
        BadgeVariant::Neutral | BadgeVariant::Outline => {
            (c.background_surface_selected, c.foreground_primary)
        }
        BadgeVariant::Success => (c.background_success, c.foreground_on_success),
        BadgeVariant::Warning => (c.background_warning, c.foreground_on_warning),
        BadgeVariant::Danger => (c.background_error, c.foreground_on_error),
    }
}

/// Row count-badge height.
const BADGE_H: f32 = 16.;
/// Corner count-badge height (without its ring): a compact pill so it only
/// covers the icon's corner.
const CORNER_BADGE_H: f32 = 14.;
/// Dot-badge size (without its ring).
const DOT: f32 = 8.;
/// Ring that separates a corner badge from what is behind it.
const RING: f32 = stroke::THICK;

/// Most a corner count badge may overhang its icon box (into the bar's own
/// padding, never across the bar edge, the indicator or a folder outline).
pub const CORNER_BADGE_OVERHANG: f32 = spacing::XX_SMALL;

/// Corner-badge offset `(top, right)` from the icon box's top-right corner
/// (negative = outside the box) for an icon box of `box_w × box_h`.
///
/// A dot sits on the icon's top-right corner, inside the box. A count pill
/// sits up and to the right of the icon's corner, so only its lower-left
/// part covers the icon, and grows to the left; it may leave the box by up
/// to `slack = (right, top)` (the clear space around the box, at most
/// [`CORNER_BADGE_OVERHANG`]).
pub fn badge_offset(
    badge: ItemBadge,
    box_w: f32,
    box_h: f32,
    icon_size: f32,
    slack: (f32, f32),
) -> (f32, f32) {
    // Icon corner, measured from the box's top and right edges.
    let corner_top = (box_h - icon_size) / 2.;
    let corner_right = (box_w - icon_size) / 2.;
    match badge {
        ItemBadge::Dot => {
            let outer = DOT + 2. * RING;
            let top = (corner_top - outer / 2. + spacing::XXX_SMALL).max(0.);
            let right = (corner_right - outer / 2. + spacing::XXX_SMALL).max(0.);
            (top, right)
        }
        ItemBadge::Count(_) => {
            let outer = CORNER_BADGE_H + 2. * RING;
            let clamp = |v: f32| v.clamp(0., CORNER_BADGE_OVERHANG);
            // Centre of a one-digit pill `XXX_SMALL + 1` beyond the corner.
            let out = spacing::XXX_SMALL + 1.;
            let top = (corner_top - out - outer / 2.).max(-clamp(slack.1));
            let right = (corner_right - out - outer / 2.).max(-clamp(slack.0));
            (top, right)
        }
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
    labels: ActivityLabels,
    indicator: ActiveIndicator,
    bar_bg: Hsla,
    size: f32,
    thickness: f32,
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
    corner_max: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Main(usize),
    Footer,
    /// Member of an open folder; in the arrow order, not in ⌥-reorder.
    Child,
}

#[derive(Clone, Debug, PartialEq)]
struct NavEntry {
    id: SharedString,
    disabled: bool,
    /// Index in the main item row; `None` for folder members and footer items.
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
        order.push(NavEntry {
            id: i.id.clone(),
            disabled: i.disabled,
            main_ix: Some(main_ix),
            is_folder: i.is_folder(),
        });
        if open.contains(&i.id)
            && let Face::Folder { members, .. } = &i.face
        {
            for m in members {
                order.push(NavEntry {
                    id: m.id.clone(),
                    disabled: m.disabled,
                    main_ix: None,
                    is_folder: m.is_folder(),
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
            is_folder: i.is_folder(),
        });
    }
    order
}

/// Next focusable index for an arrow / Home / End key, or `None` to ignore
/// the key. `cur` is the current cursor position.
fn step(order: &[NavEntry], cur: Option<usize>, key: &str, horizontal: bool) -> Option<usize> {
    let n = order.len();
    let enabled = |i: usize| !order[i].disabled;
    let (prev, next) = if horizontal {
        ("left", "right")
    } else {
        ("up", "down")
    };
    if key == next {
        let start = cur.map(|c| c + 1).unwrap_or(0);
        (start..n).find(|&i| enabled(i)).or(cur)
    } else if key == prev {
        match cur {
            None => (0..n).rev().find(|&i| enabled(i)),
            Some(c) => (0..c).rev().find(|&i| enabled(i)).or(Some(c)),
        }
    } else if key == "home" {
        (0..n).find(|&i| enabled(i))
    } else if key == "end" {
        (0..n).rev().find(|&i| enabled(i))
    } else {
        None
    }
}

fn face(item: &ActivityItem, ink: Hsla, open: bool) -> AnyElement {
    match &item.face {
        Face::Icon(i) => Icon::new(*i)
            .size(IconSize::Small)
            .color(ink)
            .into_any_element(),
        Face::Image(src) => img(src.clone())
            .size(px(icon::MEDIUM))
            .rounded(px(radius::MEDIUM))
            .into_any_element(),
        Face::Folder { icon: glyph, .. } => {
            let glyph = if open && *glyph == IconName::Folder {
                IconName::FolderOpen
            } else {
                *glyph
            };
            Icon::new(glyph)
                .size(IconSize::Small)
                .color(ink)
                .into_any_element()
        }
    }
}

/// A row badge (`ring: None`, full size, `99+`) or a corner badge (compact
/// pill in a ring, capped at `corner_max`).
fn badge_el(
    b: ItemBadge,
    variant: BadgeVariant,
    ring: Option<Hsla>,
    corner_max: u32,
    cx: &App,
) -> gpui::Div {
    let c = cx.theme().colors;
    let (fill, ink) = badge_colors(variant, &c);
    let corner = ring.is_some();
    let ring_w = if corner { 2. * RING } else { 0. };
    let (h, text_size, pad, text) = if corner {
        (CORNER_BADGE_H, 9., 3., b.label(corner_max))
    } else {
        (BADGE_H, 10., spacing::XX_SMALL, b.text())
    };
    let d = div()
        .flex_none()
        .rounded_full()
        .bg(fill)
        .when_some(ring, |d, r| d.border(px(RING)).border_color(r));
    match text {
        None => d.size(px(DOT + ring_w)),
        Some(text) => d
            .h(px(h + ring_w))
            .min_w(px(h + ring_w))
            .px(px(pad))
            .flex()
            .items_center()
            .justify_center()
            .text_color(ink)
            .text_size(px(text_size))
            .line_height(px(h))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(text),
    }
}

/// One item: plate, face, badge, indicator, tooltip and the drag / click
/// wiring. `behind` is the colour under the item (bar or folder group).
fn tile(
    item: ActivityItem,
    role: Role,
    behind: Hsla,
    t: &Ctx,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let horizontal = t.side.is_horizontal();
    let iid = ElementId::NamedChild(Box::new(t.bar.clone()), format!("item-{}", item.id).into());
    let (istate, i) = use_interaction(&iid, window, cx);
    let disabled = item.disabled;
    let active = t.selected == Some(&item.id);
    let holds_selected = t.selected.is_some_and(|s| item.contains(s));
    let hovered = i.hovered && !disabled;
    let pressed = i.pressed && !disabled;
    let size = t.size;
    let beside = t.labels == ActivityLabels::Beside;
    let below = t.labels == ActivityLabels::Below;
    let is_folder = item.is_folder();
    let open = is_folder && t.folder_open.contains(&item.id);

    let plate_target = if pressed {
        c.background_primary_active
    } else if active {
        c.background_surface_selected
    } else if hovered {
        c.background_primary_hover
    } else {
        c.background_primary_hover.opacity(0.)
    };
    let ink_target = if disabled {
        c.foreground_disabled
    } else if active || hovered || holds_selected {
        c.foreground_primary
    } else {
        c.foreground_secondary
    };
    let plate = animate(
        child_id(&iid, "plate"),
        plate_target,
        Motion::FAST,
        window,
        cx,
    );
    let ink = animate(child_id(&iid, "ink"), ink_target, Motion::FAST, window, cx);
    // What a badge ring must match: the plate where it shows, else `behind`.
    let ring = mix(behind, plate.opacity(1.), plate.a);

    let focus_ring = t.focused && t.cursor.as_ref() == Some(&item.id) && focus_visible(cx);
    let state = ActivityItemState {
        selected: active,
        hovered,
        pressed,
        focused: focus_ring,
        disabled,
        size,
        ink,
    };
    let face_el = match &t.render_item {
        Some(f) => f(&item, state, window, cx),
        None => face(&item, ink, open),
    };

    // Icon box: the plate in icon-only and caption modes.
    // Expanded rows are list-height, not square tiles.
    let row_h = (size - spacing::SMALL).max(28.);
    let (box_w, box_h) = if below {
        (size + spacing::SMALL, size - spacing::XX_SMALL)
    } else if beside {
        (size, row_h)
    } else {
        (size, size)
    };
    let badge = item.displayed_badge();
    let corner_badge = (!beside).then_some(badge).flatten().map(|b| {
        // Clear space around the box: the bar padding across the bar (less a
        // folder group's inset), keeping `XXX_SMALL` from any edge.
        let child_inset = if role == Role::Child {
            spacing::XXX_SMALL
        } else {
            0.
        };
        let room = |cross: f32| (t.thickness - cross) / 2. - child_inset - spacing::XXX_SMALL;
        let slack = if horizontal {
            (0., room(box_h))
        } else {
            (room(box_w), 0.)
        };
        let (top, right) = badge_offset(b, box_w, box_h, icon::SMALL, slack);
        div()
            .absolute()
            .top(px(top))
            .right(px(right))
            .child(badge_el(
                b,
                item.badge_variant,
                Some(ring),
                t.corner_max,
                cx,
            ))
    });
    let row_badge = beside
        .then_some(badge)
        .flatten()
        .map(|b| badge_el(b, item.badge_variant, None, t.corner_max, cx));

    let icon_box = div()
        .relative()
        .flex_none()
        .w(px(box_w))
        .h(px(box_h))
        .flex()
        .items_center()
        .justify_center()
        .when(!beside, |d| d.rounded(px(radius::LARGE)).bg(plate))
        .when(!beside && focus_ring, |d| d.shadow(theme.focus_ring()))
        .child(face_el)
        .children(corner_badge);

    let label_el = beside.then(|| {
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_style_token(role::BODY_SM)
            .text_color(ink)
            .when(active, |d| d.font_weight(gpui::FontWeight::MEDIUM))
            .child(item.label.clone())
    });
    let chevron = (beside && is_folder && !horizontal).then(|| {
        Icon::new(if open {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        })
        .size(IconSize::XXSmall)
        .color(c.foreground_tertiary)
    });
    let caption = below.then(|| {
        div()
            .w_full()
            .flex()
            .justify_center()
            .text_style_token(role::CAPTION_SM)
            .text_color(if disabled {
                c.foreground_disabled
            } else if active {
                c.foreground_primary
            } else {
                c.foreground_secondary
            })
            .when(active, |d| d.font_weight(gpui::FontWeight::MEDIUM))
            .child(div().truncate().child(item.label.clone()))
    });

    let on_select = t.on_select.clone();
    let on_folder_toggle = t.on_folder_toggle.clone();
    let cursor = t.cursor_state.clone();
    let id = item.id.clone();
    let tile_el = div()
        .id(iid.clone())
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .when(beside, |d| {
            d.h(px(row_h))
                .when(!horizontal, |d| d.w_full())
                .gap(px(spacing::X_SMALL))
                .pr(px(spacing::SMALL))
                .rounded(px(radius::LARGE))
                .bg(plate)
                .when(focus_ring, |d| d.shadow(theme.focus_ring()))
        })
        .when(below, |d| {
            d.flex_col()
                .gap(px(spacing::XXX_SMALL))
                .w(px(t.thickness - 2. * spacing::XX_SMALL))
        })
        .when(!beside && !below, |d| d.justify_center())
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
        .child(icon_box)
        .children(label_el)
        .children(row_badge)
        .children(chevron)
        .children(caption);

    // Drag to reorder (main items only).
    let drop_now = *t.drop.read(cx);
    let drop_line = match (role, drop_now) {
        (Role::Main(m), Some((target, edge))) if target == m => Some(edge),
        _ => None,
    };
    let tile_el = match (role, t.on_reorder.clone()) {
        (Role::Main(m), Some(reorder)) => {
            let payload = ActivityDrag {
                bar: t.bar.clone(),
                from: m,
                label: item.label.clone(),
                icon: item.icon_name(),
            };
            let over = t.drop.clone();
            let clear = t.drop.clone();
            let bar_for_move = t.bar.clone();
            let bar_for_drop = t.bar.clone();
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
                    let edge = if horizontal {
                        drop_edge(
                            f32::from(e.event.position.x - b.left()),
                            f32::from(b.size.width),
                            false,
                        )
                    } else {
                        drop_edge(
                            f32::from(e.event.position.y - b.top()),
                            f32::from(b.size.height),
                            false,
                        )
                    };
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
                .into_any_element()
        }
        _ => tile_el.into_any_element(),
    };

    // Tooltip in icon-only bars (never while a drag is in flight), and on
    // captioned items whose corner badge is capped: it carries the full count.
    let capped = !beside && badge.is_some_and(|b| b.is_capped(t.corner_max));
    let tile_el = if t.labels == ActivityLabels::Tooltip || capped {
        let name = item.tooltip.clone().unwrap_or(item.label.clone());
        let text: SharedString = match badge {
            Some(ItemBadge::Count(n)) if capped => format!("{name} · {n}").into(),
            _ => name,
        };
        let mut tip = Tooltip::new(child_id(&iid, "tip"), text).placement(t.side.tooltip_side());
        if let Some(a) = &item.action {
            tip = tip.action(a.clone());
        }
        tip.child(tile_el).into_any_element()
    } else {
        tile_el
    };

    // Active indicator (and unread mark) on the bar edge. Folder members
    // sit inside the group plate, so their folder carries the line.
    let lit = active || (holds_selected && is_folder);
    let edge_line = t.indicator == ActiveIndicator::Edge && role != Role::Child;
    let len = animate(
        child_id(&iid, "indicator"),
        if lit && edge_line {
            ACTIVITY_INDICATOR_LENGTH
        } else {
            0.
        },
        Motion::INDICATOR,
        window,
        cx,
    );
    let along = if below { box_h } else { size };
    let indicator = (len > 0.5).then(|| {
        let d = div().absolute().rounded_full().bg(c.background_brand);
        match t.side {
            BarSide::Leading | BarSide::Trailing => {
                let d = d
                    .w(px(stroke::THICK))
                    .h(px(len))
                    .top(px((along - len) / 2.));
                if t.side == BarSide::Leading {
                    d.left_0()
                } else {
                    d.right_0()
                }
            }
            BarSide::Top | BarSide::Bottom => {
                let d = d.h(px(stroke::THICK)).w(px(len));
                let d = if beside {
                    d.left(px(spacing::SMALL))
                } else {
                    d.left(px((size - len) / 2.))
                };
                if t.side == BarSide::Top {
                    d.bottom_0()
                } else {
                    d.top_0()
                }
            }
        }
    });
    let unread = (item.unread && !lit && role != Role::Child).then(|| {
        let d = div()
            .absolute()
            .size(px(spacing::XX_SMALL))
            .rounded_full()
            .bg(c.foreground_tertiary);
        match t.side {
            BarSide::Leading => d
                .left(px(spacing::XXX_SMALL))
                .top(px((along - spacing::XX_SMALL) / 2.)),
            BarSide::Trailing => d
                .right(px(spacing::XXX_SMALL))
                .top(px((along - spacing::XX_SMALL) / 2.)),
            BarSide::Top => d
                .bottom(px(spacing::XXX_SMALL))
                .left(px((size - spacing::XX_SMALL) / 2.)),
            BarSide::Bottom => d
                .top(px(spacing::XXX_SMALL))
                .left(px((size - spacing::XX_SMALL) / 2.)),
        }
    });

    let drop_mark = |edge: DropEdge| {
        let line = if horizontal {
            DropIndicator::between_tabs()
        } else {
            DropIndicator::between_rows()
        };
        let d = div().absolute();
        let d = match (horizontal, edge) {
            (false, DropEdge::Before) => d.top(px(-3.)).left_0().right_0(),
            (false, _) => d.bottom(px(-3.)).left_0().right_0(),
            (true, DropEdge::Before) => d.left(px(-3.)).top_0().bottom_0(),
            (true, _) => d.right(px(-3.)).top_0().bottom_0(),
        };
        d.child(line)
    };

    // The slot spans the bar's cross axis so the indicator lands on its edge.
    let pad = spacing::X_SMALL;
    div()
        .relative()
        .flex_none()
        .flex()
        .when(!horizontal, |d| {
            d.flex_col()
                .items_center()
                .when(role != Role::Child, |d| d.w(px(t.thickness)).px(px(pad)))
                .when(role == Role::Child, |d| d.w_full())
        })
        .when(horizontal, |d| d.h_full().items_center())
        .children(drop_line.filter(|e| *e == DropEdge::Before).map(drop_mark))
        .children(indicator)
        .children(unread)
        .child(
            div()
                .when(!horizontal && beside, |d| d.w_full())
                .child(tile_el),
        )
        .children(drop_line.filter(|e| *e != DropEdge::Before).map(drop_mark))
        .into_any_element()
}

fn section(title: Option<SharedString>, t: &Ctx, cx: &App) -> AnyElement {
    let c = cx.theme().colors;
    let horizontal = t.side.is_horizontal();
    match (title, t.labels == ActivityLabels::Beside && !horizontal) {
        (Some(title), true) => div()
            .w_full()
            .flex_none()
            .pt(px(spacing::SMALL))
            .pb(px(spacing::XX_SMALL))
            .px(px(spacing::X_SMALL + spacing::SMALL))
            .text_style_token(role::CAPTION_MD)
            .text_color(c.foreground_tertiary)
            .truncate()
            .child(title)
            .into_any_element(),
        _ if horizontal => div()
            .flex_none()
            .h_full()
            .flex()
            .items_center()
            .px(px(spacing::XX_SMALL))
            .child(
                div()
                    .w(px(stroke::THIN))
                    .h(px(icon::SMALL))
                    .bg(c.border_subtle),
            )
            .into_any_element(),
        _ => div()
            .w_full()
            .flex_none()
            .flex()
            .justify_center()
            .py(px(spacing::XX_SMALL))
            .child(
                div()
                    .w(px(if t.labels == ActivityLabels::Beside {
                        t.thickness - 2. * (spacing::X_SMALL + spacing::SMALL)
                    } else {
                        icon::SMALL
                    }))
                    .h(px(stroke::THIN))
                    .bg(c.border_subtle),
            )
            .into_any_element(),
    }
}

/// An open folder: the folder tile on top of its members, in a recessed
/// group plate that fades in with the panel.
fn folder_group(
    item: ActivityItem,
    main_ix: usize,
    t: &Ctx,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let c = cx.theme().colors;
    let horizontal = t.side.is_horizontal();
    let fid = item.id.clone();
    let open = t.folder_open.contains(&fid);
    let members = match &item.face {
        Face::Folder { members, .. } => members.clone(),
        _ => Vec::new(),
    };
    let gid = ElementId::NamedChild(Box::new(t.bar.clone()), format!("folder-{fid}").into());
    let open_t = animate(
        child_id(&gid, "group"),
        if open { 1.0f32 } else { 0.0 },
        Motion::ACCORDION_PANEL,
        window,
        cx,
    );
    let group_bg = c.background_surface_hover;
    let behind = mix(t.bar_bg, group_bg, open_t);
    let head = tile(item, Role::Main(main_ix), behind, t, window, cx);
    if horizontal {
        // No height to animate: members appear inline.
        return div()
            .flex_none()
            .flex()
            .items_center()
            .h_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .rounded(px(radius::X_LARGE))
                    .when(open, |d| {
                        d.bg(group_bg)
                            .border(px(stroke::HAIRLINE))
                            .border_color(c.border_subtle)
                            .pr(px(spacing::XXX_SMALL))
                    })
                    .child(head)
                    .when(open, |d| {
                        d.children(
                            members
                                .into_iter()
                                .map(|m| tile(m, Role::Child, group_bg, t, window, cx)),
                        )
                    }),
            )
            .into_any_element();
    }
    let panel = crate::components::disclosure::collapse_panel(&gid, open, window, cx, |w, cx| {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(spacing::XXX_SMALL))
            .pb(px(spacing::XXX_SMALL))
            .children(
                members
                    .into_iter()
                    .map(|m| tile(m, Role::Child, group_bg, t, w, cx)),
            )
            .into_any_element()
    });
    let inset = spacing::XXX_SMALL;
    div()
        .relative()
        .flex_none()
        .w(px(t.thickness))
        .flex()
        .flex_col()
        .items_center()
        // Group plate behind the folder tile and its members.
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(spacing::X_SMALL - inset))
                .right(px(spacing::X_SMALL - inset))
                .rounded(px(radius::LARGE + inset))
                .bg(group_bg.opacity(open_t))
                .border(px(stroke::HAIRLINE))
                .border_color(c.border_subtle.opacity(open_t)),
        )
        .child(head)
        .children(panel.map(|p| div().w(px(t.thickness - 2. * spacing::X_SMALL)).child(p)))
        .into_any_element()
}

impl RenderOnce for ActivityBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let d = theme.density.desktop();
        let side = self.side;
        let horizontal = side.is_horizontal();
        let labels = effective_labels(side, self.labels);
        let target = bar_thickness(side, labels, self.expanded_width, d.activity_bar_width);
        let thickness = animate(
            child_id(&self.id, "width"),
            target,
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
        if drop.read(cx).is_some() && !cx.has_active_drag() {
            drop.update(cx, |d, _| *d = None);
        }

        let defaults: HashSet<SharedString> = self
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Item(i) => match &i.face {
                    Face::Folder { open: true, .. } => Some(i.id.clone()),
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

        let bar_bg = c.background_primary;
        let t = Ctx {
            bar: &self.id,
            side,
            labels,
            indicator: self.indicator,
            bar_bg,
            size: d.activity_item,
            thickness,
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
            corner_max: self.corner_badge_max,
        };
        let mut main_ix = 0usize;
        let mut list: Vec<AnyElement> = Vec::new();
        for e in self.entries {
            match e {
                Entry::Item(item) if item.is_folder() => {
                    list.push(folder_group(item, main_ix, &t, window, cx));
                    main_ix += 1;
                }
                Entry::Item(item) => {
                    list.push(tile(item, Role::Main(main_ix), bar_bg, &t, window, cx));
                    main_ix += 1;
                }
                Entry::Section(title) => list.push(section(title, &t, cx)),
            }
        }
        let footer_list: Vec<AnyElement> = self
            .footer_items
            .into_iter()
            .map(|item| tile(item, Role::Footer, bar_bg, &t, window, cx))
            .collect();

        let keys = cursor_state.clone();
        let on_select = self.on_select.clone();
        let on_reorder = self.on_reorder.clone();
        let on_folder_toggle_kb = t.on_folder_toggle.clone();
        let order_kb = order.clone();
        let selected_id = self.selected.clone();
        let gap = spacing::XXX_SMALL;
        let items = if horizontal {
            div()
                .id(child_id(&self.id, "scroll"))
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_x_scroll()
                .flex()
                .items_center()
                .gap(px(gap))
                .px(px(spacing::X_SMALL))
                .children(list)
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .child(
                    ScrollArea::new(child_id(&self.id, "scroll"))
                        .edge_blur(false)
                        .scrollbar(ScrollbarStyle::Hidden)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .w_full()
                                .gap(px(gap))
                                .pt(px(spacing::XX_SMALL))
                                .pb(px(spacing::X_SMALL))
                                .children(list),
                        ),
                )
                .into_any_element()
        };
        let slot = |e: AnyElement| {
            div()
                .flex_none()
                .flex()
                .justify_center()
                .items_center()
                .when(!horizontal, |d| d.py(px(spacing::X_SMALL)))
                .when(horizontal, |d| d.px(px(spacing::X_SMALL)))
                .child(e)
        };
        div()
            .id(self.id.clone())
            .key_context("ActivityBar")
            .track_focus(&focus)
            .tab_index(0)
            .on_key_down(move |e: &KeyDownEvent, w, cx| {
                if order_kb.is_empty() {
                    return;
                }
                let cur_id = keys.read(cx).clone().or_else(|| selected_id.clone());
                let cur = cur_id
                    .as_ref()
                    .and_then(|id| order_kb.iter().position(|o| &o.id == id));
                let k = e.keystroke.key.as_str();
                let (prev, next) = if horizontal {
                    ("left", "right")
                } else {
                    ("up", "down")
                };
                if e.keystroke.modifiers.alt && (k == prev || k == next) {
                    if let (Some(c), Some(h)) = (cur, &on_reorder)
                        && let Some(from) = order_kb[c].main_ix
                    {
                        let to = if k == prev {
                            order_kb[..c].iter().rev().find_map(|o| o.main_ix)
                        } else {
                            order_kb[c + 1..].iter().find_map(|o| o.main_ix)
                        }
                        .unwrap_or(from);
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
                if k == "enter" || k == "space" {
                    if let Some(c) = cur
                        && !order_kb[c].disabled
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
                let Some(next) = step(&order_kb, cur, k, horizontal) else {
                    return;
                };
                cx.stop_propagation();
                keys.update(cx, |kk, cx| {
                    *kk = Some(order_kb[next].id.clone());
                    cx.notify();
                });
            })
            .relative()
            .flex()
            .flex_none()
            .when(!horizontal, |d| {
                d.flex_col()
                    .w(px(thickness))
                    .when(self.fill, |d| d.h_full())
            })
            .when(horizontal, |d| {
                d.flex_row()
                    .items_center()
                    .h(px(thickness))
                    .when(self.fill, |d| d.w_full())
            })
            .map(|d| {
                let (fill, layer) =
                    crate::material::chrome(crate::SurfaceKind::Sidebar, bar_bg, cx);
                d.bg(fill).children(layer)
            })
            .when(self.border, |d| {
                let d = d.border_color(c.border_subtle);
                match side {
                    BarSide::Leading => d.border_r(px(stroke::HAIRLINE)),
                    BarSide::Trailing => d.border_l(px(stroke::HAIRLINE)),
                    BarSide::Top => d.border_b(px(stroke::HAIRLINE)),
                    BarSide::Bottom => d.border_t(px(stroke::HAIRLINE)),
                }
            })
            .font_family(theme.font_family.clone())
            .text_color(c.foreground_primary)
            .children(self.header.map(slot))
            .child(items)
            .when(!footer_list.is_empty(), |d| {
                d.child(
                    div()
                        .flex_none()
                        .flex()
                        .when(!horizontal, |d| d.flex_col().pb(px(spacing::X_SMALL)))
                        // Expanded: a hairline keeps pinned rows apart from
                        // a scrolled list.
                        .when(!horizontal && labels == ActivityLabels::Beside, |d| {
                            d.border_t(px(stroke::HAIRLINE))
                                .border_color(c.border_subtle)
                                .pt(px(spacing::XX_SMALL))
                        })
                        .when(horizontal, |d| d.h_full().pr(px(spacing::X_SMALL)))
                        .items_center()
                        .gap(px(gap))
                        .children(footer_list),
                )
            })
            .children(self.footer.map(slot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar_entries() -> (Vec<Entry>, Vec<ActivityItem>) {
        let entries = vec![
            Entry::Item(ActivityItem::icon("a", "A", IconName::Bug)),
            Entry::Section(Some("Group".into())),
            Entry::Item(
                ActivityItem::folder(
                    "folder",
                    "Folder",
                    [
                        ActivityItem::icon("c1", "C1", IconName::Star).badge(3),
                        ActivityItem::icon("c2", "C2", IconName::Code).disabled(true),
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
        (entries, footer)
    }

    #[test]
    fn badges() {
        assert_eq!(
            ItemBadge::Count(7).text().as_ref().map(|s| s.as_ref()),
            Some("7")
        );
        assert_eq!(
            ItemBadge::Count(99).text().as_ref().map(|s| s.as_ref()),
            Some("99")
        );
        assert_eq!(
            ItemBadge::Count(120).text().as_ref().map(|s| s.as_ref()),
            Some("99+")
        );
        assert_eq!(ItemBadge::Dot.text(), None);
        let corner = |n| {
            ItemBadge::Count(n)
                .label(ACTIVITY_CORNER_BADGE_MAX)
                .map(|s| s.to_string())
        };
        assert_eq!(corner(1).as_deref(), Some("1"));
        assert_eq!(corner(9).as_deref(), Some("9"));
        assert_eq!(corner(42).as_deref(), Some("9+"));
        assert_eq!(corner(128).as_deref(), Some("9+"));
        assert_eq!(
            ItemBadge::Count(42)
                .label(99)
                .map(|s| s.to_string())
                .as_deref(),
            Some("42")
        );
        assert_eq!(
            ItemBadge::Count(5)
                .label(0)
                .map(|s| s.to_string())
                .as_deref(),
            Some("1+")
        );
        assert!(ItemBadge::Count(10).is_capped(9) && !ItemBadge::Count(9).is_capped(9));
        assert!(!ItemBadge::Dot.is_capped(9));
        assert!(
            ActivityItem::icon("a", "A", IconName::Bug)
                .badge(0)
                .badge
                .is_none()
        );
        let folder = ActivityItem::folder(
            "f",
            "F",
            [
                ActivityItem::icon("a", "A", IconName::Bug).badge(4),
                ActivityItem::icon("b", "B", IconName::Bug).badge(120),
                ActivityItem::icon("c", "C", IconName::Bug).dot(),
            ],
        );
        assert_eq!(folder.displayed_badge(), Some(ItemBadge::Count(124)));
        let dots = ActivityItem::folder(
            "f",
            "F",
            [ActivityItem::icon("c", "C", IconName::Bug).dot()],
        );
        assert_eq!(dots.displayed_badge(), Some(ItemBadge::Dot));
        let own = ActivityItem::folder(
            "f",
            "F",
            [ActivityItem::icon("a", "A", IconName::Bug).badge(4)],
        )
        .dot();
        assert_eq!(own.displayed_badge(), Some(ItemBadge::Dot));
        assert!(
            ActivityItem::folder("f", "F", [])
                .displayed_badge()
                .is_none()
        );
    }

    #[test]
    fn corner_badges_hug_the_icon_corner() {
        for size in [32., 36., 44.] {
            // Dots: on the icon corner, inside the box.
            let (top, right) = badge_offset(ItemBadge::Dot, size, size, icon::SMALL, (4., 4.));
            let outer = DOT + 2. * RING;
            assert!(top >= 0. && right >= 0., "{size}");
            assert!(top + outer <= size && right + outer <= size, "{size}");
            let corner = (size + icon::SMALL) / 2.;
            assert!((size - right - outer / 2. - corner).abs() <= 4., "{size}");
            // Counts: never past the slack; a one-digit pill covers under a
            // third of the icon (40% with no room around the box).
            for slack in [(0., 0.), (4., 0.), (0., 4.), (9., 9.)] {
                let (top, right) =
                    badge_offset(ItemBadge::Count(5), size, size, icon::SMALL, slack);
                let outer = CORNER_BADGE_H + 2. * RING;
                assert!(
                    top >= -slack.1.min(CORNER_BADGE_OVERHANG),
                    "{size} {slack:?}"
                );
                assert!(
                    right >= -slack.0.min(CORNER_BADGE_OVERHANG),
                    "{size} {slack:?}"
                );
                assert!(top + outer <= size && right + outer <= size, "{size}");
                let inset = (size - icon::SMALL) / 2.;
                let ox = (size - right - inset).min(icon::SMALL)
                    - (size - right - outer - inset).max(0.);
                let oy = (top + outer - inset).min(icon::SMALL) - (top - inset).max(0.);
                let covered = ox.max(0.) * oy.max(0.) / (icon::SMALL * icon::SMALL);
                let cap = if slack == (0., 0.) { 0.4 } else { 0.3 };
                assert!(covered <= cap, "{size} {slack:?} {covered}");
            }
        }
    }

    #[test]
    fn nav_order_includes_open_folder_members() {
        let (entries, footer) = bar_entries();
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
        assert!(opened[1].is_folder && opened[2].main_ix.is_none());
        assert!(opened[3].disabled);
    }

    #[test]
    fn arrows_skip_disabled_items_and_follow_the_orientation() {
        let (entries, footer) = bar_entries();
        let mut open = HashSet::new();
        open.insert("folder".into());
        let o = nav_order(&entries, &footer, &open);
        // c1 (2) → skips disabled c2 (3) → b (4).
        assert_eq!(step(&o, Some(2), "down", false), Some(4));
        assert_eq!(step(&o, Some(4), "up", false), Some(2));
        assert_eq!(step(&o, Some(2), "right", true), Some(4));
        // Wrong axis is ignored.
        assert_eq!(step(&o, Some(2), "right", false), None);
        assert_eq!(step(&o, Some(5), "down", false), Some(5));
        assert_eq!(step(&o, None, "home", false), Some(0));
        assert_eq!(step(&o, None, "end", true), Some(5));
    }

    #[test]
    fn layout_modes() {
        assert!(BarSide::Top.is_horizontal() && !BarSide::Leading.is_horizontal());
        assert_eq!(BarSide::Leading.tooltip_side(), AnchorSide::Right);
        assert_eq!(BarSide::Trailing.tooltip_side(), AnchorSide::Left);
        assert_eq!(BarSide::Top.tooltip_side(), AnchorSide::Bottom);
        assert_eq!(
            effective_labels(BarSide::Top, ActivityLabels::Below),
            ActivityLabels::Beside
        );
        assert_eq!(
            bar_thickness(BarSide::Leading, ActivityLabels::Tooltip, 200., 48.),
            48.
        );
        assert_eq!(
            bar_thickness(BarSide::Leading, ActivityLabels::Beside, 200., 48.),
            200.
        );
        assert_eq!(
            bar_thickness(BarSide::Leading, ActivityLabels::Below, 200., 48.),
            ACTIVITY_CAPTION_WIDTH
        );
        assert_eq!(
            bar_thickness(BarSide::Top, ActivityLabels::Beside, 200., 48.),
            48.
        );
        let f = ActivityItem::folder("f", "F", [ActivityItem::icon("x", "X", IconName::Bug)])
            .folder_icon(IconName::Star);
        assert_eq!(f.icon_name(), Some(IconName::Star));
        assert!(f.contains(&"x".into()) && !f.contains(&"y".into()));
    }
}
