//! Toolbar — web `Toolbar` (`components/Toolbar/style.ts`).
//!
//! `role="toolbar"` row of Buttons, ToggleButtons, groups and dividers:
//! `gap-small` (8px). `attached` puts it on a raised surface — hairline
//! `borderSubtle`, `backgroundSurface`, `p-xx-small` (4px), `shadow4`, and
//! an outer radius of Button radius + padding (12 / 16 / 20px). `size`
//! cascades to every Button / ToggleButton inside that does not set its own.
//! Groups use the [`crate::ButtonGroup`] tube; dividers are 1×16px
//! `borderDefault`. Arrow keys (Left / Right, or Up / Down when vertical)
//! move focus between items.
//!
//! **Overflow** (desktop, `.overflow(true)`, horizontal): items are
//! measured; those that no longer fit move, in order from the end, into a
//! trailing "More" (`…`) dropdown — Buttons become menu items, Toggle
//! Buttons checked items, groups their buttons, dividers separators.
//! Custom children (`.child`) and spacers always stay visible.

use gpui::{
    AnyElement, App, ElementId, IntoElement, KeyDownEvent, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, px,
};
use refineui_foundation::{spacing, stroke};

use crate::components::button_group::group_tube;
use crate::motion::{child_id, probe, use_layout_probe};
use crate::{
    ActiveTheme, Axis, Button, ControlSize, DropdownMenu, Elevation, IconName, Menu, MenuItem,
    ToggleButton,
};

/// Outer radius of an attached toolbar (`attachedRadius`): Button radius + 4px.
pub fn toolbar_radius(size: ControlSize) -> f32 {
    match size {
        ControlSize::Small => 12.,
        ControlSize::Medium => 16.,
        ControlSize::Large => 20.,
    }
}

enum GroupItem {
    Button(Button),
    Toggle(ToggleButton),
}

/// A tube of related toolbar buttons (web `ToolbarGroup`).
pub struct ToolbarGroup {
    items: Vec<GroupItem>,
    orientation: Option<Axis>,
}

impl ToolbarGroup {
    /// Empty group.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            orientation: None,
        }
    }

    /// Add a button.
    pub fn button(mut self, b: Button) -> Self {
        self.items.push(GroupItem::Button(b));
        self
    }

    /// Add a toggle button (web `ToolbarToggleButton`).
    pub fn toggle(mut self, t: ToggleButton) -> Self {
        self.items.push(GroupItem::Toggle(t));
        self
    }

    /// Override the direction inherited from the toolbar.
    pub fn orientation(mut self, o: Axis) -> Self {
        self.orientation = Some(o);
        self
    }
}

impl Default for ToolbarGroup {
    fn default() -> Self {
        Self::new()
    }
}

enum Item {
    Button(Button),
    Toggle(ToggleButton),
    Group(ToolbarGroup),
    Divider,
    Spacer,
    Any(AnyElement),
    /// Moved to the overflow menu (keeps indices stable).
    Hidden,
}

/// An action bar.
#[derive(IntoElement)]
pub struct Toolbar {
    id: ElementId,
    label: Option<SharedString>,
    attached: bool,
    orientation: Axis,
    size: ControlSize,
    items: Vec<Item>,
    fill: bool,
    overflow: bool,
}

impl Toolbar {
    /// New horizontal toolbar (web default `size="sm"`).
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            attached: false,
            orientation: Axis::Horizontal,
            size: ControlSize::Small,
            items: Vec::new(),
            fill: false,
            overflow: false,
        }
    }

    /// Accessible name (web `label` → `aria-label`).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Raised surface with full rounding — a floating action bar.
    pub fn attached(mut self, attached: bool) -> Self {
        self.attached = attached;
        self
    }

    /// Arrow-key axis and layout direction.
    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        self
    }

    /// Control size cascaded to the buttons inside.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Add a button (web `ToolbarButton`).
    pub fn button(mut self, b: Button) -> Self {
        self.items.push(Item::Button(b));
        self
    }

    /// Add a toggle button (web `ToolbarToggleButton`).
    pub fn toggle(mut self, t: ToggleButton) -> Self {
        self.items.push(Item::Toggle(t));
        self
    }

    /// Add a group (web `ToolbarGroup`).
    pub fn group(mut self, g: ToolbarGroup) -> Self {
        self.items.push(Item::Group(g));
        self
    }

    /// Add a divider (web `ToolbarDivider`).
    pub fn divider(mut self) -> Self {
        self.items.push(Item::Divider);
        self
    }

    /// Flexible space: pushes the following items to the trailing edge
    /// (use with [`Self::fill`]).
    pub fn spacer(mut self) -> Self {
        self.items.push(Item::Spacer);
        self
    }

    /// Take the parent's free width (desktop chrome rows: titlebar,
    /// toolbar row) instead of hugging the items.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Move items that don't fit into a trailing "More" menu (horizontal
    /// toolbars; see the module docs). The toolbar then takes the free
    /// width, like [`Self::fill`].
    pub fn overflow(mut self, overflow: bool) -> Self {
        self.overflow = overflow;
        self
    }

    /// Add any element (Input, Select, …).
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.items.push(Item::Any(e.into_any_element()));
        self
    }
}

