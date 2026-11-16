//! **Platform layer** — OS adapters (brief §3, §8: "isolate OS-specific
//! behavior behind platform adapters").
//!
//! Components never branch on `cfg(target_os)` for *design* decisions; they
//! ask the [`PlatformAdapter`] (window-controls side and size, titlebar
//! heights, where the menu bar lives, the primary shortcut modifier, how
//! shortcuts are written, whether the compositor can blur). AppKit / Win32
//! glue lives in `platform::macos` (native frost for [`crate::vibrancy`])
//! behind these adapters and `cfg` gates.

#[cfg(any(target_os = "macos", feature = "typecheck-macos-vibrancy"))]
// `objc`'s `sel_impl!` checks `feature = "cargo-clippy"`, hence the cfg allow.
#[allow(unsafe_code, unexpected_cfgs)]
pub(crate) mod macos;

use refineui_tokens::size::Density;
use refineui_tokens::window::{self as metrics, TitlebarMetrics, WindowControlsMetrics};

/// Operating system family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Os {
    /// macOS.
    MacOs,
    /// Windows.
    Windows,
    /// Linux / BSD (X11 or Wayland).
    Linux,
}

impl Os {
    /// The OS this binary was built for.
    pub const fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// Where the application menu bar lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuBarPlacement {
    /// The system-wide menu bar (macOS) — no in-window menu bar.
    Global,
    /// A menu bar row inside the window (merged into the titlebar when the
    /// titlebar is custom).
    InWindow,
}

/// The modifier `secondary-` resolves to in key bindings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// ⌘ Command (macOS).
    Cmd,
    /// Ctrl (Windows / Linux).
    Ctrl,
}

/// How scrollbars behave (brief §4 "platform scrollbar style").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollbarStyle {
    /// Overlay thumb that appears while hovering / scrolling and fades out
    /// (macOS, GNOME; RefineUI web).
    Overlay,
    /// Always-visible track and thumb while the content overflows
    /// (Windows).
    Persistent,
}

/// OS-specific behaviour and metrics.
pub trait PlatformAdapter: Send + Sync {
    /// The OS family.
    fn os(&self) -> Os;
    /// Window controls (traffic lights / caption buttons).
    fn controls(&self) -> WindowControlsMetrics;
    /// Titlebar heights.
    fn titlebar(&self) -> TitlebarMetrics;
    /// Where the menu bar lives.
    fn menu_bar(&self) -> MenuBarPlacement;
    /// What `secondary-` means.
    fn primary_modifier(&self) -> Modifier;
    /// The compositor can blur behind a window (native frost).
    fn native_blur(&self) -> bool;
    /// Scrollbar behaviour.
    fn scrollbar(&self) -> ScrollbarStyle;
    /// The window should ask for client-side decorations (we draw the
    /// titlebar and controls; Linux CSD).
    fn client_decorations(&self) -> bool {
        false
    }

    /// Where the OS draws its own window controls inside a transparent
    /// titlebar of height `titlebar_h` (macOS traffic lights: GPUI's
    /// `TitlebarOptions::traffic_light_position`), or `None` when RefineUI
    /// draws the controls itself.
    fn traffic_light_position(&self, titlebar_h: f32) -> Option<(f32, f32)> {
        let _ = titlebar_h;
        None
    }

    /// Width the titlebar keeps free for the window controls at
    /// `titlebar_h` (leading on macOS, trailing elsewhere).
    fn controls_reserved(&self, titlebar_h: f32) -> f32 {
        let _ = titlebar_h;
        self.controls().reserved_width
    }

    /// Titlebar height for a shell with (`unified`) or without a toolbar
    /// merged into it.
    fn titlebar_height(&self, unified: bool, density: Density) -> f32 {
        let t = self.titlebar();
        match (unified, density) {
            (false, _) => t.plain,
            (true, Density::Compact) => t.unified_compact,
            (true, _) => t.unified,
        }
    }

