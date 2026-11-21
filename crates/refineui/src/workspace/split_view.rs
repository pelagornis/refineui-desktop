//! Split View, Resize Handle and Pane Tree (brief §6 Composition).
//!
//! * [`ResizeHandle`] — the splitter between two panes. Visually a hairline
//!   (`borderSubtle`); the hit area is the density's `splitter_hit`
//!   (4–8px) centered on it. Hover / drag / keyboard focus thicken it to
//!   2px `borderFocus`. Column / row resize cursor (kept for the whole
//!   drag). One tab stop: ← / → (or ↑ / ↓) move it 8px, ⇧ 40px, Home /
//!   End collapse to the minimum / maximum; double-click resets to equal
//!   sizes. It starts a GPUI drag carrying [`ResizeDrag`]; the owner
//!   (Split View, Dock) turns the pointer position into sizes in
//!   `on_drag_move`, so the drag keeps tracking outside the handle.
//! * [`SplitView`] — children along an axis sized by ratios, with a
//!   handle on every inner boundary. Controlled (`.ratios(..)` +
//!   `.on_resize(..)`, the app stores them — usually in
//!   [`PaneNode::Split`]) or uncontrolled (keyed state). Per-child minimum
//!   in px (`min_size`, default 120); dragging pushes only the two
//!   neighbours ([`resize_at`]).
//! * [`PaneTree`] — renders a whole [`PaneNode`] tree: splits become
//!   Split Views, leaves are drawn by your `render_group` closure
//!   (usually a [`super::TabGroup`]); resizes come back as
//!   `(path, ratios)` for [`super::WorkspaceLayout::set_ratios`]. With
//!   `.zoomed(group)` only that group renders.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, CursorStyle, ElementId, Entity, IntoElement, KeyDownEvent,
    ParentElement, Render, RenderOnce, Styled, Window, div, prelude::*, px, relative,
};
use refineui_foundation::stroke;

use super::layout::{PaneNode, SplitAxis, TabGroupState, equal_ratios, resize_at};
use crate::ActiveTheme;
use crate::foundation::CursorRole;
use crate::motion::{
    LayoutProbe, TrackInteraction, child_id, focus_visible, probe, use_focus_handle,
    use_interaction, use_layout_probe,
};

/// Keyboard step of a [`ResizeHandle`], px.
pub const RESIZE_STEP: f32 = 8.0;
/// ⇧ keyboard step of a [`ResizeHandle`], px.
pub const RESIZE_STEP_LARGE: f32 = 40.0;

/// Drag payload of a [`ResizeHandle`]: which owner and which boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct ResizeDrag {
    /// Owner id (the Split View / Dock that listens for it).
    pub owner: ElementId,
    /// Boundary index (between child `index` and `index + 1`).
    pub index: usize,
}

/// Invisible drag view for resize drags (the pane itself is the preview).
pub struct ResizeDragGhost;

impl Render for ResizeDragGhost {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div().size(px(0.))
    }
}

/// Keep the resize cursor for the whole drag (call from `on_drag_move`).
pub fn hold_resize_cursor(axis: SplitAxis, window: &mut Window, cx: &mut App) {
    let style = resize_cursor(axis);
    if cx.active_drag_cursor_style() != Some(style) {
        cx.set_active_drag_cursor_style(style, window);
    }
}

fn resize_cursor(axis: SplitAxis) -> CursorStyle {
    match axis {
        SplitAxis::Horizontal => CursorRole::ResizeColumn.style(),
        SplitAxis::Vertical => CursorRole::ResizeRow.style(),
    }
}

/// Where a [`ResizeHandle`]'s hit area sits relative to its boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HandleAlign {
    /// Centered on the boundary (between two panes).
    #[default]
    Center,
    /// After the boundary (line at the start) — e.g. a right dock's edge.
    Start,
    /// Before the boundary (line at the end) — e.g. a left dock's edge.
    End,
}

type StepHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;
type ResetHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A splitter. See the module docs.
#[derive(IntoElement)]
pub struct ResizeHandle {
    id: ElementId,
    axis: SplitAxis,
    drag: Option<ResizeDrag>,
    on_step: Option<StepHandler>,
    on_reset: Option<ResetHandler>,
    focusable: bool,
    align: HandleAlign,
    quiet: bool,
}

