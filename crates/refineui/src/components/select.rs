//! Select — web `Select` (`filled | ghost | plain`, `sm | md | lg`).
//! Keyboard: Space/Enter/↓ opens, ↑/↓ move, Enter picks, Escape closes.

use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, KeyDownEvent, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, anchored, div, prelude::*, px,
};

use gpui::{CursorStyle, InteractiveElement, Transformation};
use refineui_tokens::{radius, size::icon, spacing, stroke, typography::role};

use crate::components::plate::row_plate;
use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, presence, press_scale,
    probe, scaled_plate, use_interaction, use_layout_probe,
};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, StyledExt};

/// Trigger appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectAppearance {
    /// Hairline-bordered field on `backgroundPrimary` (default).
    #[default]
    Filled,
    /// Borderless; hover / open shows the `backgroundPrimaryHover` plate.
    Ghost,
    /// Text and chevron only.
    Plain,
}

#[derive(Default)]
struct SelectState {
    open: bool,
    highlighted: Option<usize>,
    /// Set when the menu opens: scroll the list so the selected row lines up
    /// with the trigger on the next frame (web item-aligned `scrollTop`).
    align_scroll: bool,
    /// Geometry captured while open. The exit fade reuses it so the panel
    /// fades in place instead of jumping to re-align with a newly picked item.
    frozen: Option<(gpui::Point<gpui::Pixels>, f32, f32, f32)>,
}

/// Web `CONTENT_MARGIN` (`foundationSize100`).
const CONTENT_MARGIN: f32 = 10.0;
/// Item row: `p-x-small` (6) × 2 + `bodyMd` line (20).
const ITEM_HEIGHT: f32 = 32.0;
/// Group `gap-px` between rows.
const ITEM_GAP: f32 = 1.0;
/// Content shell inset: hairline border + `p-xx-small`.
const SHELL_INSET: f32 = stroke::HAIRLINE + spacing::XX_SMALL;
/// Item text start inside the shell: inset + item padding + 16px indicator + gap.
const ITEM_TEXT_LEFT: f32 = SHELL_INSET + spacing::X_SMALL + icon::X_SMALL + spacing::X_SMALL;

/// Web `Select` `position="item-aligned"` (Radix `SelectItemAlignedPosition`,
/// macOS pop-up button style): the menu opens **over** the trigger so the
/// selected item's text sits exactly on the trigger's value text, clamped to
/// the window with `CONTENT_MARGIN`. Returns `(origin, min_width, height,
/// scroll_top)` in window coordinates.
fn item_aligned_geometry(
    trigger: gpui::Bounds<gpui::Pixels>,
    value_inset: f32,
    selected: usize,
    count: usize,
    viewport: gpui::Size<gpui::Pixels>,
) -> (gpui::Point<gpui::Pixels>, f32, f32, f32) {
    let t_left = f32::from(trigger.left());
    let t_mid = f32::from(trigger.top()) + f32::from(trigger.size.height) / 2.;
    let t_w = f32::from(trigger.size.width);
    let vw = f32::from(viewport.width);
    let vh = f32::from(viewport.height);
    let left = t_left + value_inset - ITEM_TEXT_LEFT;
    let min_width = t_w + (t_left - left);
    let left = left.clamp(
        CONTENT_MARGIN,
        (vw - CONTENT_MARGIN - min_width).max(CONTENT_MARGIN),
    );
    let n = count.max(1) as f32;
    let full_h = SHELL_INSET * 2. + n * ITEM_HEIGHT + (n - 1.) * ITEM_GAP;
    let item_mid = SHELL_INSET + selected as f32 * (ITEM_HEIGHT + ITEM_GAP) + ITEM_HEIGHT / 2.;
    let desired_top = t_mid - item_mid;
    let top = desired_top.max(CONTENT_MARGIN);
    let available = (vh - CONTENT_MARGIN - top).max(ITEM_HEIGHT * 5.0_f32.min(n));
    let height = full_h.min(available).min(vh - CONTENT_MARGIN * 2.);
    let scroll = (top - desired_top).clamp(0., (full_h - height).max(0.));
    (gpui::point(px(left), px(top)), min_width, height, scroll)
}

