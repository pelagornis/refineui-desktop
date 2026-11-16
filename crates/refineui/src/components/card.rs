//! Card — web `Card` (`elevated | outlined`; header, title, description, content, footer).

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use smallvec::SmallVec;

use refineui_tokens::{radius, spacing, typography::role};

use crate::motion::{Interaction, Motion, TrackInteraction, animate, child_id, use_interaction};
use crate::{ActiveTheme, Elevation, StyledExt};

/// Card surface style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CardVariant {
    /// Shadow (default).
    #[default]
    Elevated,
    /// Border, no shadow.
    Outlined,
}

/// Forced visual state (web `state` prop, e.g. for docs / previews).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CardState {
    /// Live (pointer-driven when interactive).
    #[default]
    Default,
    /// `backgroundPrimaryHover`.
    Hover,
    /// `backgroundSurfaceActive`.
    Pressed,
    /// Same as [`Card::disabled`].
    Disabled,
}

/// A grouped content surface.
///
/// Web anatomy: `backgroundPrimary`, `roundedXXLarge`, elevated = hairline
/// `borderSubtle` + `shadow-8` / outlined = thin `borderDefault`, no
/// shadow. Header `sizeXLarge ×
/// sizeLarge` with a `titleSm` title and `bodyMd` tertiary description;
/// content `sizeXLarge` sides + `sizeLarge` bottom; footer right-aligned,
/// `sizeXLarge × sizeMedium`. With [`Card::id`] + [`Card::on_click`] it is
/// interactive: hover `backgroundPrimaryHover`, pressed `surfaceSelected`,
/// selected = 2px inset `borderFocus` ring; transitions over `duration-fast`.
#[derive(IntoElement)]
pub struct Card {
    id: Option<gpui::ElementId>,
    on_click: Option<crate::ClickHandler>,
    selected: bool,
    disabled: bool,
    state: CardState,
    variant: CardVariant,
    eyebrow: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    action: Option<AnyElement>,
    footer: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
    padded: bool,
}

impl Card {
    /// Empty card.
    pub fn new() -> Self {
        Self {
            id: None,
            on_click: None,
            selected: false,
            disabled: false,
            state: CardState::Default,
            variant: CardVariant::Elevated,
            eyebrow: None,
            title: None,
            description: None,
            action: None,
            footer: None,
            children: SmallVec::new(),
            padded: true,
        }
    }

    /// Row above the title inside `CardHeaderMain` (web demo: badge +
    /// caption).
    pub fn eyebrow(mut self, el: impl IntoElement) -> Self {
        self.eyebrow = Some(el.into_any_element());
        self
    }

