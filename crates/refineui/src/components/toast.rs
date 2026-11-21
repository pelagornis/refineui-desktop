//! Toast and Toaster — web `Toast` (`default | success | error | warning`).
//!
//! Render a [`Toaster`] entity once in your root view, then call
//! `toaster.update(cx, |t, cx| t.push(ToastData::new("Saved"), cx))`.
//!
//! Card (web `toastRecipe`): `backgroundPrimary`, hairline `borderDefault`,
//! `roundedXLarge`, `sizeMedium` block / start padding and `sizeLarge` end
//! padding, `sizeSmall` gap, filled 24px variant icon, `bodyLg` message,
//! toast elevation, max width 325px.
//!
//! Toaster (web Sonner-like stack): top-centre, newest in front. Collapsed,
//! older toasts peek below (`gap × 0.85 + front × 0.035` per step) at
//! `1 − i × 0.12` opacity; hovering the stack expands it to a full column with
//! `sizeLarge` gaps. Offsets move over `duration-panel` (emphasized). Toasts
//! enter from `translateY(-100%)` + opacity over `toastEnter` (280ms,
//! emphasized) and leave the same way over `toastLeave` (200ms, standard).

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyElement, App, Context, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, RenderOnce, SharedString, Styled, Window, anchored, canvas, div, point, prelude::*, px,
};
use refineui_foundation::{
    motion::duration, radius, size::component, spacing, stroke, typography::role,
};

use crate::motion::{Motion, animate, animate_from};
use crate::{
    ActiveTheme, Button, ButtonVariant, ControlSize, Elevation, Icon, IconName, IconSize,
    IconStyle, StyledExt,
};

/// Toast intent (sets the icon and its color).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToastVariant {
    /// `info` icon in `foregroundPrimary`.
    #[default]
    Default,
    /// `checkmark` in `foregroundSuccess`.
    Success,
    /// `error-circle` in `foregroundError`.
    Error,
    /// `warning` in `foregroundWarning`.
    Warning,
}

impl ToastVariant {
    /// All variants.
    pub const ALL: [ToastVariant; 4] = [Self::Default, Self::Success, Self::Error, Self::Warning];
}

/// Toast content.
#[derive(Clone)]
pub struct ToastData {
    /// Message (`bodyLg`).
    pub title: SharedString,
    /// Optional second line (desktop addition, `bodyXs` secondary).
    pub description: Option<SharedString>,
    /// Intent.
    pub variant: ToastVariant,
    /// Auto-dismiss delay; `None` keeps it until dismissed (adds a close button).
    pub duration: Option<Duration>,
    /// Action button (web `action`: primary `sm` button).
    pub action: Option<(SharedString, Rc<dyn Fn(&mut Window, &mut App)>)>,
}

impl ToastData {
    /// Message toast (4s).
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            variant: ToastVariant::Default,
            duration: Some(Duration::from_secs(4)),
            action: None,
        }
    }

    /// Second line.
    pub fn description(mut self, d: impl Into<SharedString>) -> Self {
        self.description = Some(d.into());
        self
    }

    /// Intent.
    pub fn variant(mut self, v: ToastVariant) -> Self {
        self.variant = v;
        self
    }

    /// Auto-dismiss delay.
    pub fn duration(mut self, d: Option<Duration>) -> Self {
        self.duration = d;
        self
    }

    /// Action button.
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.action = Some((label.into(), Rc::new(f)));
        self
    }
}

/// A single toast card (also usable inline).
#[derive(IntoElement)]
pub struct Toast {
    id: ElementId,
    data: ToastData,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    material: bool,
}

impl Toast {
    /// Card for `data`.
    pub fn new(id: impl Into<ElementId>, data: ToastData) -> Self {
        Self {
            id: id.into(),
            data,
            on_close: None,
            material: false,
        }
    }

    /// Draw the card as the Toast material (real blur, toast elevation)
    /// when [`crate::materials_active`]; otherwise unchanged. Default `false`
    /// (web parity: an opaque `backgroundPrimary` card).
    pub fn material(mut self, material: bool) -> Self {
        self.material = material;
        self
    }

