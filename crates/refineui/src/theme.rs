//! Theme: resolved tokens for the active appearance, stored as a GPUI global.

use gpui::{
    App, BoxShadow, Global, Hsla, Pixels, SharedString, Window, WindowAppearance, point, px,
};
use refineui_foundation::{
    Color, Mode,
    shadow::Elevation,
    size::{ControlMetrics, ControlSize, Density},
};

use crate::ThemeColors;

/// Light or dark.
pub type Appearance = Mode;

pub(crate) fn to_hsla(c: Color) -> Hsla {
    gpui::rgba(c.0).into()
}

/// The active RefineUI theme. Read it with [`ActiveTheme::theme`].
#[derive(Clone, Debug)]
pub struct Theme {
    /// Current appearance.
    pub appearance: Appearance,
    /// When `true`, the appearance follows the OS (see [`Theme::sync_system_appearance`]).
    pub follow_system: bool,
    /// Control density.
    pub density: Density,
    /// Semantic colors for [`Self::appearance`].
    pub colors: ThemeColors,
    /// UI font family.
    pub font_family: SharedString,
    /// Monospace font family.
    pub mono_font_family: SharedString,
}

impl Global for Theme {}

impl Theme {
    /// A theme for an explicit appearance.
    pub fn new(appearance: Appearance) -> Self {
        Self {
            appearance,
            follow_system: false,
            density: Density::default(),
            colors: ThemeColors::from_tokens(appearance.colors()),
            font_family: refineui_foundation::typography::FONT_FAMILY.into(),
            mono_font_family: refineui_foundation::typography::MONO_FONT_FAMILY.into(),
        }
    }

    /// Install the default (light, system-following) theme if none is set.
    pub fn init(cx: &mut App) {
        if !cx.has_global::<Theme>() {
            let mut theme = Theme::new(Appearance::Light);
            theme.follow_system = true;
            cx.set_global(theme);
        }
    }

    /// Whether the theme is dark.
    pub fn is_dark(&self) -> bool {
        self.appearance == Appearance::Dark
    }

    /// Switch appearance and repaint every window.
    pub fn set_appearance(appearance: Appearance, cx: &mut App) {
        let theme = cx.global_mut::<Theme>();
        theme.appearance = appearance;
        theme.follow_system = false;
        theme.colors = ThemeColors::from_tokens(appearance.colors());
        cx.refresh_windows();
    }

    /// Toggle between light and dark.
    pub fn toggle_appearance(cx: &mut App) {
        let next = cx.theme().appearance.toggled();
        Self::set_appearance(next, cx);
    }

    /// Shell / workspace metrics for the current density.
    pub fn desktop(&self) -> refineui_foundation::size::DesktopMetrics {
        self.density.desktop()
    }

    /// Change density and repaint.
    pub fn set_density(density: Density, cx: &mut App) {
        cx.global_mut::<Theme>().density = density;
        cx.refresh_windows();
    }

