//! Native backdrop blur ("vibrancy") for floating surfaces — **prototype**.
//!
//! ## Why in-window panels cannot blur
//!
//! GPUI 0.2.2 draws a frame as a flat list of primitives (`scene.rs`:
//! `PrimitiveKind::{Shadow, Quad, Path, Underline, MonochromeSprite,
//! PolychromeSprite, Surface}`), one Metal / Blade pipeline per kind, all
//! rendered in a single pass straight into the drawable. There is no
//! offscreen pass, no "read what is already drawn" step and no filter
//! primitive, so an element cannot sample (let alone blur) the pixels
//! behind it — the web `backdrop-filter: blur()` has no equivalent. In-window
//! panels therefore use [`crate::Theme::floating_fill`]: the frost colour
//! pre-composited over the page, fully opaque.
//!
//! The vendored gpui (`vendor/gpui`) adds a backdrop-blur primitive; with
//! the `material` feature in-window panels use it (see [`crate::material`]).
//! When [`FrostMode::Native`] is active it takes precedence and in-window
//! materials stay off.
//!
//! ## What *can* blur
//!
//! The OS compositor can. `WindowBackgroundAppearance::Blurred` puts an
//! `NSVisualEffectView` behind a window's content on macOS (`CGSSetWindow-
//! BackgroundBlurRadius` before macOS 12). So a floating surface rendered
//! in its **own** borderless, non-activating pop-up window
//! (`WindowKind::PopUp` → `NSPanel`) with a blurred background gets a real
//! blur of whatever is behind it — the app window *and* other apps.
//!
//! This module opens such windows. It is opt-in: [`FrostMode::Opaque`] (the
//! default) keeps the in-window opaque panels. [`FrostMode::Native`] makes
//! [`crate::DropdownMenu`], [`crate::ContextMenu`], menu submenus,
//! [`crate::Select`] and [`crate::Popover`] open their panels as blurred
//! pop-up windows on macOS; on other platforms it silently falls back to
//! `Opaque` (X11 has no blur, Wayland blurs only on KDE via
//! `org_kde_kwin_blur`).
//!
//! Behaviour of a native pop-up (`drive_native_popup`):
//! * **Submenus** open in their own pop-up window beside the row (never
//!   clipped); closing a pop-up closes every pop-up opened from it.
//! * **Window moves**: on macOS the pop-up is attached as an AppKit child
//!   window (`addChildWindow:ordered:`), so it follows its parent while it is
//!   dragged. A parent *resize* (or a move where attaching is unavailable)
//!   closes it.
//! * **Deactivation**: deactivating the parent window — switching apps,
//!   ⌘-Tab, clicking another window — closes it.
//! * **Fade-in**: the pop-up window's alpha animates 0 → 1 over
//!   `duration-normal` (`Motion::FROSTED`) through `NSAnimationContext`.
//!   Closing is immediate.
//! * **Blur view**: the window is opened `Transparent` and `macos` adds a
//!   colorless `NSVisualEffectView` (`Selection` material, blending
//!   `behindWindow`, state `active`, desktop tint and saturation filters
//!   stripped) below GPUI's view, rounded with a stretchable `maskImage`.
//!   The window's `NSAppearance` follows the RefineUI theme so the blur
//!   (and any remaining system chrome) matches light / dark even when the
//!   OS appearance differs. The plate is the Regular material tint — not
//!   Apple's `.menu` / `.popover` tint, which stacked with `surfaceFrosted`
//!   and made the colors look wrong. An earlier version used `Blurred` and
//!   rounded the content view's layer (`cornerRadius` + `masksToBounds`);
//!   a masking ancestor makes the window server fall back to a flat
//!   translucent fill, which is the "translucent but not blurred" result
//!   seen on macOS.
//! * `REFINEUI_VIBRANCY_DEBUG=1` prints each pop-up's window, view hierarchy
//!   and layer opacity to stderr (at open and after the first frame).
//! * The corner mask, hidden window buttons, child-window attachment and the
//!   fade rely on AppKit calls made via `objc` (`macos`).

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext, Bounds, Context, ElementId, Entity, Global,
    IntoElement, ParentElement, Pixels, Point, Render, Size, Styled, Subscription, Window,
    WindowBackgroundAppearance, WindowBounds, WindowId, WindowKind, WindowOptions, div, px,
};
use refineui_foundation::{radius, stroke};

