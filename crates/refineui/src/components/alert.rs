//! Alert — web `Alert` (`default | info | success | warning | danger | custom`).

use gpui::{
    AnyElement, App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, px,
};
use refineui_tokens::{radius, spacing, typography::role};
use smallvec::SmallVec;

use crate::{
    ActiveTheme, Button, ButtonVariant, ClickHandler, ControlSize, Icon, IconName, IconSize,
    StyledExt,
};

/// Alert intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AlertVariant {
    /// Neutral.
    Default,
    /// Blue (default).
    #[default]
    Info,
    /// Green.
    Success,
    /// Yellow.
    Warning,
    /// Red.
    Danger,
    /// Purple (`discovery`).
    Custom,
}

impl AlertVariant {
    /// All variants.
    pub const ALL: [AlertVariant; 6] = [
        Self::Default,
        Self::Info,
        Self::Success,
        Self::Warning,
        Self::Danger,
        Self::Custom,
    ];

    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Info => "Info",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Danger => "Danger",
            Self::Custom => "Custom",
        }
    }

    pub(crate) fn icon(self) -> IconName {
        match self {
            Self::Default => IconName::Circle,
            Self::Info => IconName::Info,
            Self::Success => IconName::Checkmark,
            Self::Warning => IconName::Warning,
            Self::Danger => IconName::ErrorCircle,
            Self::Custom => IconName::Star,
        }
    }

    pub(crate) fn accent(self, cx: &App) -> (Hsla, Hsla) {
        let c = &cx.theme().colors;
        match self {
            Self::Default => (c.foreground_primary, c.foreground_secondary),
            Self::Info => (c.foreground_info, c.foreground_info),
            Self::Success => (c.foreground_success, c.foreground_success),
            Self::Warning => (c.foreground_warning, c.foreground_warning),
            Self::Danger => (c.foreground_error, c.foreground_error),
            Self::Custom => (c.foreground_discovery, c.foreground_discovery),
        }
    }
}

/// Inline status message.
///
/// Web anatomy: `backgroundPrimary` root with a thin `borderDefault` border,
/// `roundedXLarge`, `sizeLarge × sizeMedium` padding; a 24px circular icon slot
/// in the variant color; title + description in `captionLg` medium; optional
/// ghost `sm` close button; actions are primary `sm` buttons right-aligned
/// beneath. Default variant is `info`, as on the web.
#[derive(IntoElement)]
pub struct Alert {
    id: ElementId,
    variant: AlertVariant,
    icon: Option<IconName>,
    title: SharedString,
    description: Option<SharedString>,
    actions: SmallVec<[AnyElement; 2]>,
    on_close: Option<ClickHandler>,
}

impl Alert {
    /// New alert (`info`).
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            variant: AlertVariant::Info,
            icon: None,
            title: title.into(),
            description: None,
            actions: SmallVec::new(),
            on_close: None,
        }
    }

    /// Intent.
    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Replace the variant icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Secondary text.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Action row element (web `AlertAction`: a primary `sm` [`crate::Button`]).
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    /// Show a close button.
    pub fn on_close(
        mut self,
        f: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(std::rc::Rc::new(f));
        self
    }
}

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (accent, desc) = self.variant.accent(cx);
        let close_id = ElementId::NamedChild(Box::new(self.id.clone()), "close".into());
        div()
            .id(self.id)
            .flex()
            .items_start()
            .w_full()
            .px(px(spacing::LARGE))
            .py(px(spacing::MEDIUM))
            .rounded(px(radius::X_LARGE))
            .bg(theme.colors.background_primary)
            .border_1()
            .border_color(theme.colors.border_default)
            .font_family(theme.font_family.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(spacing::MEDIUM))
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .items_start()
                            .gap(px(spacing::MEDIUM))
                            .child(
                                div()
                                    .flex()
                                    .flex_none()
                                    .size(px(spacing::XX_LARGE))
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    .overflow_hidden()
                                    .child(
                                        Icon::new(self.icon.unwrap_or(self.variant.icon()))
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
                                    .gap(px(spacing::XX_SMALL))
                                    .child(
                                        div()
                                            .text_style_token(role::CAPTION_LG)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(accent)
                                            .child(self.title),
                                    )
                                    .when_some(self.description, |d, t| {
                                        d.child(
                                            div()
                                                .text_style_token(role::CAPTION_LG)
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(desc)
                                                .child(t),
                                        )
                                    }),
                            )
                            .when_some(self.on_close, |d, handler| {
                                d.child(
                                    Button::icon_only(close_id, IconName::Dismiss)
                                        .variant(ButtonVariant::Ghost)
                                        .size(ControlSize::Small)
                                        .icon_size(IconSize::Small)
                                        .on_click(move |e, w, cx| handler(e, w, cx)),
                                )
                            }),
                    )
                    .when(!self.actions.is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_wrap()
                                .w_full()
                                .items_center()
                                .justify_end()
                                .gap(px(spacing::MEDIUM))
                                .children(self.actions),
                        )
                    }),
            )
    }
}
