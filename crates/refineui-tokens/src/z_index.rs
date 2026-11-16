//! Layering — web `zIndex` (`global/zIndex.ts`) plus the desktop layer
//! order used by `refineui::foundation::Layer`.
//!
//! GPUI paints in tree order and `deferred` / `refineui::layer` lift
//! overlays; these numbers are the *contract* (ordering), not CSS values.

/// `zIndexBackground`
pub const BACKGROUND: i32 = 0;
/// `zIndexContent`
pub const CONTENT: i32 = 1;
/// `zIndexOverlay` — scrims, docked overlays.
pub const OVERLAY: i32 = 100;
/// `zIndexPopup` — menus, popovers, selects.
pub const POPUP: i32 = 1000;
/// `zIndexFloating` — floating panels (same band as popups on web).
pub const FLOATING: i32 = 1000;
/// `zIndexMessages` — toasts, notifications.
pub const MESSAGES: i32 = 10000;
/// `zIndexPriority` — tooltips, drag previews.
pub const PRIORITY: i32 = 100000;
/// `zIndexDebug`
pub const DEBUG: i32 = 999999;

/// Desktop stacking order, lowest first. Each entry is (name, z).
///
/// Desktop splits the web `floating` band: floating *panels* (tool windows
/// inside the workspace) sit below *popups* (menus opened from them), and
/// drag previews sit above everything but debug overlays.
pub const DESKTOP_ORDER: [(&str, i32); 9] = [
    ("content", CONTENT),
    ("sticky", CONTENT + 1),
    ("floatingPanel", OVERLAY - 10),
    ("overlay", OVERLAY),
    ("popup", POPUP),
    ("messages", MESSAGES),
    ("tooltip", PRIORITY),
    ("dragPreview", PRIORITY + 1),
    ("debug", DEBUG),
];
