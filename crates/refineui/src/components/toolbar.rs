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

use gpui::{
    AnyElement, App, ElementId, IntoElement, KeyDownEvent, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{spacing, stroke};

use crate::components::button_group::group_tube;
use crate::{ActiveTheme, Axis, Button, ControlSize, Elevation, ToggleButton};

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

impl RenderOnce for Toolbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
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
            .map(|item| {
                let interactive = !matches!(item, Item::Spacer | Item::Divider);
                let el = match item {
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
        div()
            .id(self.id)
            .flex()
            .map(|d| {
                if self.fill {
                    d.flex_1().min_w_0()
                } else {
                    d.flex_none()
                }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_radius_stays_concentric() {
        let d = refineui_tokens::size::Density::Comfortable;
        for s in ControlSize::ALL {
            assert_eq!(toolbar_radius(s), d.button(s).radius + spacing::XX_SMALL);
        }
    }
}
