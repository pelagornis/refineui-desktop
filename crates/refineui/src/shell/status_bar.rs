//! Status Bar — the window's bottom strip (brief §5 Application).
//!
//! RefineUI look (not an IDE strip): the bar sits on `backgroundPrimary`
//! like the content above it, separated by a hairline `borderSubtle`; it is
//! roomier than a classic status line (density `status_bar_height` + 8 →
//! 30 / 32 / 36). Items are soft chips — `roundedMedium`, `px-small`,
//! `captionLg` in `foregroundSecondary`; clickable chips fill
//! `backgroundSurfaceHover` / `backgroundSurfaceActive` (motion `fast`).
//! Tones tint only the icon (or a 6px dot when there is none), never the
//! whole bar. Item kinds:
//!
//! * [`StatusItem::text`] — static label, optional icon;
//! * [`StatusItem::button`] — clickable chip with an optional action tooltip;
//! * [`StatusItem::progress`] — a label with a rounded determinate track;
//! * [`StatusItem::custom`] — any element.
//!
//! [`StatusBar::accent`] marks a special session (remote, debugging): a 2px
//! brand top rule and the first leading item as a `backgroundBrandSubtle`
//! pill — instead of painting the whole bar brand.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, relative,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::motion::{TrackInteraction, child_id, use_interaction};
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt, Tooltip};

/// Semantic tint of a status item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusTone {
    /// Secondary foreground.
    #[default]
    Neutral,
    /// Success foreground.
    Success,
    /// Warning foreground.
    Warning,
    /// Error foreground.
    Danger,
    /// Brand foreground.
    Accent,
}

type ClickHandler = Rc<dyn Fn(&mut Window, &mut App)>;

enum Kind {
    Text,
    Button(ClickHandler),
    Progress(f32),
    Custom(AnyElement),
}

/// One status-bar item.
pub struct StatusItem {
    id: SharedString,
    label: Option<SharedString>,
    icon: Option<IconName>,
    tone: StatusTone,
    tooltip: Option<(SharedString, Option<SharedString>)>,
    kind: Kind,
}

impl StatusItem {
    fn base(id: impl Into<SharedString>, kind: Kind) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            tone: StatusTone::Neutral,
            tooltip: None,
            kind,
        }
    }

    /// Static label.
    pub fn text(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let mut s = Self::base(id, Kind::Text);
        s.label = Some(label.into());
        s
    }

    /// Clickable item.
    pub fn button(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        let mut s = Self::base(id, Kind::Button(Rc::new(on_click)));
        s.label = Some(label.into());
        s
    }

    /// Label with a determinate progress bar (`0.0..=1.0`).
    pub fn progress(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f32,
    ) -> Self {
        let mut s = Self::base(id, Kind::Progress(value.clamp(0., 1.)));
        s.label = Some(label.into());
        s
    }

    /// Any element.
    pub fn custom(id: impl Into<SharedString>, element: impl IntoElement) -> Self {
        Self::base(id, Kind::Custom(element.into_any_element()))
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Tint.
    pub fn tone(mut self, tone: StatusTone) -> Self {
        self.tone = tone;
        self
    }

    /// Hover tooltip, with the shortcut of `action` when given.
    pub fn tooltip(mut self, text: impl Into<SharedString>, action: Option<&str>) -> Self {
        self.tooltip = Some((text.into(), action.map(|a| a.to_string().into())));
        self
    }
}

/// The status bar. See the module docs.
#[derive(IntoElement)]
pub struct StatusBar {
    id: ElementId,
    leading: Vec<StatusItem>,
    trailing: Vec<StatusItem>,
    accent: bool,
}

impl StatusBar {
    /// Empty bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            leading: Vec::new(),
            trailing: Vec::new(),
            accent: false,
        }
    }

    /// Add to the leading group.
    pub fn leading(mut self, item: StatusItem) -> Self {
        self.leading.push(item);
        self
    }

    /// Add to the trailing group.
    pub fn trailing(mut self, item: StatusItem) -> Self {
        self.trailing.push(item);
        self
    }

    /// Session accent (brand top rule + brand first item).
    pub fn accent(mut self, accent: bool) -> Self {
        self.accent = accent;
        self
    }
}

/// `(height, chip height)` for a density's status line.
pub(crate) fn status_metrics(base: f32) -> (f32, f32) {
    (base + 8., base)
}

fn tone_color(tone: StatusTone, c: &crate::ThemeColors) -> Option<Hsla> {
    match tone {
        StatusTone::Neutral => None,
        StatusTone::Success => Some(c.foreground_success),
        StatusTone::Warning => Some(c.foreground_warning),
        StatusTone::Danger => Some(c.foreground_error),
        StatusTone::Accent => Some(c.foreground_brand),
    }
}

