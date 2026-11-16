//! Chip and Tag — web `Chip` / `Tag` (`default | outline | filled`, `sm | md | lg`, removable).
//!
//! `roundedMedium`, heights 20 / 24 / 32, `labelSm / bodySm / bodyLg`, icons
//! 12 / 16 / 20; hover / pressed fills per the Web Kit, `duration-fast`.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use refineui_tokens::{radius, spacing};

use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle,
    use_interaction,
};
use crate::{ActiveTheme, ClickHandler, ControlSize, Icon, IconName, IconSize, StyledExt};

/// Chip / Tag variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChipVariant {
    /// Subtle fill (default).
    #[default]
    Default,
    /// Bordered.
    Outline,
    /// Brand fill.
    Filled,
}

impl ChipVariant {
    /// All variants.
    pub const ALL: [ChipVariant; 3] = [Self::Default, Self::Outline, Self::Filled];

    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Outline => "Outline",
            Self::Filled => "Filled",
        }
    }
}

/// Compact label for filters, selections and metadata.
#[derive(IntoElement)]
pub struct Chip {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    variant: ChipVariant,
    size: ControlSize,
    disabled: bool,
    selected: bool,
    on_click: Option<ClickHandler>,
    on_remove: Option<ClickHandler>,
}

/// Squared chip (web `Tag`).
pub type Tag = Chip;

impl Chip {
    /// New pill chip.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            variant: ChipVariant::Default,
            size: ControlSize::Medium,
            disabled: false,
            selected: false,
            on_click: None,
            on_remove: None,
        }
    }

    /// Same as [`Chip::new`] (web `Tag` re-exports `Chip`).
    pub fn tag(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label)
    }

    /// Variant.
    pub fn variant(mut self, variant: ChipVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Selected (filter-chip style).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Make the chip clickable.
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// Show a remove (×) button.
    pub fn on_remove(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Chip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let variant = if self.selected {
            ChipVariant::Filled
        } else {
            self.variant
        };
        let (state, i) = use_interaction(&self.id, window, cx);
        let i = if self.disabled {
            Interaction::default()
        } else {
            i
        };
        let clear = c.background_primary.opacity(0.);
        let (bg, fg, border) = if self.disabled {
            match variant {
                ChipVariant::Outline => (clear, c.foreground_disabled, Some(c.border_disabled)),
                _ => (c.background_surface_disabled, c.foreground_disabled, None),
            }
        } else {
            match variant {
                ChipVariant::Default => (
                    i.pick(
                        c.background_surface_selected,
                        c.background_primary_hover,
                        c.background_primary_active,
                    ),
                    c.foreground_brand,
                    None,
                ),
                ChipVariant::Outline => (
                    clear,
                    c.foreground_brand,
                    Some(i.pick(c.border_default, c.border_hover, c.border_focus)),
                ),
                ChipVariant::Filled => (
                    i.pick(
                        c.background_brand,
                        c.background_brand_hover,
                        c.background_brand_active,
                    ),
                    c.foreground_inversed,
                    None,
                ),
            }
        };
        let bg = animate(child_id(&self.id, "bg"), bg, Motion::FAST_EASE, window, cx);
        let border =
            border.map(|b| animate(child_id(&self.id, "bd"), b, Motion::FAST_EASE, window, cx));
        let m = theme.chip_metrics(self.size);
        let focus = use_focus_handle(&self.id, 0, window, cx);
        let focused = focus_visible(cx) && focus.is_focused(window);
        let interactive = self.on_click.is_some() && !self.disabled;
        let remove_id = child_id(&self.id, "remove");
        let (rstate, ri) = use_interaction(&remove_id, window, cx);
        let remove_opacity = animate(
            child_id(&remove_id, "o"),
            if ri.hovered { 1.0f32 } else { 0.7 },
            Motion::FAST_EASE,
            window,
            cx,
        );
        let icon = IconSize::Custom(px(m.icon));

        div()
            .id(self.id.clone())
            .flex()
            .flex_none()
            .max_w_full()
            .items_center()
            .gap(px(spacing::XX_SMALL))
            .min_h(px(m.height))
            .px(px(m.padding_x))
            .rounded(px(radius::MEDIUM))
            .when_some(border, |d, b| d.border_1().border_color(b))
            .bg(bg)
            .text_color(fg)
            .text_style_token(m.text)
            .whitespace_nowrap()
            .when(focused, |d| d.shadow(theme.focus_ring()))
            .when(!self.disabled, |d| d.track_interaction(&state))
            .when(self.disabled, |d| {
                d.cursor(gpui::CursorStyle::OperationNotAllowed)
            })
            .when_some(self.icon, |this, ic| {
                this.child(Icon::new(ic).size(icon).color(fg))
            })
            .child(self.label)
            .when(interactive, |this| {
                let handler = self.on_click.clone().expect("checked");
                this.cursor_action()
                    .track_focus(&focus)
                    .on_click(move |e, w, cx| handler(e, w, cx))
            })
            .when_some(self.on_remove, |this, handler| {
                let disabled = self.disabled;
                this.child(
                    div()
                        .id(remove_id)
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .opacity(if disabled { 1. } else { remove_opacity })
                        .when(!disabled, |d| {
                            d.cursor_action().track_interaction(&rstate).on_click(
                                move |e, w, cx| {
                                    cx.stop_propagation();
                                    handler(e, w, cx)
                                },
                            )
                        })
                        .child(Icon::new(IconName::Dismiss).size(icon).color(fg)),
                )
            })
    }
}

/// Web `TagVariant` (alias of [`ChipVariant`]).
pub type TagVariant = ChipVariant;
