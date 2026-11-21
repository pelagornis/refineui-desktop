//! Popover — web `PopOver` (click-to-open floating panel).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString,
    Styled, Window, anchored, div, prelude::*, px,
};

use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::components::floating::{Beak, BeakAlign, BeakSide, floating_gap};
use crate::components::positioning::{
    AnchorAlign, AnchorSide, CollisionOptions, PlacementRect, PlacementViewport, place_anchored,
};
use crate::motion::{child_id, presence, probe, use_layout_probe};
use crate::{ActiveTheme, Button, ButtonVariant, ControlSize, Elevation, IconName, StyledExt};

/// Content builder: receives a `close` callback.
pub type PopoverContent = Rc<dyn Fn(Rc<dyn Fn(&mut App)>, &mut Window, &mut App) -> AnyElement>;

/// Panel alignment against the trigger — web `PopoverAlign`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PopoverAlign {
    /// Panel start edge flush with the trigger's start edge.
    Start,
    /// Panel centered under the trigger (web default).
    #[default]
    Center,
    /// Panel end edge flush with the trigger's end edge.
    End,
}

/// Floating panel anchored to a trigger.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: AnyElement,
    title: Option<SharedString>,
    content: PopoverContent,
    align: PopoverAlign,
    /// Preferred side; flips with [`place_anchored`] when the panel would overflow.
    placement: AnchorSide,
    /// Gap between the trigger and the panel. Defaults to the web `POPOVER_GAP`.
    side_offset: Option<f32>,
    inline_open: bool,
}

impl Popover {
    /// New popover (bottom placement, centered — web defaults).
    pub fn new(
        id: impl Into<ElementId>,
        trigger: impl IntoElement,
        content: impl Fn(Rc<dyn Fn(&mut App)>, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: trigger.into_any_element(),
            title: None,
            content: Rc::new(content),
            align: PopoverAlign::Center,
            placement: AnchorSide::Bottom,
            side_offset: None,
            inline_open: false,
        }
    }

    /// Optional heading with a close button.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Preferred side of the trigger (web `side`). The panel flips when this
    /// side does not have room.
    pub fn placement(mut self, side: AnchorSide) -> Self {
        self.placement = side;
        self
    }

    /// Gap between the trigger and the panel (web `sideOffset`).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = Some(offset.max(0.));
        self
    }

    /// Panel alignment (web `align`, default `center`).
    pub fn align(mut self, align: PopoverAlign) -> Self {
        self.align = align;
        self
    }

    /// Align the panel's end edge with the trigger (`align("end")`).
    pub fn align_end(self) -> Self {
        self.align(PopoverAlign::End)
    }

    /// Render the panel open, in flow (for docs / screenshots).
    pub fn force_open(mut self) -> Self {
        self.inline_open = true;
        self
    }
}

/// The popover panel chrome on its own (for custom anchoring and docs):
/// web `FloatingSurface` — one frosted outline (panel + 24×12 beak with a
/// 4px rounded tip) stroked with a hairline `borderDefault`, `roundedXXLarge`,
/// floating elevation. `align` places the beak like the web (start inset 34,
/// end inset 30, or centered).
pub fn popover_panel(
    title: Option<SharedString>,
    body: AnyElement,
    close: Option<Rc<dyn Fn(&mut App)>>,
    align: PopoverAlign,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    popover_panel_at(title, body, close, align, AnchorSide::Bottom, None, cx)
}

/// Like [`popover_panel`], with an explicit placement and optional beak
/// center (distance along the facing edge, aimed at the trigger midpoint).
pub fn popover_panel_at(
    title: Option<SharedString>,
    body: AnyElement,
    close: Option<Rc<dyn Fn(&mut App)>>,
    align: PopoverAlign,
    placement: AnchorSide,
    beak_center: Option<f32>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    popover_panel_impl(
        title,
        body,
        close,
        Some((align, placement, beak_center)),
        cx,
    )
}

fn beak_side_for(placement: AnchorSide) -> BeakSide {
    match placement {
        AnchorSide::Bottom => BeakSide::Top,
        AnchorSide::Top => BeakSide::Bottom,
        AnchorSide::Right => BeakSide::Left,
        AnchorSide::Left => BeakSide::Right,
    }
}

fn anchor_align(align: PopoverAlign) -> AnchorAlign {
    match align {
        PopoverAlign::Start => AnchorAlign::Start,
        PopoverAlign::Center => AnchorAlign::Center,
        PopoverAlign::End => AnchorAlign::End,
    }
}