fn render_item(
    bar: &ElementId,
    item: StatusItem,
    pill: bool,
    chip_h: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let tint = tone_color(item.tone, &c);
    let fg = if pill {
        c.foreground_brand
    } else {
        c.foreground_secondary
    };
    let icon_fg = if pill {
        c.foreground_brand
    } else {
        tint.unwrap_or(fg)
    };
    let id = ElementId::NamedChild(Box::new(bar.clone()), item.id.clone());
    let tip_id = child_id(&id, "tip");
    let content = div()
        .flex()
        .items_center()
        .gap(px(spacing::XX_SMALL))
        .text_color(fg)
        .map(|d| match (item.icon, tint) {
            (Some(i), _) => d.child(Icon::new(i).size(IconSize::XSmall).color(icon_fg)),
            (None, Some(t)) if !pill => d.child(div().size(px(6.)).rounded_full().bg(t)),
            _ => d,
        })
        .when_some(item.label.clone(), |d, l| {
            d.child(div().whitespace_nowrap().child(l))
        });
    let chip = |d: gpui::Stateful<gpui::Div>| {
        d.h(px(chip_h))
            .px(px(spacing::SMALL))
            .flex()
            .items_center()
            .rounded(px(radius::MEDIUM))
            .when(pill, |d| d.bg(c.background_brand_subtle))
    };
    let el = match item.kind {
        Kind::Text => chip(div().id(id)).child(content),
        Kind::Custom(e) => chip(div().id(id)).child(e),
        Kind::Progress(v) => chip(div().id(id))
            .gap(px(spacing::X_SMALL))
            .child(content)
            .child(
                div()
                    .w(px(72.))
                    .h(px(4.))
                    .rounded_full()
                    .bg(c.background_surface_selected)
                    .child(
                        div()
                            .h_full()
                            .w(relative(v.clamp(0., 1.)))
                            .rounded_full()
                            .bg(c.background_brand),
                    ),
            ),
        Kind::Button(on_click) => {
            let (state, i) = use_interaction(&id, window, cx);
            let bg = if i.pressed {
                c.background_surface_active
            } else if i.hovered {
                c.background_surface_hover
            } else if pill {
                c.background_brand_subtle
            } else {
                gpui::transparent_black()
            };
            let bg = crate::motion::animate(
                crate::motion::child_id(&id, "bg"),
                bg,
                crate::motion::Motion::FAST,
                window,
                cx,
            );
            chip(div().id(id))
                .bg(bg)
                .cursor_action()
                .track_interaction(&state)
                .on_click(move |_, window, cx| on_click(window, cx))
                .child(content)
        }
    };
    match item.tooltip {
        Some((text, action)) => {
            let mut tip = Tooltip::new(tip_id, text);
            if let Some(action) = action {
                tip = tip.action(action);
            }
            tip.child(el).into_any_element()
        }
        None => el.into_any_element(),
    }
}

impl RenderOnce for StatusBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let (h, chip_h) = status_metrics(theme.density.desktop().status_bar_height);
        let accent = self.accent;
        let leading: Vec<AnyElement> = self
            .leading
            .into_iter()
            .enumerate()
            .map(|(ix, i)| render_item(&self.id, i, accent && ix == 0, chip_h, window, cx))
            .collect();
        let trailing: Vec<AnyElement> = self
            .trailing
            .into_iter()
            .map(|i| render_item(&self.id, i, false, chip_h, window, cx))
            .collect();
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(px(h))
            .px(px(spacing::X_SMALL))
            .gap(px(spacing::SMALL))
            .map(|d| {
                let (fill, layer) =
                    crate::material::chrome(crate::SurfaceKind::Titlebar, c.background_primary, cx);
                d.bg(fill).children(layer)
            })
            .border_t(px(stroke::HAIRLINE))
            .border_color(c.border_subtle)
            .when(accent, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(-stroke::HAIRLINE))
                        .left_0()
                        .right_0()
                        .h(px(stroke::THICK))
                        .bg(c.background_brand),
                )
            })
            .text_style_token(role::CAPTION_LG)
            .font_family(theme.font_family.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::XXX_SMALL))
                    .children(leading),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::XXX_SMALL))
                    .children(trailing),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bar_is_roomier_than_an_ide_strip() {
        assert_eq!(status_metrics(22.), (30., 22.));
        assert_eq!(status_metrics(28.), (36., 28.));
    }
}