use crate::motion::{child_id, probe, use_layout_probe};
use crate::{ActiveTheme, Menu};

/// How floating surfaces (menus, popovers…) are filled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrostMode {
    /// In-window panel with the opaque pre-composited frost fill (default,
    /// every platform).
    #[default]
    Opaque,
    /// macOS: separate pop-up window with an OS-blurred background. Falls back
    /// to [`FrostMode::Opaque`] elsewhere.
    Native,
}

struct FrostGlobal(FrostMode);
impl Global for FrostGlobal {}

/// Choose how floating surfaces are filled (default [`FrostMode::Opaque`]).
pub fn set_frost_mode(mode: FrostMode, cx: &mut App) {
    cx.set_global(FrostGlobal(mode));
    cx.refresh_windows();
}

/// The configured [`FrostMode`].
pub fn frost_mode(cx: &App) -> FrostMode {
    cx.try_global::<FrostGlobal>()
        .map(|g| g.0)
        .unwrap_or_default()
}

/// Whether this platform can do [`FrostMode::Native`].
pub const fn native_frost_supported() -> bool {
    cfg!(target_os = "macos")
}

/// `Native` requested *and* supported.
pub fn native_frost_active(cx: &App) -> bool {
    native_frost_supported() && frost_mode(cx) == FrostMode::Native
}

/// How a native-frost pop-up should present to AppKit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NativePopupAppearance {
    /// Inherit the OS / parent window appearance (`follow_system`).
    #[default]
    System,
    /// Force `NSAppearanceNameAqua`.
    Light,
    /// Force `NSAppearanceNameDarkAqua`.
    Dark,
}

/// [`NativePopupAppearance`] for the current theme: an explicit gallery
/// / app theme wins over the OS, otherwise the pop-up inherits.
pub(crate) fn native_popup_appearance(cx: &App) -> NativePopupAppearance {
    let theme = cx.theme();
    if theme.follow_system {
        NativePopupAppearance::System
    } else if theme.is_dark() {
        NativePopupAppearance::Dark
    } else {
        NativePopupAppearance::Light
    }
}

/// Translucent fill painted over the OS blur. This is the Regular material
/// tint (`backgroundPrimary` at the material alpha) so a native-frost
/// pop-up stays on-token with the in-window material plate. It is *not*
/// the opaque [`crate::Theme::floating_fill`] (that one is pre-composited
/// for panels that cannot blur) and it is slightly denser than the raw
/// `surfaceFrosted` token (0.55).
pub(crate) fn frost_overlay(theme: &crate::Theme) -> gpui::Hsla {
    let tokens = refineui_foundation::material::MaterialRole::Regular.tokens(theme.appearance);
    crate::theme::to_hsla(tokens.tint)
}

/// Keep a native-frost pop-up's AppKit appearance in sync with the theme.
/// No-op on platforms without the macOS glue; still evaluates the
/// appearance so the helper stays live in every target.
fn sync_native_popup_appearance(window: &Window, cx: &App) {
    let appearance = native_popup_appearance(cx);
    #[cfg(any(target_os = "macos", feature = "typecheck-macos-vibrancy"))]
    crate::platform::macos::sync_popup_appearance(window, appearance);
    #[cfg(not(any(target_os = "macos", feature = "typecheck-macos-vibrancy")))]
    let _ = (window, appearance);
}

/// Map a point in `window`'s content coordinates to screen coordinates
/// (accounts for a standard title bar above the content).
pub fn to_screen(window: &Window, local: Point<Pixels>) -> Point<Pixels> {
    let frame = window.bounds();
    let titlebar = (frame.size.height - window.viewport_size().height).max(px(0.));
    Point::new(
        frame.origin.x + local.x,
        frame.origin.y + titlebar + local.y,
    )
}

/// Open a borderless, non-activating pop-up window at `bounds` (screen
/// coordinates) with an OS-blurred background and `corner_radius` corners.
/// Returns `None` where native frost is unsupported.
pub fn open_frosted_window<V: Render + 'static>(
    bounds: Bounds<Pixels>,
    corner_radius: f32,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> Option<AnyWindowHandle> {
    open_frosted_child(None, 0, bounds, corner_radius, false, cx, build)
}

