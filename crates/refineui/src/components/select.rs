//! Select — web `Select` (`filled | ghost | plain`, `sm | md | lg`).
//! Keyboard: Space/Enter/↓ opens, ↑/↓ move, Enter picks, Escape closes.

use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, KeyDownEvent, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, anchored, div, prelude::*, px,
};

use gpui::{CursorStyle, InteractiveElement, Transformation};
use refineui_foundation::{radius, size::icon, spacing, stroke, typography::role};

use crate::components::plate::row_plate;
use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, presence, press_scale,
    probe, scaled_plate, use_interaction, use_layout_probe,
};
use crate::{
    ActiveTheme, CollisionOptions, ControlSize, FloatingLayout, FloatingSide, Icon, IconName,
    IconSize, PlacementRect, PlacementViewport, ScrollArea, StyledExt, place_item_aligned,
    place_popper, scroll_edge_item,
};

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
    frozen: Option<SelectLayout>,
    /// Last highlighted index we scrolled into view, so arrow keys move the
    /// viewport without fighting a later pointer scroll.
    last_scrolled_hl: Option<usize>,
}

/// How the popup is placed relative to the trigger (web `position`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectPosition {
    /// Radix default: the selected item overlays the trigger (macOS
    /// pop-up button style), shifting and scrolling to stay in view.
    #[default]
    ItemAligned,
    /// Below the trigger (or above when there is not enough room), with
    /// [`Select::side_offset`] and collision flipping.
    Popper,
}

/// Placement of an open Select popup. Alias of [`FloatingLayout`]; see that
/// type for the mapping to `--refineui-select-content-available-height`,
/// `--refineui-select-trigger-height`, and
/// `--refineui-select-content-transform-origin`.
pub type SelectLayout = FloatingLayout;

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

fn list_full_height(count: usize) -> f32 {
    let n = count.max(1) as f32;
    SHELL_INSET * 2. + n * ITEM_HEIGHT + (n - 1.) * ITEM_GAP
}

fn selected_item_mid(selected: usize) -> f32 {
    SHELL_INSET + selected as f32 * (ITEM_HEIGHT + ITEM_GAP) + ITEM_HEIGHT / 2.
}

/// Collision-aware placement for an open Select, in window coordinates.
fn select_layout(
    position: SelectPosition,
    trigger: gpui::Bounds<gpui::Pixels>,
    value_inset: f32,
    selected: usize,
    count: usize,
    viewport: gpui::Size<gpui::Pixels>,
    opts: CollisionOptions,
) -> SelectLayout {
    let trigger = PlacementRect::from_bounds(trigger);
    let viewport = PlacementViewport::from_size(viewport);
    let full_h = list_full_height(count);
    let selected = selected.min(count.saturating_sub(1));
    match position {
        SelectPosition::ItemAligned => {
            let left = trigger.x + value_inset - ITEM_TEXT_LEFT;
            let min_width = trigger.width + (trigger.x - left);
            place_item_aligned(
                trigger,
                viewport,
                left,
                min_width,
                full_h,
                selected_item_mid(selected),
                CollisionOptions {
                    collision_padding: opts.collision_padding,
                    side_offset: 0.,
                },
            )
        }
        SelectPosition::Popper => place_popper(
            trigger,
            viewport,
            trigger.width,
            full_h,
            FloatingSide::Bottom,
            opts,
        ),
    }
}