    /// If [`Self::follow_system`] is set, adopt the window's OS appearance.
    /// Call from a `cx.observe_window_appearance` callback.
    pub fn sync_system_appearance(window: &Window, cx: &mut App) {
        if !cx.theme().follow_system {
            return;
        }
        let appearance = match window.appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
        };
        let theme = cx.global_mut::<Theme>();
        if theme.appearance != appearance {
            theme.appearance = appearance;
            theme.colors = ThemeColors::from_tokens(appearance.colors());
            cx.refresh_windows();
        }
    }

    /// GPUI box shadows for an elevation step in the current appearance.
    pub fn shadow(&self, elevation: Elevation) -> Vec<BoxShadow> {
        elevation
            .layers(self.appearance)
            .iter()
            .map(|l| BoxShadow {
                color: to_hsla(l.color),
                offset: point(px(l.x), px(l.y)),
                blur_radius: px(l.blur),
                spread_radius: px(l.spread),
            })
            .collect()
    }

    /// Focus ring — `outline: 2px solid var(--refineui-focus-color)` with a 2px
    /// offset (`semantic/focus.ts`). GPUI has no outlines, so it is drawn as two
    /// zero-blur shadows: the ring, then a gap in the page background on top.
    pub fn focus_ring(&self) -> Vec<BoxShadow> {
        let offset = refineui_foundation::spacing::XXX_SMALL;
        let width = refineui_foundation::stroke::THICK;
        vec![
            BoxShadow {
                color: self.focus_color(),
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(offset + width),
            },
            BoxShadow {
                color: self.colors.background_primary,
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(offset),
            },
        ]
    }

    /// Focus ring in a specific color (2px, 2px offset) — e.g. Switch / Slider use `borderFocus`.
    pub fn focus_ring_color(&self, color: Hsla) -> Vec<BoxShadow> {
        let mut ring = self.focus_ring();
        ring[0].color = color;
        ring
    }

    /// A ring hugging the element (no offset) — inputs' 0.5px focus halo, inset rings.
    pub fn ring(&self, color: Hsla, width: f32) -> Vec<BoxShadow> {
        vec![BoxShadow {
            color,
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(width),
        }]
    }

    /// The one fill for every frosted floating surface (Menu and its
    /// submenus, ContextMenu, Dropdown, Select, Popover, Tooltip,
    /// SidebarPeek): web `surfaceFrosted` pre-composited over
    /// `backgroundPrimary`. Stock GPUI cannot blur what is behind a panel
    /// (`backdrop-filter`), so a raw translucent fill would show the content
    /// behind sharply; this is the color the frost produces over a plain page,
    /// and it is always fully opaque. With the `material` feature these
    /// panels draw a real blurred material instead (see [`crate::material`]),
    /// and this stays their fallback.
    pub fn floating_fill(&self) -> Hsla {
        let mut frost = self.colors.surface_frosted;
        let alpha = frost.a;
        frost.a = 1.0;
        let mut page = self.colors.background_primary;
        page.a = 1.0;
        let mut fill = crate::motion::mix(page, frost, alpha);
        fill.a = 1.0;
        fill
    }

    /// Alias of [`Self::floating_fill`].
    pub fn frosted_fill(&self) -> Hsla {
        self.floating_fill()
    }

    /// `--refineui-focus-color`: primary black (light) / neutral white (dark).
    pub fn focus_color(&self) -> Hsla {
        // Web `--refineui-focus-color`: `primaryBlack` (#212121) / `neutralWhite`.
        if self.is_dark() {
            gpui::white()
        } else {
            crate::theme::to_hsla(refineui_foundation::palette::PRIMARY_BLACK)
        }
    }

    /// Button metrics for a size in the current density.
    pub fn button_metrics(&self, size: ControlSize) -> ControlMetrics {
        self.density.button(size)
    }

    /// Text-field metrics (Input, SpinButton, SearchField, Textarea).
    pub fn field_metrics(&self, size: ControlSize) -> ControlMetrics {
        self.density.field(size)
    }

    /// Select trigger metrics.
    pub fn select_metrics(&self, size: ControlSize) -> ControlMetrics {
        self.density.select(size)
    }

    /// Chip metrics.
    pub fn chip_metrics(&self, size: ControlSize) -> ControlMetrics {
        self.density.chip(size)
    }

    /// Extra vertical padding (per side) that turns a web-parity row
    /// (Comfortable, 32px) into this density's row: −3 compact, 0
    /// comfortable, +4 spacious. Rows apply it on top of their web padding, so
    /// Comfortable stays pixel-identical to the web.
    pub fn row_inset(&self) -> f32 {
        (self.density.row_height() - Density::Comfortable.row_height()) / 2.
    }

    /// Same as [`Self::row_inset`] for tabs (desktop `tab_height`).
    pub fn tab_inset(&self) -> f32 {
        (self.density.desktop().tab_height - Density::Comfortable.desktop().tab_height) / 2.
    }

    /// List / menu row height.
    pub fn row_height(&self) -> Pixels {
        px(self.density.row_height())
    }
}

/// Access the RefineUI theme from any GPUI context.
pub trait ActiveTheme {
    /// The active theme. Panics if [`crate::init`] was not called.
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floating_fill_is_opaque_in_both_themes() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let theme = Theme::new(appearance);
            // The raw token is translucent (web pairs it with a backdrop blur)…
            assert!(theme.colors.surface_frosted.a < 1.0);
            // …but every floating surface paints the pre-blended, opaque color.
            assert_eq!(theme.floating_fill().a, 1.0, "{appearance:?}");
            assert_eq!(theme.frosted_fill(), theme.floating_fill());
        }
    }
}
