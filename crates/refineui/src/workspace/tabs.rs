//! Tab Bar, Tab Group and Document Tab (brief §6 Composition).
//!
//! * [`DocumentTab`] — one open document: title, icon, and the editor
//!   states **dirty** (a dot in the close slot that turns into × on
//!   hover), **pinned** (compact icon-only tab in the leading run, no
//!   close button) and **preview** (italic title; replaced by the next
//!   preview, promoted by a double-click or an edit).
//! * [`TabBar`] — the strip. Height `tab_height`; the bar is
//!   `surfaceSunken`, the active tab `backgroundPrimary` with a 2px brand
//!   line on top when its group has focus (`borderStrong` otherwise).
//!   Tabs activate on mouse-down; middle-click closes; the close button
//!   shows on the active / hovered tab. Right-click opens the tab context
//!   menu ([`TabCommand`]). Tabs drag ([`TabDrag`], [`DragPreview`] chip)
//!   to reorder (a [`DropIndicator`] marks the slot) or to another group.
//!   When tabs overflow the strip scrolls (the active tab is kept in view)
//!   and a chevron lists every tab. The strip is one tab stop: ← / →
//!   activate the neighbour, Home / End the ends.
//! * [`TabGroup`] — bar + content. While a tab is dragged over the
//!   content a [`DropZone`] shows where it lands: the middle joins the
//!   group, an edge splits it ([`crate::interaction::dnd::dock_zone`]).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, IntoElement, KeyDownEvent, MouseButton, ParentElement,
    RenderOnce, ScrollHandle, SharedString, Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{spacing, stroke, typography::role};

use super::{insertion_slot, is_dragging, mark_drag};
use crate::foundation::CursorRole;
use crate::interaction::dnd::{DockZone, dock_zone};
use crate::motion::{
    TrackInteraction, child_id, focus_visible, probe, use_focus_handle, use_interaction,
    use_layout_probe,
};
use crate::{
    ActiveTheme, ContextMenu, DragPreview, DropIndicator, DropZone, DropdownMenu, Icon, IconName,
    IconSize, Menu, MenuItem, StyledExt, Tooltip,
};

/// One open document. See the module docs.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentTab {
    id: String,
    title: SharedString,
    icon: Option<IconName>,
    dirty: bool,
    pinned: bool,
    preview: bool,
    tooltip: Option<SharedString>,
}

impl DocumentTab {
    /// Document `id` titled `title`.
    pub fn new(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            dirty: false,
            pinned: false,
            preview: false,
            tooltip: None,
        }
    }

    /// File-type icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Unsaved changes.
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Pinned (compact, no close button).
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Preview (italic; replaced by the next preview).
    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// Hover text (e.g. the full path).
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Document id.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Drag payload of a document tab.
#[derive(Clone, Debug, PartialEq)]
pub struct TabDrag {
    /// Source group id.
    pub group: String,
    /// Document id.
    pub doc: String,
    /// Title (preview chip).
    pub title: SharedString,
    /// Icon (preview chip).
    pub icon: Option<IconName>,
}

/// A tab context-menu command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabCommand {
    /// Close this tab.
    Close,
    /// Close every other unpinned tab.
    CloseOthers,
    /// Close unpinned tabs to the right.
    CloseToRight,
    /// Close every unpinned tab.
    CloseAll,
    /// Pin.
    Pin,
    /// Unpin.
    Unpin,
    /// Promote a preview tab.
    KeepOpen,
    /// Move into a new group on the right.
    SplitRight,
    /// Move into a new group below.
    SplitDown,
    /// Copy the document path (`tooltip`).
    CopyPath,
}

