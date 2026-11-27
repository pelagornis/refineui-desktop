//! macOS (AppKit) glue — the only `unsafe` code in the crate.
//!
//! Native frost for pop-up windows (`NSVisualEffectView` behind a masked,
//! rounded content view) and the `REFINEUI_VIBRANCY_DEBUG` hierarchy dump,
//! called by [`crate::vibrancy`]. Compiled on macOS, and on other hosts with
//! the `typecheck-macos-vibrancy` feature for `cargo check` only — it is
//! never called there.
use std::ffi::CString;
use std::sync::OnceLock;

use gpui::Window;
use objc::declare::ClassDecl;
use objc::runtime::{BOOL, Class, NO, Object, Sel, YES};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::vibrancy::NativePopupAppearance;

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

/// `NSVisualEffectMaterialSelection` — a colorless semantic material. Apple's
/// `.menu` / `.popover` materials already carry a system tint, extra
/// saturation and a wallpaper "chameleon" color; painting RefineUI's frost
/// overlay on top of that made the plate look off (and wildly so when the
/// window's `NSAppearance` did not match the app theme).
const MATERIAL_SELECTION: i64 = 4;
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

/// Hide the (titled-panel) window buttons, put a rounded, colorless
/// `NSVisualEffectView` (behind-window blending, always active, system
/// tint/saturation stripped) under GPUI's view, apply the theme
/// appearance, let AppKit draw the window shadow, attach the pop-up to
/// `parent_ns` (so it follows the parent's moves) and fade its alpha in
/// over `fade_secs`.
///
/// Why not `WindowBackgroundAppearance::Blurred`: GPUI's blur view uses
/// the same colorless `Selection` material, but the earlier prototype
/// rounded the corners by setting `cornerRadius` + `masksToBounds` on the
/// *content view's* layer — the effect view's backdrop layer then sits
/// under a masking ancestor, which makes the window server fall back to a
/// flat translucent fill (no blur). Apple's documented way to shape
/// vibrancy is the effect view's own `maskImage`, which is what this
/// does; the pop-up window is opened `Transparent` so GPUI adds no second
/// (square) effect view. The content view is left without an extra layer
/// backing: a flattening ancestor also kills behind-window vibrancy.
pub(crate) fn shape_popup(
    window: &Window,
    corner_radius: f32,
    parent_ns: usize,
    fade_secs: f64,
    appearance: NativePopupAppearance,
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
        apply_appearance(ns_window, appearance);
        let content: *mut Object = msg_send![ns_window, contentView];
        if !content.is_null() {
            let bounds: CGRect = msg_send![content, bounds];
            let class = frost_view_class();
            let effect: *mut Object = msg_send![class, alloc];
            let effect: *mut Object = msg_send![effect, initWithFrame: bounds];
            let _: () = msg_send![effect, setMaterial: MATERIAL_SELECTION];
            let _: () = msg_send![effect, setBlendingMode: BLENDING_BEHIND_WINDOW];
            let _: () = msg_send![effect, setState: STATE_ACTIVE];
            let _: () = msg_send![effect, setAutoresizingMask: AUTORESIZE_FILL];
            let mask = rounded_mask(corner_radius as f64);
            if !mask.is_null() {
                let _: () = msg_send![effect, setMaskImage: mask];
            }
            // Strip once now; `updateLayer` strips again when AppKit
            // rebuilds the effect filters.
            strip_vibrant_tint(effect);
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

/// Resize a pop-up's content to `width` × `height`, keeping its bottom edge
/// (`keep_bottom`, it grows up) or its top edge (it grows down) in place.
/// `-[NSWindow setContentSize:]` (GPUI's `Window::resize`) always keeps the
/// bottom-left corner, so a top-anchored pop-up would grow the wrong way.
/// Runs on the next main-loop turn, like GPUI's own resize, so AppKit's
/// resize callbacks never re-enter a GPUI update.
pub(crate) fn resize_popup(
    window: &Window,
    width: f32,
    height: f32,
    keep_bottom: bool,
    cx: &gpui::App,
) {
    let ns_window = ns_window_of(window);
    if ns_window == 0 {
        return;
    }
    cx.foreground_executor()
        .spawn(async move {
            let ns_window = ns_window as *mut Object;
            // SAFETY: `ns_window` is the live NSWindow of the pop-up being
            // rendered (closing it goes through GPUI on this same thread,
            // after this task); `frame`, `contentRectForFrameRect:`,
            // `frameRectForContentRect:`, `setFrame:display:` and
            // `invalidateShadow` are NSWindow API called on the main thread.
            unsafe {
                let frame: CGRect = msg_send![ns_window, frame];
                let content: CGRect = msg_send![ns_window, contentRectForFrameRect: frame];
                let (w, h) = (width as f64, height as f64);
                // AppKit screen space grows upward: the top edge is y + h.
                let y = if keep_bottom {
                    content.origin.y
                } else {
                    content.origin.y + content.size.height - h
                };
                let next = CGRect {
                    origin: CGPoint {
                        x: content.origin.x,
                        y,
                    },
                    size: CGSize {
                        width: w,
                        height: h,
                    },
                };
                let next: CGRect = msg_send![ns_window, frameRectForContentRect: next];
                let _: () = msg_send![ns_window, setFrame: next display: YES];
                let _: () = msg_send![ns_window, invalidateShadow];
            }
        })
        .detach();
}

/// Keep a live pop-up's `NSAppearance` in sync with the RefineUI theme
/// (e.g. after the gallery toggles light / dark).
pub(crate) fn sync_popup_appearance(window: &Window, appearance: NativePopupAppearance) {
    let ns_window = ns_window_of(window) as *mut Object;
    if ns_window.is_null() {
        return;
    }
    // SAFETY: `ns_window` is this pop-up's live NSWindow; `-setAppearance:`
    // is a no-side-effect style update on the main thread.
    unsafe {
        apply_appearance(ns_window, appearance);
    }
}

unsafe fn apply_appearance(ns_window: *mut Object, appearance: NativePopupAppearance) {
    // SAFETY: `appearanceNamed:` returns a system appearance (or nil);
    // `setAppearance:` accepts nil to inherit from the parent / app.
    unsafe {
        let app = match appearance {
            NativePopupAppearance::System => std::ptr::null_mut(),
            NativePopupAppearance::Light => appearance_named("NSAppearanceNameAqua"),
            NativePopupAppearance::Dark => appearance_named("NSAppearanceNameDarkAqua"),
        };
        let _: () = msg_send![ns_window, setAppearance: app];
    }
}

unsafe fn appearance_named(name: &str) -> *mut Object {
    // SAFETY: `name` is a documented NSAppearanceName constant.
    unsafe {
        let s = nsstring(name);
        msg_send![class!(NSAppearance), appearanceNamed: s]
    }
}

unsafe fn nsstring(s: &str) -> *mut Object {
    let c = CString::new(s).expect("NSString");
    // SAFETY: `c` lives for the call; the returned NSString is autoreleased.
    unsafe { msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()] }
}

static FROST_VIEW_CLASS: OnceLock<&'static Class> = OnceLock::new();

fn frost_view_class() -> &'static Class {
    FROST_VIEW_CLASS.get_or_init(|| {
        let mut decl = ClassDecl::new("RefineUIFrostView", class!(NSVisualEffectView))
            .expect("RefineUIFrostView already registered");
        unsafe {
            decl.add_method(
                sel!(updateLayer),
                frost_view_update_layer as extern "C" fn(&Object, Sel),
            );
        }
        decl.register()
    })
}

