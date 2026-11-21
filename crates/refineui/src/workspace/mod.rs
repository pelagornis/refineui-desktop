//! **Composition layer** — workspace layout (brief §3, §5 Workspace, §6).
//!
//! Two kinds of state, kept apart (brief §8):
//!
//! * **Layout state** ([`layout`]) — what the user arranged: splits and
//!   their ratios, tab groups and their documents, dock areas (visible,
//!   size, panels), floating panels. Plain data, `serde`-serializable with
//!   the `serde` feature, owned by the application and persisted between
//!   sessions.
//! * **Transient UI state** — hover, pressed, in-flight drags, animation
//!   progress, scroll offsets. Owned by components (GPUI element state),
//!   never persisted.
//!
//! The views (Phase 3) render that model and report every user change
//! back through callbacks, so the app updates its `WorkspaceLayout` and
//! re-renders:
//!
//! | View | Model |
//! | --- | --- |
//! | [`SplitView`] + [`ResizeHandle`], [`PaneTree`] | `PaneNode::Split` ratios |
//! | [`TabBar`] / [`TabGroup`] / [`DocumentTab`] | `TabGroupState` |
//! | [`Panel`] / [`PanelGroup`] | a dock's panels |
//! | [`Dock`] / [`DockArea`] | `Docks`, `zoomed` |
//! | [`Inspector`] | — (property grid for the right dock) |
//!
//! Floating panels own a window, so they live in the application layer
//! (`crate::shell::FloatingPanel`), as does multi-window persistence
//! (`crate::shell::session`, `serde` feature).

mod dock;
mod inspector;
pub mod layout;
mod panel;
mod split_view;
mod tabs;

pub use dock::*;
pub use inspector::*;
pub use layout::{
    DockSide, DockState, Docks, FloatingPanelState, PaneNode, PanelLocation, SplitAxis,
    TabGroupState, WorkspaceLayout, ZoomTarget,
};
pub use panel::*;
pub use split_view::*;
pub use tabs::*;

use std::any::TypeId;

use gpui::{AnyView, AnyWeakView, App, Entity, Global, Render};

/// Which workspace drag (tab, panel) is in flight, so drop targets that
/// are invisible at rest (edge zones, split overlays) can show up.
struct ActiveWorkspaceDrag {
    kind: TypeId,
    view: AnyWeakView,
}
impl Global for ActiveWorkspaceDrag {}

/// Record that the drag whose preview is `view` carries a `T` (call from
/// the `on_drag` constructor) and return the view.
pub fn mark_drag<T: 'static, V: Render>(view: Entity<V>, cx: &mut App) -> Entity<V> {
    cx.set_global(ActiveWorkspaceDrag {
        kind: TypeId::of::<T>(),
        view: AnyView::from(view.clone()).downgrade(),
    });
    view
}

/// A drag of `T` started with [`mark_drag`] is in flight.
pub fn is_dragging<T: 'static>(cx: &App) -> bool {
    cx.has_active_drag()
        && cx
            .try_global::<ActiveWorkspaceDrag>()
            .is_some_and(|d| d.kind == TypeId::of::<T>() && d.view.upgrade().is_some())
}

/// Insertion slot (`0..=items.len()`) for a pointer at `pos` along a strip
/// whose items span `(start, extent)`: the number of items whose midpoint
/// lies before the pointer.
pub fn insertion_slot(pos: f32, items: &[(f32, f32)]) -> usize {
    items.iter().filter(|(s, e)| s + e / 2. < pos).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_slots() {
        let items = [(0., 100.), (100., 80.), (180., 120.)];
        assert_eq!(insertion_slot(-5., &items), 0);
        assert_eq!(insertion_slot(49., &items), 0);
        assert_eq!(insertion_slot(51., &items), 1);
        assert_eq!(insertion_slot(150., &items), 2);
        assert_eq!(insertion_slot(1000., &items), 3);
        assert_eq!(insertion_slot(10., &[]), 0);
    }
}
