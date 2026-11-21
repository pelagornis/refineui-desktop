//! Dock and Dock Area (brief §6 Composition).
//!
//! * [`Dock`] — one window edge (left, right, bottom) holding a
//!   [`PanelGroup`]. Its inner edge is a quiet [`ResizeHandle`] (the
//!   container's hairline border is the rest state); sizes are clamped to
//!   `min`…`max` and reported through `on_resize`. The group header gets
//!   Maximize / Restore and Hide buttons. Dropping a [`PanelDrag`] on the
//!   dock appends it (the strip itself reports precise slots); while a
//!   panel drag hovers, the dock tints `backgroundBrandSubtle`.
//! * [`DockArea`] — center + docks. While a panel is being dragged, edge
//!   drop zones appear along the center's left, right and bottom edges so
//!   a panel can go to a hidden dock. `.zoomed(side)` fills the area with
//!   that dock. Inside an [`crate::ApplicationShell`] pass it to
//!   `.workspace(area)` so the docks become F6 regions.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use super::is_dragging;
use super::layout::{DockSide, SplitAxis};
use super::panel::{PanelDrag, PanelGroup};
use super::split_view::{HandleAlign, ResizeDrag, ResizeHandle, hold_resize_cursor};
use crate::motion::child_id;
use crate::{ActiveTheme, Button, ButtonVariant, ControlSize, IconName, StyledExt};

type SizeHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;
type Action = Rc<dyn Fn(&mut Window, &mut App)>;
type PanelDropHandler = Rc<dyn Fn(&PanelDrag, &mut Window, &mut App)>;

/// Clamp a dock size (`min` wins when the limits cross).
pub fn clamp_dock_size(size: f32, min: f32, max: f32) -> f32 {
    if !size.is_finite() {
        return if size > 0. { max } else { min };
    }
    size.max(min).min(max.max(min))
}

/// One dock. See the module docs.
#[derive(IntoElement)]
pub struct Dock {
    id: ElementId,
    side: DockSide,
    size: f32,
    min: f32,
    max: f32,
    default_size: Option<f32>,
    group: Option<PanelGroup>,
    content: Option<AnyElement>,
    pub(crate) zoomed: bool,
    on_resize: Option<SizeHandler>,
    on_zoom: Option<Action>,
    on_hide: Option<Action>,
    on_drop: Option<PanelDropHandler>,
}

impl Dock {
    /// Dock on `side`.
    pub fn new(id: impl Into<ElementId>, side: DockSide) -> Self {
        Self {
            id: id.into(),
            side,
            size: match side {
                DockSide::Bottom => 240.,
                _ => 256.,
            },
            min: 120.,
            max: 640.,
            default_size: None,
            group: None,
            content: None,
            zoomed: false,
            on_resize: None,
            on_zoom: None,
            on_hide: None,
            on_drop: None,
        }
    }

    /// Width (side docks) or height (bottom), px.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Size limits, px (default 120…640).
    pub fn limits(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Size restored by double-clicking the edge.
    pub fn default_size(mut self, size: f32) -> Self {
        self.default_size = Some(size);
        self
    }

    /// The dock's panels (gets the zoom / hide buttons).
    pub fn group(mut self, group: PanelGroup) -> Self {
        self.group = Some(group.side(self.side));
        self
    }

    /// Custom content instead of a [`PanelGroup`].
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content = Some(e.into_any_element());
        self
    }

    /// Filling the workspace (shows Restore instead of Maximize).
    pub fn zoomed(mut self, zoomed: bool) -> Self {
        self.zoomed = zoomed;
        self
    }

    /// New size from the edge handle.
    pub fn on_resize(mut self, f: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_resize = Some(Rc::new(f));
        self
    }

    /// Maximize / Restore pressed.
    pub fn on_zoom(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_zoom = Some(Rc::new(f));
        self
    }

    /// Hide pressed.
    pub fn on_hide(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_hide = Some(Rc::new(f));
        self
    }

    /// A panel dropped on the dock outside its strip (append).
    pub fn on_drop(mut self, f: impl Fn(&PanelDrag, &mut Window, &mut App) + 'static) -> Self {
        self.on_drop = Some(Rc::new(f));
        self
    }

    /// The side.
    pub fn side(&self) -> DockSide {
        self.side
    }

    /// Current size.
    pub fn current_size(&self) -> f32 {
        self.size
    }
}

impl RenderOnce for Dock {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let side = self.side;
        let (min, max, size) = (self.min, self.max, self.size);
        let tint = c.background_brand_subtle;
        let dragging_panel = is_dragging::<PanelDrag>(cx);

