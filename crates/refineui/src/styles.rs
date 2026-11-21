//! Styling helpers shared by components.

use gpui::{App, Styled};
use refineui_foundation::typography::TextStyle;

use crate::{ActiveTheme, Elevation};

/// Extra fluent styling methods backed by RefineUI tokens.
pub trait StyledExt: Styled + Sized {
    /// Apply an elevation shadow (`Elevation::RAISED`, `Elevation::FLOATING`, …).
    fn elevation(self, elevation: Elevation, cx: &App) -> Self {
        self.shadow(cx.theme().shadow(elevation))
    }

    /// Set the cursor for a semantic [`crate::foundation::CursorRole`].
    fn cursor_role(self, role: crate::foundation::CursorRole) -> Self {
        self.cursor(role.style())
    }

    /// Desktop control cursor ([`crate::foundation::CursorRole::Action`]:
    /// the arrow — native desktop buttons do not switch to the hand; only
    /// links do).
    fn cursor_action(self) -> Self {
        self.cursor_role(crate::foundation::CursorRole::Action)
    }

    /// Apply a typography token (size, line height, weight).
    fn text_style_token(self, style: TextStyle) -> Self {
        self.text_size(gpui::px(style.size))
            .line_height(gpui::px(style.line_height))
            .font_weight(gpui::FontWeight(style.weight.0))
    }

    /// Frosted floating panel (Menu, Dropdown, Select, Popover, Tooltip):
    /// `surfaceFrosted` fill, hairline `borderDefault`, `roundedXXLarge`, `shadow-refineui-2`.
    fn popover_surface(self, cx: &App) -> Self {
        let theme = cx.theme();
        self.bg(theme.floating_fill())
            .border(gpui::px(refineui_foundation::stroke::HAIRLINE))
            .border_color(theme.colors.border_default)
            .rounded(gpui::px(refineui_foundation::radius::XX_LARGE))
            .shadow(theme.shadow(Elevation::SURFACE))
    }
}

impl<T: Styled> StyledExt for T {}