impl TabCommand {
    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::CloseOthers => "Close Others",
            Self::CloseToRight => "Close to the Right",
            Self::CloseAll => "Close All",
            Self::Pin => "Pin Tab",
            Self::Unpin => "Unpin Tab",
            Self::KeepOpen => "Keep Open",
            Self::SplitRight => "Split Right",
            Self::SplitDown => "Split Down",
            Self::CopyPath => "Copy Path",
        }
    }

    /// The menu for `tab` at `ix` of `len` (sections, `None` = separator).
    pub fn menu_for(tab: &DocumentTab, ix: usize, len: usize) -> Vec<Option<(TabCommand, bool)>> {
        vec![
            Some((Self::Close, true)),
            Some((Self::CloseOthers, len > 1)),
            Some((Self::CloseToRight, ix + 1 < len)),
            Some((Self::CloseAll, true)),
            None,
            Some((if tab.pinned { Self::Unpin } else { Self::Pin }, true)),
            Some((Self::KeepOpen, tab.preview)),
            None,
            Some((Self::SplitRight, true)),
            Some((Self::SplitDown, true)),
            None,
            Some((Self::CopyPath, tab.tooltip.is_some())),
        ]
    }
}

type DocHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;
type CommandHandler = Rc<dyn Fn(TabCommand, &str, &mut Window, &mut App)>;
type TabDropHandler = Rc<dyn Fn(&TabDrag, usize, &mut Window, &mut App)>;

/// The tab strip. See the module docs.
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    group: String,
    tabs: Vec<DocumentTab>,
    active: Option<usize>,
    focused: bool,
    trailing: Vec<AnyElement>,
    on_select: Option<DocHandler>,
    on_close: Option<DocHandler>,
    on_promote: Option<DocHandler>,
    on_command: Option<CommandHandler>,
    on_drop: Option<TabDropHandler>,
}