        let mut group = self.group;
        if let Some(g) = group.take() {
            let zoom_label = if self.zoomed {
                "Restore panel size"
            } else {
                "Maximize panel"
            };
            let mut g = g;
            if let Some(f) = self.on_zoom.clone() {
                g = g.action(
                    Button::icon_only(child_id(&self.id, "zoom"), IconName::FullScreenMaximize)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Small)
                        .selected(self.zoomed)
                        .tooltip(zoom_label)
                        .on_click(move |_, w, cx| f(w, cx)),
                );
            }
            if let Some(f) = self.on_hide.clone() {
                g = g.action(
                    Button::icon_only(child_id(&self.id, "hide"), IconName::Dismiss)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Small)
                        .tooltip("Hide panel")
                        .on_click(move |_, w, cx| f(w, cx)),
                );
            }
            group = Some(g);
        }

        let mut root = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .children(group)
            .children(self.content);

        if let Some(f) = self.on_drop.clone() {
            root = root
                .on_drop::<PanelDrag>(move |d, w, cx| f(d, w, cx))
                .when(dragging_panel, |d| {
                    d.drag_over::<PanelDrag>(move |s, _, _, _| s.bg(tint.opacity(0.6)))
                });
        }

        if let (Some(on_resize), false) = (self.on_resize.clone(), self.zoomed) {
            let axis = if side == DockSide::Bottom {
                SplitAxis::Vertical
            } else {
                SplitAxis::Horizontal
            };
            let id = self.id.clone();
            let drag_resize = on_resize.clone();
            root = root.on_drag_move::<ResizeDrag>(move |e, w, cx| {
                if e.drag(cx).owner != id {
                    return;
                }
                hold_resize_cursor(axis, w, cx);
                let b = e.bounds;
                let p = e.event.position;
                let next = f32::from(match side {
                    DockSide::Left => p.x - b.left(),
                    DockSide::Right => b.right() - p.x,
                    DockSide::Bottom => b.bottom() - p.y,
                });
                let next = clamp_dock_size(next, min, max);
                if (next - size).abs() >= 0.5 {
                    drag_resize(next, w, cx);
                }
            });
            let step_resize = on_resize.clone();
            let reset_resize = on_resize.clone();
            let default_size = self.default_size;
            let handle = ResizeHandle::new(child_id(&self.id, "edge"), axis)
                .quiet(true)
                .align(match side {
                    DockSide::Left => HandleAlign::End,
                    _ => HandleAlign::Start,
                })
                .drag(ResizeDrag {
                    owner: self.id.clone(),
                    index: 0,
                })
                .on_step(move |delta, w, cx| {
                    // The handle moves by `delta`; left grows with it.
                    let grow = if side == DockSide::Left {
                        delta
                    } else {
                        -delta
                    };
                    step_resize(clamp_dock_size(size + grow, min, max), w, cx);
                })
                .on_reset(move |w, cx| {
                    if let Some(d) = default_size {
                        reset_resize(clamp_dock_size(d, min, max), w, cx);
                    }
                });
            let edge = div().absolute().child(handle);
            let edge = match side {
                DockSide::Left => edge.top_0().bottom_0().right_0().w(px(0.)),
                DockSide::Right => edge.top_0().bottom_0().left_0().w(px(0.)),
                DockSide::Bottom => edge.left_0().right_0().top_0().h(px(0.)),
            };
            root = root.child(edge);
        }
        root
    }
}

type EdgeDrop = Rc<dyn Fn(&PanelDrag, DockSide, &mut Window, &mut App)>;

/// Center + docks. See the module docs.
#[derive(IntoElement)]
pub struct DockArea {
    id: ElementId,
    center: Option<AnyElement>,
    left: Option<Dock>,
    right: Option<Dock>,
    bottom: Option<Dock>,
    zoomed: Option<DockSide>,
    on_edge_drop: Option<EdgeDrop>,
}

/// A [`DockArea`] split into shell regions.
pub(crate) struct DockParts {
    pub left: Option<(AnyElement, f32)>,
    pub right: Option<(AnyElement, f32)>,
    pub bottom: Option<(AnyElement, f32)>,
    pub center: AnyElement,
}