extern "C" fn frost_view_update_layer(this: &Object, _: Sel) {
    // SAFETY: `this` is a RefineUIFrostView; we call through to
    // NSVisualEffectView's `updateLayer` then strip the filters it just
    // re-applied.
    unsafe {
        let _: () = msg_send![super(this, class!(NSVisualEffectView)), updateLayer];
        strip_vibrant_tint(this as *const Object as *mut Object);
    }
}

/// Remove the desktop-tint chameleon layer and the extra saturation
/// filter `NSVisualEffectView` installs, so only the blur remains and
/// RefineUI's frost overlay can carry the color.
unsafe fn strip_vibrant_tint(view: *mut Object) {
    if view.is_null() {
        return;
    }
    // SAFETY: `view` is a live NSVisualEffectView (or a sublayer walk
    // starting at its layer). Same technique as GPUI's `BlurredView`.
    unsafe {
        let layer: *mut Object = msg_send![view, layer];
        if !layer.is_null() {
            strip_layer_tint(layer);
        }
    }
}

unsafe fn strip_layer_tint(layer: *mut Object) {
    // SAFETY: `layer` is a live CALayer in the effect view's tree.
    unsafe {
        let _: () = msg_send![layer, setBackgroundColor: std::ptr::null_mut::<Object>()];

        let class_name: *mut Object = msg_send![layer, className];
        let chameleon = nsstring("CAChameleonLayer");
        let is_chameleon: BOOL = msg_send![class_name, isEqualToString: chameleon];
        if is_chameleon != NO {
            let _: () = msg_send![layer, setHidden: YES];
            return;
        }

        let filters: *mut Object = msg_send![layer, filters];
        if !filters.is_null() {
            let test: *mut Object = nsstring("Saturat");
            let count: u64 = msg_send![filters, count];
            for i in 0..count {
                let filter: *mut Object = msg_send![filters, objectAtIndex: i];
                let description: *mut Object = msg_send![filter, description];
                let hit: BOOL = msg_send![description, containsString: test];
                if hit == NO {
                    continue;
                }
                let all = NSRange {
                    location: 0,
                    length: count,
                };
                let indices: *mut Object = msg_send![class!(NSMutableIndexSet), indexSet];
                let _: () = msg_send![indices, addIndexesInRange: all];
                let _: () = msg_send![indices, removeIndex: i];
                let filtered: *mut Object = msg_send![filters, objectsAtIndexes: indices];
                let _: () = msg_send![layer, setFilters: filtered];
                break;
            }
        }

        let sublayers: *mut Object = msg_send![layer, sublayers];
        if !sublayers.is_null() {
            let count: u64 = msg_send![sublayers, count];
            for i in 0..count {
                let sub: *mut Object = msg_send![sublayers, objectAtIndex: i];
                strip_layer_tint(sub);
            }
        }
    }
}