impl TabBar {
    /// Strip for tab group `group`.
    pub fn new(id: impl Into<ElementId>, group: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            group: group.into(),
            tabs: Vec::new(),
            active: None,
            focused: true,
            trailing: Vec::new(),
            on_select: None,
            on_close: None,
            on_promote: None,
            on_command: None,
            on_drop: None,
        }
    }

    /// Add a tab.
    pub fn tab(mut self, tab: DocumentTab) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Add tabs.
    pub fn tabs(mut self, tabs: impl IntoIterator<Item = DocumentTab>) -> Self {
        self.tabs.extend(tabs);
        self
    }

    /// Active tab index.
    pub fn active(mut self, ix: Option<usize>) -> Self {
        self.active = ix;
        self
    }

    /// The group has focus (brand line on the active tab).
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Trailing actions (split, more).
    pub fn trailing(mut self, e: impl IntoElement) -> Self {
        self.trailing.push(e.into_any_element());
        self
    }

    /// A tab was activated (mouse-down, keyboard, overflow menu).
    pub fn on_select(mut self, f: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// A tab's close button or middle-click.
    pub fn on_close(mut self, f: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    /// A preview tab was double-clicked.
    pub fn on_promote(mut self, f: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_promote = Some(Rc::new(f));
        self
    }

    /// A context-menu command.
    pub fn on_command(
        mut self,
        f: impl Fn(TabCommand, &str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_command = Some(Rc::new(f));
        self
    }

    /// A tab was dropped at insertion slot `usize` (as shown).
    pub fn on_drop(mut self, f: impl Fn(&TabDrag, usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_drop = Some(Rc::new(f));
        self
    }
}

/// The strip overflows when its items are wider than the viewport.
pub fn tabs_overflow(viewport: f32, item_widths: &[f32]) -> bool {
    item_widths.iter().sum::<f32>() > viewport + 0.5
}

struct BarScroll {
    handle: ScrollHandle,
    last_active: Option<usize>,
}

impl RenderOnce for TabBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let m = theme.desktop();
        let n = self.tabs.len();
        let active = self.active.filter(|a| *a < n);
        let bar = use_layout_probe(&self.id, window, cx);
        let strip_id = child_id(&self.id, "strip");
        let strip = use_layout_probe(&strip_id, window, cx);
        let scroll: Entity<BarScroll> =
            window.use_keyed_state(child_id(&self.id, "scroll"), cx, |_, _| BarScroll {
                handle: ScrollHandle::new(),
                last_active: None,
            });
        let handle = scroll.read(cx).handle.clone();
        if scroll.read(cx).last_active != active {
            if let Some(a) = active {
                handle.scroll_to_item(a);
            }
            scroll.update(cx, |s, _| s.last_active = active);
        }
        let slot: Entity<Option<usize>> =
            window.use_keyed_state(child_id(&self.id, "drop-slot"), cx, |_, _| None);
        let shown_slot = if is_dragging::<TabDrag>(cx) {
            *slot.read(cx)
        } else {
            None
        };
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let ring = focus.is_focused(window) && focus_visible(cx);
        let ids: Vec<String> = self.tabs.iter().map(|t| t.id.clone()).collect();

        let overflow = {
            let p = strip.read(cx);
            let widths: Vec<f32> = p
                .items
                .iter()
                .take(n)
                .flatten()
                .map(|b| f32::from(b.size.width))
                .collect();
            p.container
                .is_some_and(|v| tabs_overflow(f32::from(v.size.width), &widths))
        };

        let mut tab_els: Vec<AnyElement> = Vec::with_capacity(n);
        for (ix, tab) in self.tabs.iter().enumerate() {
            tab_els.push(self.tab_el(tab, ix, n, active, &strip, window, cx));
        }

        // Pointer → insertion slot.
        let track = {
            let strip = strip.clone();
            let slot = slot.clone();
            move |e: &gpui::DragMoveEvent<TabDrag>, _: &mut Window, cx: &mut App| {
                let inside = e.bounds.contains(&e.event.position);
                let next = inside.then(|| {
                    let items: Vec<(f32, f32)> = strip
                        .read(cx)
                        .items
                        .iter()
                        .take(n)
                        .flatten()
                        .map(|b| (f32::from(b.left()), f32::from(b.size.width)))
                        .collect();
                    insertion_slot(f32::from(e.event.position.x), &items)
                });
                slot.update(cx, |s, cx| {
                    if *s != next {
                        *s = next;
                        cx.notify();
                    }
                });
            }
        };
        let drop = {
            let slot = slot.clone();
            let on_drop = self.on_drop.clone();
            move |d: &TabDrag, w: &mut Window, cx: &mut App| {
                let at = slot.update(cx, |s, _| s.take()).unwrap_or(n);
                if let Some(f) = &on_drop {
                    f(d, at, w, cx);
                }
            }
        };
        let indicator = shown_slot.and_then(|s| {
            let bp = bar.read(cx).container?;
            let sp = strip.read(cx);
            let view = sp.container?;
            let item = |i: usize| sp.items.get(i).copied().flatten();
            let x = if s < n {
                item(s)?.left()
            } else {
                item(n.checked_sub(1)?)?.right()
            };
            let x = x.max(view.left()).min(view.right());
            Some(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(x - bp.left() - px(3.))
                    .child(DropIndicator::between_tabs()),
            )
        });

        let overflow_menu = overflow.then(|| {
            let mut menu = Menu::new(child_id(&self.id, "overflow-menu"));
            for (ix, t) in self.tabs.iter().enumerate() {
                let sel = self.on_select.clone();
                let doc = t.id.clone();
                let mut item = MenuItem::new(if t.dirty {
                    SharedString::from(format!("{} ●", t.title))
                } else {
                    t.title.clone()
                })
                .checked(Some(ix) == active)
                .on_click(move |w, cx| {
                    if let Some(f) = &sel {
                        f(&doc, w, cx);
                    }
                });
                if let Some(i) = t.icon {
                    item = item.icon(i);
                }
                menu = menu.item(item);
            }
            let trigger = Tooltip::new(child_id(&self.id, "overflow-tip"), "Show all tabs").child(
                div()
                    .id(child_id(&self.id, "overflow-trigger"))
                    .size(px(m.tab_height - 8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(refineui_foundation::radius::MEDIUM))
                    .cursor_action()
                    .hover(|s| s.bg(c.background_primary_hover))
                    .child(
                        Icon::new(IconName::ChevronDown)
                            .size(IconSize::XSmall)
                            .color(c.foreground_secondary),
                    ),
            );
            DropdownMenu::new(child_id(&self.id, "overflow"), trigger, menu).align_end()
        });

        let on_key = self.on_select.clone();
        div()
            .id(self.id.clone())
            .relative()
            .flex_none()
            .h(px(m.tab_height))
            .w_full()
            .flex()
            .map(|d| {
                let (fill, layer) =
                    crate::material::chrome(crate::SurfaceKind::Titlebar, c.surface_sunken, cx);
                d.bg(fill).children(layer)
            })
            .border_b(px(stroke::HAIRLINE))
            .border_color(c.border_subtle)
            .track_focus(&focus)
            .when(ring, |d| d.shadow(theme.focus_ring()))
            .on_key_down(move |ev: &KeyDownEvent, w, cx| {
                let len = ids.len();
                let next = match (ev.keystroke.key.as_str(), active) {
                    ("left", Some(a)) => a.checked_sub(1),
                    ("right", Some(a)) => (a + 1 < len).then_some(a + 1),
                    ("home", _) => (len > 0).then_some(0),
                    ("end", _) => len.checked_sub(1),
                    _ => return,
                };
                cx.stop_propagation();
                if let (Some(nx), Some(f)) = (next, &on_key) {
                    f(&ids[nx], w, cx);
                }
            })
            .on_drag_move::<TabDrag>(track)
            .on_drop::<TabDrag>(drop)
            .child(probe(&bar, None))
            .child(
                div()
                    .id(strip_id)
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .overflow_x_scroll()
                    .track_scroll(&handle)
                    .children(tab_els)
                    .child(probe(&strip, None)),
            )
            .child(
                div()
                    .flex_none()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(spacing::XX_SMALL))
                    .px(px(spacing::XX_SMALL))
                    .children(overflow_menu)
                    .children(self.trailing),
            )
            .children(indicator)
    }
}

impl TabBar {
    #[allow(clippy::too_many_arguments)]
    fn tab_el(
        &self,
        tab: &DocumentTab,
        ix: usize,
        n: usize,
        active: Option<usize>,
        strip: &Entity<crate::motion::LayoutProbe>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let m = theme.desktop();
        let is_active = Some(ix) == active;
        let tab_id =
            ElementId::NamedChild(Box::new(self.id.clone()), format!("tab:{}", tab.id).into());
        let (st, i) = use_interaction(&tab_id, window, cx);
        let close_id = child_id(&tab_id, "close");
        let (cst, ci) = use_interaction(&close_id, window, cx);
        let fg = if is_active || i.hovered {
            c.foreground_primary
        } else {
            c.foreground_secondary
        };
        let top_line = if self.focused {
            c.background_brand
        } else {
            c.border_strong
        };
        let doc = tab.id.clone();
        let show_close = !tab.pinned && (is_active || i.hovered);
        let close_slot = {
            let on_close = self.on_close.clone();
            let doc = doc.clone();
            let glyph = if tab.dirty && !(i.hovered || ci.hovered) {
                div()
                    .size(px(8.))
                    .rounded_full()
                    .bg(c.foreground_secondary)
                    .into_any_element()
            } else if show_close || (tab.dirty && i.hovered) {
                Icon::new(IconName::Dismiss)
                    .size(IconSize::XSmall)
                    .color(if ci.hovered {
                        c.foreground_primary
                    } else {
                        c.foreground_secondary
                    })
                    .into_any_element()
            } else {
                div().into_any_element()
            };
            div()
                .id(close_id)
                .flex_none()
                .size(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(refineui_foundation::radius::SMALL))
                .when(ci.hovered && (show_close || tab.dirty), |d| {
                    d.bg(c.background_primary_active)
                })
                .track_interaction(&cst)
                .child(glyph)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, w, cx| {
                    cx.stop_propagation();
                    if let Some(f) = &on_close {
                        f(&doc, w, cx);
                    }
                })
        };

        let select = self.on_select.clone();
        let promote = self.on_promote.clone();
        let middle = self.on_close.clone();
        let (d1, d2, d3) = (doc.clone(), doc.clone(), doc.clone());
        let is_preview = tab.preview;
        let mut el = div()
            .id(tab_id.clone())
            .relative()
            .flex_none()
            .h(px(m.tab_height))
            .flex()
            .items_center()
            .gap(px(spacing::X_SMALL))
            .when(tab.pinned, |d| d.px(px(spacing::SMALL)))
            .when(!tab.pinned, |d| {
                d.pl(px(spacing::MEDIUM)).pr(px(spacing::X_SMALL))
            })
            .max_w(px(240.))
            .border_r(px(stroke::HAIRLINE))
            .border_color(c.border_subtle)
            .text_style_token(role::BODY_SM)
            .text_color(fg)
            .when(is_active, |d| d.bg(c.background_primary))
            .when(!is_active && i.hovered, |d| {
                d.bg(c.background_primary_hover)
            })
            .cursor_role(CursorRole::Action)
            .track_interaction(&st)
            .child(probe(strip, Some(ix)))
            .when(is_active, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(stroke::THICK))
                        .bg(top_line),
                )
            })
            .when_some(tab.icon, |d, icon| {
                d.child(Icon::new(icon).size(IconSize::XSmall).color(fg))
            })
            .when(tab.pinned && tab.icon.is_none(), |d| {
                d.child(Icon::new(IconName::Pin).size(IconSize::XSmall).color(fg))
            })
            .when(!tab.pinned, |d| {
                d.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .when(tab.preview, |d| d.italic())
                        .child(tab.title.clone()),
                )
                .child(close_slot)
            })
            .when(tab.pinned && tab.dirty, |d| {
                d.child(div().size(px(6.)).rounded_full().bg(c.foreground_secondary))
            })
            .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                if let Some(f) = &select {
                    f(&d1, w, cx);
                }
            })
            .on_mouse_down(MouseButton::Middle, move |_, w, cx| {
                if let Some(f) = &middle {
                    f(&d2, w, cx);
                }
            })
            .on_click(move |e, w, cx| {
                if e.click_count() >= 2
                    && is_preview
                    && let Some(f) = &promote
                {
                    f(&d3, w, cx);
                }
            });
        let tip = match (&tab.tooltip, tab.pinned) {
            (Some(t), _) => Some(t.clone()),
            (None, true) => Some(tab.title.clone()),
            _ => None,
        };
        let drag = TabDrag {
            group: self.group.clone(),
            doc: doc.clone(),
            title: tab.title.clone(),
            icon: tab.icon,
        };
        el = el.on_drag(drag, |d, _, _, cx| {
            let mut p = DragPreview::new(d.title.clone());
            if let Some(i) = d.icon {
                p = p.icon(i);
            }
            let v = cx.new(|_| p);
            mark_drag::<TabDrag, _>(v, cx)
        });
        let el: AnyElement = if let Some(t) = tip {
            Tooltip::new(child_id(&tab_id, "tip"), t)
                .child(el)
                .into_any_element()
        } else {
            el.into_any_element()
        };

        let mut menu = Menu::new(child_id(&tab_id, "menu"));
        for entry in TabCommand::menu_for(tab, ix, n) {
            match entry {
                None => menu = menu.divider(),
                Some((cmd, enabled)) => {
                    let f = self.on_command.clone();
                    let doc = doc.clone();
                    menu = menu.item(MenuItem::new(cmd.label()).disabled(!enabled).on_click(
                        move |w, cx| {
                            if let Some(f) = &f {
                                f(cmd, &doc, w, cx);
                            }
                        },
                    ));
                }
            }
        }
        div()
            .flex_none()
            .h(px(m.tab_height))
            .child(ContextMenu::new(child_id(&tab_id, "context"), el, menu))
            .into_any_element()
    }
}

