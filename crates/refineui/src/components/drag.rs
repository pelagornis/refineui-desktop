//! Drag and drop parts (brief §5 Interaction: Drag Handle, Drag Preview,
//! Drop Zone).
//!
//! GPUI owns the gesture (`.on_drag(payload, …)`, `.drag_over::<T>`,
//! `.on_drag_move::<T>`, `.on_drop::<T>`); these are the *visuals*, and
//! [`crate::interaction::dnd`] is the geometry (`drop_edge`, `dock_zone`,
//! `reorder`), so every list, tree, tab strip and pane drops the same way:
//!
//! * [`DragHandle`] — a 2 × 3 dot grip (`foregroundTertiary` →
//!   `foregroundSecondary` on hover), `Grab` cursor. Not a tab stop: lists
//!   reorder from the keyboard with ⌥↑ / ⌥↓ on the focused row.
//! * [`DragPreview`] — the floating chip that follows the pointer: frosted
//!   popover surface, `shadow` floating elevation, optional icon and a
//!   count badge for multi-item drags. Return it from `on_drag`.
//! * [`DropIndicator`] — the insertion line between rows / tabs: 2px
//!   `borderFocus` with a 6px ring cap at the leading end.
//! * [`DropZone`] — pane overlay highlighting where a docking drop lands
//!   ([`crate::interaction::dnd::DockZone`]): `backgroundBrandSubtle` fill
//!   with a hairline `borderFocus` outline over the target half / whole.

use gpui::{
    App, Context, ElementId, IntoElement, ParentElement, Render, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, px, relative,
};
use refineui_tokens::{radius, spacing, stroke, typography::role};

use crate::foundation::CursorRole;
use crate::interaction::dnd::{DockZone, dock_zone_fraction};
use crate::motion::{TrackInteraction, use_interaction};
use crate::{ActiveTheme, Axis, Elevation, Icon, IconName, IconSize, StyledExt};

/// A grip that starts a drag. See the module docs.
#[derive(IntoElement)]
pub struct DragHandle {
    id: ElementId,
    axis: Axis,
}

impl DragHandle {
    /// Vertical grip (2 columns × 3 rows of dots) for rows.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            axis: Axis::Vertical,
        }
    }

    /// `Horizontal`: 3 columns × 2 rows, for tabs and cards.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

impl RenderOnce for DragHandle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (state, i) = use_interaction(&self.id, window, cx);
        let c = cx.theme().colors;
        let fg = if i.hovered || i.pressed {
            c.foreground_secondary
        } else {
            c.foreground_tertiary
        };
        let (cols, rows) = match self.axis {
            Axis::Vertical => (2, 3),
            Axis::Horizontal => (3, 2),
        };
        let dot = move || div().size(px(3.)).rounded_full().bg(fg);
        div()
            .id(self.id)
            .flex_none()
            .p(px(spacing::XX_SMALL))
            .rounded(px(radius::SMALL))
            .when(i.hovered, |d| d.bg(c.background_primary_hover))
            .cursor_role(if i.pressed {
                CursorRole::Grabbing
            } else {
                CursorRole::Grab
            })
            .track_interaction(&state)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children((0..rows).map(move |_| {
                        div()
                            .flex()
                            .gap(px(2.))
                            .children((0..cols).map(move |_| dot()))
                    })),
            )
    }
}

/// The chip that follows the pointer during a drag. Use it as the
/// `on_drag` view: `.on_drag(payload, |p, _, _, cx| cx.new(|_| DragPreview::new(p.label())))`.
#[derive(Clone)]
pub struct DragPreview {
    label: SharedString,
    icon: Option<IconName>,
    count: usize,
}

impl DragPreview {
    /// Preview showing `label`.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            count: 1,
        }
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Number of dragged items; above 1 a count badge shows.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    fn chip(&self, cx: &App) -> gpui::Div {
        let theme = cx.theme();
        let c = theme.colors;
        div()
            .flex()
            .items_center()
            .gap(px(spacing::X_SMALL))
            .px(px(spacing::MEDIUM))
            .py(px(spacing::X_SMALL))
            .max_w(px(280.))
            .bg(theme.floating_fill())
            .border(px(stroke::HAIRLINE))
            .border_color(c.border_default)
            .rounded(px(radius::LARGE))
            .shadow(theme.shadow(Elevation::FLOATING))
            .opacity(0.96)
            .text_style_token(role::BODY_SM)
            .text_color(c.foreground_primary)
            .font_family(theme.font_family.clone())
            .when_some(self.icon, |d, icon| {
                d.child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .color(c.foreground_secondary),
                )
            })
            .child(div().truncate().child(self.label.clone()))
            .when(self.count > 1, |d| {
                d.child(
                    div()
                        .flex_none()
                        .min_w(px(18.))
                        .h(px(18.))
                        .px(px(spacing::XX_SMALL))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(c.background_brand)
                        .text_color(c.foreground_on_brand)
                        .text_style_token(role::CAPTION_MD)
                        .child(self.count.to_string()),
                )
            })
    }

    /// The chip as a plain element (docs, screenshots).
    pub fn element(&self, cx: &App) -> gpui::Div {
        self.chip(cx)
    }
}

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Offset so the pointer sits just inside the chip's leading edge.
        div().pl(px(spacing::MEDIUM)).child(self.chip(cx))
    }
}

/// Insertion line between rows (vertical lists) or tabs (horizontal strips).
#[derive(IntoElement)]
pub struct DropIndicator {
    axis: Axis,
}

impl DropIndicator {
    /// Line for a vertical list (horizontal line between rows).
    pub fn between_rows() -> Self {
        Self {
            axis: Axis::Vertical,
        }
    }

    /// Line for a horizontal strip (vertical line between tabs).
    pub fn between_tabs() -> Self {
        Self {
            axis: Axis::Horizontal,
        }
    }
}

impl RenderOnce for DropIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let ring = div()
            .flex_none()
            .size(px(6.))
            .rounded_full()
            .border(px(stroke::THICK))
            .border_color(c.border_focus)
            .bg(c.background_primary);
        match self.axis {
            Axis::Vertical => div()
                .w_full()
                .h(px(6.))
                .flex()
                .items_center()
                .child(ring)
                .child(div().flex_1().h(px(stroke::THICK)).bg(c.border_focus)),
            Axis::Horizontal => div()
                .h_full()
                .w(px(6.))
                .flex()
                .flex_col()
                .items_center()
                .child(ring)
                .child(div().flex_1().w(px(stroke::THICK)).bg(c.border_focus)),
        }
    }
}

/// Docking overlay: put it as the last child of a `relative` pane; it
/// highlights where a drop at `zone` lands (`None` hides it).
#[derive(IntoElement)]
pub struct DropZone {
    zone: Option<DockZone>,
}

impl DropZone {
    /// Overlay for `zone`.
    pub fn new(zone: Option<DockZone>) -> Self {
        Self { zone }
    }
}

impl RenderOnce for DropZone {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let Some(zone) = self.zone else {
            return div();
        };
        let (x, y, w, h) = dock_zone_fraction(zone);
        div().absolute().inset_0().p(px(spacing::XX_SMALL)).child(
            div().relative().size_full().child(
                div()
                    .absolute()
                    .left(relative(x))
                    .top(relative(y))
                    .w(relative(w))
                    .h(relative(h))
                    .rounded(px(radius::LARGE))
                    .bg(c.background_brand_subtle.opacity(0.7))
                    .border(px(stroke::THICK))
                    .border_color(c.border_focus),
            ),
        )
    }
}
