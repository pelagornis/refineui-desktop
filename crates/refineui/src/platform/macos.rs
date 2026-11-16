//! macOS (AppKit) glue — the only `unsafe` code in the crate.
//!
//! Native frost for pop-up windows (`NSVisualEffectView` behind a masked,
//! rounded content view) and the `REFINEUI_VIBRANCY_DEBUG` hierarchy dump,
//! called by [`crate::vibrancy`]. Compiled on macOS, and on other hosts with
//! the `typecheck-macos-vibrancy` feature for `cargo check` only — it is
//! never called there.
use gpui::Window;
use objc::runtime::{BOOL, NO, Object, YES};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The `NSWindow` of `window` as an address (0 if unavailable).
pub(crate) fn ns_window_of(window: &Window) -> usize {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return 0;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return 0;
    };
    let view = appkit.ns_view.as_ptr() as *mut Object;
    // SAFETY: `view` is GPUI's live NSView; `-window` has no side effects.
    unsafe {
        let ns_window: *mut Object = msg_send![view, window];
        ns_window as usize
    }
}

/// `NSVisualEffectMaterial` raw values (AppKit headers).
const MATERIAL_MENU: i64 = 5;
const MATERIAL_POPOVER: i64 = 6;
/// `NSVisualEffectBlendingModeBehindWindow`.
const BLENDING_BEHIND_WINDOW: i64 = 0;
/// `NSVisualEffectStateActive`.
const STATE_ACTIVE: i64 = 1;
/// `NSViewWidthSizable | NSViewHeightSizable`.
const AUTORESIZE_FILL: u64 = 2 | 16;
/// `NSWindowBelow`.
const ORDER_BELOW: i64 = -1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct NSEdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