fn group_item(i: GroupItem, size: ControlSize, vertical: bool) -> AnyElement {
    match i {
        GroupItem::Button(b) => b
            .inherit(Some(size), Some(crate::ButtonVariant::Ghost))
            .full_width(vertical)
            .into_any_element(),
        GroupItem::Toggle(t) => t
            .toolbar_button(size)
            .full_width(vertical)
            .into_any_element(),
    }
}

/// How many hideable items fit, in order. `widths[i]` is item `i`'s
/// width (`None` = never hidden, its width still counts), `available` the
/// toolbar width, `more` the "More" button width, `gap` the item gap.
/// Returns the count of hideable items to keep (all of them when
/// everything fits without the More button).
pub fn overflow_fit(widths: &[(f32, bool)], available: f32, more: f32, gap: f32) -> usize {
    let hideable = widths.iter().filter(|(_, h)| *h).count();
    let n = widths.len();
    let total: f32 = widths.iter().map(|(w, _)| w).sum::<f32>() + gap * n.saturating_sub(1) as f32;
    if total <= available + 0.5 {
        return hideable;
    }
    // Fixed items + the More button are always there.
    let fixed: Vec<f32> = widths
        .iter()
        .filter(|(_, h)| !*h)
        .map(|(w, _)| *w)
        .collect();
    let mut used = fixed.iter().sum::<f32>() + more + gap * fixed.len() as f32;
    let mut keep = 0;
    for (w, _) in widths.iter().filter(|(_, h)| *h) {
        if used + w + gap > available + 0.5 {
            break;
        }
        used += w + gap;
        keep += 1;
    }
    keep
}

fn overflow_menu(id: &ElementId, hidden: Vec<Item>) -> Menu {
    let mut menu = Menu::new(child_id(id, "more-menu"));
    let button = |menu: Menu, b: &Button| {
        let (label, icon, disabled, click) = b.menu_parts();
        let mut item = MenuItem::new(label)
            .disabled(disabled)
            .on_click(move |w, cx| {
                if let Some(f) = &click {
                    f(&gpui::ClickEvent::default(), w, cx);
                }
            });
        if let Some(i) = icon {
            item = item.icon(i);
        }
        menu.item(item)
    };
    let toggle = |menu: Menu, t: &ToggleButton| {
        let (label, icon, pressed, disabled, change) = t.menu_parts();
        let mut item = MenuItem::new(label)
            .checked(pressed)
            .disabled(disabled)
            .on_click(move |w, cx| {
                if let Some(f) = &change {
                    f(&!pressed, w, cx);
                }
            });
        if let Some(i) = icon {
            item = item.icon(i);
        }
        menu.item(item)
    };
    let mut last_divider = true;
    for it in hidden {
        match it {
            Item::Button(b) => {
                menu = button(menu, &b);
                last_divider = false;
            }
            Item::Toggle(t) => {
                menu = toggle(menu, &t);
                last_divider = false;
            }
            Item::Group(g) => {
                for gi in &g.items {
                    menu = match gi {
                        GroupItem::Button(b) => button(menu, b),
                        GroupItem::Toggle(t) => toggle(menu, t),
                    };
                }
                last_divider = false;
            }
            Item::Divider if !last_divider => {
                menu = menu.divider();
                last_divider = true;
            }
            _ => {}
        }
    }
    menu
}

impl RenderOnce for Toolbar {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Overflow: decide which items stay from last frame's widths.
        let mut more: Option<AnyElement> = None;
        let mut measure = None;
        if self.overflow && self.orientation == Axis::Horizontal {
            let p = use_layout_probe(&self.id, window, cx);
            let more_w = cx.theme().density.button(self.size).height;
            let probe_state = p.read(cx).clone();
            let widths: Option<Vec<(f32, bool)>> = probe_state.container.and_then(|_| {
                self.items
                    .iter()
                    .enumerate()
                    .map(|(i, it)| {
                        let hideable = !matches!(it, Item::Any(_) | Item::Spacer);
                        let w = match it {
                            Item::Spacer => Some(0.),
                            _ => probe_state
                                .items
                                .get(i)
                                .copied()
                                .flatten()
                                .map(|b| f32::from(b.size.width)),
                        };
                        w.map(|w| (w, hideable))
                    })
                    .collect()
            });
            if let (Some(widths), Some(c)) = (widths, probe_state.container) {
                let keep = overflow_fit(&widths, f32::from(c.size.width), more_w, spacing::SMALL);
                let hideable = widths.iter().filter(|(_, h)| *h).count();
                if keep < hideable {
                    let mut seen = 0;
                    let mut shown = Vec::new();
                    let mut hidden = Vec::new();
                    for (i, it) in std::mem::take(&mut self.items).into_iter().enumerate() {
                        if widths[i].1 {
                            seen += 1;
                            if seen > keep {
                                hidden.push(it);
                                // Keep the slot so indices stay stable.
                                shown.push(Item::Hidden);
                                continue;
                            }
                        }
                        shown.push(it);
                    }
                    self.items = shown;
                    let trigger =
                        Button::icon_only(child_id(&self.id, "more"), IconName::MoreHorizontal)
                            .inherit(Some(self.size), Some(crate::ButtonVariant::Ghost))
                            .tooltip("More");
                    more = Some(
                        DropdownMenu::new(
                            child_id(&self.id, "more-dropdown"),
                            trigger,
                            overflow_menu(&self.id, hidden),
                        )
                        .align_end()
                        .into_any_element(),
                    );
                }
            }
            measure = Some(p);
        }

