//! Popover — web `PopOver` (click-to-open floating panel).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString,
    Styled, Window, anchored, div, prelude::*, px,
};

use refineui_tokens::{radius, spacing, stroke, typography::role};

use crate::components::floating::{Beak, BeakAlign, BeakSide, floating_gap, floating_surface};
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
            inline_open: false,
        }
    }

    /// Optional heading with a close button.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
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
    popover_panel_impl(title, body, close, Some(align), cx)
}

/// `chrome = None`: no surface / beak / shadow (a native-frost pop-up
/// window draws the frosted fill and edge).
fn popover_panel_impl(
    title: Option<SharedString>,
    body: AnyElement,
    close: Option<Rc<dyn Fn(&mut App)>>,
    chrome: Option<PopoverAlign>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    use refineui_tokens::size::component::POPOVER_PANEL_WIDTH;
    let theme = cx.theme();
    let c = theme.colors;
    let fill = theme.floating_fill();
    let beak_align = chrome.map(|align| match align {
        PopoverAlign::Start => BeakAlign::Start,
        PopoverAlign::Center => BeakAlign::Center,
        PopoverAlign::End => BeakAlign::End,
    });
    div()
        .id("refineui-popover-panel")
        .occlude()
        .relative()
        .min_w(px(POPOVER_PANEL_WIDTH))
        // Transparent hairline border keeps the web box size.
        .p(px(spacing::LARGE + stroke::HAIRLINE))
        .flex()
        .flex_col()
        .gap(px(spacing::SMALL))
        .rounded(px(radius::XX_LARGE))
        .when(chrome.is_some(), |d| d.shadow(theme.shadow(Elevation::FLOATING)))
        .font_family(theme.font_family.clone())
        .text_style_token(role::BODY_MD)
        .text_color(c.foreground_primary)
        .when_some(beak_align, |d, beak_align| {
            d.child(floating_surface(
                radius::XX_LARGE,
                Some(Beak::popover(BeakSide::Top, beak_align)),
                fill,
                c.border_default,
            ))
        })
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
        .child(body)
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
        // Horizontal offset of the panel against the trigger's left edge.
        let offset_x = {
            let m = measure.read(cx);
            match (m.container, m.items.first().copied().flatten()) {
                (Some(t), Some(panel)) => match align {
                    PopoverAlign::Start => px(0.),
                    PopoverAlign::Center => (t.size.width - panel.size.width) / 2.,
                    PopoverAlign::End => t.size.width - panel.size.width,
                },
                _ => px(0.),
            }
        };
        let measured = measure.read(cx).items.first().copied().flatten().is_some();
        let gap = floating_gap(refineui_tokens::size::component::POPOVER_BEAK_HEIGHT);
        if !inline && crate::vibrancy::native_frost_active(cx) {
            // FrostMode::Native: the panel (without beak — the pop-up window
            // is a rounded rectangle) opens in an OS-blurred pop-up window.
            let size = window.use_keyed_state(child_id(&self.id, "native-size"), cx, |_, _| {
                None::<gpui::Size<gpui::Pixels>>
            });
            let w = size
                .read(cx)
                .map(|s| s.width)
                .unwrap_or(px(refineui_tokens::size::component::POPOVER_PANEL_WIDTH));
            let bounds = measure.read(cx).container.map(|t| {
                let x = match align {
                    PopoverAlign::Start => t.left(),
                    PopoverAlign::Center => t.left() + (t.size.width - w) / 2.,
                    PopoverAlign::End => t.right() - w,
                };
                gpui::Bounds::new(gpui::point(x, t.bottom() + gap), gpui::size(w, px(120.)))
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
                let panel = popover_panel(
                    self.title.clone(),
                    body,
                    // Web `PopOver` has no dismiss button; a titled panel gets one.
                    self.title.is_some().then(|| close.clone()),
                    align,
                    cx,
                )
                .child(probe(&measure, Some(0)));
                if inline {
                    d.child(
                        div()
                            .mt(gap + slide)
                            .ml(offset_x)
                            .opacity(p.progress)
                            .child(panel),
                    )
                } else {
                    d.child(
                        crate::layer(
                            anchored()
                                .offset(gpui::point(offset_x, px(0.)))
                                .snap_to_window_with_margin(px(8.))
                                .child(
                                    div()
                                        .id("popover-wrap")
                                        // Hidden for the one frame before the panel is measured.
                                        .opacity(if measured { p.progress } else { 0. })
                                        .mt(gap + slide)
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