impl ResizeHandle {
    /// A handle between panes laid out along `axis` (`Horizontal` = side
    /// by side → a vertical line with a column-resize cursor).
    pub fn new(id: impl Into<ElementId>, axis: SplitAxis) -> Self {
        Self {
            id: id.into(),
            axis,
            drag: None,
            on_step: None,
            on_reset: None,
            focusable: true,
            align: HandleAlign::Center,
            quiet: false,
        }
    }

    /// Start a drag with this payload (the owner listens with
    /// `on_drag_move::<ResizeDrag>`).
    pub fn drag(mut self, drag: ResizeDrag) -> Self {
        self.drag = Some(drag);
        self
    }

    /// Keyboard nudge, px (negative = toward the start; `±INFINITY` for
    /// Home / End).
    pub fn on_step(mut self, f: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(f));
        self
    }

    /// Double-click.
    pub fn on_reset(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_reset = Some(Rc::new(f));
        self
    }

    /// Hit area placement (default centered).
    pub fn align(mut self, align: HandleAlign) -> Self {
        self.align = align;
        self
    }

    /// No hairline at rest (the container already draws a border); the
    /// 2px focus line still shows on hover / drag / focus.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Not a tab stop (default: one tab stop).
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }
}

impl RenderOnce for ResizeHandle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let c = theme.colors;
        let hit = theme.desktop().splitter_hit.max(4.);
        let (state, i) = use_interaction(&self.id, window, cx);
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = self.focusable && focus.is_focused(window) && focus_visible(cx);
        let active = i.hovered || i.pressed || focused;
        let line_w = if active {
            stroke::THICK
        } else {
            stroke::HAIRLINE
        };
        let line = if active {
            c.border_focus
        } else if self.quiet {
            gpui::transparent_black()
        } else {
            c.border_subtle
        };
        let align = self.align;
        let horizontal = self.axis == SplitAxis::Horizontal;
        let axis = self.axis;
        let on_step = self.on_step.clone();
        let on_reset = self.on_reset.clone();

        let mut el = div()
            .id(self.id.clone())
            .absolute()
            .flex()
            .items_center()
            .map(|d| match align {
                HandleAlign::Center => d.justify_center(),
                HandleAlign::Start => d.justify_start(),
                HandleAlign::End => d.justify_end(),
            })
            .when(!horizontal, |d| d.flex_col())
            .cursor(resize_cursor(axis))
            .track_interaction(&state)
            .child(
                div()
                    .flex_none()
                    .bg(line)
                    .when(horizontal, |d| d.h_full().w(px(line_w)))
                    .when(!horizontal, |d| d.w_full().h(px(line_w))),
            );
        let shift = match align {
            HandleAlign::Center => -hit / 2.,
            HandleAlign::Start => 0.,
            HandleAlign::End => -hit,
        };
        el = if horizontal {
            el.top_0().bottom_0().w(px(hit)).ml(px(shift))
        } else {
            el.left_0().right_0().h(px(hit)).mt(px(shift))
        };
        if self.focusable {
            el = el
                .track_focus(&focus)
                .on_key_down(move |e: &KeyDownEvent, w, cx| {
                    let big = e.keystroke.modifiers.shift;
                    let step = if big { RESIZE_STEP_LARGE } else { RESIZE_STEP };
                    let (back, fwd) = if horizontal {
                        ("left", "right")
                    } else {
                        ("up", "down")
                    };
                    let k = e.keystroke.key.as_str();
                    let delta = if k == back {
                        -step
                    } else if k == fwd {
                        step
                    } else if k == "home" {
                        f32::NEG_INFINITY
                    } else if k == "end" {
                        f32::INFINITY
                    } else {
                        return;
                    };
                    cx.stop_propagation();
                    if let Some(f) = &on_step {
                        f(delta, w, cx);
                    }
                });
        }
        if let Some(drag) = self.drag {
            el = el.on_drag(drag, |_, _, _, cx| cx.new(|_| ResizeDragGhost));
        }
        el.on_click(move |e: &ClickEvent, w, cx| {
            if e.click_count() >= 2
                && let Some(f) = &on_reset
            {
                f(w, cx);
            }
        })
    }
}

type ResizeHandler = Rc<dyn Fn(&[f32], &mut Window, &mut App)>;