    /// Human label for a keystroke sequence in GPUI syntax
    /// (`"secondary-shift-p"`, `"ctrl-k ctrl-s"`): `⇧⌘P` on macOS,
    /// `Ctrl+Shift+P` elsewhere.
    fn shortcut_label(&self, keys: &str) -> String {
        let mac = self.os() == Os::MacOs;
        keys.split_whitespace()
            .map(|stroke| {
                let parts: Vec<&str> = stroke.split('-').collect();
                let (mods, key) = parts.split_at(parts.len().saturating_sub(1));
                let mut ctrl = false;
                let mut alt = false;
                let mut shift = false;
                let mut cmd = false;
                for m in mods {
                    match *m {
                        "ctrl" => ctrl = true,
                        "alt" => alt = true,
                        "shift" => shift = true,
                        "cmd" | "super" | "win" => cmd = true,
                        "secondary" => match self.primary_modifier() {
                            Modifier::Cmd => cmd = true,
                            Modifier::Ctrl => ctrl = true,
                        },
                        _ => {}
                    }
                }
                let key = key_label(key.first().copied().unwrap_or(""), mac);
                if mac {
                    let mut s = String::new();
                    for (on, sym) in [(ctrl, "⌃"), (alt, "⌥"), (shift, "⇧"), (cmd, "⌘")] {
                        if on {
                            s.push_str(sym);
                        }
                    }
                    s + &key
                } else {
                    let mut v: Vec<String> = Vec::new();
                    for (on, name) in [(ctrl, "Ctrl"), (alt, "Alt"), (shift, "Shift"), (cmd, "Win")]
                    {
                        if on {
                            v.push(name.into());
                        }
                    }
                    v.push(key);
                    v.join("+")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn key_label(key: &str, mac: bool) -> String {
    let named = match key {
        "enter" => Some(if mac { "↩" } else { "Enter" }),
        "escape" => Some(if mac { "⎋" } else { "Esc" }),
        "backspace" => Some(if mac { "⌫" } else { "Backspace" }),
        "delete" => Some(if mac { "⌦" } else { "Del" }),
        "tab" => Some(if mac { "⇥" } else { "Tab" }),
        "space" => Some("Space"),
        "up" => Some("↑"),
        "down" => Some("↓"),
        "left" => Some("←"),
        "right" => Some("→"),
        "pageup" => Some(if mac { "⇞" } else { "PgUp" }),
        "pagedown" => Some(if mac { "⇟" } else { "PgDn" }),
        "home" => Some(if mac { "↖" } else { "Home" }),
        "end" => Some(if mac { "↘" } else { "End" }),
        _ => None,
    };
    match named {
        Some(n) => n.to_string(),
        None if key.chars().count() == 1 => key.to_uppercase(),
        None => {
            let mut c = key.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    }
}

/// macOS: traffic lights on the leading edge, global menu bar, ⌘, native
/// vibrancy.
#[derive(Clone, Copy, Debug, Default)]
pub struct MacOsAdapter;

impl PlatformAdapter for MacOsAdapter {
    fn os(&self) -> Os {
        Os::MacOs
    }
    fn controls(&self) -> WindowControlsMetrics {
        metrics::MACOS_CONTROLS
    }
    fn titlebar(&self) -> TitlebarMetrics {
        metrics::MACOS_TITLEBAR
    }
    fn menu_bar(&self) -> MenuBarPlacement {
        MenuBarPlacement::Global
    }
    fn primary_modifier(&self) -> Modifier {
        Modifier::Cmd
    }
    fn native_blur(&self) -> bool {
        true
    }
    fn scrollbar(&self) -> ScrollbarStyle {
        ScrollbarStyle::Overlay
    }
    fn traffic_light_position(&self, titlebar_h: f32) -> Option<(f32, f32)> {
        // The AppKit button frame is 14pt tall (12pt dot); centre it and use
        // the same distance from the leading edge, like native unified
        // toolbars (28 → 7, 38 → 12, 52 → 19).
        let y = ((titlebar_h - 14.) / 2.).round().max(6.);
        Some((y.clamp(7., 20.), y))
    }
    fn controls_reserved(&self, titlebar_h: f32) -> f32 {
        let (x, _) = self.traffic_light_position(titlebar_h).unwrap_or((12., 0.));
        // Three 14pt buttons, 6pt apart, plus the same gap after them.
        x + 3. * 14. + 2. * 6. + x
    }
}

/// Windows: caption buttons on the trailing edge, in-window menu bar, Ctrl.
/// (Mica / Acrylic backdrops are a later platform task.)
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsAdapter;

impl PlatformAdapter for WindowsAdapter {
    fn os(&self) -> Os {
        Os::Windows
    }
    fn controls(&self) -> WindowControlsMetrics {
        metrics::WINDOWS_CONTROLS
    }
    fn titlebar(&self) -> TitlebarMetrics {
        metrics::WINDOWS_TITLEBAR
    }
    fn menu_bar(&self) -> MenuBarPlacement {
        MenuBarPlacement::InWindow
    }
    fn primary_modifier(&self) -> Modifier {
        Modifier::Ctrl
    }
    fn native_blur(&self) -> bool {
        false
    }
    fn scrollbar(&self) -> ScrollbarStyle {
        ScrollbarStyle::Persistent
    }
}

/// Linux: client-side header bar with trailing controls, in-window menu
/// bar, Ctrl, no blur (X11) / KDE-only blur (Wayland).
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxAdapter;

impl PlatformAdapter for LinuxAdapter {
    fn os(&self) -> Os {
        Os::Linux
    }
    fn controls(&self) -> WindowControlsMetrics {
        metrics::LINUX_CONTROLS
    }
    fn titlebar(&self) -> TitlebarMetrics {
        metrics::LINUX_TITLEBAR
    }
    fn menu_bar(&self) -> MenuBarPlacement {
        MenuBarPlacement::InWindow
    }
    fn primary_modifier(&self) -> Modifier {
        Modifier::Ctrl
    }
    fn native_blur(&self) -> bool {
        false
    }
    fn scrollbar(&self) -> ScrollbarStyle {
        ScrollbarStyle::Overlay
    }
    fn client_decorations(&self) -> bool {
        true
    }
}

/// The adapter for `os`.
pub fn for_os(os: Os) -> &'static dyn PlatformAdapter {
    match os {
        Os::MacOs => &MacOsAdapter,
        Os::Windows => &WindowsAdapter,
        Os::Linux => &LinuxAdapter,
    }
}

/// The adapter for the OS this binary runs on.
pub fn current() -> &'static dyn PlatformAdapter {
    for_os(Os::current())
}

/// Global: draw chrome as if running on another OS (gallery previews,
/// screenshots, design review). Only *drawing* follows it — window
/// behaviour (native traffic lights, system menu bar) stays the host's.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChromePreview(pub Option<Os>);

impl gpui::Global for ChromePreview {}

/// Draw shell chrome for `os` (or the host OS with `None`) and repaint.
pub fn set_chrome_preview(os: Option<Os>, cx: &mut gpui::App) {
    cx.set_global(ChromePreview(os));
    cx.refresh_windows();
}

/// The OS chrome is drawn for: the [`ChromePreview`] if set, else the host.
pub fn chrome_os(cx: &gpui::App) -> Os {
    cx.try_global::<ChromePreview>()
        .and_then(|p| p.0)
        .unwrap_or(Os::current())
}

/// The adapter chrome components draw with ([`chrome_os`]).
pub fn active(cx: &gpui::App) -> &'static dyn PlatformAdapter {
    for_os(chrome_os(cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_labels_follow_the_platform() {
        let mac = for_os(Os::MacOs);
        let win = for_os(Os::Windows);
        assert_eq!(mac.shortcut_label("secondary-shift-p"), "⇧⌘P");
        assert_eq!(win.shortcut_label("secondary-shift-p"), "Ctrl+Shift+P");
        assert_eq!(mac.shortcut_label("ctrl-k ctrl-s"), "⌃K ⌃S");
        assert_eq!(win.shortcut_label("alt-enter"), "Alt+Enter");
        assert_eq!(mac.shortcut_label("escape"), "⎋");
        assert_eq!(win.shortcut_label("f5"), "F5");
    }

    #[test]
    fn window_controls_side() {
        assert!(for_os(Os::MacOs).controls().leading);
        assert!(!for_os(Os::Windows).controls().leading);
        assert_eq!(for_os(Os::MacOs).menu_bar(), MenuBarPlacement::Global);
        assert_eq!(
            for_os(Os::MacOs).titlebar_height(true, Density::Compact),
            38.0
        );
        assert_eq!(
            for_os(Os::Windows).titlebar_height(false, Density::Comfortable),
            32.0
        );
    }

    #[test]
    fn traffic_lights_centre_in_the_titlebar() {
        let mac = for_os(Os::MacOs);
        assert_eq!(mac.traffic_light_position(28.), Some((7., 7.)));
        assert_eq!(mac.traffic_light_position(52.), Some((19., 19.)));
        assert_eq!(mac.controls_reserved(52.), 19. + 54. + 19.);
        assert_eq!(for_os(Os::Windows).traffic_light_position(32.), None);
        assert_eq!(for_os(Os::Windows).controls_reserved(32.), 138.);
        assert!(for_os(Os::Linux).client_decorations());
        assert_eq!(for_os(Os::Windows).scrollbar(), ScrollbarStyle::Persistent);
        assert_eq!(for_os(Os::MacOs).scrollbar(), ScrollbarStyle::Overlay);
    }
}