#[repr(C)]
struct NSRange {
    location: u64,
    length: u64,
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
    // SAFETY: `ns_window` is a live NSWindow; every selector is a read.
    unsafe {
        let os: *mut Object = {
            let p: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
            msg_send![p, operatingSystemVersionString]
        };
        let opaque: BOOL = msg_send![ns_window, isOpaque];
        let alpha: f64 = msg_send![ns_window, alphaValue];
        let level: i64 = msg_send![ns_window, level];
        let style: u64 = msg_send![ns_window, styleMask];
        let shadow: BOOL = msg_send![ns_window, hasShadow];
        let bg: *mut Object = msg_send![ns_window, backgroundColor];
        let bg_desc: *mut Object = msg_send![bg, description];
        let parent: *mut Object = msg_send![ns_window, parentWindow];
        let appearance: *mut Object = msg_send![ns_window, effectiveAppearance];
        let appearance_name: *mut Object = if appearance.is_null() {
            std::ptr::null_mut()
        } else {
            msg_send![appearance, name]
        };
        let reduce: BOOL = {
            let ws: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
            msg_send![ws, accessibilityDisplayShouldReduceTransparency]
        };
        out.push_str(&format!(
            "[refineui vibrancy] {stage}: macOS {} window={:p} class={} opaque={} alpha={alpha:.2} level={level} styleMask={style:#x} hasShadow={} parent={:p} background={} appearance={} reduceTransparency={}\n",
            ns_string(os),
            ns_window,
            class_name(ns_window),
            opaque != NO,
            shadow != NO,
            parent,
            ns_string(bg_desc),
            ns_string(appearance_name),
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