/// Children along an axis with handles between them. See the module docs.
#[derive(IntoElement)]
pub struct SplitView {
    id: ElementId,
    axis: SplitAxis,
    children: Vec<AnyElement>,
    ratios: Option<Vec<f32>>,
    min_size: Vec<f32>,
    on_resize: Option<ResizeHandler>,
}

impl SplitView {
    /// An empty split.
    pub fn new(id: impl Into<ElementId>, axis: SplitAxis) -> Self {
        Self {
            id: id.into(),
            axis,
            children: Vec::new(),
            ratios: None,
            min_size: Vec::new(),
            on_resize: None,
        }
    }

    /// Side-by-side split.
    pub fn horizontal(id: impl Into<ElementId>) -> Self {
        Self::new(id, SplitAxis::Horizontal)
    }

    /// Stacked split.
    pub fn vertical(id: impl Into<ElementId>) -> Self {
        Self::new(id, SplitAxis::Vertical)
    }

    /// Add a pane.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Add panes.
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }

    /// Controlled sizes (fractions; normalized if they don't fit).
    pub fn ratios(mut self, ratios: Vec<f32>) -> Self {
        self.ratios = Some(ratios);
        self
    }

    /// Minimum px per pane (one value for all, or one per pane).
    pub fn min_size(mut self, min: impl Into<Vec<f32>>) -> Self {
        self.min_size = min.into();
        self
    }

    /// New ratios after a drag, keyboard step or reset.
    pub fn on_resize(mut self, f: impl Fn(&[f32], &mut Window, &mut App) + 'static) -> Self {
        self.on_resize = Some(Rc::new(f));
        self
    }
}

/// Make `ratios` fit `n` panes (equal when they don't).
pub fn fit_ratios(ratios: Option<&[f32]>, n: usize) -> Vec<f32> {
    match ratios {
        Some(r) if r.len() == n && n > 0 && r.iter().all(|x| x.is_finite() && *x >= 0.) => {
            let sum: f32 = r.iter().sum();
            if sum <= f32::EPSILON {
                equal_ratios(n)
            } else {
                r.iter().map(|x| x / sum).collect()
            }
        }
        _ => equal_ratios(n),
    }
}

/// Move boundary `ix` by `delta_px` of `extent_px` (keyboard nudges;
/// `±INFINITY` clamps to the minimum / maximum).
pub fn nudge_ratios(
    ratios: &[f32],
    ix: usize,
    delta_px: f32,
    extent_px: f32,
    mins: &[f32],
) -> Vec<f32> {
    if ix + 1 >= ratios.len() || extent_px <= 0. {
        return ratios.to_vec();
    }
    let boundary: f32 = ratios[..=ix].iter().sum::<f32>() + delta_px / extent_px;
    let boundary = if boundary.is_finite() {
        boundary
    } else if delta_px > 0. {
        1.0
    } else {
        0.0
    };
    resize_at(ratios, ix, boundary, mins)
}

fn mins_frac(min_size: &[f32], n: usize, extent: f32) -> Vec<f32> {
    let extent = extent.max(1.);
    (0..n)
        .map(|i| {
            let px = match min_size.len() {
                0 => 120.,
                1 => min_size[0],
                _ => min_size.get(i).copied().unwrap_or(0.),
            };
            // Never let the minimums exceed the space.
            (px / extent).min(1. / n as f32)
        })
        .collect()
}

fn extent_of(p: &LayoutProbe, axis: SplitAxis) -> f32 {
    p.container
        .map(|b| match axis {
            SplitAxis::Horizontal => f32::from(b.size.width),
            SplitAxis::Vertical => f32::from(b.size.height),
        })
        .unwrap_or(0.)
}