    /// Surface style.
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Header title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Header description.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Header trailing action (e.g. an icon button).
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }

    /// Footer row (e.g. buttons), right-aligned.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Element id (required for interactive cards).
    pub fn id(mut self, id: impl Into<gpui::ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Make the card clickable (needs [`Card::id`]).
    pub fn on_click(
        mut self,
        f: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(f));
        self
    }

    /// Selected (2px `borderFocus` inset ring).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disabled (`surfaceDisabled`, disabled text).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Force a visual state (web `state`): `Hover` / `Pressed` fills,
    /// `Disabled` = [`Card::disabled`].
    pub fn state(mut self, state: CardState) -> Self {
        self.state = state;
        if state == CardState::Disabled {
            self.disabled = true;
        }
        self
    }

    /// Remove inner padding (for full-bleed content such as tables).
    pub fn flush(mut self) -> Self {
        self.padded = false;
        self
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Card {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let has_header = self.eyebrow.is_some()
            || self.title.is_some()
            || self.description.is_some()
            || self.action.is_some();
        let interactive = self.id.is_some() && self.on_click.is_some() && !self.disabled;
        let id = self.id.clone().unwrap_or_else(|| "refineui-card".into());
        let (state, i) = if interactive {
            let (s, i) = use_interaction(&id, window, cx);
            (Some(s), i)
        } else {
            (None, Interaction::default())
        };
        let outlined = self.variant == CardVariant::Outlined;
        // Web: `state*` classes set the resting fill; interactive hover /
        // active classes override it.
        let resting = match self.state {
            CardState::Default => c.background_primary,
            CardState::Hover => c.background_primary_hover,
            CardState::Pressed => c.background_surface_active,
            CardState::Disabled => c.background_surface_disabled,
        };
        let bg = if self.disabled {
            c.background_surface_disabled
        } else if interactive {
            i.pick(
                resting,
                c.background_primary_hover,
                c.background_surface_selected,
            )
        } else {
            resting
        };
        // Outlined: `borderDefault`; interactive pressed `borderStrong`.
        let border = if interactive {
            i.pick(c.border_default, c.border_default, c.border_strong)
        } else {
            c.border_default
        };
        let (bg, border) = if interactive {
            (
                animate(child_id(&id, "bg"), bg, Motion::FAST, window, cx),
                animate(child_id(&id, "bd"), border, Motion::FAST, window, cx),
            )
        } else {
            (bg, border)
        };
        let ring = if interactive || self.selected {
            animate(
                child_id(&id, "sel"),
                if self.selected { 1.0f32 } else { 0.0 },
                Motion::FAST,
                window,
                cx,
            )
        } else {
            0.
        };
        let disabled = self.disabled;
        let title_fg = if disabled {
            c.foreground_disabled
        } else {
            c.foreground_primary
        };
        let desc_fg = if disabled {
            c.foreground_disabled
        } else {
            c.foreground_tertiary
        };
        let pad = self.padded;
        let on_click = self.on_click.clone();
        div()
            .id(id)
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(radius::XX_LARGE))
            .bg(bg)
            .text_color(title_fg)
            .map(|d| {
                if outlined {
                    d.border_1().border_color(border)
                } else {
                    d.border(px(refineui_tokens::stroke::HAIRLINE))
                        .border_color(c.border_subtle)
                        .shadow(theme.shadow(Elevation::FLOATING))
                }
            })
            .when_some(state, |d, s| d.cursor_action().track_interaction(&s))
            .when(interactive, |d| {
                d.when_some(on_click, |d, h| d.on_click(move |e, w, cx| h(e, w, cx)))
            })
            .when(has_header, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .justify_between()
                        .gap(px(spacing::MEDIUM))
                        .px(px(spacing::X_LARGE))
                        .py(px(spacing::LARGE))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap(px(spacing::XX_SMALL))
                                .when_some(self.eyebrow, |d, e| d.child(e))
                                .when_some(self.title, |d, t| {
                                    d.child(div().text_style_token(role::TITLE_SM).child(t))
                                })
                                .when_some(self.description, |d, t| {
                                    d.child(
                                        div()
                                            .text_style_token(role::BODY_MD)
                                            .text_color(desc_fg)
                                            .child(t),
                                    )
                                }),
                        )
                        .when_some(self.action, |d, a| {
                            d.child(
                                div()
                                    .flex()
                                    .flex_none()
                                    .items_center()
                                    .justify_end()
                                    .child(a),
                            )
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .when(disabled, |d| d.text_color(c.foreground_disabled))
                    .when(pad, |d| {
                        // Web `CardContent`: `px-x-large pb-large` (no top
                        // padding — the header provides it).
                        d.px(px(spacing::X_LARGE)).pb(px(spacing::LARGE))
                    })
                    .children(self.children),
            )
            .when_some(self.footer, |d, f| {
                d.child(
                    div()
                        .mt_auto()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_end()
                        .gap(px(spacing::X_SMALL))
                        .px(px(spacing::X_LARGE))
                        .py(px(spacing::MEDIUM))
                        .child(f),
                )
            })
            .when(ring > 0.001, |d| {
                d.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(px(radius::XX_LARGE))
                        .border(px(refineui_tokens::stroke::THICK))
                        .border_color(c.border_focus.opacity(ring)),
                )
            })
    }
}
