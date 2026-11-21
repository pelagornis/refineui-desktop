//! GPUI-resolved semantic colors. Generated from `refineui_foundation::SemanticColors`;
//! field names match the web alias tokens in snake_case.

use gpui::Hsla;
use refineui_foundation::SemanticColors;

use crate::theme::to_hsla;

/// Semantic colors resolved to [`Hsla`] for the active appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeColors {
    #[allow(missing_docs)]
    pub background_brand_active: Hsla,
    #[allow(missing_docs)]
    pub background_brand_disabled: Hsla,
    #[allow(missing_docs)]
    pub background_brand_hover: Hsla,
    #[allow(missing_docs)]
    pub background_brand_strong: Hsla,
    #[allow(missing_docs)]
    pub background_brand_subtle: Hsla,
    #[allow(missing_docs)]
    pub background_brand: Hsla,
    #[allow(missing_docs)]
    pub background_discovery_hover: Hsla,
    #[allow(missing_docs)]
    pub background_discovery_subtle: Hsla,
    #[allow(missing_docs)]
    pub background_discovery: Hsla,
    #[allow(missing_docs)]
    pub background_error_hover: Hsla,
    #[allow(missing_docs)]
    pub background_error_subtle: Hsla,
    #[allow(missing_docs)]
    pub background_error: Hsla,
    #[allow(missing_docs)]
    pub background_info_hover: Hsla,
    #[allow(missing_docs)]
    pub background_info_subtle: Hsla,
    #[allow(missing_docs)]
    pub background_info: Hsla,
    #[allow(missing_docs)]
    pub background_primary_active: Hsla,
    #[allow(missing_docs)]
    pub background_primary_hover: Hsla,
    #[allow(missing_docs)]
    pub background_primary: Hsla,
    #[allow(missing_docs)]
    pub background_success_hover: Hsla,
    #[allow(missing_docs)]
    pub background_success_subtle: Hsla,
    #[allow(missing_docs)]
    pub background_success: Hsla,
    #[allow(missing_docs)]
    pub background_surface_active: Hsla,
    #[allow(missing_docs)]
    pub background_surface_disabled: Hsla,
    #[allow(missing_docs)]
    pub background_surface_hover: Hsla,
    #[allow(missing_docs)]
    pub background_surface_selected: Hsla,
    #[allow(missing_docs)]
    pub background_surface: Hsla,
    #[allow(missing_docs)]
    pub background_warning_hover: Hsla,
    #[allow(missing_docs)]
    pub background_warning_subtle: Hsla,
    #[allow(missing_docs)]
    pub background_warning: Hsla,
    #[allow(missing_docs)]
    pub border_default: Hsla,
    #[allow(missing_docs)]
    pub border_disabled: Hsla,
    #[allow(missing_docs)]
    pub border_discovery: Hsla,
    #[allow(missing_docs)]
    pub border_error: Hsla,
    #[allow(missing_docs)]
    pub border_focus: Hsla,
    #[allow(missing_docs)]
    pub border_hover: Hsla,
    #[allow(missing_docs)]
    pub border_info: Hsla,
    #[allow(missing_docs)]
    pub border_strong: Hsla,
    #[allow(missing_docs)]
    pub border_subtle: Hsla,
    #[allow(missing_docs)]
    pub border_success: Hsla,
    #[allow(missing_docs)]
    pub border_warning: Hsla,
    #[allow(missing_docs)]
    pub foreground_brand_strong: Hsla,
    #[allow(missing_docs)]
    pub foreground_brand: Hsla,
    #[allow(missing_docs)]
    pub foreground_disabled: Hsla,
    #[allow(missing_docs)]
    pub foreground_discovery: Hsla,
    #[allow(missing_docs)]
    pub foreground_error: Hsla,
    #[allow(missing_docs)]
    pub foreground_info: Hsla,
    #[allow(missing_docs)]
    pub foreground_inversed: Hsla,
    #[allow(missing_docs)]
    pub foreground_link_active: Hsla,
    #[allow(missing_docs)]
    pub foreground_link_hover: Hsla,
    #[allow(missing_docs)]
    pub foreground_link_visited: Hsla,
    #[allow(missing_docs)]
    pub foreground_link: Hsla,
    #[allow(missing_docs)]
    pub foreground_on_brand: Hsla,
    #[allow(missing_docs)]
    pub foreground_on_discovery: Hsla,
    #[allow(missing_docs)]
    pub foreground_on_error: Hsla,
    #[allow(missing_docs)]
    pub foreground_on_info: Hsla,
    #[allow(missing_docs)]
    pub foreground_on_success: Hsla,
    #[allow(missing_docs)]
    pub foreground_on_warning: Hsla,
    #[allow(missing_docs)]
    pub foreground_placeholder: Hsla,
    #[allow(missing_docs)]
    pub foreground_primary_hover: Hsla,
    #[allow(missing_docs)]
    pub foreground_primary: Hsla,
    #[allow(missing_docs)]
    pub foreground_secondary: Hsla,
    #[allow(missing_docs)]
    pub foreground_success: Hsla,
    #[allow(missing_docs)]
    pub foreground_tertiary: Hsla,
    #[allow(missing_docs)]
    pub foreground_warning: Hsla,
    #[allow(missing_docs)]
    pub surface_elevated: Hsla,
    #[allow(missing_docs)]
    pub surface_inverse: Hsla,
    #[allow(missing_docs)]
    pub surface_popover: Hsla,
    #[allow(missing_docs)]
    pub surface_sunken: Hsla,
    #[allow(missing_docs)]
    pub surface_tooltip: Hsla,
    #[allow(missing_docs)]
    pub surface_overlay: Hsla,
    #[allow(missing_docs)]
    pub surface_frosted: Hsla,
}