impl RenderOnce for SplitView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let n = self.children.len();
        let own: Entity<Option<Vec<f32>>> =
            window.use_keyed_state(child_id(&self.id, "ratios"), cx, |_, _| None);
        let ratios = match &self.ratios {
            Some(r) => fit_ratios(Some(r), n),
            None => fit_ratios(own.read(cx).as_deref(), n),
        };
        let measure = use_layout_probe(&self.id, window, cx);
        let axis = self.axis;
        let horizontal = axis == SplitAxis::Horizontal;
        let min_size = Rc::new(self.min_size);
        let apply: ResizeHandler = {
            let user = self.on_resize.clone();
            let own = own.clone();
            Rc::new(move |r: &[f32], w: &mut Window, cx: &mut App| {
                own.update(cx, |s, cx| {
                    *s = Some(r.to_vec());
                    cx.notify();
                });
                if let Some(f) = &user {
                    f(r, w, cx);
                }
            })
        };

        let mut root = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .flex()
            .when(!horizontal, |d| d.flex_col())
            .child(probe(&measure, None));

        // Drag tracking (capture phase: keeps working outside the handle).
        {
            let id = self.id.clone();
            let ratios = ratios.clone();
            let min_size = min_size.clone();
            let apply = apply.clone();
            root = root.on_drag_move::<ResizeDrag>(move |e, w, cx| {
                let d = e.drag(cx);
                if d.owner != id {
                    return;
                }
                let ix = d.index;
                hold_resize_cursor(axis, w, cx);
                let b = e.bounds;
                let (pos, extent) = if horizontal {
                    (e.event.position.x - b.left(), b.size.width)
                } else {
                    (e.event.position.y - b.top(), b.size.height)
                };
                let extent = f32::from(extent);
                if extent <= 0. {
                    return;
                }
                let mins = mins_frac(&min_size, ratios.len(), extent);
                let next = resize_at(&ratios, ix, f32::from(pos) / extent, &mins);
                if next != ratios {
                    apply(&next, w, cx);
                }
            });
        }

        let mut offset = 0.;
        let count = n;
        for (i, (child, r)) in self.children.into_iter().zip(ratios.iter()).enumerate() {
            let pane = div()
                .flex_none()
                .relative()
                .overflow_hidden()
                .when(horizontal, |d| d.h_full().w(relative(*r)))
                .when(!horizontal, |d| d.w_full().h(relative(*r)))
                .child(child);
            root = root.child(pane);
            offset += r;
            if i + 1 < count {
                let step_ratios = ratios.clone();
                let measure = measure.clone();
                let min_size = min_size.clone();
                let apply_step = apply.clone();
                let apply_reset = apply.clone();
                let handle = ResizeHandle::new(child_id(&self.id, HANDLE_IDS[i.min(15)]), axis)
                    .drag(ResizeDrag {
                        owner: self.id.clone(),
                        index: i,
                    })
                    .on_step(move |delta, w, cx| {
                        let extent = extent_of(measure.read(cx), axis);
                        let mins = mins_frac(&min_size, step_ratios.len(), extent);
                        let next = nudge_ratios(&step_ratios, i, delta, extent, &mins);
                        apply_step(&next, w, cx);
                    })
                    .on_reset(move |w, cx| apply_reset(&equal_ratios(count), w, cx));
                let at = relative(offset);
                root = root.child(
                    div()
                        .absolute()
                        .when(horizontal, |d| d.top_0().bottom_0().left(at).w(px(0.)))
                        .when(!horizontal, |d| d.left_0().right_0().top(at).h(px(0.)))
                        .child(handle),
                );
            }
        }
        root
    }
}

const HANDLE_IDS: [&str; 16] = [
    "handle-0",
    "handle-1",
    "handle-2",
    "handle-3",
    "handle-4",
    "handle-5",
    "handle-6",
    "handle-7",
    "handle-8",
    "handle-9",
    "handle-10",
    "handle-11",
    "handle-12",
    "handle-13",
    "handle-14",
    "handle-15",
];

type GroupRenderer = Rc<dyn Fn(&TabGroupState, &mut Window, &mut App) -> AnyElement>;
type PathResize = Rc<dyn Fn(&[usize], &[f32], &mut Window, &mut App)>;

/// Renders a [`PaneNode`] tree. See the module docs.
#[derive(IntoElement)]
pub struct PaneTree {
    id: ElementId,
    node: PaneNode,
    render_group: GroupRenderer,
    on_resize: Option<PathResize>,
    zoomed: Option<String>,
    min_size: f32,
}

impl PaneTree {
    /// Render `node`, drawing each tab group with `render_group`.
    pub fn new(
        id: impl Into<ElementId>,
        node: &PaneNode,
        render_group: impl Fn(&TabGroupState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            node: node.clone(),
            render_group: Rc::new(render_group),
            on_resize: None,
            zoomed: None,
            min_size: 160.,
        }
    }

