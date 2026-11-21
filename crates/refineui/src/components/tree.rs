//! Tree — web `Tree` (nested, expandable rows; click to select).

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Transformation, Window, div, prelude::*, px,
    radians,
};
use refineui_foundation::{radius, spacing, typography::role};

use crate::components::disclosure::collapse_panel;
use crate::components::plate::row_plate;
use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, StyledExt};

/// A tree node.
#[derive(Clone, Debug)]
pub struct TreeNode {
    /// Stable id (used for expansion / selection).
    pub id: SharedString,
    /// Label.
    pub label: SharedString,
    /// Icon (defaults to folder / document).
    pub icon: Option<IconName>,
    /// Children.
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Leaf node.
    pub fn leaf(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            children: Vec::new(),
        }
    }

    /// Branch node.
    pub fn branch(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        children: Vec<TreeNode>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            children,
        }
    }

    /// Custom icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

#[derive(Default)]
struct TreeState {
    expanded: BTreeSet<SharedString>,
    selected: Option<SharedString>,
}

/// Expandable hierarchy. Uncontrolled: remembers expansion and selection.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    size: ControlSize,
    nodes: Vec<TreeNode>,
    default_expanded: Vec<SharedString>,
    on_select: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App)>>,
}

impl Tree {
    /// New tree.
    pub fn new(id: impl Into<ElementId>, nodes: Vec<TreeNode>) -> Self {
        Self {
            id: id.into(),
            size: ControlSize::Medium,
            nodes,
            default_expanded: Vec::new(),
            on_select: None,
        }
    }

    /// Node ids expanded initially.
    pub fn default_expanded(
        mut self,
        ids: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.default_expanded = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Selection callback (node id).
    /// Row size (`sm | md | lg` → `bodySm / bodyMd / bodyLg`).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Called with the selected node id.
    pub fn on_select(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

type SelectHandler = Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App)>>;

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let defaults = self.default_expanded.clone();
        let state =
            window.use_keyed_state(child_id(&self.id, "state"), cx, move |_, _| TreeState {
                expanded: defaults.into_iter().collect(),
                selected: None,
            });
        let text = match self.size {
            ControlSize::Small => role::BODY_SM,
            ControlSize::Medium => role::BODY_MD,
            ControlSize::Large => role::BODY_LG,
        };
        let icon = if self.size == ControlSize::Small {
            12.
        } else {
            16.
        };
        let host = crate::motion::use_layout_probe(&child_id(&self.id, "highlight"), window, cx);
        let ctx = Ctx {
            state,
            parent: self.id.clone(),
            on_select: self.on_select.clone(),
            icon,
            host: host.clone(),
            next_row: Rc::default(),
            plate_target: Rc::default(),
        };
        let rows: Vec<AnyElement> = self
            .nodes
            .iter()
            .map(|n| render_node(n, 0, &ctx, window, cx))
            .collect();
        let plate = crate::components::sliding_highlight::sliding_plate(
            &child_id(&self.id, "highlight"),
            &host,
            ctx.plate_target.get(),
            cx.theme().colors.background_primary_hover,
            radius::LARGE,
            window,
            cx,
        );
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .gap(px(1.))
            .w_full()
            .text_style_token(text)
            .child(crate::motion::probe(&host, None))
            .child(plate)
            .children(rows)
    }
}

struct Ctx {
    state: gpui::Entity<TreeState>,
    parent: ElementId,
    on_select: SelectHandler,
    icon: f32,
    host: gpui::Entity<crate::motion::LayoutProbe>,
    next_row: Rc<std::cell::Cell<usize>>,
    plate_target: Rc<std::cell::Cell<Option<usize>>>,
}

