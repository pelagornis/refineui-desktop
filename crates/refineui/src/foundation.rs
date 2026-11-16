//! **Foundation layer** (GPUI side) — brief §3/§4.
//!
//! The GPUI-free values live in `refineui_tokens` (colors, typography,
//! spacing, sizing, radius, stroke, elevation, opacity, motion, blur,
//! z-index, density, window metrics, component contracts). This module is
//! the one place that turns them into GPUI behaviour:
//!
//! | Concern | Where |
//! | --- | --- |
//! | Theme (light / dark semantic colors, density, font) | [`Theme`] |
//! | Density (compact / comfortable / spacious) | [`Density`], [`DesktopMetrics`], [`Theme::desktop`] |
//! | Elevation / shadows | [`crate::Elevation`], [`Theme::shadow`] |
//! | Motion | [`crate::motion`] |
//! | Focus ring | [`Theme::focus_ring`] |
//! | Layers / z-order | [`LayerOrder`] + [`crate::layer()`] |
//! | Blur / frost | [`crate::vibrancy`] ([`FrostMode`]) |
//! | Pointer and cursor | [`CursorRole`] |
//! | Window metrics | [`crate::platform::PlatformAdapter`] over `refineui_tokens::window` |
//!
//! Components must take every visual value from here or from tokens —
//! never a literal when a semantic token exists (brief §8).

pub use refineui_tokens::size::{ControlSize, Density, DesktopMetrics};
pub use refineui_tokens::{blur, window as window_metrics, z_index};

pub use crate::theme::{ActiveTheme, Appearance, Theme};
pub use crate::vibrancy::{FrostMode, frost_mode, set_frost_mode};

/// Desktop stacking layers, lowest first (`refineui_tokens::z_index`).
///
/// GPUI paints in tree order; overlays are lifted with [`crate::layer()`].
/// [`LayerOrder::priority`] is the `layer(..).priority(..)` value that keeps
/// this order when several overlays are open at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerOrder {
    /// Normal content.
    Content,
    /// Sticky headers, pinned rows.
    Sticky,
    /// Floating tool panels inside the workspace.
    FloatingPanel,
    /// Scrims, docked overlays (Drawer / Sheet).
    Overlay,
    /// Menus, popovers, selects.
    Popup,
    /// Toasts and notifications.
    Messages,
    /// Tooltips.
    Tooltip,
    /// Drag previews.
    DragPreview,
    /// Debug overlays.
    Debug,
}

impl LayerOrder {
    /// All layers, lowest first.
    pub const ALL: [LayerOrder; 9] = [
        Self::Content,
        Self::Sticky,
        Self::FloatingPanel,
        Self::Overlay,
        Self::Popup,
        Self::Messages,
        Self::Tooltip,
        Self::DragPreview,
        Self::Debug,
    ];

    /// Contract z value (`z_index::DESKTOP_ORDER`).
    pub fn z(self) -> i32 {
        z_index::DESKTOP_ORDER[self as usize].1
    }

    /// `crate::layer(..).priority(..)` for this layer.
    pub const fn priority(self) -> usize {
        self as usize
    }
}

/// Semantic pointer roles → GPUI cursors (brief §4 "Pointer and Cursor").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CursorRole {
    /// Plain surface.
    Default,
    /// Clickable control (desktop convention: arrow for buttons, hand only
    /// for links — see [`CursorRole::Link`]).
    Action,
    /// Hyperlink.
    Link,
    /// Editable text.
    Text,
    /// Draggable item at rest.
    Grab,
    /// Item being dragged.
    Grabbing,
    /// Vertical splitter (resizes columns).
    ResizeColumn,
    /// Horizontal splitter (resizes rows).
    ResizeRow,
    /// Disabled control.
    NotAllowed,
}

impl CursorRole {
    /// The GPUI cursor.
    pub fn style(self) -> gpui::CursorStyle {
        use gpui::CursorStyle as C;
        match self {
            Self::Default | Self::Action => C::Arrow,
            Self::Link => C::PointingHand,
            Self::Text => C::IBeam,
            Self::Grab => C::OpenHand,
            Self::Grabbing => C::ClosedHand,
            Self::ResizeColumn => C::ResizeColumn,
            Self::ResizeRow => C::ResizeRow,
            Self::NotAllowed => C::OperationNotAllowed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_are_strictly_ordered() {
        let zs: Vec<i32> = LayerOrder::ALL.iter().map(|l| l.z()).collect();
        assert!(zs.windows(2).all(|w| w[0] < w[1]), "{zs:?}");
        assert_eq!(LayerOrder::Popup.z(), z_index::POPUP);
        assert_eq!(LayerOrder::ALL.len(), z_index::DESKTOP_ORDER.len());
    }

    #[test]
    fn densities_grow_monotonically() {
        let m: Vec<DesktopMetrics> = Density::ALL.iter().map(|d| d.desktop()).collect();
        for w in m.windows(2) {
            assert!(w[0].row_height < w[1].row_height);
            assert!(w[0].tab_height < w[1].tab_height);
            assert!(w[0].toolbar_height < w[1].toolbar_height);
            assert!(w[0].panel_padding < w[1].panel_padding);
        }
        for s in ControlSize::ALL {
            let h: Vec<f32> = Density::ALL.iter().map(|d| d.button(s).height).collect();
            assert!(h[0] < h[1] && h[1] < h[2], "button {s:?}: {h:?}");
            let h: Vec<f32> = Density::ALL.iter().map(|d| d.field(s).height).collect();
            assert!(h[0] < h[1] && h[1] < h[2], "field {s:?}: {h:?}");
        }
    }
}