    /// A split's ratios changed: `(path to the split, ratios)`.
    pub fn on_resize(
        mut self,
        f: impl Fn(&[usize], &[f32], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Rc::new(f));
        self
    }

    /// Show only this group (zoom).
    pub fn zoomed(mut self, group: Option<String>) -> Self {
        self.zoomed = group;
        self
    }

    /// Minimum pane size, px (default 160).
    pub fn min_size(mut self, px: f32) -> Self {
        self.min_size = px;
        self
    }
}

fn find_group<'a>(node: &'a PaneNode, id: &str) -> Option<&'a TabGroupState> {
    match node {
        PaneNode::Tabs(g) => (g.id == id).then_some(g),
        PaneNode::Split { children, .. } => children.iter().find_map(|c| find_group(c, id)),
    }
}

fn split_id(root: &ElementId, path: &[usize]) -> ElementId {
    let key = path
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(".");
    ElementId::Name(format!("{root}/split:{key}").into())
}

impl PaneTree {
    fn node_el(
        &self,
        node: &PaneNode,
        path: &mut Vec<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        match node {
            PaneNode::Tabs(g) => (self.render_group)(g, window, cx),
            PaneNode::Split {
                axis,
                children,
                ratios,
            } => {
                let mut split = SplitView::new(split_id(&self.id, path), *axis)
                    .ratios(ratios.clone())
                    .min_size(vec![self.min_size]);
                if let Some(f) = self.on_resize.clone() {
                    let at = path.clone();
                    split = split.on_resize(move |r, w, cx| f(&at, r, w, cx));
                }
                for (i, c) in children.iter().enumerate() {
                    path.push(i);
                    let el = self.node_el(c, path, window, cx);
                    path.pop();
                    split = split.child(el);
                }
                split.into_any_element()
            }
        }
    }
}

impl RenderOnce for PaneTree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zoomed = self
            .zoomed
            .as_deref()
            .and_then(|z| find_group(&self.node, z))
            .cloned();
        let body = match zoomed {
            Some(g) => (self.render_group)(&g, window, cx),
            None => {
                let node = self.node.clone();
                self.node_el(&node, &mut Vec::new(), window, cx)
            }
        };
        div().id(self.id.clone()).size_full().child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_fit_and_nudge() {
        assert_eq!(fit_ratios(None, 2), vec![0.5, 0.5]);
        assert_eq!(fit_ratios(Some(&[1.0, 3.0]), 2), vec![0.25, 0.75]);
        assert_eq!(fit_ratios(Some(&[0.5]), 2), vec![0.5, 0.5]);
        assert_eq!(fit_ratios(Some(&[f32::NAN, 1.0]), 2), vec![0.5, 0.5]);
        // 1000px wide, 8px step → 0.8%.
        let r = nudge_ratios(&[0.5, 0.5], 0, 8.0, 1000.0, &[0.1, 0.1]);
        assert!((r[0] - 0.508).abs() < 1e-5 && (r[1] - 0.492).abs() < 1e-5);
        // Home / End clamp to the minimums.
        let r = nudge_ratios(&[0.5, 0.5], 0, f32::NEG_INFINITY, 1000.0, &[0.1, 0.1]);
        assert!((r[0] - 0.1).abs() < 1e-5);
        let r = nudge_ratios(&[0.5, 0.5], 0, f32::INFINITY, 1000.0, &[0.1, 0.1]);
        assert!((r[1] - 0.1).abs() < 1e-5);
        // Out-of-range boundary is a no-op.
        assert_eq!(nudge_ratios(&[1.0], 0, 8.0, 100.0, &[0.1]), vec![1.0]);
        // Minimums never exceed an equal share.
        let m = mins_frac(&[500.0], 3, 900.0);
        assert!(m.iter().all(|x| (*x - 1.0 / 3.0).abs() < 1e-6));
        assert_eq!(mins_frac(&[], 2, 1200.0), vec![0.1, 0.1]);
    }

    #[test]
    fn split_ids_are_unique_per_path() {
        let root: ElementId = "tree".into();
        assert_ne!(split_id(&root, &[0]), split_id(&root, &[0, 0]));
        assert_ne!(split_id(&root, &[1]), split_id(&root, &[]));
    }
}
