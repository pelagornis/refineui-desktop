//! Windows 11 system backdrops (Mica, Mica Alt, Acrylic) via DWM.
//!
//! GPUI 0.2.2 has no Mica variant: its Windows backend maps
//! `WindowBackgroundAppearance::Blurred` to the legacy acrylic blur
//! (`SetWindowCompositionAttribute`, accent 4) and `Transparent` to a
//! premultiplied-alpha swap chain. [`set_backdrop`] asks DWM for the
//! Windows 11 (build 22621+) system backdrop instead, through
//! `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE = 38)`, and extends the
//! frame into the whole client area (`DwmExtendFrameIntoClientArea` with
//! −1 margins) so the backdrop shows behind the content.
//!
//! **Requirements** — open the window with
//! `WindowBackgroundAppearance::Transparent` and paint translucent
//! surfaces (`surfaceFrosted`, or a `backgroundPrimary` with alpha); an
//! opaque root hides the backdrop. Call it once the window exists (from
//! the root view's constructor). On older Windows the attribute is
//! ignored and the call returns an error code; fall back to GPUI's
//! `Blurred`.
//!
//! **Verification status** — the FFI is compiled for
//! `target_os = "windows"` only; on other systems [`set_backdrop`] returns
//! [`BackdropError::Unsupported`]. `scripts/typecheck_windows.sh` checks
//! and lints this module for `x86_64-pc-windows-msvc` from any OS (the
//! whole crate can't be cross-checked: gpui's Windows deps need MSVC
//! tools). It has not been run on a Windows machine yet.

use gpui::Window;

/// A DWM system backdrop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backdrop {
    /// No system backdrop (`DWMSBT_NONE`).
    #[default]
    None,
    /// Mica — the desktop wallpaper, heavily blurred and tinted; for the
    /// main window background (`DWMSBT_MAINWINDOW`).
    Mica,
    /// Mica Alt — stronger tint; for tabbed titlebars
    /// (`DWMSBT_TABBEDWINDOW`).
    MicaAlt,
    /// Acrylic — blur of what is behind the window; for transient
    /// surfaces: menus, flyouts, floating panels (`DWMSBT_TRANSIENTWINDOW`).
    Acrylic,
}

impl Backdrop {
    /// The `DWM_SYSTEMBACKDROP_TYPE` value.
    pub fn dwm_value(self) -> u32 {
        match self {
            Backdrop::None => 1,
            Backdrop::Mica => 2,
            Backdrop::Acrylic => 3,
            Backdrop::MicaAlt => 4,
        }
    }

    /// Whether the client area must be transparent for it to show.
    pub fn needs_transparent_window(self) -> bool {
        self != Backdrop::None
    }
}

/// `DWMWA_USE_IMMERSIVE_DARK_MODE`.
pub const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
/// `DWMWA_SYSTEMBACKDROP_TYPE`.
pub const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;

/// Why [`set_backdrop`] failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackdropError {
    /// Not Windows.
    Unsupported,
    /// The window has no Win32 handle.
    NoHandle,
    /// DWM returned this `HRESULT` (e.g. Windows 10).
    Dwm(i32),
}

/// Apply `backdrop` to `window` (and DWM's dark frame when `dark`).
/// See the module docs for requirements.
pub fn set_backdrop(window: &Window, backdrop: Backdrop, dark: bool) -> Result<(), BackdropError> {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let handle = HasWindowHandle::window_handle(window).map_err(|_| BackdropError::NoHandle)?;
        let RawWindowHandle::Win32(h) = handle.as_raw() else {
            return Err(BackdropError::NoHandle);
        };
        ffi::apply(h.hwnd.get(), backdrop, dark)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, backdrop, dark);
        Err(BackdropError::Unsupported)
    }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
mod ffi {
    use super::{
        Backdrop, BackdropError, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use core::ffi::c_void;

    #[repr(C)]
    struct Margins {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(hwnd: isize, attr: u32, value: *const c_void, size: u32) -> i32;
        fn DwmExtendFrameIntoClientArea(hwnd: isize, margins: *const Margins) -> i32;
    }

    pub(super) fn apply(hwnd: isize, backdrop: Backdrop, dark: bool) -> Result<(), BackdropError> {
        let dark: i32 = dark as i32;
        let kind: u32 = backdrop.dwm_value();
        let all = Margins {
            left: -1,
            right: -1,
            top: -1,
            bottom: -1,
        };
        let none = Margins {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        };
        // SAFETY: `hwnd` comes from GPUI's live window; the attribute
        // payloads are plain integers of the documented sizes and outlive
        // the calls.
        unsafe {
            let hr = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark as *const i32).cast(),
                4,
            );
            if hr < 0 {
                return Err(BackdropError::Dwm(hr));
            }
            let margins = if backdrop == Backdrop::None {
                &none
            } else {
                &all
            };
            let hr = DwmExtendFrameIntoClientArea(hwnd, margins);
            if hr < 0 {
                return Err(BackdropError::Dwm(hr));
            }
            let hr = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&kind as *const u32).cast(),
                4,
            );
            if hr < 0 {
                return Err(BackdropError::Dwm(hr));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backdrop_values_match_dwm() {
        assert_eq!(Backdrop::None.dwm_value(), 1);
        assert_eq!(Backdrop::Mica.dwm_value(), 2);
        assert_eq!(Backdrop::Acrylic.dwm_value(), 3);
        assert_eq!(Backdrop::MicaAlt.dwm_value(), 4);
        assert!(!Backdrop::None.needs_transparent_window());
        assert!(Backdrop::Mica.needs_transparent_window());
    }
}
