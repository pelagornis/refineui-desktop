//! Semantic (alias) colors — ported from `@refineui/tokens` `semantic/colors.ts`
//! (`SEMANTIC_PALETTE_PAIRS`). Each alias resolves to a palette entry per mode.

use crate::{Color, palette::*};

macro_rules! semantic_colors {
    ($($(#[$doc:meta])* $name:ident: ($light:ident, $dark:ident),)*) => {
        /// Resolved semantic colors for one appearance (light or dark).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct SemanticColors {
            $($(#[$doc])* pub $name: Color,)*
            /// `surfaceOverlay` — scrim behind modals.
            pub surface_overlay: Color,
            /// `surfaceFrosted` — translucent menu / popover fill.
            pub surface_frosted: Color,
        }

        impl SemanticColors {
            /// Light appearance.
            pub const LIGHT: Self = Self {
                $($name: $light,)*
                surface_overlay: Color::rgba(0x0000001A),
                surface_frosted: NEUTRAL_WHITE.with_alpha(0.55),
            };
            /// Dark appearance.
            pub const DARK: Self = Self {
                $($name: $dark,)*
                surface_overlay: Color::rgba(0x00000099),
                surface_frosted: NEUTRAL_900.with_alpha(0.55),
            };

            /// `(web alias name, color)` pairs for documentation / inspection.
            pub fn entries(&self) -> Vec<(&'static str, Color)> {
                vec![$((stringify!($name), self.$name),)* ("surface_overlay", self.surface_overlay), ("surface_frosted", self.surface_frosted)]
            }
        }
    };
}

semantic_colors! {
    /// `backgroundBrandActive` — light `neutralBlack`, dark `primaryLightGray`.
    background_brand_active: (NEUTRAL_BLACK, PRIMARY_LIGHT_GRAY),
    /// `backgroundBrandDisabled` — light `neutral150`, dark `neutral650`.
    background_brand_disabled: (NEUTRAL_150, NEUTRAL_650),
    /// `backgroundBrandHover` — light `primaryDarkGray`, dark `primaryLightGray`.
    background_brand_hover: (PRIMARY_DARK_GRAY, PRIMARY_LIGHT_GRAY),
    /// `backgroundBrandStrong` — light `primaryDarkGray`, dark `primaryGray`.
    background_brand_strong: (PRIMARY_DARK_GRAY, PRIMARY_GRAY),
    /// `backgroundBrandSubtle` — light `primaryLightGray`, dark `neutral800`.
    background_brand_subtle: (PRIMARY_LIGHT_GRAY, NEUTRAL_800),
    /// `backgroundBrand` — light `primaryBlack`, dark `primaryGray`.
    background_brand: (PRIMARY_BLACK, PRIMARY_GRAY),
    /// `backgroundDiscoveryHover` — light `purple700`, dark `purple300`.
    background_discovery_hover: (PURPLE_700, PURPLE_300),
    /// `backgroundDiscoverySubtle` — light `purple100`, dark `purple900`.
    background_discovery_subtle: (PURPLE_100, PURPLE_900),
    /// `backgroundDiscovery` — light `purple500`, dark `purple500`.
    background_discovery: (PURPLE_500, PURPLE_500),
    /// `backgroundErrorHover` — light `red700`, dark `red300`.
    background_error_hover: (RED_700, RED_300),
    /// `backgroundErrorSubtle` — light `red100`, dark `red900`.
    background_error_subtle: (RED_100, RED_900),
    /// `backgroundError` — light `red500`, dark `red500`.
    background_error: (RED_500, RED_500),
    /// `backgroundInfoHover` — light `blue700`, dark `blue300`.
    background_info_hover: (BLUE_700, BLUE_300),
    /// `backgroundInfoSubtle` — light `blue100`, dark `blue900`.
    background_info_subtle: (BLUE_100, BLUE_900),
    /// `backgroundInfo` — light `blue500`, dark `blue500`.
    background_info: (BLUE_500, BLUE_500),
    /// `backgroundPrimaryActive` — light `neutral150`, dark `neutral800`.
    background_primary_active: (NEUTRAL_150, NEUTRAL_800),
    /// `backgroundPrimaryHover` — light `neutral100`, dark `neutral850`.
    background_primary_hover: (NEUTRAL_100, NEUTRAL_850),
    /// `backgroundPrimary` — light `neutralWhite`, dark `neutral900`.
    background_primary: (NEUTRAL_WHITE, NEUTRAL_900),
    /// `backgroundSuccessHover` — light `green700`, dark `green300`.
    background_success_hover: (GREEN_700, GREEN_300),
    /// `backgroundSuccessSubtle` — light `green100`, dark `green900`.
    background_success_subtle: (GREEN_100, GREEN_900),
    /// `backgroundSuccess` — light `green500`, dark `green500`.
    background_success: (GREEN_500, GREEN_500),
    /// `backgroundSurfaceActive` — light `neutral150`, dark `neutral700`.
    background_surface_active: (NEUTRAL_150, NEUTRAL_700),
    /// `backgroundSurfaceDisabled` — light `neutral150`, dark `neutral850`.
    background_surface_disabled: (NEUTRAL_150, NEUTRAL_850),
    /// `backgroundSurfaceHover` — light `neutral100`, dark `neutral750`.
    background_surface_hover: (NEUTRAL_100, NEUTRAL_750),
    /// `backgroundSurfaceSelected` — light `neutral200`, dark `neutral650`.
    background_surface_selected: (NEUTRAL_200, NEUTRAL_650),
    /// `backgroundSurface` — light `neutralWhite`, dark `neutral800`.
    background_surface: (NEUTRAL_WHITE, NEUTRAL_800),
    /// `backgroundWarningHover` — light `yellow500`, dark `yellow200`.
    background_warning_hover: (YELLOW_500, YELLOW_200),
    /// `backgroundWarningSubtle` — light `yellow100`, dark `yellow900`.
    background_warning_subtle: (YELLOW_100, YELLOW_900),
    /// `backgroundWarning` — light `yellow300`, dark `yellow300`.
    background_warning: (YELLOW_300, YELLOW_300),
    /// `borderDefault` — light `neutral300`, dark `neutral650`.
    border_default: (NEUTRAL_300, NEUTRAL_650),
    /// `borderDisabled` — light `neutral250`, dark `neutral700`.
    border_disabled: (NEUTRAL_250, NEUTRAL_700),
    /// `borderDiscovery` — light `purple500`, dark `purple500`.
    border_discovery: (PURPLE_500, PURPLE_500),
    /// `borderError` — light `red500`, dark `red500`.
    border_error: (RED_500, RED_500),
    /// `borderFocus` — light `neutral750`, dark `neutral350`.
    border_focus: (NEUTRAL_750, NEUTRAL_350),
    /// `borderHover` — light `neutral500`, dark `neutral500`.
    border_hover: (NEUTRAL_500, NEUTRAL_500),
    /// `borderInfo` — light `blue500`, dark `blue500`.
    border_info: (BLUE_500, BLUE_500),
    /// `borderStrong` — light `neutral450`, dark `neutral550`.
    border_strong: (NEUTRAL_450, NEUTRAL_550),
    /// `borderSubtle` — light `neutral200`, dark `neutral750`.
    border_subtle: (NEUTRAL_200, NEUTRAL_750),
    /// `borderSuccess` — light `green500`, dark `green500`.
    border_success: (GREEN_500, GREEN_500),
    /// `borderWarning` — light `yellow300`, dark `yellow300`.
    border_warning: (YELLOW_300, YELLOW_300),
    /// `foregroundBrandStrong` — light `neutralBlack`, dark `primaryLightGray`.
    foreground_brand_strong: (NEUTRAL_BLACK, PRIMARY_LIGHT_GRAY),
    /// `foregroundBrand` — light `primaryBlack`, dark `primaryGray`.
    foreground_brand: (PRIMARY_BLACK, PRIMARY_GRAY),
    /// `foregroundDisabled` — light `neutral400`, dark `neutral600`.
    foreground_disabled: (NEUTRAL_400, NEUTRAL_600),
    /// `foregroundDiscovery` — light `purple700`, dark `purple200`.
    foreground_discovery: (PURPLE_700, PURPLE_200),
    /// `foregroundError` — light `red700`, dark `red200`.
    foreground_error: (RED_700, RED_200),
    /// `foregroundInfo` — light `blue700`, dark `blue200`.
    foreground_info: (BLUE_700, BLUE_200),
    /// `foregroundInversed` — light `neutralWhite`, dark `primaryBlack`.
    foreground_inversed: (NEUTRAL_WHITE, PRIMARY_BLACK),
    /// `foregroundLinkActive` — light `blue900`, dark `blue200`.
    foreground_link_active: (BLUE_900, BLUE_200),
    /// `foregroundLinkHover` — light `blue800`, dark `blue300`.
    foreground_link_hover: (BLUE_800, BLUE_300),
    /// `foregroundLinkVisited` — light `blue500`, dark `blue100`.
    foreground_link_visited: (BLUE_500, BLUE_100),
    /// `foregroundLink` — light `blue700`, dark `blue400`.
    foreground_link: (BLUE_700, BLUE_400),
    /// `foregroundOnBrand` — light `neutralWhite`, dark `neutralBlack`.
    foreground_on_brand: (NEUTRAL_WHITE, NEUTRAL_BLACK),
    /// `foregroundOnDiscovery` — light `neutralWhite`, dark `neutralBlack`.
    foreground_on_discovery: (NEUTRAL_WHITE, NEUTRAL_BLACK),
    /// `foregroundOnError` — light `neutralWhite`, dark `neutralBlack`.
    foreground_on_error: (NEUTRAL_WHITE, NEUTRAL_BLACK),
    /// `foregroundOnInfo` — light `neutralWhite`, dark `neutralBlack`.
    foreground_on_info: (NEUTRAL_WHITE, NEUTRAL_BLACK),
    /// `foregroundOnSuccess` — light `neutralWhite`, dark `neutralBlack`.
    foreground_on_success: (NEUTRAL_WHITE, NEUTRAL_BLACK),
    /// `foregroundOnWarning` — light `neutralWhite`, dark `neutralBlack`.
    foreground_on_warning: (NEUTRAL_WHITE, NEUTRAL_BLACK),
    /// `foregroundPlaceholder` — light `neutral450`, dark `neutral500`.
    foreground_placeholder: (NEUTRAL_450, NEUTRAL_500),
    /// `foregroundPrimaryHover` — light `neutralBlack`, dark `neutral100`.
    foreground_primary_hover: (NEUTRAL_BLACK, NEUTRAL_100),
    /// `foregroundPrimary` — light `primaryBlack`, dark `neutralWhite`.
    foreground_primary: (PRIMARY_BLACK, NEUTRAL_WHITE),
    /// `foregroundSecondary` — light `neutral600`, dark `neutral350`.
    foreground_secondary: (NEUTRAL_600, NEUTRAL_350),
    /// `foregroundSuccess` — light `green700`, dark `green200`.
    foreground_success: (GREEN_700, GREEN_200),
    /// `foregroundTertiary` — light `neutral500`, dark `neutral450`.
    foreground_tertiary: (NEUTRAL_500, NEUTRAL_450),
    /// `foregroundWarning` — light `yellow700`, dark `yellow200`.
    foreground_warning: (YELLOW_700, YELLOW_200),
    /// `surfaceElevated` — light `neutralWhite`, dark `neutral700`.
    surface_elevated: (NEUTRAL_WHITE, NEUTRAL_700),
    /// `surfaceInverse` — light `neutralBlack`, dark `neutralWhite`.
    surface_inverse: (NEUTRAL_BLACK, NEUTRAL_WHITE),
    /// `surfacePopover` — light `neutralWhite`, dark `neutral750`.
    surface_popover: (NEUTRAL_WHITE, NEUTRAL_750),
    /// `surfaceSunken` — light `neutral150`, dark `neutral800`.
    surface_sunken: (NEUTRAL_150, NEUTRAL_800),
    /// `surfaceTooltip` — light `neutral750`, dark `neutral300`.
    surface_tooltip: (NEUTRAL_750, NEUTRAL_300),
}
