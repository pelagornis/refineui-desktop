//! Window metrics per platform — titlebar, window controls and resize
//! hit areas. GPUI-free data consumed by `refineui::platform` adapters.
//!
//! Numbers follow each OS's native chrome (macOS AppKit titlebar / unified
//! toolbar, Windows 11 caption buttons, GNOME/libadwaita header bar), so a
//! RefineUI titlebar lines up with the system controls it hosts.

/// Window-controls placement and size for one platform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowControlsMetrics {
    /// Controls sit at the leading (left) edge (macOS) or trailing edge.
    pub leading: bool,
    /// One control's hit width.
    pub button_width: f32,
    /// One control's hit height.
    pub button_height: f32,
    /// Gap between controls.
    pub gap: f32,
    /// Inset from the window edge to the first control.
    pub inset: f32,
    /// Width the titlebar must keep free for the controls.
    pub reserved_width: f32,
}

/// Titlebar heights for one platform: plain / with a unified toolbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitlebarMetrics {
    /// Title-only titlebar.
    pub plain: f32,
    /// Titlebar merged with a toolbar (compact density).
    pub unified_compact: f32,
    /// Titlebar merged with a toolbar (comfortable / spacious density).
    pub unified: f32,
}

/// macOS traffic lights: 12px dots at 20px pitch, 12px from the leading
/// edge; ~78px reserved.
pub const MACOS_CONTROLS: WindowControlsMetrics = WindowControlsMetrics {
    leading: true,
    button_width: 12.0,
    button_height: 12.0,
    gap: 8.0,
    inset: 12.0,
    reserved_width: 78.0,
};
/// macOS titlebar: 28 plain, 38 unified-compact, 52 unified.
pub const MACOS_TITLEBAR: TitlebarMetrics = TitlebarMetrics {
    plain: 28.0,
    unified_compact: 38.0,
    unified: 52.0,
};

/// Windows 11 caption buttons: 46 × 32, flush with the trailing edge.
pub const WINDOWS_CONTROLS: WindowControlsMetrics = WindowControlsMetrics {
    leading: false,
    button_width: 46.0,
    button_height: 32.0,
    gap: 0.0,
    inset: 0.0,
    reserved_width: 138.0,
};
/// Windows titlebar: 32 plain (caption height), 40 / 48 with toolbar.
pub const WINDOWS_TITLEBAR: TitlebarMetrics = TitlebarMetrics {
    plain: 32.0,
    unified_compact: 40.0,
    unified: 48.0,
};

/// GNOME header bar: 24px round buttons, 6px apart, trailing.
pub const LINUX_CONTROLS: WindowControlsMetrics = WindowControlsMetrics {
    leading: false,
    button_width: 24.0,
    button_height: 24.0,
    gap: 6.0,
    inset: 8.0,
    reserved_width: 100.0,
};
/// Linux (client-side decorations) header bar: 38 / 38 / 46.
pub const LINUX_TITLEBAR: TitlebarMetrics = TitlebarMetrics {
    plain: 38.0,
    unified_compact: 38.0,
    unified: 46.0,
};

/// Resize hit area along a frameless window edge.
pub const RESIZE_EDGE: f32 = 6.0;
/// Resize hit area at a frameless window corner.
pub const RESIZE_CORNER: f32 = 12.0;
/// Default minimum window size for an application shell.
pub const MIN_WINDOW: (f32, f32) = (640.0, 400.0);

/// macOS traffic-light fills (close / minimize / zoom) — drawn only for
/// chrome previews on other hosts; on macOS AppKit draws the real ones.
pub const MACOS_TRAFFIC_LIGHTS: [u32; 3] = [0xFF5F57, 0xFEBC2E, 0x28C840];
/// Windows 11 close-button hover fill (system red).
pub const WINDOWS_CLOSE_HOVER: u32 = 0xC42B1C;
/// Windows 11 close-button pressed fill.
pub const WINDOWS_CLOSE_PRESSED: u32 = 0xC83C31;
/// Linux client-side-decorated window corner radius (libadwaita).
pub const CSD_RADIUS: f32 = 12.0;
/// Linux client-side-decorated window shadow inset (the area outside the
/// visible frame that carries the shadow and the resize hit area).
pub const CSD_SHADOW: f32 = 10.0;
