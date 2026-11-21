//! NotificationCenter — the panel that collects an app's notifications
//! (desktop addition; toasts are transient, this is the history).
//!
//! A 360px column on a `Sheet` material (blur + tint + rim when
//! [`crate::material::materials_active`], otherwise the opaque floating
//! fill), with a header (title, unread count, "Clear All", close), a list of
//! [`Notification`] cards and a compact [`crate::ResultView`] (`Empty`) when empty.
//! Opening slides in from the trailing edge over [`crate::motion::Motion::SHEET_ENTER`]
//! (240ms, ease-out) with opacity; closing uses [`crate::motion::Motion::SHEET_LEAVE`]
//! (180ms, ease-in). The panel stays mounted until the exit finishes, and the
//! material blur travels with the plate (no delayed pop-in). While the sheet
//! is traveling, a rest-sized overflow-hidden slot clips blur kernels and
//! drop shadows so they cannot smear into the vacated region, and `.occlude()`
//! is lifted so the backdrop is repainted every frame (occluding a fading
//! plate would leave stale pixels behind). Reduced motion snaps.
//! Dismissing a notification slides and fades it out while the remaining cards
//! collapse their height; new items fade and slide in.
//!
//! Place it as an overlay anchored to the window's trailing edge (e.g.
//! `absolute().top(..).right(..)`), or inline with `inline(true)` (docs,
//! sidebars), which skips the slide.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, Window, canvas, div, prelude::*, px,
};
use refineui_foundation::{radius, shadow::Elevation, spacing, stroke, typography::role};

use crate::material::SurfaceKind;
use crate::motion::{Motion, animate, child_id, presence};
use crate::{
    ActiveTheme, Button, ButtonVariant, ControlSize, Icon, IconName, IconSize, ResultVariant,
    ResultView, StyledExt,
};

/// Tone of a notification's app glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotificationTone {
    /// Neutral (default).
    #[default]
    Neutral,
    /// Brand accent.
    Brand,
    /// Success.
    Success,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

/// One entry in a [`NotificationCenter`].
pub struct Notification {
    id: Option<SharedString>,
    title: SharedString,
    body: Option<SharedString>,
    source: Option<SharedString>,
    time: Option<SharedString>,
    icon: IconName,
    tone: NotificationTone,
    unread: bool,
    actions: Vec<AnyElement>,
}

impl Notification {
    /// Notification with a title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            id: None,
            title: title.into(),
            body: None,
            source: None,
            time: None,
            icon: IconName::Alert,
            tone: NotificationTone::Neutral,
            unread: false,
            actions: Vec::new(),
        }
    }

    /// Stable id used for enter / exit animation. Falls back to a fingerprint of the copy.
    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Body text.
    pub fn body(mut self, b: impl Into<SharedString>) -> Self {
        self.body = Some(b.into());
        self
    }

    /// Source (app or feature) name, shown above the title.
    pub fn source(mut self, s: impl Into<SharedString>) -> Self {
        self.source = Some(s.into());
        self
    }

    /// Relative time label ("2m", "Yesterday").
    pub fn time(mut self, t: impl Into<SharedString>) -> Self {
        self.time = Some(t.into());
        self
    }

    /// Glyph and its tone.
    pub fn icon(mut self, icon: IconName, tone: NotificationTone) -> Self {
        self.icon = icon;
        self.tone = tone;
        self
    }

    /// Mark unread (brand dot).
    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    /// Inline action (usually a small `Button`).
    pub fn action(mut self, a: impl IntoElement) -> Self {
        self.actions.push(a.into_any_element());
        self
    }
}

type IndexHandler = Rc<dyn Fn(&usize, &mut Window, &mut App)>;
type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Notification history panel.
#[derive(IntoElement)]
pub struct NotificationCenter {
    id: ElementId,
    title: SharedString,
    items: Vec<Notification>,
    open: bool,
    inline: bool,
    width: Pixels,
    max_height: Option<Pixels>,
    on_dismiss: Option<IndexHandler>,
    on_clear_all: Option<Handler>,
    on_close: Option<Handler>,
    /// When set, freeze the panel enter animation at this progress (docs / stills).
    sample: Option<f32>,
}