/// Single-choice dropdown.
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    options: Vec<SharedString>,
    selected: Option<usize>,
    placeholder: SharedString,
    size: ControlSize,
    appearance: SelectAppearance,
    disabled: bool,
    width: Option<gpui::Pixels>,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Select {
    /// New select.
    pub fn new(
        id: impl Into<ElementId>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        selected: Option<usize>,
    ) -> Self {
        Self {
            id: id.into(),
            options: options.into_iter().map(Into::into).collect(),
            selected,
            placeholder: "Select…".into(),
            size: ControlSize::Medium,
            appearance: SelectAppearance::Filled,
            disabled: false,
            width: None,
            on_change: None,
        }
    }

    /// Placeholder when nothing is selected.
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }

    /// Size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Trigger appearance.
    pub fn appearance(mut self, a: SelectAppearance) -> Self {
        self.appearance = a;
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fixed width (default: fill parent).
    pub fn width(mut self, w: gpui::Pixels) -> Self {
        self.width = Some(w);
        self
    }

    /// Called with the picked index.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let state = window.use_keyed_state(
            ElementId::NamedChild(Box::new(self.id.clone()), "state".into()),
            cx,
            |_, _| SelectState::default(),
        );
        let (open, highlighted) = {
            let s = state.read(cx);
            (s.open, s.highlighted)
        };
        let p = presence(
            child_id(&self.id, "presence"),
            open,
            Motion::FROSTED,
            Motion::FROSTED,
            window,
            cx,
        );
        let probe_state = use_layout_probe(&self.id, window, cx);
        let trigger_width = probe_state.read(cx).container.map(|b| b.size.width);
        let label = self.selected.and_then(|i| self.options.get(i).cloned());
        let has_value = label.is_some();
        let n = self.options.len();
        let selected = self.selected;
        let m = theme.select_metrics(self.size);
        let trigger_id = child_id(&self.id, "trigger");
        let (tstate, ti) = use_interaction(&trigger_id, window, cx);
        let ti = if self.disabled {
            Interaction::default()
        } else {
            ti
        };
        // refineui.css select-trigger: hover / active / open plate.
        let plate = match self.appearance {
            SelectAppearance::Plain => gpui::transparent_black(),
            SelectAppearance::Filled if self.disabled => c.background_surface_disabled,
            _ if self.disabled => gpui::transparent_black(),
            SelectAppearance::Filled => {
                if ti.pressed {
                    c.background_primary_active
                } else if ti.hovered || open {
                    c.background_primary_hover
                } else {
                    c.background_primary
                }
            }
            SelectAppearance::Ghost => {
                if ti.pressed {
                    c.background_primary_active
                } else if ti.hovered || open {
                    c.background_primary_hover
                } else {
                    gpui::transparent_black()
                }
            }
        };
        let fg = if self.disabled {
            c.foreground_disabled
        } else if !has_value {
            c.foreground_placeholder
        } else if ti.hovered && !ti.pressed && !open {
            c.foreground_primary_hover
        } else {
            c.foreground_primary
        };
        let plate = animate(child_id(&self.id, "plate"), plate, Motion::FAST, window, cx);
        // `select-trigger:active` / `[data-pressed]` → press scale.
        let t_scale = press_scale(child_id(&self.id, "press"), ti.pressed, window, cx);
        let trigger_border = match self.appearance {
            SelectAppearance::Filled if self.disabled => Some(c.border_disabled),
            SelectAppearance::Filled => Some(c.border_default),
            _ => None,
        };
        let value_inset = match self.appearance {
            SelectAppearance::Filled => stroke::HAIRLINE + spacing::MEDIUM,
            SelectAppearance::Ghost => spacing::X_SMALL,
            SelectAppearance::Plain => 0.,
        };
        let scroll = window.use_keyed_state(child_id(&self.id, "scroll"), cx, |_, _| {
            gpui::ScrollHandle::new()
        });
        let scroll = scroll.read(cx).clone();
        let geometry = probe_state.read(cx).container.map(|t| {
            item_aligned_geometry(
                t,
                value_inset,
                self.selected
                    .unwrap_or(0)
                    .min(self.options.len().saturating_sub(1)),
                self.options.len(),
                window.viewport_size(),
            )
        });
        // Freeze the panel geometry during the exit so picking an item does not
        // re-align (move) the fading panel to the new selection.
        let geometry = if p.open {
            if geometry.is_some() && state.read(cx).frozen != geometry {
                state.update(cx, |s, _| s.frozen = geometry);
            }
            geometry
        } else {
            state.read(cx).frozen.or(geometry)
        };
        if p.mounted
            && !crate::vibrancy::native_frost_active(cx)
            && state.read(cx).align_scroll
            && let Some((_, _, _, top)) = geometry
        {
            scroll.set_offset(gpui::point(px(0.), px(-top)));
            state.update(cx, |s, _| s.align_scroll = false);
        }
        let fg = animate(child_id(&self.id, "fg"), fg, Motion::FAST, window, cx);
        let chevron = animate(
            child_id(&self.id, "chevron"),
            if open { 1.0 } else { 0.0 },
            Motion::FAST,
            window,
            cx,
        );
        let appearance = self.appearance;
        let ring = theme.focus_ring();
        let show_focus = focus_visible(cx);

        let toggle = state.clone();
        let keys = state.clone();
        let close_out = state.clone();
        let native_close = state.clone();
        let on_change_key = self.on_change.clone();

        let native = crate::vibrancy::native_frost_active(cx);
        let host = use_layout_probe(&child_id(&self.id, "highlight"), window, cx);
        let (options, slide_plate) = if p.mounted && !native {
            let (rows, plate) = select_rows(
                &self.id,
                self.options.clone(),
                selected,
                highlighted,
                &state,
                self.on_change.clone(),
                window,
                cx,
            );
            (rows, Some(plate))
        } else {
            (Vec::new(), None)
        };
        if native {
            let bounds = geometry
                .map(|(origin, w, h, _)| gpui::Bounds::new(origin, gpui::size(px(w), px(h))));
            let list = self.options.clone();
            let list_state = state.clone();
            let still = state.clone();
            let close_state = state.clone();
            let on_change = self.on_change.clone();
            let parent = self.id.clone();
            crate::vibrancy::drive_native_popup(
                &self.id,
                open,
                bounds,
                move || crate::vibrancy::NativePopupSpec {
                    radius: radius::XX_LARGE,
                    close: Rc::new(move |cx| {
                        close_state.update(cx, |s, cx| {
                            if s.open {
                                s.open = false;
                                cx.notify();
                            }
                        })
                    }),
                    still_open: Rc::new(move |cx| still.read(cx).open),
                    content: Rc::new(move |window, cx| {
                        let (_, w, h, top) = geometry.unwrap_or_default();
                        native_select_list(
                            &parent,
                            list.clone(),
                            selected,
                            &list_state,
                            on_change.clone(),
                            w,
                            h,
                            top,
                            window,
                            cx,
                        )
                    }),
                    close_on_deactivate: true,
                    report_size: None,
                    popover: false,
                },
                window,
                cx,
            );
        }

        div()
            .id(self.id)
            .relative()
            .map(|d| match self.width {
                Some(w) => d.w(w),
                None => d.w_full(),
            })
            .child(
                div()
                    .id(trigger_id)
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::X_SMALL))
                    .map(|d| match appearance {
                        // Hairline border is painted on the plate; the box
                        // reserves it as padding.
                        SelectAppearance::Filled => d
                            .min_h(px(m.height))
                            .px(px(spacing::MEDIUM + stroke::HAIRLINE))
                            .py(px(spacing::X_SMALL + stroke::HAIRLINE))
                            .rounded(px(radius::LARGE)),
                        SelectAppearance::Ghost => d
                            .min_h(px(m.height))
                            .px(px(spacing::X_SMALL))
                            .py(px(spacing::XX_SMALL))
                            .rounded(px(radius::LARGE)),
                        SelectAppearance::Plain => d,
                    })
                    .child(
                        scaled_plate(t_scale)
                            .rounded(px(radius::LARGE * t_scale))
                            .bg(plate)
                            .when_some(trigger_border, |d, b| {
                                d.border(px(stroke::HAIRLINE)).border_color(b)
                            }),
                    )
                    .text_style_token(m.text)
                    .text_color(fg)
                    .child(probe(&probe_state, None))
                    .when(!self.disabled, |d| {
                        d.cursor_action()
                            .track_interaction(&tstate)
                            .tab_index(0)
                            .when(show_focus, |d| d.focus(move |s| s.shadow(ring)))
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |s, cx| {
                                    s.open = !s.open;
                                    s.highlighted = None;
                                    s.align_scroll = s.open;
                                    cx.notify();
                                })
                            })
                            .on_key_down(move |e: &KeyDownEvent, w, cx| {
                                let key = e.keystroke.key.as_str();
                                let (is_open, hl) = {
                                    let s = keys.read(cx);
                                    (s.open, s.highlighted)
                                };
                                match key {
                                    "down" | "up" if n > 0 => {
                                        cx.stop_propagation();
                                        keys.update(cx, |s, cx| {
                                            if !s.open {
                                                s.open = true;
                                                s.align_scroll = true;
                                                s.highlighted = selected.or(Some(0));
                                            } else {
                                                let cur = hl.or(selected).unwrap_or(0);
                                                s.highlighted = Some(if key == "down" {
                                                    (cur + 1).min(n - 1)
                                                } else {
                                                    cur.saturating_sub(1)
                                                });
                                            }
                                            cx.notify();
                                        });
                                    }
                                    "enter" | "space" => {
                                        cx.stop_propagation();
                                        if is_open && let (Some(ix), Some(h)) = (hl, &on_change_key)
                                        {
                                            h(&ix, w, cx);
                                        }
                                        keys.update(cx, |s, cx| {
                                            s.open = !is_open;
                                            s.align_scroll = !is_open;
                                            s.highlighted =
                                                if is_open { None } else { selected.or(Some(0)) };
                                            cx.notify();
                                        });
                                    }
                                    "escape" if is_open => {
                                        cx.stop_propagation();
                                        keys.update(cx, |s, cx| {
                                            s.open = false;
                                            cx.notify();
                                        });
                                    }
                                    _ => {}
                                }
                            })
                    })
                    .when(self.disabled, |d| {
                        d.cursor(CursorStyle::OperationNotAllowed)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(label.unwrap_or(self.placeholder)),
                    )
                    .child(
                        Icon::new(IconName::ChevronDown)
                            .size(IconSize::XSmall)
                            .color(fg)
                            .transform(Transformation::rotate(gpui::radians(
                                chevron * std::f32::consts::PI,
                            ))),
                    ),
            )
            .when(native && open, |d| {
                // Clicks in the pop-up window never reach this window.
                d.on_mouse_down_out(move |_, _, cx| {
                    native_close.update(cx, |s, cx| {
                        s.open = false;
                        cx.notify();
                    })
                })
            })
            .when(p.mounted && !native, |d| {
                d.child(
                    crate::layer(
                        anchored()
                            .when_some(geometry, |a, (origin, _, _, _)| a.position(origin))
                            .snap_to_window_with_margin(px(CONTENT_MARGIN))
                            .child(
                                div()
                                .id("menu")
                                // Hidden until the trigger has been measured.
                                .opacity(if geometry.is_some() { p.progress } else { 0. })
                                .when_some(trigger_width, |d, w| d.min_w(w))
                                .when_some(geometry, |d, (_, w, h, _)| {
                                    d.min_w(px(w)).h(px(h))
                                })
                                .overflow_y_scroll()
                                .track_scroll(&scroll)
                                .p(px(spacing::XX_SMALL))
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .popover_surface(cx)
                                .font_family(theme.font_family.clone())
                                .when(p.open, |d| {
                                    d.occlude().on_mouse_down_out(move |_, _, cx| {
                                        close_out.update(cx, |s, cx| {
                                            s.open = false;
                                            cx.notify();
                                        })
                                    })
                                })
                                .child(
                                    // Non-scrolling content box: the plate is
                                    // positioned against it so it scrolls with
                                    // the rows.
                                    div()
                                        .relative()
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.))
                                        .child(probe(&host, None))
                                        .children(slide_plate)
                                        .children(options),
                                ),
                            ),
                    )
                    .priority(1),
                )
            })
    }
}

