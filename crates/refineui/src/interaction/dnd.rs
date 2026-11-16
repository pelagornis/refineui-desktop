//! Drag and drop geometry (brief §4 "Drag and Drop", §5 Drag Handle /
//! Drag Preview / Drop Zone).
//!
//! GPUI provides the transport (`on_drag` with a typed payload,
//! `drag_over::<T>`, `on_drop::<T>`); this module decides *where* a drop
//! lands so every list, tree, tab bar and dock area behaves the same.
//! Drag previews paint on [`crate::foundation::LayerOrder::DragPreview`].

/// Where a dragged item lands relative to a row / tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropEdge {
    /// Before the target (above a row, left of a tab).
    Before,
    /// Into the target (tree folder, tab group).
    Inside,
    /// After the target.
    After,
}

/// Drop position along a row of `extent` px (height for rows, width for
/// tabs) at `offset` px from its start. With `allow_inside` the middle
/// half means [`DropEdge::Inside`]; otherwise the row splits in two.
pub fn drop_edge(offset: f32, extent: f32, allow_inside: bool) -> DropEdge {
    if extent <= 0.0 {
        return DropEdge::After;
    }
    let t = (offset / extent).clamp(0.0, 1.0);
    if allow_inside {
        if t < 0.25 {
            DropEdge::Before
        } else if t > 0.75 {
            DropEdge::After
        } else {
            DropEdge::Inside
        }
    } else if t < 0.5 {
        DropEdge::Before
    } else {
        DropEdge::After
    }
}

/// Where a panel / document dropped on a pane goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockZone {
    /// Join the pane's tab group.
    Center,
    /// Split the pane; the new part goes left.
    Left,
    /// Split; new part right.
    Right,
    /// Split; new part on top.
    Top,
    /// Split; new part at the bottom.
    Bottom,
}

/// Dock zone for a point `(x, y)` inside a pane of `width × height`
/// (pane-local px). The outer `edge` fraction (default 0.25) of each side
/// splits; the rest joins. Corners pick the nearer edge.
pub fn dock_zone(x: f32, y: f32, width: f32, height: f32, edge: f32) -> DockZone {
    if width <= 0.0 || height <= 0.0 {
        return DockZone::Center;
    }
    let fx = (x / width).clamp(0.0, 1.0);
    let fy = (y / height).clamp(0.0, 1.0);
    let d = [
        (fx, DockZone::Left),
        (1.0 - fx, DockZone::Right),
        (fy, DockZone::Top),
        (1.0 - fy, DockZone::Bottom),
    ];
    let (dist, zone) = d
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or((1.0, DockZone::Center));
    if dist < edge { zone } else { DockZone::Center }
}

/// Index a dragged item is inserted at in a list of `len` when dropped on
/// `target` at `edge` (insertion index *before* removing the source).
pub fn insertion_index(target: usize, edge: DropEdge, len: usize) -> usize {
    match edge {
        DropEdge::Before | DropEdge::Inside => target.min(len),
        DropEdge::After => (target + 1).min(len),
    }
}

/// Move `from` to insertion index `to` (as from [`insertion_index`]) in
/// `items`, returning the item's final index.
pub fn reorder<T>(items: &mut Vec<T>, from: usize, to: usize) -> usize {
    if from >= items.len() {
        return from;
    }
    let item = items.remove(from);
    let to = if to > from { to - 1 } else { to }.min(items.len());
    items.insert(to, item);
    to
}

/// The part of a pane (`x`, `y`, `width`, `height` as fractions of the pane)
/// a [`DockZone`] drop would occupy — what a drop-zone overlay highlights.
pub fn dock_zone_fraction(zone: DockZone) -> (f32, f32, f32, f32) {
    match zone {
        DockZone::Center => (0., 0., 1., 1.),
        DockZone::Left => (0., 0., 0.5, 1.),
        DockZone::Right => (0.5, 0., 0.5, 1.),
        DockZone::Top => (0., 0., 1., 0.5),
        DockZone::Bottom => (0., 0.5, 1., 0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_and_zones() {
        assert_eq!(drop_edge(2.0, 28.0, true), DropEdge::Before);
        assert_eq!(drop_edge(14.0, 28.0, true), DropEdge::Inside);
        assert_eq!(drop_edge(27.0, 28.0, true), DropEdge::After);
        assert_eq!(drop_edge(13.0, 28.0, false), DropEdge::Before);
        assert_eq!(dock_zone(10.0, 200.0, 400.0, 400.0, 0.25), DockZone::Left);
        assert_eq!(
            dock_zone(200.0, 390.0, 400.0, 400.0, 0.25),
            DockZone::Bottom
        );
        assert_eq!(
            dock_zone(200.0, 200.0, 400.0, 400.0, 0.25),
            DockZone::Center
        );
    }

    #[test]
    fn reorder_moves_items() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        // Drag 'a' after 'c'.
        let to = insertion_index(2, DropEdge::After, v.len());
        assert_eq!(reorder(&mut v, 0, to), 2);
        assert_eq!(v, ['b', 'c', 'a', 'd']);
        // Drag 'd' before 'b'.
        let to = insertion_index(0, DropEdge::Before, v.len());
        assert_eq!(reorder(&mut v, 3, to), 0);
        assert_eq!(v, ['d', 'b', 'c', 'a']);
    }

    #[test]
    fn zone_fractions_cover_half_the_pane() {
        assert_eq!(dock_zone_fraction(DockZone::Center), (0., 0., 1., 1.));
        assert_eq!(dock_zone_fraction(DockZone::Right), (0.5, 0., 0.5, 1.));
        assert_eq!(dock_zone_fraction(DockZone::Bottom), (0., 0.5, 1., 0.5));
    }
}