impl NotificationCenter {
    /// Empty, open panel titled "Notifications".
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: "Notifications".into(),
            items: Vec::new(),
            open: true,
            inline: false,
            width: px(360.),
            max_height: None,
            on_dismiss: None,
            on_clear_all: None,
            on_close: None,
            sample: None,
        }
    }

    /// Header title.
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = t.into();
        self
    }

    /// Add a notification (newest first is the caller's order).
    pub fn notification(mut self, n: Notification) -> Self {
        self.items.push(n);
        self
    }

    /// Add several notifications.
    pub fn notifications(mut self, n: impl IntoIterator<Item = Notification>) -> Self {
        self.items.extend(n);
        self
    }

    /// Shown (slides in from the trailing edge; stays mounted through the exit).
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Freeze the panel enter animation at `t` in `0..=1` (gallery / documentation stills).
    pub fn sample_progress(mut self, t: f32) -> Self {
        self.sample = Some(t.clamp(0., 1.));
        self
    }

    /// Lay out in flow, always shown, no slide.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Panel width (default 360px).
    pub fn width(mut self, w: impl Into<Pixels>) -> Self {
        self.width = w.into();
        self
    }

    /// Cap the height; the list scrolls.
    pub fn max_height(mut self, h: impl Into<Pixels>) -> Self {
        self.max_height = Some(h.into());
        self
    }

    /// Per-item dismiss (shows an × on each card).
    pub fn on_dismiss(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }

    /// "Clear All" in the header.
    pub fn on_clear_all(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_clear_all = Some(Rc::new(f));
        self
    }

    /// Close button in the header.
    pub fn on_close(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }
}

/// Horizontal travel of the sheet plate, in px, for a given presence
/// progress (`1` = rest, `0` = fully off-screen toward the trailing edge).
fn sheet_travel(progress: f32, width: f32) -> f32 {
    (1.0 - progress.clamp(0.0, 1.0)) * width
}

/// Whether the rest-sized slot should clip the traveling plate. While the
/// sheet is moving, an overflow-hidden slot of the resting bounds keeps
/// backdrop-blur kernels and drop shadows from smearing into the region
/// the plate just vacated. At rest the clip is lifted so shadows can
/// paint outside the plate. Inline panels never travel, so they never clip.
fn sheet_clips_travel(progress: f32, inline: bool) -> bool {
    !inline && progress < 1.0
}

fn notice_key(n: &Notification, ix: usize) -> SharedString {
    if let Some(id) = &n.id {
        return id.clone();
    }
    let body = n.body.as_ref().map(|s| s.as_ref()).unwrap_or("");
    let source = n.source.as_ref().map(|s| s.as_ref()).unwrap_or("");
    let time = n.time.as_ref().map(|s| s.as_ref()).unwrap_or("");
    format!("{}|{body}|{source}|{time}|{ix}", n.title.as_ref()).into()
}

impl Clone for Notification {
    fn clone(&self) -> Self {
        self.clone_visual()
    }
}