    /// Show a ghost dismiss button (desktop addition; the web swipes away).
    pub fn on_close(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Toast {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let (icon, accent) = match self.data.variant {
            ToastVariant::Default => (IconName::Info, c.foreground_primary),
            ToastVariant::Success => (IconName::Checkmark, c.foreground_success),
            ToastVariant::Error => (IconName::ErrorCircle, c.foreground_error),
            ToastVariant::Warning => (IconName::Warning, c.foreground_warning),
        };
        let close = self.on_close.clone();
        let parent = self.id.clone();
        let plate = if self.material {
            crate::material::backing(
                crate::SurfaceKind::Toast,
                px(radius::X_LARGE),
                stroke::HAIRLINE,
                c.background_primary,
                Some(c.border_default),
                theme.shadow(Elevation::TOAST),
                cx,
            )
        } else {
            crate::material::Backing {
                fill: c.background_primary,
                border: c.border_default,
                shadows: theme.shadow(Elevation::TOAST),
                layer: None,
            }
        };
        div()
            .id(self.id)
            .occlude()
            .relative()
            .max_w(px(component::TOAST_MAX_WIDTH))
            .rounded(px(radius::X_LARGE))
            .bg(plate.fill)
            .border(px(stroke::HAIRLINE))
            .border_color(plate.border)
            .shadow(plate.shadows)
            .children(plate.layer)
            .font_family(theme.font_family.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::SMALL))
                    .py(px(spacing::MEDIUM))
                    .pl(px(spacing::MEDIUM))
                    .pr(px(spacing::LARGE))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(icon)
                                    .style(IconStyle::Filled)
                                    .size(IconSize::Medium)
                                    .color(accent),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_style_token(role::BODY_LG)
                                    .text_color(c.foreground_primary)
                                    .child(self.data.title),
                            )
                            .when_some(self.data.description, |d, t| {
                                d.child(
                                    div()
                                        .text_style_token(role::BODY_XS)
                                        .text_color(c.foreground_secondary)
                                        .child(t),
                                )
                            }),
                    )
                    .when_some(self.data.action, |d, (label, f)| {
                        d.child(
                            div().flex_none().child(
                                Button::new(
                                    ElementId::NamedChild(
                                        Box::new(parent.clone()),
                                        "action".into(),
                                    ),
                                    label,
                                )
                                .variant(ButtonVariant::Primary)
                                .size(ControlSize::Small)
                                .on_click(move |_, w, cx| f(w, cx)),
                            ),
                        )
                    })
                    .when_some(close, |d, f| {
                        d.child(
                            Button::icon_only(
                                ElementId::NamedChild(Box::new(parent), "close".into()),
                                IconName::Dismiss,
                            )
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Small)
                            .on_click(move |_, w, cx| f(w, cx)),
                        )
                    }),
            )
    }
}

struct Entry {
    id: u64,
    data: ToastData,
    leaving: bool,
}

/// Toast stack host. Create with `cx.new(|_| Toaster::new())` and render it
/// once (it draws in a deferred top layer).
pub struct Toaster {
    toasts: Vec<Entry>,
    heights: HashMap<u64, Pixels>,
    next_id: u64,
    max_toasts: usize,
    material: bool,
}

