//! Overlay layers that may nest.
//!
//! GPUI's `deferred` cannot be used while a deferred element is drawing
//! (`"cannot call defer_draw during deferred drawing"`), so a Select inside
//! a Dialog, a submenu inside a context menu, or a Select inside a Popover
//! would panic. [`layer`] behaves like `deferred` at the top level; inside
//! another layer it queues itself and is prepainted and painted right after
//! the enclosing layer's content (with its own tree context, through the
//! vendored gpui's `Window::element_context`), so it paints *and hit-tests*
//! above everything in that layer — like the web, where a Select inside a
//! Popover portals its list to `body` as a later `z-popup` layer. Its
//! backdrop blur therefore samples the finished enclosing layer.

use std::cell::RefCell;

use gpui::{
    AnyElement, App, Bounds, Element, ElementContext, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Point, Window,
};

thread_local! {
    /// How many layers are currently prepainting / painting.
    static DEPTH: RefCell<usize> = const { RefCell::new(0) };
    /// Layers met while prepainting each enclosing layer (innermost last).
    static QUEUE: RefCell<Vec<Vec<Nested>>> = const { RefCell::new(Vec::new()) };
}

thread_local! {
    /// Set when an overlay surface took the current mouse down.
    static CLAIMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// An overlay surface (Select list, menu) took this mouse down. A press on a
/// Select list that overhangs its Popover is still "inside" the Popover.
pub(crate) fn claim_mouse_down() {
    CLAIMED.with(|c| c.set(true));
}

/// Outside-press handler for a popover-like layer that can hold nested
/// overlays: runs `close` after the event unless a nested overlay surface
/// claimed the press ([`claim_mouse_down`]). Outer layers paint (and so
/// capture) before their nested layers, so this resets the claim, and the
/// nested surface sets it later in the same dispatch.
pub(crate) fn close_on_outside_press(
    window: &mut Window,
    cx: &mut App,
    close: impl FnOnce(&mut Window, &mut App) + 'static,
) {
    CLAIMED.with(|c| c.set(false));
    window.defer(cx, move |window, cx| {
        if !CLAIMED.with(|c| c.get()) {
            close(window, cx);
        }
    });
}

/// A layer inside another layer, drawn after that layer's content.
struct Nested {
    element: AnyElement,
    offset: Point<Pixels>,
    priority: usize,
    context: ElementContext,
}

fn depth() -> usize {
    DEPTH.with(|d| *d.borrow())
}

/// Draw `child` on top of everything else (like `gpui::deferred`), safely
/// nestable.
pub fn layer(child: impl IntoElement) -> Layer {
    Layer {
        child: Some(
            Guard {
                child: child.into_any_element(),
                nested: Vec::new(),
            }
            .into_any_element(),
        ),
        priority: 0,
    }
}

/// See [`layer`].
pub struct Layer {
    child: Option<AnyElement>,
    priority: usize,
}

impl Layer {
    /// Draw order among top-level layers (higher on top).
    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

impl IntoElement for Layer {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Layer {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.as_mut().unwrap().request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _cx: &mut App,
    ) {
        let child = self.child.take().unwrap();
        if depth() > 0 {
            let nested = Nested {
                element: child,
                offset: window.element_offset(),
                priority: self.priority,
                context: window.element_context(),
            };
            QUEUE.with(|q| q.borrow_mut().last_mut().unwrap().push(nested));
        } else {
            window.defer_draw(child, window.element_offset(), self.priority);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
        // Drawn by `defer_draw` or by the enclosing layer's `Guard`.
    }
}

/// Marks "inside a layer" while its child draws, then draws the layers
/// nested in it on top.
struct Guard {
    child: AnyElement,
    nested: Vec<Nested>,
}

impl IntoElement for Guard {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Guard {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        DEPTH.with(|d| *d.borrow_mut() += 1);
        QUEUE.with(|q| q.borrow_mut().push(Vec::new()));
        self.child.prepaint(window, cx);
        let mut nested = QUEUE.with(|q| q.borrow_mut().pop()).unwrap_or_default();
        nested.sort_by_key(|n| n.priority);
        for n in &mut nested {
            let (element, offset) = (&mut n.element, n.offset);
            window.with_element_context(&n.context, |window| {
                window.with_absolute_element_offset(offset, |window| element.prepaint(window, cx))
            });
        }
        self.nested = nested;
        DEPTH.with(|d| *d.borrow_mut() -= 1);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        DEPTH.with(|d| *d.borrow_mut() += 1);
        self.child.paint(window, cx);
        for n in &mut self.nested {
            let element = &mut n.element;
            window.with_element_context(&n.context, |window| element.paint(window, cx));
        }
        DEPTH.with(|d| *d.borrow_mut() -= 1);
    }
}