        let theme = cx.theme().clone();
        let c = theme.colors;
        let vertical = self.orientation == Axis::Vertical;
        let size = self.size;
        let thin = px(stroke::THIN);
        let rule = px(spacing::LARGE);
        let fill = self.fill;
        let items: Vec<AnyElement> = self
            .items
            .into_iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item, Item::Hidden))
            .map(|(ix, item)| {
                let interactive = !matches!(item, Item::Spacer | Item::Divider);
                let spacer = matches!(item, Item::Spacer);
                let el = match item {
                    Item::Hidden => div().into_any_element(),
                    Item::Spacer => div().flex_1().into_any_element(),
                    Item::Button(b) => b
                        .inherit(Some(size), Some(crate::ButtonVariant::Ghost))
                        .into_any_element(),
                    Item::Toggle(t) => t.toolbar_button(size).into_any_element(),
                    Item::Group(g) => {
                        let gv = g
                            .orientation
                            .map(|o| o == Axis::Vertical)
                            .unwrap_or(vertical);
                        group_tube(size, gv, cx)
                            .children(g.items.into_iter().map(|i| group_item(i, size, gv)))
                            .into_any_element()
                    }
                    Item::Divider => {
                        let d = div().flex_none().bg(c.border_default);
                        if vertical {
                            d.h(thin).w(rule)
                        } else {
                            d.w(thin).h(rule)
                        }
                        .into_any_element()
                    }
                    Item::Any(e) => e,
                };
                // Measured (overflow): wrap each item with a probe.
                let el = match &measure {
                    Some(p) if !spacer => div()
                        .relative()
                        .flex_none()
                        .flex()
                        .child(probe(p, Some(ix)))
                        .child(el)
                        .into_any_element(),
                    _ => el,
                };
                // Chrome toolbar (titlebar / toolbar row): each item
                // occludes the window drag area; spacers and gaps still drag.
                if fill && interactive {
                    div()
                        .flex_none()
                        .flex()
                        .occlude()
                        .child(el)
                        .into_any_element()
                } else {
                    el
                }
            })
            .collect();
        let (next, prev) = if vertical {
            ("down", "up")
        } else {
            ("right", "left")
        };
        let overflowing = measure.is_some();
        div()
            .id(self.id)
            .flex()
            .map(|d| {
                if self.fill || overflowing {
                    d.flex_1().min_w_0()
                } else {
                    d.flex_none()
                }
            })
            .when_some(measure, |d, p| {
                d.relative().overflow_hidden().child(probe(&p, None))
            })
            .items_center()
            .when(vertical, |d| d.flex_col())
            .gap(px(spacing::SMALL))
            .when(self.attached, |d| {
                d.border(px(stroke::HAIRLINE))
                    .border_color(c.border_subtle)
                    .bg(c.background_surface)
                    .p(px(spacing::XX_SMALL))
                    .shadow(theme.shadow(Elevation::RAISED))
                    .rounded(px(
                        cx.theme().density.button(size).radius + spacing::XX_SMALL
                    ))
            })
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                let k = e.keystroke.key.as_str();
                if k == next {
                    window.focus_next();
                    cx.stop_propagation();
                } else if k == prev {
                    window.focus_prev();
                    cx.stop_propagation();
                }
            })
            .children(items)
            .children(more)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_keeps_what_fits() {
        let items = [(40., true), (40., true), (100., false), (40., true)];
        // Everything fits: 40+40+100+40 + 3 gaps of 8 = 244.
        assert_eq!(overflow_fit(&items, 244., 28., 8.), 3);
        // 200px: fixed 100 + More 28 + gap 8 = 136; +48 → 184 (1); +48 → 232 ✗.
        assert_eq!(overflow_fit(&items, 200., 28., 8.), 1);
        // Too small even for the fixed item: hide every hideable one.
        assert_eq!(overflow_fit(&items, 50., 28., 8.), 0);
        assert_eq!(overflow_fit(&[], 10., 28., 8.), 0);
    }

    #[test]
    fn attached_radius_stays_concentric() {
        let d = refineui_foundation::size::Density::Comfortable;
        for s in ControlSize::ALL {
            assert_eq!(toolbar_radius(s), d.button(s).radius + spacing::XX_SMALL);
        }
    }
}