impl Toaster {
    /// Empty toaster (3 visible toasts).
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            heights: HashMap::new(),
            next_id: 0,
            max_toasts: 3,
            material: false,
        }
    }

    /// Draw toasts as the Toast material (see [`Toast::material`]).
    pub fn material(mut self, material: bool) -> Self {
        self.material = material;
        self
    }

    /// Maximum toasts kept in the stack (web default is 1).
    pub fn max_toasts(mut self, n: usize) -> Self {
        self.max_toasts = n.max(1);
        self
    }

    /// Show a toast; returns its id.
    pub fn push(&mut self, data: ToastData, cx: &mut Context<Self>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        if let Some(duration) = data.duration {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(duration).await;
                this.update(cx, |this, cx| this.dismiss(id, cx)).ok();
            })
            .detach();
        }
        self.toasts.insert(
            0,
            Entry {
                id,
                data,
                leaving: false,
            },
        );
        // Older toasts beyond the limit leave.
        let live: Vec<u64> = self
            .toasts
            .iter()
            .filter(|e| !e.leaving)
            .map(|e| e.id)
            .collect();
        for old in live.into_iter().skip(self.max_toasts) {
            self.dismiss(old, cx);
        }
        cx.notify();
        id
    }

    /// Animate a toast out, then remove it.
    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(entry) = self.toasts.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entry.leaving {
            return;
        }
        entry.leaving = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(duration::TOAST_LEAVE + 20))
                .await;
            this.update(cx, |this, cx| {
                this.toasts.retain(|e| e.id != id);
                this.heights.remove(&id);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

impl Default for Toaster {
    fn default() -> Self {
        Self::new()
    }
}

const STACK_ID: &str = "refineui-toaster";

impl Render for Toaster {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let material = self.material;
        let hover_state = window.use_keyed_state("refineui-toaster-hover", cx, |_, _| false);
        if self.toasts.is_empty() {
            hover_state.update(cx, |h, _| *h = false);
            return div().into_any_element();
        }
        let gap = spacing::LARGE;
        let stack_id: ElementId = STACK_ID.into();
        // Hover is tracked with a window-level mouse listener over the stack
        // bounds: the cards occlude (so clicks don't fall through to the page),
        // which would hide a plain `:hover` on the stack itself.
        let expanded = *hover_state.read(cx);
        let live: Vec<&Entry> = self.toasts.iter().filter(|e| !e.leaving).collect();
        let front = live
            .first()
            .and_then(|e| self.heights.get(&e.id))
            .map(|h| f32::from(*h))
            .unwrap_or(48.);
        let step = gap * 0.85 + front * 0.035;

        // Target offsets for live toasts (leaving toasts keep their slot).
        let mut collapsed = HashMap::new();
        let mut expanded_off = HashMap::new();
        let mut y = 0.;
        for (ix, e) in live.iter().enumerate() {
            collapsed.insert(e.id, ix as f32 * step);
            expanded_off.insert(e.id, y);
            y += self
                .heights
                .get(&e.id)
                .map(|h| f32::from(*h))
                .unwrap_or(front)
                + gap;
        }
        let collapsed_h = live
            .iter()
            .enumerate()
            .map(|(ix, _)| ix as f32 * step + front)
            .fold(0., f32::max);
        let expanded_h = (y - gap).max(0.);
        let stack_h = animate(
            "refineui-toaster-h",
            if expanded { expanded_h } else { collapsed_h },
            Motion::TOAST_STACK,
            window,
            cx,
        );

        let this = cx.entity();
        let mut items: Vec<AnyElement> = Vec::new();
        // Paint back-to-front so the newest toast is on top.
        for (pos, e) in self.toasts.iter().enumerate().rev() {
            let id = e.id;
            let stack_ix = live.iter().position(|l| l.id == id).unwrap_or(pos);
            let target = if expanded {
                expanded_off.get(&id).copied()
            } else {
                collapsed.get(&id).copied()
            };
            let offset = animate(
                ("refineui-toast-y", id),
                target.unwrap_or(stack_ix as f32 * step),
                Motion::TOAST_STACK,
                window,
                cx,
            );
            let presence = animate_from(
                ("refineui-toast-p", id),
                0.0f32,
                if e.leaving { 0.0 } else { 1.0 },
                if e.leaving {
                    Motion::TOAST_LEAVE
                } else {
                    Motion::TOAST_ENTER
                },
                window,
                cx,
            );
            let stack_opacity = animate(
                ("refineui-toast-o", id),
                if expanded {
                    1.0
                } else {
                    1. - stack_ix as f32 * 0.12
                },
                Motion::new(
                    duration::SLOW,
                    refineui_foundation::motion::easing::EASE_IN_OUT,
                ),
                window,
                cx,
            );
            let own_h = self
                .heights
                .get(&id)
                .map(|h| f32::from(*h))
                .unwrap_or(front);
            let slide = -(1. - presence) * own_h;
            let measure = this.clone();
            let close_target = this.clone();
            let toast = Toast::new(("refineui-toast", id), e.data.clone()).material(material);
            let toast = if e.data.duration.is_none() {
                toast.on_close(move |_, cx| close_target.update(cx, |t, cx| t.dismiss(id, cx)))
            } else {
                toast
            };
            items.push(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(offset + slide))
                    .flex()
                    .justify_center()
                    .opacity((presence * stack_opacity).clamp(0., 1.))
                    .child(
                        div().relative().child(toast).child(
                            canvas(
                                move |b, window, cx| {
                                    let h = b.size.height;
                                    let changed = measure
                                        .update(cx, |t, _| t.heights.insert(id, h) != Some(h));
                                    if changed {
                                        window.refresh();
                                    }
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        ),
                    )
                    .into_any_element(),
            );
        }
        let viewport = window.viewport_size();
        let top = spacing::XXX_LARGE + spacing::LARGE;
        crate::layer(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .w(viewport.width)
                    .flex()
                    .justify_center()
                    .pt(px(top))
                    .child(
                        div()
                            .id(stack_id)
                            .relative()
                            .w(px(component::TOAST_MAX_WIDTH))
                            .max_w(viewport.width - px(2. * spacing::LARGE))
                            .h(px(stack_h.max(front)))
                            .children(items)
                            .child(
                                canvas(
                                    |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        let hover_state = hover_state.clone();
                                        window.on_mouse_event(
                                            move |e: &gpui::MouseMoveEvent, phase, window, cx| {
                                                if phase != gpui::DispatchPhase::Bubble {
                                                    return;
                                                }
                                                let inside = bounds.contains(&e.position);
                                                if *hover_state.read(cx) != inside {
                                                    hover_state.update(cx, |h, _| *h = inside);
                                                    window.refresh();
                                                }
                                            },
                                        );
                                    },
                                )
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full(),
                            ),
                    ),
            ),
        )
        .priority(3)
        .into_any_element()
    }
}