/// [`open_frosted_window`] owned by `parent` (closed with it) and, on macOS,
/// attached to the AppKit window `parent_ns` (0 = none) so it follows moves.
fn open_frosted_child<V: Render + 'static>(
    parent: Option<AnyWindowHandle>,
    parent_ns: usize,
    bounds: Bounds<Pixels>,
    corner_radius: f32,
    popover: bool,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> Option<AnyWindowHandle> {
    if !native_frost_supported() {
        let _ = (parent, parent_ns, bounds, corner_radius, popover, build);
        return None;
    }
    // `popover` used to pick NSVisualEffectMaterial.popover vs .menu. Both
    // system materials carry their own tint, which fought the RefineUI
    // overlay; the effect view is now a colorless blur for every pop-up.
    let _ = popover;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: false,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // `Transparent`, not `Blurred`: `crate::platform::macos::shape_popup` adds its own
        // rounded behind-window `NSVisualEffectView` (see there for why).
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let handle: Option<AnyWindowHandle> = cx
        .open_window(options, |window, cx| {
            #[cfg(any(target_os = "macos", feature = "typecheck-macos-vibrancy"))]
            crate::platform::macos::shape_popup(
                window,
                corner_radius,
                parent_ns,
                crate::motion::Motion::FROSTED.duration.as_secs_f64(),
                native_popup_appearance(cx),
            );
            #[cfg(any(target_os = "macos", feature = "typecheck-macos-vibrancy"))]
            if crate::platform::macos::debug_enabled() {
                // Second dump once GPUI has drawn the first frame.
                window.on_next_frame(|window, _| {
                    crate::platform::macos::debug_dump(window, "first-frame")
                });
            }
            let _ = (parent_ns, popover);
            build(window, cx)
        })
        .ok()
        .map(|h| h.into());

    if let (Some(parent), Some(child)) = (parent, handle) {
        cx.default_global::<PopupTree>()
            .0
            .entry(parent.window_id())
            .or_default()
            .push(child);
    }
    handle
}

/// Pop-up windows opened from each window, closed together with it.
#[derive(Default)]
struct PopupTree(HashMap<WindowId, Vec<AnyWindowHandle>>);
impl Global for PopupTree {}

/// Close a window opened by [`open_frosted_window`] and every pop-up opened
/// from it.
pub fn close_frosted_window(handle: AnyWindowHandle, cx: &mut App) {
    let mut stack = vec![handle];
    let mut all = Vec::new();
    while let Some(h) = stack.pop() {
        if cx.has_global::<PopupTree>()
            && let Some(kids) = cx.global_mut::<PopupTree>().0.remove(&h.window_id())
        {
            stack.extend(kids);
        }
        all.push(h);
    }
    if cx.has_global::<PopupTree>() {
        for kids in cx.global_mut::<PopupTree>().0.values_mut() {
            kids.retain(|k| !all.iter().any(|h| h.window_id() == k.window_id()));
        }
    }
    // Innermost first.
    for h in all.into_iter().rev() {
        let _ = h.update(cx, |_, window, _| window.remove_window());
    }
}

/// Builds a native pop-up's content inside the pop-up window.
pub(crate) type PopupContent = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Bookkeeping for one native pop-up (keyed element state of its owner).
#[derive(Default)]
pub(crate) struct NativePopupState {
    pending: bool,
    handle: Option<AnyWindowHandle>,
    /// Window-local origin it was opened at (a new origin reopens it).
    origin: Option<Point<Pixels>>,
    _subs: Vec<Subscription>,
}

/// What [`drive_native_popup`] opens.
pub(crate) struct NativePopupSpec {
    /// Corner radius of the pop-up.
    pub radius: f32,
    /// Closes the owner (sets its open state to false).
    pub close: Rc<dyn Fn(&mut App)>,
    /// Owner still wants the pop-up (checked after the window is created).
    pub still_open: Rc<dyn Fn(&App) -> bool>,
    /// The pop-up's content, rendered in the pop-up window.
    pub content: PopupContent,
    /// Close when the owning window deactivates (top-level pop-ups; a
    /// submenu's owner is a non-activating pop-up).
    pub close_on_deactivate: bool,
    /// Receives the pop-up's measured size (owners that align the pop-up by
    /// its width reopen it once it is known, and cache it).
    pub report_size: Option<Entity<Option<Size<Pixels>>>>,
    /// `NSVisualEffectMaterial.popover` instead of `.menu` (macOS).
    pub popover: bool,
}