impl Notification {
    fn clone_visual(&self) -> Self {
        Self {
            id: self.id.clone(),
            title: self.title.clone(),
            body: self.body.clone(),
            source: self.source.clone(),
            time: self.time.clone(),
            icon: self.icon,
            tone: self.tone,
            unread: self.unread,
            actions: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct TrackedNotice {
    key: SharedString,
    item: Notification,
    index: usize,
    leaving: bool,
}

fn tone_colors(tone: NotificationTone, cx: &App) -> (gpui::Hsla, gpui::Hsla) {
    let theme = cx.theme();
    let c = theme.colors;
    // Dark themes: a wash of the ink instead of the saturated subtle fills.
    let dark = theme.is_dark();
    let wash = |ink: gpui::Hsla, subtle: gpui::Hsla| {
        if dark { ink.opacity(0.16) } else { subtle }
    };
    match tone {
        NotificationTone::Neutral => (c.foreground_secondary, c.background_surface_hover),
        NotificationTone::Brand => (
            c.foreground_brand,
            wash(c.foreground_brand, c.background_brand_subtle),
        ),
        NotificationTone::Success => (
            c.foreground_success,
            wash(c.foreground_success, c.background_success_subtle),
        ),
        NotificationTone::Warning => (
            c.foreground_warning,
            wash(c.foreground_warning, c.background_warning_subtle),
        ),
        NotificationTone::Error => (
            c.foreground_error,
            wash(c.foreground_error, c.background_error_subtle),
        ),
    }
}

fn render_card(
    n: Notification,
    ix: usize,
    dismiss: Option<IndexHandler>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    let theme = cx.theme();
    let c = theme.colors;
    let (glyph, plate_bg) = tone_colors(n.tone, cx);
    div()
        .id(ElementId::NamedInteger("notification".into(), ix as u64))
        .group("notification")
        .relative()
        .flex()
        .gap(px(spacing::SMALL))
        .p(px(spacing::SMALL))
        .rounded(px(radius::X_LARGE))
        .bg(c
            .background_primary
            .opacity(if n.unread { 0.72 } else { 0.45 }))
        .border(px(stroke::HAIRLINE))
        .border_color(c.border_subtle)
        .child(
            div()
                .flex_none()
                .size(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(radius::LARGE))
                .bg(plate_bg)
                .child(
                    Icon::new(n.icon)
                        .size(IconSize::Custom(px(16.)))
                        .color(glyph),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(spacing::XX_SMALL))
                        .text_style_token(role::CAPTION_MD)
                        .text_color(c.foreground_tertiary)
                        .when_some(n.source, |d, s| d.child(div().flex_1().truncate().child(s)))
                        .when_some(n.time, |d, t| d.child(t))
                        .when(n.unread, |d| {
                            d.child(
                                div()
                                    .size(px(6.))
                                    .rounded(px(radius::CIRCLE))
                                    .bg(c.background_brand),
                            )
                        }),
                )
                .child(
                    div()
                        .text_style_token(role::BODY_MD)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(c.foreground_primary)
                        .child(n.title),
                )
                .when_some(n.body, |d, b| {
                    d.child(
                        div()
                            .text_style_token(role::BODY_SM)
                            .text_color(c.foreground_secondary)
                            .child(b),
                    )
                })
                .when(!n.actions.is_empty(), |d| {
                    d.child(
                        div()
                            .pt(px(spacing::XX_SMALL))
                            .flex()
                            .gap(px(spacing::X_SMALL))
                            .children(n.actions),
                    )
                }),
        )
        .when_some(dismiss, |d, f| {
            d.child(
                div()
                    .id(ElementId::NamedInteger("dismiss".into(), ix as u64))
                    .absolute()
                    .top(px(spacing::X_SMALL))
                    .right(px(spacing::X_SMALL))
                    .p(px(2.))
                    .rounded(px(radius::SMALL))
                    .invisible()
                    .group_hover("notification", |s| s.visible())
                    .hover(|s| s.bg(c.background_surface_hover))
                    .cursor_action()
                    .on_click(move |_, w, cx| f(&ix, w, cx))
                    .child(
                        Icon::new(IconName::Dismiss)
                            .size(IconSize::Custom(px(12.)))
                            .color(c.foreground_tertiary),
                    ),
            )
        })
}

impl RenderOnce for NotificationCenter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let width = f32::from(self.width);
        let (progress, mounted) = if self.inline {
            (1.0, true)
        } else if let Some(t) = self.sample {
            (t, true)
        } else {
            let p = presence(
                child_id(&self.id, "slide"),
                self.open,
                Motion::SHEET_ENTER,
                Motion::SHEET_LEAVE,
                window,
                cx,
            );
            (p.progress.clamp(0., 1.), p.mounted)
        };
        if !mounted {
            return div().id(self.id).into_any_element();
        }

        let theme = cx.theme();
        let c = theme.colors;
        let mut shadows = theme.shadow(Elevation::Shadow16);
        if self.inline {
            shadows = theme.shadow(Elevation::Shadow8);
        }
        let plate = crate::material::backing(
            SurfaceKind::Sheet,
            px(radius::XX_LARGE),
            stroke::HAIRLINE,
            theme.floating_fill(),
            Some(c.border_default),
            shadows,
            cx,
        );
        let unread = self.items.iter().filter(|n| n.unread).count();
        let count = self.items.len();
        let id = self.id.clone();

        // Track live + leaving items so dismiss can fade/slide out and height can collapse.
        let incoming: Vec<(SharedString, Notification, usize)> = self
            .items
            .into_iter()
            .enumerate()
            .map(|(ix, n)| (notice_key(&n, ix), n, ix))
            .collect();
        let incoming_keys: Vec<SharedString> = incoming.iter().map(|(k, _, _)| k.clone()).collect();
        let track = window.use_keyed_state(child_id(&id, "track"), cx, |_, _| {
            Vec::<TrackedNotice>::new()
        });
        track.update(cx, |track, _| {
            for row in track.iter_mut() {
                if !incoming_keys.contains(&row.key) {
                    row.leaving = true;
                }
            }
            for (key, item, index) in &incoming {
                if let Some(row) = track.iter_mut().find(|r| &r.key == key) {
                    if !row.leaving {
                        row.item = item.clone_visual();
                        // Keep live actions on the incoming value by replacing when still live.
                        row.index = *index;
                    }
                } else {
                    track.push(TrackedNotice {
                        key: key.clone(),
                        item: item.clone_visual(),
                        index: *index,
                        leaving: false,
                    });
                }
            }
        });
        // Swap live items back in (with actions) for cards still present.
        let mut live_map: HashMap<SharedString, Notification> =
            incoming.into_iter().map(|(k, n, _)| (k, n)).collect();
        let tracked = track.read(cx).clone();

        let heights = window.use_keyed_state(child_id(&id, "heights"), cx, |_, _| {
            HashMap::<SharedString, f32>::new()
        });

        let header = div()
            .flex()
            .items_center()
            .gap(px(spacing::X_SMALL))
            .px(px(spacing::LARGE))
            .pt(px(spacing::MEDIUM))
            .pb(px(spacing::X_SMALL))
            .child(
                div()
                    .text_style_token(role::SUBTITLE_MD)
                    .text_color(c.foreground_primary)
                    .child(self.title.clone()),
            )
            .when(unread > 0, |d| {
                d.child(
                    div()
                        .px(px(spacing::X_SMALL))
                        .rounded(px(radius::CIRCLE))
                        .bg(c.background_brand)
                        .text_style_token(role::CAPTION_MD)
                        .text_color(c.foreground_on_brand)
                        .child(unread.to_string()),
                )
            })
            .child(div().flex_1())
            .when_some(self.on_clear_all.filter(|_| count > 0), |d, f| {
                d.child(
                    Button::new(child_id(&id, "clear"), "Clear All")
                        .size(ControlSize::Small)
                        .variant(ButtonVariant::Ghost)
                        .on_click(move |e, w, cx| f(e, w, cx)),
                )
            })
            .when_some(self.on_close, |d, f| {
                d.child(
                    Button::icon_only(child_id(&id, "close"), IconName::Dismiss)
                        .size(ControlSize::Small)
                        .variant(ButtonVariant::Ghost)
                        .on_click(move |e, w, cx| f(e, w, cx)),
                )
            });

        let dismiss = self.on_dismiss.clone();
        let mut still_leaving = Vec::new();
        let mut cards: Vec<AnyElement> = Vec::new();
        for row in tracked {
            let item_motion_id =
                ElementId::NamedChild(Box::new(id.clone()), format!("item-{}", row.key).into());
            let p = presence(
                item_motion_id,
                !row.leaving,
                Motion::TOAST_ENTER,
                Motion::TOAST_LEAVE,
                window,
                cx,
            );
            if row.leaving && !p.mounted {
                continue;
            }
            if row.leaving {
                still_leaving.push(row.key.clone());
            }
            let measured = heights.read(cx).get(&row.key).copied().unwrap_or(0.);
            let target_h = if row.leaving { 0. } else { measured };
            let h = if measured > 0. {
                let h_id =
                    ElementId::NamedChild(Box::new(id.clone()), format!("h-{}", row.key).into());
                animate(h_id, target_h, Motion::TOAST_LEAVE, window, cx)
            } else {
                0.
            };
            let item = live_map.remove(&row.key).unwrap_or(row.item);
            let ix = row.index;
            let key = row.key.clone();
            let heights_e = heights.clone();
            let card_dismiss = if row.leaving { None } else { dismiss.clone() };
            let card = render_card(item, ix, card_dismiss, cx).child(
                canvas(
                    move |b, window, cx| {
                        let next = f32::from(b.size.height);
                        let changed =
                            heights_e.update(cx, |m, _| m.insert(key.clone(), next) != Some(next));
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
            );
            let slide = (1.0 - p.progress) * 16.0;
            let mut wrap = div()
                .overflow_hidden()
                .opacity(p.progress.clamp(0., 1.))
                .ml(px(slide))
                .child(card);
            if measured > 0. && (row.leaving || (h - measured).abs() > 0.5) {
                wrap = wrap.h(px(h.max(0.)));
            }
            cards.push(wrap.into_any_element());
        }
        track.update(cx, |track, _| {
            track.retain(|row| !row.leaving || still_leaving.contains(&row.key));
        });

        let empty = count == 0 && still_leaving.is_empty();
        let list = div()
            .id(child_id(&id, "list"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(spacing::X_SMALL))
            .px(px(spacing::SMALL))
            .pb(px(spacing::SMALL))
            .children(cards)
            .when(empty, |d| {
                d.child(
                    ResultView::new(ResultVariant::Empty)
                        .icon(IconName::Alert)
                        .title("No Notifications")
                        .description("You're all caught up.")
                        .compact(true),
                )
            });

        // Full-width slide from the trailing edge; opacity follows the travel.
        // The material layer is the first child of the clipped inner plate so
        // blur moves with the panel instead of appearing after it settles.
        // The outer slot occupies the resting bounds for the whole motion;
        // while traveling it clips smear from blur kernels / shadows, and
        // `.occlude()` is applied only at rest so a fading plate does not
        // punch a stale hole in the backdrop.
        let dx = sheet_travel(progress, width);
        let traveling = sheet_clips_travel(progress, self.inline);
        let plate_el = div()
            .relative()
            .flex()
            .flex_col()
            .w(self.width)
            .when_some(self.max_height, |d, h| d.max_h(h))
            .when(!self.inline, |d| d.h_full())
            .overflow_hidden()
            .rounded(px(radius::XX_LARGE))
            .border(px(stroke::HAIRLINE))
            .border_color(plate.border)
            .bg(plate.fill)
            .shadow(plate.shadows)
            .children(plate.layer)
            .when(!traveling, |d| d.occlude())
            .ml(px(dx))
            .opacity(progress)
            .text_color(c.foreground_primary)
            .child(header)
            .child(list);
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .w(self.width)
            .when_some(self.max_height, |d, h| d.max_h(h))
            .when(!self.inline, |d| d.h_full())
            .when(traveling, |d| d.overflow_hidden())
            .child(plate_el)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_travel_and_clip_prevent_vacated_smear() {
        assert_eq!(sheet_travel(1., 360.), 0.);
        assert_eq!(sheet_travel(0., 360.), 360.);
        assert!((sheet_travel(0.5, 360.) - 180.).abs() < f32::EPSILON);
        assert!(!sheet_clips_travel(1., false));
        assert!(sheet_clips_travel(0.45, false));
        assert!(sheet_clips_travel(0., false));
        // Inline panels are already in-flow: never clip, never travel.
        assert!(!sheet_clips_travel(0.45, true));
        assert!(!sheet_clips_travel(1., true));
        assert_eq!(sheet_travel(1.5, 360.), 0.);
        assert_eq!(sheet_travel(-0.2, 360.), 360.);
    }
}