/// `chrome = None`: no surface / beak / shadow (a native-frost pop-up
/// window draws the frosted fill and edge).
fn popover_panel_impl(
    title: Option<SharedString>,
    body: AnyElement,
    close: Option<Rc<dyn Fn(&mut App)>>,
    chrome: Option<(PopoverAlign, AnchorSide, Option<f32>)>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    use refineui_foundation::size::component::POPOVER_PANEL_WIDTH;
    let theme = cx.theme();
    let c = theme.colors;
    let fill = theme.floating_fill();
    let beak = chrome.map(|(align, placement, center)| {
        let align = match align {
            PopoverAlign::Start => BeakAlign::Start,
            PopoverAlign::Center => BeakAlign::Center,
            PopoverAlign::End => BeakAlign::End,
        };
        let beak = Beak::popover(beak_side_for(placement), align);
        match center {
            Some(c) => beak.with_center(c),
            None => beak,
        }
    });
    // One closed outline (panel + beak). A rectangular material plate would
    // draw a seam through the beak, so beaked chrome uses the path fill.
    let (chrome_layers, shadows) = match beak {
        Some(beak) => crate::components::floating::floating_chrome(
            crate::SurfaceKind::Popover,
            radius::XX_LARGE,
            Some(beak),
            fill,
            c.border_default,
            theme.shadow(Elevation::FLOATING),
            cx,
        ),
        None => (Vec::new(), Vec::new()),
    };
    div()
        .id("refineui-popover-panel")
        .occlude()
        .relative()
        .min_w(px(POPOVER_PANEL_WIDTH))
        .flex()
        .flex_col()
        .rounded(px(radius::XX_LARGE))
        .shadow(shadows)
        .font_family(theme.font_family.clone())
        .text_style_token(role::BODY_MD)
        .text_color(c.foreground_primary)
        .children(chrome_layers)
        .child(
            div()
                // Transparent hairline padding keeps the web box size.
                // Kept off the plate node so the material covers the full surface.
                .p(px(spacing::LARGE + stroke::HAIRLINE))
                .flex()
                .flex_col()
                .gap(px(spacing::SMALL))
                .when(title.is_some() || close.is_some(), |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::X_SMALL))
                            .child(
                                div()
                                    .flex_1()
                                    .text_style_token(role::SUBTITLE_MD)
                                    .children(title),
                            )
                            .when_some(close, |d, close| {
                                d.child(
                                    Button::icon_only("refineui-popover-close", IconName::Dismiss)
                                        .variant(ButtonVariant::Ghost)
                                        .size(ControlSize::Small)
                                        .on_click(move |_, _, cx| close(cx)),
                                )
                            }),
                    )
                })
                .child(body),
        )
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = crate::components::menu::popup_state(&self.id, window, cx);
        let is_open = *open.read(cx) || self.inline_open;
        let p = presence(
            child_id(&self.id, "presence"),
            is_open,
            crate::components::picker::FLOAT_ENTER,
            crate::components::picker::FLOAT_ENTER,
            window,
            cx,
        );
        // Web `useFloatingPresence`: fade + 6px slide from the trigger side
        // over `duration-fast` (emphasized). (The web also scales from 0.96;
        // GPUI cannot scale a subtree, so that part is not reproduced.)
        let slide = px(-spacing::X_SMALL * (1. - p.progress));
        let measure = use_layout_probe(&self.id, window, cx);
        let toggle = open.clone();
        let close_out = open.clone();
        let close_state = open.clone();
        let close: Rc<dyn Fn(&mut App)> = Rc::new(move |cx| {
            close_state.update(cx, |o, cx| {
                *o = false;
                cx.notify();
            })
        });
        let inline = self.inline_open;
        let align = self.align;
        let preferred = self.placement;
        let default_gap = f32::from(floating_gap(
            refineui_foundation::size::component::POPOVER_BEAK_HEIGHT,
        ));
        let gap_px = self.side_offset.unwrap_or(default_gap);
        let gap = px(gap_px);
        let opts = CollisionOptions::new()
            .collision_padding(8.)
            .side_offset(gap_px);
        let viewport = PlacementViewport::from_size(window.viewport_size());
        let placed = {
            let m = measure.read(cx);
            match (m.container, m.items.first().copied().flatten()) {
                (Some(t), Some(panel)) => Some(place_anchored(
                    PlacementRect::from_bounds(t),
                    viewport,
                    f32::from(panel.size.width),
                    f32::from(panel.size.height),
                    preferred,
                    anchor_align(align),
                    opts,
                )),
                _ => None,
            }
        };
        let (offset_x, offset_y, resolved_side, beak_center) =
            match (placed, measure.read(cx).container) {
                (Some(l), Some(t)) => (
                    px(l.x - f32::from(t.origin.x)),
                    px(l.y - f32::from(t.origin.y) - f32::from(t.size.height)),
                    l.side,
                    Some(l.beak_center),
                ),
                _ => (px(0.), px(0.), preferred, None),
            };
        let measured = measure.read(cx).items.first().copied().flatten().is_some();
        if !inline && crate::vibrancy::native_frost_active(cx) {
            // FrostMode::Native: the panel (without beak — the pop-up window
            // is a rounded rectangle) opens in an OS-blurred pop-up window.
            let size = window.use_keyed_state(child_id(&self.id, "native-size"), cx, |_, _| {
                None::<gpui::Size<gpui::Pixels>>
            });
            let w = size.read(cx).map(|s| s.width).unwrap_or(px(
                refineui_foundation::size::component::POPOVER_PANEL_WIDTH,
            ));
            let bounds = measure.read(cx).container.map(|t| {
                let l = place_anchored(
                    PlacementRect::from_bounds(t),
                    viewport,
                    f32::from(w),
                    120.,
                    preferred,
                    anchor_align(align),
                    opts,
                );
                gpui::Bounds::new(
                    gpui::point(px(l.x), px(l.y)),
                    gpui::size(w, px(l.height.max(1.))),
                )
            });
            let content = self.content.clone();
            let title = self.title.clone();
            let spec_close = close.clone();
            let still = open.clone();
            crate::vibrancy::drive_native_popup(
                &self.id,
                is_open,
                bounds,
                move || crate::vibrancy::NativePopupSpec {
                    radius: radius::XX_LARGE,
                    close: spec_close.clone(),
                    still_open: Rc::new(move |cx| *still.read(cx)),
                    content: Rc::new(move |window, cx| {
                        let body = content(spec_close.clone(), window, cx);
                        popover_panel_impl(
                            title.clone(),
                            body,
                            title.is_some().then(|| spec_close.clone()),
                            None,
                            cx,
                        )
                        .into_any_element()
                    }),
                    close_on_deactivate: true,
                    report_size: Some(size),
                    popover: true,
                },
                window,
                cx,
            );
            return div()
                .id(self.id)
                .relative()
                .flex_none()
                .child(
                    div()
                        .id("trigger")
                        .relative()
                        .child(probe(&measure, None))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            toggle.update(cx, |o, cx| {
                                *o = !*o;
                                cx.notify();
                            })
                        })
                        .when(is_open, |d| {
                            d.on_mouse_down_out(move |_, _, cx| {
                                close_out.update(cx, |o, cx| {
                                    *o = false;
                                    cx.notify();
                                })
                            })
                        })
                        .child(self.trigger),
                )
                .into_any_element();
        }
        div()
            .id(self.id)
            .relative()
            .flex_none()
            .flex()
            .flex_col()
            .items_start()
            .child(
                div()
                    .id("trigger")
                    .relative()
                    .child(probe(&measure, None))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        toggle.update(cx, |o, cx| {
                            *o = !*o;
                            cx.notify();
                        })
                    })
                    .child(self.trigger),
            )
            .when(p.mounted, |d| {
                let body = (self.content)(close.clone(), window, cx);
                let panel = popover_panel_at(
                    self.title.clone(),
                    body,
                    // Web `PopOver` has no dismiss button; a titled panel gets one.
                    self.title.is_some().then(|| close.clone()),
                    align,
                    resolved_side,
                    beak_center,
                    cx,
                )
                .child(probe(&measure, Some(0)));
                // `offset_y` is relative to the trigger's bottom; the gap already
                // includes the beak height via `place_anchored` side_offset.
                if inline {
                    d.child(
                        div()
                            .mt(if resolved_side == AnchorSide::Top {
                                px(0.)
                            } else {
                                gap + slide
                            })
                            .ml(offset_x)
                            .opacity(p.progress)
                            .child(panel),
                    )
                } else {
                    d.child(
                        crate::layer(
                            anchored()
                                .offset(gpui::point(offset_x, offset_y + slide))
                                .snap_to_window_with_margin(px(8.))
                                .child(
                                    div()
                                        .id("popover-wrap")
                                        // Hidden for the one frame before the panel is measured.
                                        .opacity(if measured { p.progress } else { 0. })
                                        .when(p.open, |d| {
                                            d.on_mouse_down_out(move |_, _, cx| {
                                                close_out.update(cx, |o, cx| {
                                                    *o = false;
                                                    cx.notify();
                                                })
                                            })
                                        })
                                        .child(panel),
                                ),
                        )
                        .priority(1),
                    )
                }
            })
            .into_any_element()
    }
}