/// Keep the highlighted row inside the visible viewport when the user
/// arrows through a list taller than the open panel.
fn ensure_item_visible(scroll: &gpui::ScrollHandle, index: usize, viewport_h: f32) {
    if viewport_h <= 0. {
        return;
    }
    let stride = ITEM_HEIGHT + ITEM_GAP;
    let pad = spacing::XX_SMALL;
    let item_top = pad + index as f32 * stride;
    let item_bottom = item_top + ITEM_HEIGHT;
    let current = -f32::from(scroll.offset().y);
    let mut next = current;
    if item_top < current + pad {
        next = (item_top - pad).max(0.);
    } else if item_bottom > current + viewport_h - pad {
        next = (item_bottom + pad - viewport_h).max(0.);
    }
    if (next - current).abs() > 0.5 {
        scroll.set_offset(gpui::point(px(0.), px(-next)));
    }
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
    /// Open on first mount (stories / screenshots). After that the
    /// element's own open state takes over.
    default_open: bool,
    position: SelectPosition,
    collision_padding: f32,
    side_offset: f32,
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
            default_open: false,
            position: SelectPosition::ItemAligned,
            collision_padding: CONTENT_MARGIN,
            side_offset: crate::DEFAULT_SIDE_OFFSET,
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

    /// Start with the list open (first frame only; used by the gallery
    /// screenshot of the open popup).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Popup placement (default [`SelectPosition::ItemAligned`]).
    pub fn position(mut self, position: SelectPosition) -> Self {
        self.position = position;
        self
    }

    /// Viewport-edge padding used when flipping and clamping the popup
    /// (web `collisionPadding`, default 10).
    pub fn collision_padding(mut self, padding: f32) -> Self {
        self.collision_padding = padding.max(0.);
        self
    }

    /// Gap between the trigger and a [`SelectPosition::Popper`] popup
    /// (web `sideOffset`, default 4). Ignored for item-aligned placement.
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset.max(0.);
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
        let default_open = self.default_open;
        let state = window.use_keyed_state(
            ElementId::NamedChild(Box::new(self.id.clone()), "state".into()),
            cx,
            |_, _| SelectState {
                open: default_open,
                align_scroll: default_open,
                ..SelectState::default()
            },
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
        let list_id = child_id(&self.id, "list");
        let scroll = ScrollArea::handle(list_id.clone(), window, cx);
        let collision = CollisionOptions {
            collision_padding: self.collision_padding,
            side_offset: self.side_offset,
        };
        let geometry = probe_state.read(cx).container.map(|t| {
            select_layout(
                self.position,
                t,
                value_inset,
                self.selected
                    .unwrap_or(0)
                    .min(self.options.len().saturating_sub(1)),
                self.options.len(),
                window.viewport_size(),
                collision,
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
        if p.mounted && !crate::vibrancy::native_frost_active(cx) {
            if state.read(cx).align_scroll
                && let Some(layout) = geometry
            {
                if layout.scroll_top > 0. {
                    scroll.set_offset(gpui::point(px(0.), px(-layout.scroll_top)));
                } else if let Some(ix) = self
                    .selected
                    .or(highlighted)
                    .filter(|_| self.position == SelectPosition::Popper)
                {
                    let vh = f32::from(scroll.bounds().size.height);
                    if vh > 0. {
                        ensure_item_visible(&scroll, ix, vh);
                    }
                }
                state.update(cx, |s, _| {
                    s.align_scroll = false;
                    s.last_scrolled_hl = highlighted.or(self.selected);
                });
            } else if p.open
                && let Some(ix) = highlighted
                && state.read(cx).last_scrolled_hl != Some(ix)
            {
                let vh = f32::from(scroll.bounds().size.height);
                if vh > 0. {
                    ensure_item_visible(&scroll, ix, vh);
                    state.update(cx, |s, _| s.last_scrolled_hl = Some(ix));
                }
            }
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
                .map(|l| gpui::Bounds::new(l.origin(), gpui::size(px(l.width), px(l.height))));
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
                        let layout = geometry.unwrap_or(SelectLayout {
                            x: 0.,
                            y: 0.,
                            width: 0.,
                            height: 0.,
                            available_height: 0.,
                            trigger_height: 0.,
                            transform_origin: crate::TransformOrigin { x: 0., y: 0. },
                            side: FloatingSide::Bottom,
                            scroll_top: 0.,
                        });
                        native_select_list(
                            &parent,
                            list.clone(),
                            selected,
                            &list_state,
                            on_change.clone(),
                            layout.width,
                            layout.height,
                            layout.scroll_top,
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
                            .when_some(geometry, |a, layout| {
                                // Slide a few px out of the trigger as the
                                // popup enters, along `transform_origin`.
                                let mut y = layout.y;
                                if p.progress < 0.999 {
                                    let slide = (1.0 - p.progress) * 6.0;
                                    y += match layout.side {
                                        FloatingSide::Bottom => -slide,
                                        FloatingSide::Top => slide,
                                    };
                                }
                                a.position(gpui::point(px(layout.x), px(y)))
                            })
                            .child(
                                // Outer box: the frosted plate (a material
                                // when active) — it must not scroll, so the
                                // list scrolls in an inner box.
                                crate::material::frosted_plate(
                                    div()
                                        // Hidden until the trigger has been measured.
                                        .opacity(if geometry.is_some() { p.progress } else { 0. })
                                        .when_some(trigger_width, |d, w| d.min_w(w))
                                        .when_some(geometry, |d, layout| {
                                            d.min_w(px(layout.width)).h(px(layout.height))
                                        })
                                        .flex()
                                        .flex_col()
                                        .font_family(theme.font_family.clone())
                                        .when(p.open, |d| {
                                            d.occlude().on_mouse_down_out(move |_, _, cx| {
                                                close_out.update(cx, |s, cx| {
                                                    s.open = false;
                                                    cx.notify();
                                                })
                                            })
                                        }),
                                    crate::SurfaceKind::Menu,
                                    cx,
                                )
                                .child(
                                    ScrollArea::new(list_id).child(
                                        div()
                                            .p(px(spacing::XX_SMALL))
                                            .flex()
                                            .flex_col()
                                            .gap(px(1.))
                                            .child(
                                                // Non-scrolling content box: the
                                                // hover plate is positioned
                                                // against it so it scrolls with
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
) -> (Vec<gpui::AnyElement>, gpui::AnyElement) {
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
                scroll_edge_item(
                    child_id(&item_id, "edge"),
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
                )
                .into_any_element(),
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
    let list_id = child_id(parent, "native-list");
    let scroll = ScrollArea::handle(list_id.clone(), window, cx);
    if state.read(cx).align_scroll {
        scroll.set_offset(gpui::point(px(0.), px(-scroll_top)));
        state.update(cx, |s, _| {
            s.align_scroll = false;
            s.last_scrolled_hl = s.highlighted;
        });
    } else if let Some(ix) = state.read(cx).highlighted
        && state.read(cx).last_scrolled_hl != Some(ix)
    {
        let vh = f32::from(scroll.bounds().size.height);
        if vh > 0. {
            ensure_item_visible(&scroll, ix, vh);
            state.update(cx, |s, _| s.last_scrolled_hl = Some(ix));
        }
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
    let list_h = height - 2. * stroke::HAIRLINE;
    ScrollArea::new(list_id)
        .max_height(px(list_h))
        .child(
            div()
                .min_w(px(min_width - 2. * stroke::HAIRLINE))
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
                ),
        )
        .into_any_element()
}