type ZoneDrop = Rc<dyn Fn(&TabDrag, DockZone, &mut Window, &mut App)>;
type FocusHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Bar + content. See the module docs.
#[derive(IntoElement)]
pub struct TabGroup {
    id: ElementId,
    bar: Option<TabBar>,
    content: Vec<AnyElement>,
    on_zone_drop: Option<ZoneDrop>,
    on_focus: Option<FocusHandler>,
}

impl TabGroup {
    /// Empty group view.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bar: None,
            content: Vec::new(),
            on_zone_drop: None,
            on_focus: None,
        }
    }

    /// The tab strip.
    pub fn bar(mut self, bar: TabBar) -> Self {
        self.bar = Some(bar);
        self
    }

    /// The active document's view.
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content.push(e.into_any_element());
        self
    }

    /// A tab was dropped on the content at `zone` (Center = join,
    /// edges = split; see [`super::WorkspaceLayout::split_with`]).
    pub fn on_zone_drop(
        mut self,
        f: impl Fn(&TabDrag, DockZone, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_zone_drop = Some(Rc::new(f));
        self
    }

    /// Pointer-down anywhere in the group (make it the active group).
    pub fn on_focus(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_focus = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for TabGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let zone: Entity<Option<DockZone>> =
            window.use_keyed_state(child_id(&self.id, "zone"), cx, |_, _| None);
        let dragging = is_dragging::<TabDrag>(cx) && self.on_zone_drop.is_some();
        let shown = if dragging { *zone.read(cx) } else { None };
        let focus = self.on_focus.clone();
        let overlay = dragging.then(|| {
            let track = zone.clone();
            let take = zone.clone();
            let f = self.on_zone_drop.clone();
            div()
                .id(child_id(&self.id, "drop"))
                .absolute()
                .inset_0()
                .on_drag_move::<TabDrag>(move |e, _, cx| {
                    let b = e.bounds;
                    let p = e.event.position;
                    let next = b.contains(&p).then(|| {
                        dock_zone(
                            f32::from(p.x - b.left()),
                            f32::from(p.y - b.top()),
                            f32::from(b.size.width),
                            f32::from(b.size.height),
                            0.25,
                        )
                    });
                    track.update(cx, |s, cx| {
                        if *s != next {
                            *s = next;
                            cx.notify();
                        }
                    });
                })
                .on_drop::<TabDrag>(move |d, w, cx| {
                    let z = take.update(cx, |s, _| s.take()).unwrap_or(DockZone::Center);
                    if let Some(f) = &f {
                        f(d, z, w, cx);
                    }
                })
                .child(DropZone::new(shown))
        });
        div()
            .id(self.id.clone())
            .size_full()
            .flex()
            .flex_col()
            .bg(c.background_primary)
            .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                if let Some(f) = &focus {
                    f(w, cx);
                }
            })
            .children(self.bar)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(self.content)
                    .children(overlay),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_and_menu() {
        assert!(!tabs_overflow(300., &[100., 100., 100.]));
        assert!(tabs_overflow(300., &[100., 100., 101.]));
        assert!(!tabs_overflow(300., &[]));
        let t = DocumentTab::new("a", "a.rs").preview(true);
        let m = TabCommand::menu_for(&t, 0, 1);
        let enabled =
            |cmd: TabCommand| m.iter().flatten().find(|(c, _)| *c == cmd).map(|(_, e)| *e);
        assert_eq!(enabled(TabCommand::CloseOthers), Some(false));
        assert_eq!(enabled(TabCommand::CloseToRight), Some(false));
        assert_eq!(enabled(TabCommand::KeepOpen), Some(true));
        assert_eq!(enabled(TabCommand::Pin), Some(true));
        assert_eq!(enabled(TabCommand::Unpin), None);
        assert_eq!(enabled(TabCommand::CopyPath), Some(false));
        let p = DocumentTab::new("b", "b.rs").pinned(true).tooltip("/b.rs");
        let m = TabCommand::menu_for(&p, 0, 3);
        assert!(
            m.iter()
                .flatten()
                .any(|(c, e)| *c == TabCommand::Unpin && *e)
        );
        assert!(
            m.iter()
                .flatten()
                .any(|(c, e)| *c == TabCommand::CopyPath && *e)
        );
    }
}
