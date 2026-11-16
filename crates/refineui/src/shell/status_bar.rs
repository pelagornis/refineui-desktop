//! Status Bar — the window's bottom strip (brief §5 Application).
//!
//! Height from density (`status_bar_height`: 22 / 24 / 28),
//! `surfaceSunken` with a hairline `borderSubtle` top edge, `captionMd`
//! text in `foregroundSecondary`. Items sit in a leading group (context:
//! branch, problems, sync) and a trailing group (cursor position,
//! encoding, language, notifications). Item kinds:
//!
//! * [`StatusItem::text`] — static label, optional icon;
//! * [`StatusItem::button`] — clickable (hover `backgroundPrimaryHover`,
//!   `roundedSmall`), with an optional action tooltip;
//! * [`StatusItem::progress`] — a label with a thin determinate bar;
//! * [`StatusItem::custom`] — any element.
//!
//! An item can carry a [`StatusTone`] (`Warning` / `Danger` / `Accent`)
//! that tints its icon and text; [`StatusBar::accent`] turns the whole
//! bar `backgroundBrand` (e.g. remote / debugging session).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, relative,
};
use refineui_tokens::{radius, spacing, stroke, typography::role};

use crate::motion::{TrackInteraction, use_interaction};
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

    /// Brand-coloured bar.
    pub fn accent(mut self, accent: bool) -> Self {
        self.accent = accent;
        self
    }
}

fn render_item(
    bar: &ElementId,
    item: StatusItem,
    accent: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let fg: Hsla = if accent {
        c.foreground_on_brand
    } else {
        match item.tone {
            StatusTone::Neutral => c.foreground_secondary,
            StatusTone::Success => c.foreground_success,
            StatusTone::Warning => c.foreground_warning,
            StatusTone::Danger => c.foreground_error,
            StatusTone::Accent => c.foreground_brand,
        }
    };
    let id = ElementId::NamedChild(Box::new(bar.clone()), item.id.clone());
    let content = div()
        .flex()
        .items_center()
        .gap(px(spacing::XX_SMALL))
        .text_color(fg)
        .when_some(item.icon, |d, i| {
            d.child(Icon::new(i).size(IconSize::XSmall).color(fg))
        })
        .when_some(item.label.clone(), |d, l| {
            d.child(div().whitespace_nowrap().child(l))
        });
    let el = match item.kind {
        Kind::Text => div().id(id).px(px(spacing::XX_SMALL)).child(content),
        Kind::Custom(e) => div().id(id).px(px(spacing::XX_SMALL)).child(e),
        Kind::Progress(v) => div()
            .id(id)
            .px(px(spacing::XX_SMALL))
            .flex()
            .items_center()
            .gap(px(spacing::X_SMALL))
            .child(content)
            .child(
                div()
                    .w(px(64.))
                    .h(px(stroke::THICK * 2.))
                    .rounded_full()
                    .bg(if accent {
                        c.foreground_on_brand.opacity(0.3)
                    } else {
                        c.background_surface_selected
                    })
                    .child(div().h_full().w(relative(v)).rounded_full().bg(if accent {
                        c.foreground_on_brand
                    } else {
                        c.background_brand
                    })),
            ),
        Kind::Button(on_click) => {
            let (state, i) = use_interaction(&id, window, cx);
            let bg = if i.pressed {
                if accent {
                    c.foreground_on_brand.opacity(0.24)
                } else {
                    c.background_primary_active
                }
            } else if i.hovered {
                if accent {
                    c.foreground_on_brand.opacity(0.16)
                } else {
                    c.background_primary_hover
                }
            } else {
                gpui::transparent_black()
            };
            div()
                .id(id)
                .px(px(spacing::XX_SMALL))
                .h_full()
                .flex()
                .items_center()
                .rounded(px(radius::SMALL))
                .bg(bg)
                .cursor_action()
                .track_interaction(&state)
                .on_click(move |_, window, cx| on_click(window, cx))
                .child(content)
        }
    };
    match item.tooltip {
        Some((text, Some(action))) => el
            .tooltip(Tooltip::for_action(text, action))
            .into_any_element(),
        Some((text, None)) => el.tooltip(Tooltip::text(text)).into_any_element(),
        None => el.into_any_element(),
    }
}

impl RenderOnce for StatusBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let h = theme.density.desktop().status_bar_height;
        let accent = self.accent;
        let leading: Vec<AnyElement> = self
            .leading
            .into_iter()
            .map(|i| render_item(&self.id, i, accent, window, cx))
            .collect();
        let trailing: Vec<AnyElement> = self
            .trailing
            .into_iter()
            .map(|i| render_item(&self.id, i, accent, window, cx))
            .collect();
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .w_full()
            .h(px(h))
            .px(px(spacing::X_SMALL))
            .gap(px(spacing::XX_SMALL))
            .bg(if accent {
                c.background_brand
            } else {
                c.surface_sunken
            })
            .border_t(px(stroke::HAIRLINE))
            .border_color(if accent {
                c.background_brand
            } else {
                c.border_subtle
            })
            .text_style_token(role::CAPTION_MD)
            .font_family(theme.font_family.clone())
            .child(
                div()
                    .flex()
                    .h_full()
                    .py(px(2.))
                    .items_center()
                    .gap(px(spacing::XXX_SMALL))
                    .children(leading),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .h_full()
                    .py(px(2.))
                    .items_center()
                    .gap(px(spacing::XXX_SMALL))
                    .children(trailing),
            )
    }
}
