//! Button — web `Button` (`primary | secondary | outline | ghost`, `sm | md | lg`, label / icon layout).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, CursorStyle, ElementId, Hsla, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use refineui_tokens::{spacing, stroke};

use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, is_scaled,
    scaled_plate, use_interaction,
};
use crate::{
    ActiveTheme, ControlSize, Icon, IconName, IconSize, Spinner, SpinnerSize, StyledExt, Tooltip,
};

/// Click callback shared by interactive components.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Visual style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    /// Filled brand (default).
    #[default]
    Primary,
    /// Subtle filled.
    Secondary,
    /// Bordered.
    Outline,
    /// Transparent until hovered.
    Ghost,
}

impl ButtonVariant {
    /// All variants, for galleries and tests.
    pub const ALL: [ButtonVariant; 4] =
        [Self::Primary, Self::Secondary, Self::Outline, Self::Ghost];

    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary",
            Self::Outline => "Outline",
            Self::Ghost => "Ghost",
        }
    }
}

/// Resolved colors for one interaction state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ButtonPaint {
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
}

/// `componentColorTokens.button` + the `:hover` / `:active` rules in `refineui.css`.
pub(crate) fn button_paint(
    variant: ButtonVariant,
    icon_only: bool,
    disabled: bool,
    state: Interaction,
    cx: &App,
) -> ButtonPaint {
    let c = &cx.theme().colors;
    let clear = gpui::transparent_black();
    if disabled {
        return match variant {
            ButtonVariant::Primary => ButtonPaint {
                bg: c.background_brand_disabled,
                fg: c.foreground_disabled,
                border: clear,
            },
            ButtonVariant::Secondary => ButtonPaint {
                bg: c.background_surface_disabled,
                fg: c.foreground_disabled,
                border: clear,
            },
            ButtonVariant::Outline => ButtonPaint {
                bg: clear,
                fg: c.foreground_disabled,
                border: c.border_disabled,
            },
            ButtonVariant::Ghost => ButtonPaint {
                bg: clear,
                fg: c.foreground_disabled,
                border: clear,
            },
        };
    }
    match variant {
        ButtonVariant::Primary => ButtonPaint {
            bg: state.pick(
                c.background_brand,
                c.background_brand_hover,
                c.background_brand_active,
            ),
            fg: c.foreground_on_brand,
            border: clear,
        },
        ButtonVariant::Secondary => ButtonPaint {
            bg: state.pick(
                c.background_surface_hover,
                c.background_surface_active,
                c.background_surface_active,
            ),
            fg: state.pick(
                c.foreground_primary,
                c.foreground_primary_hover,
                c.foreground_primary,
            ),
            border: clear,
        },
        ButtonVariant::Outline => ButtonPaint {
            bg: clear,
            fg: state.pick(
                c.foreground_primary,
                c.foreground_primary_hover,
                c.foreground_primary,
            ),
            border: state.pick(c.border_default, c.border_strong, c.border_strong),
        },
        ButtonVariant::Ghost => ButtonPaint {
            bg: state.pick(
                clear,
                c.background_primary_hover,
                c.background_primary_active,
            ),
            // Ghost icon buttons rest on `foregroundSecondary` (compound variant).
            fg: state.pick(
                if icon_only {
                    c.foreground_secondary
                } else {
                    c.foreground_primary
                },
                c.foreground_primary_hover,
                c.foreground_primary,
            ),
            border: clear,
        },
    }
}