/// One node: the row plate (tertiary text; hover / pressed / selected per the
/// shared list plate, selected medium), a 16px chevron slot whose chevron
/// rotates 90° over `duration-medium` (emphasized), a 22px indent per depth,
/// and the children in an animated collapse panel.
fn render_node(
    node: &TreeNode,
    depth: usize,
    ctx: &Ctx,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let (expanded, selected) = {
        let s = ctx.state.read(cx);
        (
            s.expanded.contains(&node.id),
            s.selected.as_ref() == Some(&node.id),
        )
    };
    let is_branch = !node.children.is_empty();
    let row_id = ElementId::NamedChild(Box::new(ctx.parent.clone()), node.id.clone());
    let (istate, mut i) = use_interaction(&row_id, window, cx);
    let row_ix = ctx.next_row.get();
    ctx.next_row.set(row_ix + 1);
    if i.hovered {
        ctx.plate_target.set(Some(row_ix));
    }
    // The sliding plate carries hover (web `[data-sliding-highlight]`).
    if !i.pressed {
        i.hovered = false;
    }
    let focus = use_focus_handle(&row_id, 0, window, cx);
    let focused = focus_visible(cx) && focus.is_focused(window);
    let (bg, fg) = row_plate(i, selected, false, c.foreground_tertiary, cx);
    let bg = animate(child_id(&row_id, "bg"), bg, Motion::FAST, window, cx);
    let fg = animate(child_id(&row_id, "fg"), fg, Motion::FAST, window, cx);
    let rot = animate(
        child_id(&row_id, "chev"),
        if expanded { 1.0f32 } else { 0.0 },
        Motion::CHEVRON,
        window,
        cx,
    );
    let st = ctx.state.clone();
    let id = node.id.clone();
    let handler = ctx.on_select.clone();
    let activate = Rc::new(move |w: &mut Window, cx: &mut App| {
        st.update(cx, |s, cx| {
            if is_branch && !s.expanded.remove(&id) {
                s.expanded.insert(id.clone());
            }
            s.selected = Some(id.clone());
            cx.notify();
        });
        if let Some(h) = &handler {
            h(&id, w, cx);
        }
    });
    let click = activate.clone();
    let key = activate.clone();
    let icon_px = ctx.icon;
    let indent = (spacing::LARGE + spacing::X_SMALL) * depth as f32;
    let row = div()
        .id(row_id.clone())
        .relative()
        .child(crate::motion::probe(&ctx.host, Some(row_ix)))
        .flex()
        .w_full()
        .min_w_0()
        .items_center()
        .px(px(spacing::MEDIUM))
        .py(px(spacing::X_SMALL + theme.row_inset()))
        .rounded(px(radius::LARGE))
        .bg(bg)
        .text_color(fg)
        .when(selected, |d| d.font_weight(gpui::FontWeight::MEDIUM))
        .when(focused, |d| d.shadow(theme.focus_ring()))
        .cursor_action()
        .track_focus(&focus)
        .track_interaction(&istate)
        .on_click(move |_, w, cx| click(w, cx))
        .on_key_down(move |e: &gpui::KeyDownEvent, w, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                cx.stop_propagation();
                key(w, cx)
            }
        })
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(px(spacing::X_SMALL))
                .pl(px(indent))
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(px(16.))
                        .when(is_branch, |d| {
                            d.child(
                                Icon::new(IconName::ChevronRight)
                                    .size(IconSize::Custom(px(icon_px)))
                                    .color(fg)
                                    .transform(Transformation::rotate(radians(
                                        rot * std::f32::consts::FRAC_PI_2,
                                    ))),
                            )
                        }),
                )
                .when_some(node.icon, |d, ic| {
                    d.child(Icon::new(ic).size(IconSize::Custom(px(icon_px))).color(fg))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(node.label.clone()),
                ),
        );
    if !is_branch {
        return row.into_any_element();
    }
    let children = node.children.clone();
    let panel_ctx = Ctx {
        state: ctx.state.clone(),
        parent: ctx.parent.clone(),
        on_select: ctx.on_select.clone(),
        icon: ctx.icon,
        host: ctx.host.clone(),
        next_row: ctx.next_row.clone(),
        plate_target: ctx.plate_target.clone(),
    };
    let panel = collapse_panel(
        &child_id(&row_id, "panel"),
        expanded,
        window,
        cx,
        move |w, cx| {
            div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .pt(px(1.))
                .children(
                    children
                        .iter()
                        .map(|n| render_node(n, depth + 1, &panel_ctx, w, cx))
                        .collect::<Vec<_>>(),
                )
                .into_any_element()
        },
    );
    div()
        .flex()
        .flex_col()
        .w_full()
        .child(row)
        .children(panel)
        .into_any_element()
}