/// `REFINEUI_VIBRANCY_DEBUG=1` → dump the pop-up's AppKit state to
/// stderr.
pub(crate) fn debug_enabled() -> bool {
    std::env::var("REFINEUI_VIBRANCY_DEBUG").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Hide the (titled-panel) window buttons, put a rounded
/// `NSVisualEffectView` (behind-window blending, always active) under
/// GPUI's view, let AppKit draw the window shadow, attach the pop-up to
/// `parent_ns` (so it follows the parent's moves) and fade its alpha in
/// over `fade_secs`.
///
/// Why not `WindowBackgroundAppearance::Blurred`: GPUI's blur view uses
/// the `Selection` material with its tint/saturation filters stripped,
/// and the earlier prototype rounded the corners by setting
/// `cornerRadius` + `masksToBounds` on the *content view's* layer — the
/// effect view's backdrop layer then sits under a masking ancestor, which
/// makes the window server fall back to a flat translucent fill (no
/// blur). Apple's documented way to shape vibrancy is the effect view's
/// own `maskImage`, which is what this does; the pop-up window is opened
/// `Transparent` so GPUI adds no second (square) effect view.
pub(crate) fn shape_popup(
    window: &Window,
    corner_radius: f32,
    parent_ns: usize,
    fade_secs: f64,
    popover: bool,
) {
    let ns_window = ns_window_of(window) as *mut Object;
    if ns_window.is_null() {
        return;
    }
    // SAFETY: `ns_window` is GPUI's live NSWindow for this pop-up and
    // `parent_ns` (when non-zero) the live NSWindow of the window that
    // is rendering the owner this frame; every selector below exists on
    // NSWindow / NSView / NSVisualEffectView / NSImage / NSBezierPath /
    // NSAnimationContext, and all calls happen on the main thread inside
    // the window builder.
    unsafe {
        for button in 0u64..3 {
            let b: *mut Object = msg_send![ns_window, standardWindowButton: button];
            if !b.is_null() {
                let _: () = msg_send![b, setHidden: YES];
            }
        }
        let _: () = msg_send![ns_window, setOpaque: NO];
        let content: *mut Object = msg_send![ns_window, contentView];
        if !content.is_null() {
            let _: () = msg_send![content, setWantsLayer: YES];
            let bounds: CGRect = msg_send![content, bounds];
            let effect: *mut Object = msg_send![class!(NSVisualEffectView), alloc];
            let effect: *mut Object = msg_send![effect, initWithFrame: bounds];
            let material = if popover {
                MATERIAL_POPOVER
            } else {
                MATERIAL_MENU
            };
            let _: () = msg_send![effect, setMaterial: material];
            let _: () = msg_send![effect, setBlendingMode: BLENDING_BEHIND_WINDOW];
            let _: () = msg_send![effect, setState: STATE_ACTIVE];
            let _: () = msg_send![effect, setAutoresizingMask: AUTORESIZE_FILL];
            let mask = rounded_mask(corner_radius as f64);
            if !mask.is_null() {
                let _: () = msg_send![effect, setMaskImage: mask];
            }
            let _: () = msg_send![
                content,
                addSubview: effect
                positioned: ORDER_BELOW
                relativeTo: std::ptr::null_mut::<Object>()
            ];
            // The superview retains it now.
            let _: () = msg_send![effect, release];
        }
        let _: () = msg_send![ns_window, setHasShadow: YES];
        let _: () = msg_send![ns_window, invalidateShadow];
        // Re-derive the shadow from the drawn (rounded) alpha once GPUI
        // has rendered the first frame.
        let _: () = msg_send![
            ns_window,
            performSelector: sel!(invalidateShadow)
            withObject: std::ptr::null_mut::<Object>()
            afterDelay: 0.05f64
        ];
        let parent = parent_ns as *mut Object;
        if !parent.is_null() {
            // NSWindowAbove = 1.
            let _: () = msg_send![parent, addChildWindow: ns_window ordered: 1i64];
        }
        if fade_secs > 0.0 {
            let _: () = msg_send![ns_window, setAlphaValue: 0.0f64];
            let ctx_class = class!(NSAnimationContext);
            let _: () = msg_send![ctx_class, beginGrouping];
            let ctx: *mut Object = msg_send![ctx_class, currentContext];
            let _: () = msg_send![ctx, setDuration: fade_secs];
            let animator: *mut Object = msg_send![ns_window, animator];
            let _: () = msg_send![animator, setAlphaValue: 1.0f64];
            let _: () = msg_send![ctx_class, endGrouping];
        }
    }
    if debug_enabled() {
        debug_dump(window, "shape_popup");
    }
}

/// A stretchable rounded-rect mask (`capInsets` = radius), the AppKit
/// way to round an `NSVisualEffectView`.
unsafe fn rounded_mask(radius: f64) -> *mut Object {
    let r = radius.max(0.0);
    let d = (r * 2.0 + 1.0).max(1.0);
    // SAFETY: plain AppKit drawing on the main thread; the image is
    // autoreleased and retained by `setMaskImage:`.
    unsafe {
        let size = CGSize {
            width: d,
            height: d,
        };
        let image: *mut Object = msg_send![class!(NSImage), alloc];
        let image: *mut Object = msg_send![image, initWithSize: size];
        if image.is_null() {
            return image;
        }
        let _: () = msg_send![image, lockFocus];
        let black: *mut Object = msg_send![class!(NSColor), blackColor];
        let _: () = msg_send![black, set];
        let rect = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size,
        };
        let path: *mut Object = msg_send![
            class!(NSBezierPath),
            bezierPathWithRoundedRect: rect
            xRadius: r
            yRadius: r
        ];
        let _: () = msg_send![path, fill];
        let _: () = msg_send![image, unlockFocus];
        let insets = NSEdgeInsets {
            top: r,
            left: r,
            bottom: r,
            right: r,
        };
        let _: () = msg_send![image, setCapInsets: insets];
        // NSImageResizingModeStretch = 1.
        let _: () = msg_send![image, setResizingMode: 1i64];
        let _: *mut Object = msg_send![image, autorelease];
        image
    }
}

/// `-description` / `-className` as a Rust string.
unsafe fn ns_string(obj: *mut Object) -> String {
    if obj.is_null() {
        return "nil".into();
    }
    // SAFETY: `obj` is an NSString; `UTF8String` is valid while it lives.
    unsafe {
        let c: *const std::os::raw::c_char = msg_send![obj, UTF8String];
        if c.is_null() {
            return String::new();
        }
        std::ffi::CStr::from_ptr(c).to_string_lossy().into_owned()
    }
}

unsafe fn class_name(obj: *mut Object) -> String {
    // SAFETY: every NSObject answers `className`.
    unsafe {
        let n: *mut Object = msg_send![obj, className];
        ns_string(n)
    }
}