/// A push button. Focusable (Tab), activates on click, Enter or Space.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<IconName>,
    icon_end: Option<IconName>,
    variant: Option<ButtonVariant>,
    size: Option<ControlSize>,
    disabled: bool,
    loading: bool,
    selected: bool,
    full_width: bool,
    tooltip: Option<SharedString>,
    tooltip_action: Option<SharedString>,
    tab_index: isize,
    icon_size: Option<IconSize>,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// Labelled button.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: Some(label.into()),
            icon: None,
            icon_end: None,
            variant: None,
            size: None,
            disabled: false,
            loading: false,
            selected: false,
            full_width: false,
            tooltip: None,
            tooltip_action: None,
            tab_index: 0,
            icon_size: None,
            on_click: None,
        }
    }

    /// Icon-only button (web `layout="icon"`). Give it a [`Self::tooltip`] for accessibility.
    pub fn icon_only(id: impl Into<ElementId>, icon: IconName) -> Self {
        let mut this = Self::new(id, "");
        this.label = None;
        this.icon = Some(icon);
        this
    }

    /// Visual variant.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Size / variant a [`crate::ButtonGroup`] or [`crate::Toolbar`] cascades:
    /// applied only where the button did not set its own (web context).
    pub(crate) fn inherit(
        mut self,
        size: Option<ControlSize>,
        variant: Option<ButtonVariant>,
    ) -> Self {
        if self.size.is_none() {
            self.size = size;
        }
        if self.variant.is_none() {
            self.variant = variant;
        }
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Trailing icon.
    pub fn icon_end(mut self, icon: IconName) -> Self {
        self.icon_end = Some(icon);
        self
    }

    /// Disable interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show a spinner and ignore clicks (web `data-state="loading"`).
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Pressed / toggled-on appearance (used by toolbars and [`crate::ToggleButton`]).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Stretch to the parent's width.
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Hover tooltip that also shows the shortcut bound to a GPUI action
    /// (from the installed keymap, in the platform's notation), e.g.
    /// `.tooltip("Toggle Sidebar").tooltip_action("workspace::ToggleSidebar")`.
    pub fn tooltip_action(mut self, action: impl Into<SharedString>) -> Self {
        self.tooltip_action = Some(action.into());
        self
    }

    /// Hover tooltip.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Override the icon size (defaults to the size's metric: 16 / 20 / 24).
    pub fn icon_size(mut self, size: IconSize) -> Self {
        self.icon_size = Some(size);
        self
    }

    /// Tab order (default `0`).
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Click / Enter / Space handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let size = self.size.unwrap_or_default();
        let variant = self.variant.unwrap_or_default();
        let inactive = self.disabled || self.loading;
        let icon_only = self.label.is_none();
        let m = theme.button_metrics(size);
        let (state, interaction) = use_interaction(&self.id, window, cx);
        let interaction = if inactive {
            Interaction::default()
        } else {
            interaction
        };
        let mut paint = button_paint(variant, icon_only, self.disabled, interaction, cx);
        if self.selected && !self.disabled {
            paint.bg = theme.colors.background_primary_active;
            paint.fg = theme.colors.foreground_primary;
        }
        // CSS: `transition: background-color, border-color, color var(--duration-fast) ease-out`.
        let bg = animate(child_id(&self.id, "bg"), paint.bg, Motion::FAST, window, cx);
        let fg = animate(child_id(&self.id, "fg"), paint.fg, Motion::FAST, window, cx);
        let border = animate(
            child_id(&self.id, "border"),
            paint.border,
            Motion::FAST,
            window,
            cx,
        );
        let icon_px = self.icon_size.map(|s| f32::from(s.px())).unwrap_or(m.icon);
        // Web `buttonActive` = `scale(0.96)` (deeper than generic controls' 0.98)
        // over `duration-fast` ease.
        let scale = crate::motion::press_scale_to(
            child_id(&self.id, "press"),
            interaction.pressed,
            refineui_tokens::motion::scale::BUTTON_PRESS,
            window,
            cx,
        );
        let scaled = is_scaled(scale);
        let text = m.text;
        let (loading, icon, label, icon_end) =
            (self.loading, self.icon, self.label.clone(), self.icon_end);
        let content = move |s: f32| -> Vec<AnyElement> {
            let icon_size = IconSize::Custom(px(icon_px * s));
            let mut children: Vec<AnyElement> = Vec::new();
            if loading {
                children.push(
                    Spinner::new()
                        .size(SpinnerSize::XSmall)
                        .color(fg)
                        .into_any_element(),
                );
            } else if let Some(icon) = icon {
                children.push(Icon::new(icon).size(icon_size).color(fg).into_any_element());
            }
            if let Some(label) = label.clone() {
                children.push(
                    div()
                        .text_size(px(text.size * s))
                        .line_height(px(text.line_height * s))
                        .child(label)
                        .into_any_element(),
                );
            }
            if let Some(icon) = icon_end {
                children.push(Icon::new(icon).size(icon_size).color(fg).into_any_element());
            }
            children
        };

        let focus_ring = theme.focus_ring();
        let show_focus = focus_visible(cx);
        let on_click = self.on_click.clone();
        let tooltip = self.tooltip.clone();
        let tooltip_action = self.tooltip_action.clone();
        // Icon layout on filled variants drops the stroke (`border-0`).
        let stroke = !(icon_only && variant != ButtonVariant::Outline);

        // Stroke lives on the plate; the layout box reserves it as padding.
        let bw = if stroke { stroke::THIN } else { 0. };
        let gap = px(spacing::XX_SMALL);
        let (pad_x, pad_y) = if icon_only {
            let p = match size {
                ControlSize::Small => spacing::XX_SMALL,
                ControlSize::Medium => spacing::X_SMALL,
                ControlSize::Large => spacing::MEDIUM,
            };
            (p, p)
        } else {
            (m.padding_x, m.padding_y)
        };
        let plate = scaled_plate(scale)
            .rounded(px(m.radius * scale))
            .bg(bg)
            .when(stroke, |p| p.border_1().border_color(border));

        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(gap)
            .min_h(px(m.height))
            .when(icon_only, |this| this.min_w(px(m.height)))
            .px(px(pad_x + bw))
            .py(px(pad_y + bw))
            .when(self.full_width, |this| this.w_full())
            .rounded(px(m.radius))
            .text_color(fg)
            .text_style_token(m.text)
            .whitespace_nowrap()
            .when(!inactive, |this| {
                this.cursor_action()
                    .track_interaction(&state)
                    .tab_index(self.tab_index)
                    .when(show_focus, |this| this.focus(move |s| s.shadow(focus_ring)))
                    .when_some(on_click, |this, handler| {
                        this.on_click(move |event, window, cx| handler(event, window, cx))
                    })
            })
            .when(inactive, |this| {
                this.cursor(CursorStyle::OperationNotAllowed)
            })
            .when_some(tooltip, |this, text| match tooltip_action {
                Some(a) => this.tooltip(Tooltip::for_action(text, a)),
                None => this.tooltip(Tooltip::text(text)),
            })
            .child(plate)
            .map(|this| {
                if scaled {
                    // Ghost content keeps the layout size; the scaled copy is
                    // centered on top, like a CSS transform about the center.
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(gap)
                            .opacity(0.)
                            .children(content(1.0)),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(gap * scale)
                            .children(content(scale)),
                    )
                } else {
                    this.children(content(1.0))
                }
            })
    }
}
