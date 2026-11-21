//! ToggleButton — a button with a pressed state (desktop addition; web `Toggle` is the switch).

use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use crate::{Button, ButtonVariant, ControlSize, IconName, ToggleHandler};

/// A two-state button (e.g. **B** / *I* in a toolbar).
#[derive(IntoElement)]
pub struct ToggleButton {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<IconName>,
    pressed: bool,
    size: Option<ControlSize>,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_change: Option<ToggleHandler>,
}

impl ToggleButton {
    /// Overflow-menu entry: `(label, icon, pressed, disabled, change)`.
    #[allow(clippy::type_complexity)]
    pub(crate) fn menu_parts(
        &self,
    ) -> (
        SharedString,
        Option<IconName>,
        bool,
        bool,
        Option<ToggleHandler>,
    ) {
        let label = self
            .label
            .clone()
            .filter(|l| !l.is_empty())
            .or_else(|| self.tooltip.clone())
            .unwrap_or_else(|| self.id.to_string().into());
        (
            label,
            self.icon,
            self.pressed,
            self.disabled,
            self.on_change.clone(),
        )
    }

    /// Labelled toggle.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, pressed: bool) -> Self {
        Self {
            id: id.into(),
            label: Some(label.into()),
            icon: None,
            pressed,
            size: None,
            disabled: false,
            tooltip: None,
            on_change: None,
        }
    }

    /// Icon-only toggle.
    pub fn icon_only(id: impl Into<ElementId>, icon: IconName, pressed: bool) -> Self {
        let mut t = Self::new(id, "", pressed);
        t.label = None;
        t.icon = Some(icon);
        t
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Tooltip.
    pub fn tooltip(mut self, t: impl Into<SharedString>) -> Self {
        self.tooltip = Some(t.into());
        self
    }

    /// Called with the new pressed state.
    pub fn on_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl ToggleButton {
    /// Web `ToolbarToggleButton`: a ghost toolbar Button whose pressed state
    /// is simply the Button's own `secondary` variant.
    pub(crate) fn toolbar_button(self, size: ControlSize) -> Button {
        let pressed = self.pressed;
        let size = self.size.unwrap_or(size);
        self.button()
            .variant(if pressed {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            })
            .selected(false)
            .size(size)
    }

    fn button(self) -> Button {
        let mut button = match (self.label, self.icon) {
            (None, Some(icon)) => Button::icon_only(self.id, icon),
            (Some(label), icon) => {
                let b = Button::new(self.id, label);
                match icon {
                    Some(i) => b.icon(i),
                    None => b,
                }
            }
            (None, None) => Button::new(self.id, ""),
        };
        button = button
            .variant(ButtonVariant::Ghost)
            .size(self.size.unwrap_or_default())
            .selected(self.pressed)
            .disabled(self.disabled);
        if let Some(t) = self.tooltip {
            button = button.tooltip(t);
        }
        if let Some(handler) = self.on_change {
            let next = !self.pressed;
            button = button.on_click(move |_, w, cx| handler(&next, w, cx));
        }
        button
    }
}

impl RenderOnce for ToggleButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.button()
    }
}