type OnChange = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// The option rows (and the sliding hover plate) of an open select,
/// rendered in whichever window shows the list.
#[allow(clippy::too_many_arguments)]
fn select_rows(
    parent: &ElementId,
    options_in: Vec<SharedString>,
    selected: Option<usize>,
    highlighted: Option<usize>,
    state: &gpui::Entity<SelectState>,
    on_change: Option<OnChange>,
    window: &mut Window,
    cx: &mut App,
) -> (Vec<gpui::Stateful<gpui::Div>>, gpui::AnyElement) {
    let c = cx.theme().colors;
    let parent = parent.clone();
    let parent = &parent;
    let mut options = Vec::new();
    let mut plate_target = None;
    let host = use_layout_probe(&child_id(parent, "highlight"), window, cx);
    {
        for (ix, opt) in options_in.into_iter().enumerate() {
            let item_id =
                ElementId::NamedChild(Box::new(parent.clone()), format!("opt-{ix}").into());
            let (istate, mut ii) = use_interaction(&item_id, window, cx);
            if Some(ix) == highlighted || ii.hovered {
                plate_target = Some(ix);
            }
            // The sliding plate carries hover; rows keep only their
            // selected / pressed fills (web `[data-sliding-highlight]`).
            if !ii.pressed {
                ii.hovered = false;
            }
            let is_sel = Some(ix) == selected;
            let (bg, ifg) = row_plate(ii, is_sel, false, c.foreground_primary, cx);
            let bg = animate(child_id(&item_id, "bg"), bg, Motion::FAST, window, cx);
            let scale = press_scale(child_id(&item_id, "press"), ii.pressed, window, cx);
            let handler = on_change.clone();
            let st = state.clone();
            options.push(
                div()
                    .id(item_id)
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(spacing::X_SMALL))
                    .p(px(spacing::X_SMALL))
                    .rounded(px(radius::LARGE))
                    .child(
                        scaled_plate(scale)
                            .rounded(px(radius::LARGE * scale))
                            .bg(bg),
                    )
                    .text_color(ifg)
                    .text_style_token(role::BODY_MD)
                    .cursor_action()
                    .track_interaction(&istate)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(move |_, w, cx| {
                        if let Some(h) = &handler {
                            h(&ix, w, cx);
                        }
                        st.update(cx, |s, cx| {
                            s.open = false;
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex_none()
                            .size(px(icon::X_SMALL))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_sel, |d| {
                                d.child(
                                    Icon::new(IconName::Checkmark)
                                        .size(IconSize::XSmall)
                                        .color(ifg),
                                )
                            }),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(opt))
                    .child(probe(&host, Some(ix))),
            );
        }
    }
    let slide_plate = {
        crate::components::sliding_highlight::sliding_plate(
            &child_id(parent, "highlight"),
            &host,
            plate_target,
            c.background_primary_hover,
            radius::LARGE,
            window,
            cx,
        )
    };
    (options, slide_plate.into_any_element())
}

/// The list panel of a native-frost select (inside its pop-up window).
#[allow(clippy::too_many_arguments)]
fn native_select_list(
    parent: &ElementId,
    options: Vec<SharedString>,
    selected: Option<usize>,
    state: &gpui::Entity<SelectState>,
    on_change: Option<OnChange>,
    min_width: f32,
    height: f32,
    scroll_top: f32,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let theme = cx.theme().clone();
    let scroll = window.use_keyed_state(child_id(parent, "native-scroll"), cx, |_, _| {
        gpui::ScrollHandle::new()
    });
    let scroll = scroll.read(cx).clone();
    if state.read(cx).align_scroll {
        scroll.set_offset(gpui::point(px(0.), px(-scroll_top)));
        state.update(cx, |s, _| s.align_scroll = false);
    }
    let highlighted = state.read(cx).highlighted;
    let (rows, plate) = select_rows(
        parent,
        options,
        selected,
        highlighted,
        state,
        on_change,
        window,
        cx,
    );
    let host = use_layout_probe(&child_id(parent, "highlight"), window, cx);
    div()
        .id("menu")
        .min_w(px(min_width - 2. * stroke::HAIRLINE))
        .h(px(height - 2. * stroke::HAIRLINE))
        .overflow_y_scroll()
        .track_scroll(&scroll)
        .p(px(spacing::XX_SMALL))
        .font_family(theme.font_family.clone())
        .child(
            div()
                .relative()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(probe(&host, None))
                .child(plate)
                .children(rows),
        )
        .into_any_element()
}
