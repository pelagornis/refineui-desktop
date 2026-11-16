//! Overlay layers that may nest.
//!
//! GPUI's `deferred` cannot be used while a deferred element is drawing
//! (`"cannot call defer_draw during deferred drawing"`), so a Select inside
//! a Dialog, a submenu inside a context menu, or a Select inside a Popover
//! would panic. [`layer`] behaves like `deferred` at the top level; inside
//! another layer it queues itself and is drawn right after the enclosing
//! layer's content, so it still paints above it.

use std::cell::RefCell;

use gpui::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Window,
};

thread_local! {
    /// How many layers are currently prepainting / painting.
    static DEPTH: RefCell<usize> = const { RefCell::new(0) };
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
            }
            .into_any_element(),
        ),
        priority: 0,
        inline: false,
    }
}

/// See [`layer`].
pub struct Layer {
    child: Option<AnyElement>,
    priority: usize,
    /// Drawn in place because an enclosing layer is already drawing.
    inline: bool,
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
        cx: &mut App,
    ) {
        if depth() > 0 {
            self.inline = true;
            self.child.as_mut().unwrap().prepaint(window, cx);
        } else {
            let child = self.child.take().unwrap();
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
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.inline
            && let Some(child) = self.child.as_mut()
        {
            child.paint(window, cx);
        }
    }
}

/// Marks "inside a layer" while its child draws.
struct Guard {
    child: AnyElement,
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
        self.child.prepaint(window, cx);
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
        DEPTH.with(|d| *d.borrow_mut() -= 1);
    }
}