impl ThemeColors {
    /// Convert token colors into GPUI colors.
    pub fn from_tokens(c: &SemanticColors) -> Self {
        Self {
            background_brand_active: to_hsla(c.background_brand_active),
            background_brand_disabled: to_hsla(c.background_brand_disabled),
            background_brand_hover: to_hsla(c.background_brand_hover),
            background_brand_strong: to_hsla(c.background_brand_strong),
            background_brand_subtle: to_hsla(c.background_brand_subtle),
            background_brand: to_hsla(c.background_brand),
            background_discovery_hover: to_hsla(c.background_discovery_hover),
            background_discovery_subtle: to_hsla(c.background_discovery_subtle),
            background_discovery: to_hsla(c.background_discovery),
            background_error_hover: to_hsla(c.background_error_hover),
            background_error_subtle: to_hsla(c.background_error_subtle),
            background_error: to_hsla(c.background_error),
            background_info_hover: to_hsla(c.background_info_hover),
            background_info_subtle: to_hsla(c.background_info_subtle),
            background_info: to_hsla(c.background_info),
            background_primary_active: to_hsla(c.background_primary_active),
            background_primary_hover: to_hsla(c.background_primary_hover),
            background_primary: to_hsla(c.background_primary),
            background_success_hover: to_hsla(c.background_success_hover),
            background_success_subtle: to_hsla(c.background_success_subtle),
            background_success: to_hsla(c.background_success),
            background_surface_active: to_hsla(c.background_surface_active),
            background_surface_disabled: to_hsla(c.background_surface_disabled),
            background_surface_hover: to_hsla(c.background_surface_hover),
            background_surface_selected: to_hsla(c.background_surface_selected),
            background_surface: to_hsla(c.background_surface),
            background_warning_hover: to_hsla(c.background_warning_hover),
            background_warning_subtle: to_hsla(c.background_warning_subtle),
            background_warning: to_hsla(c.background_warning),
            border_default: to_hsla(c.border_default),
            border_disabled: to_hsla(c.border_disabled),
            border_discovery: to_hsla(c.border_discovery),
            border_error: to_hsla(c.border_error),
            border_focus: to_hsla(c.border_focus),
            border_hover: to_hsla(c.border_hover),
            border_info: to_hsla(c.border_info),
            border_strong: to_hsla(c.border_strong),
            border_subtle: to_hsla(c.border_subtle),
            border_success: to_hsla(c.border_success),
            border_warning: to_hsla(c.border_warning),
            foreground_brand_strong: to_hsla(c.foreground_brand_strong),
            foreground_brand: to_hsla(c.foreground_brand),
            foreground_disabled: to_hsla(c.foreground_disabled),
            foreground_discovery: to_hsla(c.foreground_discovery),
            foreground_error: to_hsla(c.foreground_error),
            foreground_info: to_hsla(c.foreground_info),
            foreground_inversed: to_hsla(c.foreground_inversed),
            foreground_link_active: to_hsla(c.foreground_link_active),
            foreground_link_hover: to_hsla(c.foreground_link_hover),
            foreground_link_visited: to_hsla(c.foreground_link_visited),
            foreground_link: to_hsla(c.foreground_link),
            foreground_on_brand: to_hsla(c.foreground_on_brand),
            foreground_on_discovery: to_hsla(c.foreground_on_discovery),
            foreground_on_error: to_hsla(c.foreground_on_error),
            foreground_on_info: to_hsla(c.foreground_on_info),
            foreground_on_success: to_hsla(c.foreground_on_success),
            foreground_on_warning: to_hsla(c.foreground_on_warning),
            foreground_placeholder: to_hsla(c.foreground_placeholder),
            foreground_primary_hover: to_hsla(c.foreground_primary_hover),
            foreground_primary: to_hsla(c.foreground_primary),
            foreground_secondary: to_hsla(c.foreground_secondary),
            foreground_success: to_hsla(c.foreground_success),
            foreground_tertiary: to_hsla(c.foreground_tertiary),
            foreground_warning: to_hsla(c.foreground_warning),
            surface_elevated: to_hsla(c.surface_elevated),
            surface_inverse: to_hsla(c.surface_inverse),
            surface_popover: to_hsla(c.surface_popover),
            surface_sunken: to_hsla(c.surface_sunken),
            surface_tooltip: to_hsla(c.surface_tooltip),
            surface_overlay: to_hsla(c.surface_overlay),
            surface_frosted: to_hsla(c.surface_frosted),
        }
    }
}