impl DockArea {
    /// Empty area.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            center: None,
            left: None,
            right: None,
            bottom: None,
            zoomed: None,
            on_edge_drop: None,
        }
    }

    /// The editor area.
    pub fn center(mut self, e: impl IntoElement) -> Self {
        self.center = Some(e.into_any_element());
        self
    }

    /// A visible dock (its side decides where it goes).
    pub fn dock(mut self, dock: Dock) -> Self {
        match dock.side {
            DockSide::Left => self.left = Some(dock),
            DockSide::Right => self.right = Some(dock),
            DockSide::Bottom => self.bottom = Some(dock),
        }
        self
    }

    /// Fill the area with the dock on this side.
    pub fn zoomed(mut self, side: Option<DockSide>) -> Self {
        self.zoomed = side;
        self
    }

    /// A panel was dropped on an edge zone.
    pub fn on_edge_drop(
        mut self,
        f: impl Fn(&PanelDrag, DockSide, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_edge_drop = Some(Rc::new(f));
        self
    }

    pub(crate) fn into_parts(mut self, cx: &App) -> DockParts {
        let center_el = self.center.take();
        let edge = self.on_edge_drop.clone();
        let show_edges = edge.is_some() && is_dragging::<PanelDrag>(cx);
        if let Some(side) = self.zoomed {
            let dock = match side {
                DockSide::Left => self.left.take(),
                DockSide::Right => self.right.take(),
                DockSide::Bottom => self.bottom.take(),
            };
            if let Some(mut d) = dock {
                d.zoomed = true;
                return DockParts {
                    left: None,
                    right: None,
                    bottom: None,
                    center: div().size_full().child(d).into_any_element(),
                };
            }
        }
        let mut center = div()
            .id(child_id(&self.id, "center"))
            .relative()
            .size_full()
            .children(center_el);
        if let (true, Some(f)) = (show_edges, edge) {
            let c = cx.theme().colors;
            for side in DockSide::ALL {
                let f = f.clone();
                let zone = div()
                    .id(ElementId::NamedChild(
                        Box::new(self.id.clone()),
                        format!("edge:{side:?}").into(),
                    ))
                    .absolute()
                    .p(px(spacing::XX_SMALL))
                    .on_drop::<PanelDrag>(move |d, w, cx| f(d, side, w, cx))
                    .child(
                        div()
                            .size_full()
                            .rounded(px(radius::LARGE))
                            .border(px(stroke::THIN))
                            .border_dashed()
                            .border_color(c.border_focus.opacity(0.6))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_style_token(role::CAPTION_MD)
                            .text_color(c.foreground_brand)
                            .child(match side {
                                DockSide::Left => "Left dock",
                                DockSide::Right => "Right dock",
                                DockSide::Bottom => "Bottom dock",
                            }),
                    )
                    .drag_over::<PanelDrag>(move |s, _, _, _| s.bg(c.background_brand_subtle));
                let zone = match side {
                    DockSide::Left => zone.left_0().top_0().bottom(px(EDGE_ZONE)).w(px(EDGE_ZONE)),
                    DockSide::Right => zone
                        .right_0()
                        .top_0()
                        .bottom(px(EDGE_ZONE))
                        .w(px(EDGE_ZONE)),
                    DockSide::Bottom => zone.left_0().right_0().bottom_0().h(px(EDGE_ZONE)),
                };
                center = center.child(zone);
            }
        }
        let part = |d: Option<Dock>| d.map(|d| (d.size, d.into_any_element()));
        DockParts {
            left: part(self.left).map(|(s, e)| (e, s)),
            right: part(self.right).map(|(s, e)| (e, s)),
            bottom: part(self.bottom).map(|(s, e)| (e, s)),
            center: center.into_any_element(),
        }
    }
}

/// Thickness of a [`DockArea`] edge drop zone, px.
pub const EDGE_ZONE: f32 = 56.;

impl RenderOnce for DockArea {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let hair = px(stroke::HAIRLINE);
        let id = self.id.clone();
        let parts = self.into_parts(cx);
        div()
            .id(id)
            .size_full()
            .flex()
            .bg(c.background_primary)
            .children(parts.left.map(|(e, s)| {
                div()
                    .flex_none()
                    .w(px(s))
                    .h_full()
                    .overflow_hidden()
                    .map(|d| {
                        let (fill, layer) = crate::material::chrome(
                            crate::SurfaceKind::Sidebar,
                            c.surface_sunken,
                            cx,
                        );
                        d.relative().bg(fill).children(layer)
                    })
                    .border_r(hair)
                    .border_color(c.border_subtle)
                    .child(e)
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(div().flex_1().min_h_0().child(parts.center))
                    .children(parts.bottom.map(|(e, s)| {
                        div()
                            .flex_none()
                            .h(px(s))
                            .w_full()
                            .overflow_hidden()
                            .border_t(hair)
                            .border_color(c.border_subtle)
                            .child(e)
                    })),
            )
            .children(parts.right.map(|(e, s)| {
                div()
                    .flex_none()
                    .w(px(s))
                    .h_full()
                    .overflow_hidden()
                    .map(|d| {
                        let (fill, layer) = crate::material::chrome(
                            crate::SurfaceKind::Sidebar,
                            c.surface_sunken,
                            cx,
                        );
                        d.relative().bg(fill).children(layer)
                    })
                    .border_l(hair)
                    .border_color(c.border_subtle)
                    .child(e)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_sizes_clamp() {
        assert_eq!(clamp_dock_size(50., 120., 640.), 120.);
        assert_eq!(clamp_dock_size(900., 120., 640.), 640.);
        assert_eq!(clamp_dock_size(300., 120., 640.), 300.);
        assert_eq!(clamp_dock_size(f32::INFINITY, 120., 640.), 640.);
        assert_eq!(clamp_dock_size(f32::NEG_INFINITY, 120., 640.), 120.);
        // Crossed limits: min wins as the floor.
        assert_eq!(clamp_dock_size(10., 200., 100.), 200.);
    }
}