unsafe fn dump_view(view: *mut Object, depth: usize, out: &mut String) {
    use std::fmt::Write as _;
    // SAFETY: `view` is a live NSView of the pop-up window.
    unsafe {
        let frame: CGRect = msg_send![view, frame];
        let hidden: BOOL = msg_send![view, isHidden];
        let wants: BOOL = msg_send![view, wantsLayer];
        let layer: *mut Object = msg_send![view, layer];
        let _ = write!(
            out,
            "{:indent$}{} frame=({:.0},{:.0} {:.0}x{:.0}) hidden={} wantsLayer={}",
            "",
            class_name(view),
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
            hidden != NO,
            wants != NO,
            indent = depth * 2
        );
        if !layer.is_null() {
            let opaque: BOOL = msg_send![layer, isOpaque];
            let radius: f64 = msg_send![layer, cornerRadius];
            let masks: BOOL = msg_send![layer, masksToBounds];
            let opacity: f32 = msg_send![layer, opacity];
            let _ = write!(
                out,
                " layer={} opaque={} opacity={} cornerRadius={} masksToBounds={}",
                class_name(layer),
                opaque != NO,
                opacity,
                radius,
                masks != NO
            );
        }
        let is_effect: BOOL = msg_send![view, isKindOfClass: class!(NSVisualEffectView)];
        if is_effect != NO {
            let material: i64 = msg_send![view, material];
            let blending: i64 = msg_send![view, blendingMode];
            let state: i64 = msg_send![view, state];
            let mask: *mut Object = msg_send![view, maskImage];
            let _ = write!(
                out,
                " material={material} blendingMode={blending} state={state} maskImage={}",
                !mask.is_null()
            );
        }
        out.push('\n');
        let subviews: *mut Object = msg_send![view, subviews];
        let count: u64 = msg_send![subviews, count];
        for i in 0..count {
            let child: *mut Object = msg_send![subviews, objectAtIndex: i];
            dump_view(child, depth + 1, out);
        }
    }
}

/// Print the pop-up window, its view hierarchy and layer opacity
/// (`REFINEUI_VIBRANCY_DEBUG=1`).
pub(crate) fn debug_dump(window: &Window, stage: &str) {
    let ns_window = ns_window_of(window) as *mut Object;
    if ns_window.is_null() {
        eprintln!("[refineui vibrancy] {stage}: no NSWindow");
        return;
    }
    let mut out = String::new();
    // SAFETY: read-only queries on the live pop-up NSWindow.
    unsafe {
        let info: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
        let os: *mut Object = msg_send![info, operatingSystemVersionString];
        let opaque: BOOL = msg_send![ns_window, isOpaque];
        let alpha: f64 = msg_send![ns_window, alphaValue];
        let level: i64 = msg_send![ns_window, level];
        let style: u64 = msg_send![ns_window, styleMask];
        let shadow: BOOL = msg_send![ns_window, hasShadow];
        let bg: *mut Object = msg_send![ns_window, backgroundColor];
        let bg_desc: *mut Object = msg_send![bg, description];
        let parent: *mut Object = msg_send![ns_window, parentWindow];
        let reduce: BOOL = {
            let ws: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
            msg_send![ws, accessibilityDisplayShouldReduceTransparency]
        };
        out.push_str(&format!(
            "[refineui vibrancy] {stage}: macOS {} window={:p} class={} opaque={} alpha={alpha:.2} level={level} styleMask={style:#x} hasShadow={} parent={:p} background={} reduceTransparency={}\n",
            ns_string(os),
            ns_window,
            class_name(ns_window),
            opaque != NO,
            shadow != NO,
            parent,
            ns_string(bg_desc),
            reduce != NO,
        ));
        let frame_view: *mut Object = {
            let content: *mut Object = msg_send![ns_window, contentView];
            if content.is_null() {
                content
            } else {
                msg_send![content, superview]
            }
        };
        let root: *mut Object = if frame_view.is_null() {
            msg_send![ns_window, contentView]
        } else {
            frame_view
        };
        if !root.is_null() {
            dump_view(root, 1, &mut out);
        }
    }
    eprint!("{out}");
}