/// Open / close the native-frost pop-up of an owner element. Call every
/// render with the owner's `is_open` and the panel's window-local `bounds`
/// (`size.height` is only the first guess — the pop-up resizes itself to
/// its content). `spec` is built only when the pop-up opens.
pub(crate) fn drive_native_popup(
    key: &ElementId,
    is_open: bool,
    bounds: Option<Bounds<Pixels>>,
    spec: impl FnOnce() -> NativePopupSpec,
    window: &mut Window,
    cx: &mut App,
) {
    let state = window.use_keyed_state(child_id(key, "native-popup"), cx, |_, _| {
        NativePopupState::default()
    });
    let (pending, handle, origin) = {
        let s = state.read(cx);
        (s.pending, s.handle, s.origin)
    };
    let moved = match (origin, bounds) {
        (Some(o), Some(b)) => {
            (o.x - b.origin.x).abs() > px(0.5) || (o.y - b.origin.y).abs() > px(0.5)
        }
        _ => false,
    };
    if !is_open || moved {
        if let Some(h) = handle {
            state.update(cx, |s, _| *s = NativePopupState::default());
            cx.defer(move |cx| close_frosted_window(h, cx));
            if is_open {
                window.request_animation_frame();
            }
        }
        return;
    }
    let (Some(b), None, false) = (bounds, handle, pending) else {
        return;
    };
    let spec = spec();
    let screen = Bounds::new(to_screen(window, b.origin), b.size);
    let parent = window.window_handle();
    #[cfg(any(target_os = "macos", feature = "typecheck-macos-vibrancy"))]
    let parent_ns = crate::platform::macos::ns_window_of(window);
    #[cfg(not(any(target_os = "macos", feature = "typecheck-macos-vibrancy")))]
    let parent_ns = 0usize;
    let parent_size = window.bounds().size;
    let attached = cfg!(target_os = "macos") && parent_ns != 0;
    let on_bounds = spec.close.clone();
    let on_active = spec.close.clone();
    let deactivate = spec.close_on_deactivate;
    state.update(cx, |s, cx| {
        s.pending = true;
        s.origin = Some(b.origin);
        s._subs
            .push(cx.observe_window_bounds(window, move |_, w, cx| {
                // A child window follows moves; anything else invalidates the
                // pop-up's placement.
                if !attached || w.bounds().size != parent_size {
                    on_bounds(cx);
                }
            }));
        if deactivate {
            s._subs
                .push(cx.observe_window_activation(window, move |_, w, cx| {
                    if !w.is_window_active() {
                        on_active(cx);
                    }
                }));
        }
    });
    let NativePopupSpec {
        radius,
        still_open,
        content,
        report_size,
        popover,
        ..
    } = spec;
    cx.defer(move |cx| {
        let handle = open_frosted_child(
            Some(parent),
            parent_ns,
            screen,
            radius,
            popover,
            cx,
            move |_, cx| {
                cx.new(|_| FrostedPanelWindow {
                    content,
                    radius,
                    report_size,
                })
            },
        );
        if !still_open(cx) {
            // Closed while the window was being created.
            if let Some(h) = handle {
                close_frosted_window(h, cx);
            }
            state.update(cx, |s, _| *s = NativePopupState::default());
            return;
        }
        state.update(cx, |s, cx| {
            s.pending = false;
            s.handle = handle;
            cx.notify();
        });
    });
}

/// Root view of a native-frost pop-up: the Regular material tint over the
/// OS blur, a hairline `borderDefault` edge, and the content. The window
/// resizes itself to the content once measured.
pub struct FrostedPanelWindow {
    content: PopupContent,
    radius: f32,
    report_size: Option<Entity<Option<Size<Pixels>>>>,
}

fn fit_window(measure: &Entity<crate::motion::LayoutProbe>, window: &mut Window, cx: &App) {
    if let Some(b) = measure.read(cx).container {
        let want: Size<Pixels> = b.size;
        if (window.viewport_size().height - want.height).abs() > px(0.5)
            || (window.viewport_size().width - want.width).abs() > px(0.5)
        {
            window.resize(want);
        }
    }
}

impl Render for FrostedPanelWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        sync_native_popup_appearance(window, cx);
        let measure = use_layout_probe(&ElementId::Name("frosted-panel".into()), window, cx);
        fit_window(&measure, window, cx);
        if let (Some(report), Some(b)) = (&self.report_size, measure.read(cx).container)
            && *report.read(cx) != Some(b.size)
        {
            report.update(cx, |r, cx| {
                *r = Some(b.size);
                cx.notify();
            });
        }
        let content = (self.content)(window, cx);
        div()
            .size_full()
            .font_family(theme.font_family.clone())
            .text_color(theme.colors.foreground_primary)
            .child(
                div()
                    .relative()
                    .flex_none()
                    .bg(frost_overlay(&theme))
                    .border(px(stroke::HAIRLINE))
                    .border_color(theme.colors.border_default)
                    .rounded(px(self.radius))
                    .child(probe(&measure, None))
                    .child(content),
            )
    }
}

/// The root view of a native-frost menu pop-up: the Regular material tint
/// over the OS blur, a hairline `borderDefault` edge, and the [`Menu`]
/// rows. The window resizes itself to the menu once measured.
pub struct FrostedMenuWindow {
    menu: Menu,
    on_dismiss: Rc<dyn Fn(&mut App)>,
}

impl FrostedMenuWindow {
    /// New root for `menu`; `on_dismiss` runs when an item is activated.
    pub fn new(menu: Menu, on_dismiss: impl Fn(&mut App) + 'static) -> Self {
        Self {
            menu,
            on_dismiss: Rc::new(on_dismiss),
        }
    }
}

impl Render for FrostedMenuWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        sync_native_popup_appearance(window, cx);
        let measure = use_layout_probe(&gpui::ElementId::Name("frosted-menu".into()), window, cx);
        fit_window(&measure, window, cx);
        let dismiss = self.on_dismiss.clone();
        let menu = self
            .menu
            .clone()
            .native_frost(true)
            .on_dismiss(Rc::new(move |_, cx| dismiss(cx)));
        div()
            .size_full()
            .font_family(theme.font_family.clone())
            .text_color(theme.colors.foreground_primary)
            .child(
                div()
                    .relative()
                    .flex_none()
                    .bg(frost_overlay(&theme))
                    .border(px(stroke::HAIRLINE))
                    .border_color(theme.colors.border_default)
                    .rounded(px(radius::XX_LARGE))
                    .child(probe(&measure, None))
                    .child(menu),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn opaque_is_the_default(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            assert_eq!(frost_mode(cx), FrostMode::Opaque);
            assert!(!native_frost_active(cx));
            set_frost_mode(FrostMode::Native, cx);
            assert_eq!(frost_mode(cx), FrostMode::Native);
            assert_eq!(native_frost_active(cx), native_frost_supported());
        });
    }

    #[test]
    fn frost_overlay_is_the_regular_material_tint() {
        use refineui_foundation::material::MaterialRole;
        for appearance in [crate::Appearance::Light, crate::Appearance::Dark] {
            let theme = crate::Theme::new(appearance);
            let fill = frost_overlay(&theme);
            let expected = crate::theme::to_hsla(MaterialRole::Regular.tokens(appearance).tint);
            assert_eq!(fill, expected, "{appearance:?}");
            assert!(
                fill.a > 0.0 && fill.a < 1.0,
                "overlay must stay translucent so the OS blur shows through ({appearance:?})"
            );
            assert_eq!(theme.floating_fill().a, 1.0, "{appearance:?}");
            assert_ne!(
                fill,
                theme.floating_fill(),
                "must not use the opaque pre-composited fill in a native pop-up ({appearance:?})"
            );
        }
    }

    #[gpui::test]
    fn popup_appearance_tracks_an_explicit_theme(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            crate::Theme::init(cx);
            assert!(cx.theme().follow_system);
            assert_eq!(native_popup_appearance(cx), NativePopupAppearance::System);
            crate::Theme::set_appearance(crate::Appearance::Dark, cx);
            assert_eq!(native_popup_appearance(cx), NativePopupAppearance::Dark);
            crate::Theme::set_appearance(crate::Appearance::Light, cx);
            assert_eq!(native_popup_appearance(cx), NativePopupAppearance::Light);
        });
    }
}
