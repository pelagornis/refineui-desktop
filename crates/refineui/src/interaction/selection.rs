//! Selection model shared by lists, tables, trees and tab groups.
//!
//! Desktop conventions (brief §4 "Selection", §5 Multi / Range Selection):
//!
//! | Gesture | Effect |
//! | --- | --- |
//! | Click / Enter | select only this item; it becomes the anchor |
//! | ⌘-click (macOS) / Ctrl-click | toggle this item; anchor moves here |
//! | Shift-click | select the range anchor..item (replacing the previous range) |
//! | ⌘/Ctrl-Shift-click | add the range anchor..item to the selection |
//! | ↑ / ↓ | move the cursor and select only it |
//! | Shift-↑ / Shift-↓ | move the cursor, extending the range from the anchor |
//! | ⌘/Ctrl-A | select all (multiple mode) |
//! | Escape | clear (when the component allows an empty selection) |
//!
//! The model stores keys, not indices, so it survives sorting, filtering
//! and virtualization; operations that need order take the current visible
//! order as a slice. It is plain data — keep it in your app state.

/// Single or multiple selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// At most one item.
    #[default]
    Single,
    /// Any number of items (⌘/Ctrl-click, Shift ranges, select-all).
    Multiple,
}

/// Selection state over keys `K`.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionModel<K> {
    mode: SelectionMode,
    selected: Vec<K>,
    anchor: Option<K>,
    cursor: Option<K>,
    /// Keys of the range last created from the anchor (replaced by the
    /// next Shift gesture).
    range: Vec<K>,
}

impl<K> Default for SelectionModel<K> {
    fn default() -> Self {
        Self {
            mode: SelectionMode::Single,
            selected: Vec::new(),
            anchor: None,
            cursor: None,
            range: Vec::new(),
        }
    }
}

/// Modifier state of a selection gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectGesture {
    /// ⌘ (macOS) / Ctrl (others) held.
    pub toggle: bool,
    /// Shift held.
    pub extend: bool,
}

impl<K: Clone + PartialEq> SelectionModel<K> {
    /// Empty model in `mode`.
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    /// Selection mode.
    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Selected keys, in the order they were selected.
    pub fn selected(&self) -> &[K] {
        &self.selected
    }

    /// Is `key` selected?
    pub fn is_selected(&self, key: &K) -> bool {
        self.selected.contains(key)
    }

    /// The keyboard cursor (focused item).
    pub fn cursor(&self) -> Option<&K> {
        self.cursor.as_ref()
    }

    /// The range anchor.
    pub fn anchor(&self) -> Option<&K> {
        self.anchor.as_ref()
    }

    /// Clear everything.
    pub fn clear(&mut self) {
        self.selected.clear();
        self.range.clear();
    }

    /// Replace the selection with `keys` (marquee result); the cursor moves
    /// to the last one, the anchor stays.
    pub fn set(&mut self, keys: Vec<K>) {
        if self.mode == SelectionMode::Single {
            self.selected = keys.into_iter().last().into_iter().collect();
        } else {
            self.selected = keys;
        }
        self.range.clear();
        self.cursor = self.selected.last().cloned();
    }

    /// Select only `key`.
    pub fn select(&mut self, key: K) {
        self.selected = vec![key.clone()];
        self.range.clear();
        self.anchor = Some(key.clone());
        self.cursor = Some(key);
    }

    /// Pointer gesture on `key`; `order` is the current visible order.
    pub fn click(&mut self, key: K, gesture: SelectGesture, order: &[K]) {
        if self.mode == SelectionMode::Single {
            self.select(key);
            return;
        }
        match (gesture.toggle, gesture.extend) {
            (_, true) => self.extend_to(key, gesture.toggle, order),
            (true, false) => {
                if let Some(i) = self.selected.iter().position(|k| *k == key) {
                    self.selected.remove(i);
                } else {
                    self.selected.push(key.clone());
                }
                self.range.clear();
                self.anchor = Some(key.clone());
                self.cursor = Some(key);
            }
            (false, false) => self.select(key),
        }
    }

    /// Select the range anchor..`key` (adding to the selection when `add`).
    pub fn extend_to(&mut self, key: K, add: bool, order: &[K]) {
        let anchor = self.anchor.clone().unwrap_or_else(|| key.clone());
        let (Some(a), Some(b)) = (
            order.iter().position(|k| *k == anchor),
            order.iter().position(|k| *k == key),
        ) else {
            self.select(key);
            return;
        };
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let range: Vec<K> = order[lo..=hi].to_vec();
        if add {
            // Replace the previous range, keep the rest.
            let prev = std::mem::take(&mut self.range);
            self.selected.retain(|k| !prev.contains(k));
        } else {
            self.selected.clear();
        }
        for k in &range {
            if !self.selected.contains(k) {
                self.selected.push(k.clone());
            }
        }
        self.range = range;
        self.anchor = Some(anchor);
        self.cursor = Some(key);
    }

    /// Arrow-key move by `delta` over `order`; `extend` = Shift held.
    pub fn move_cursor(&mut self, delta: isize, extend: bool, order: &[K]) {
        if order.is_empty() {
            return;
        }
        let cur = self
            .cursor
            .as_ref()
            .and_then(|c| order.iter().position(|k| k == c));
        let next = match cur {
            None => {
                if delta >= 0 {
                    0
                } else {
                    order.len() - 1
                }
            }
            Some(i) => (i as isize + delta).clamp(0, order.len() as isize - 1) as usize,
        };
        let key = order[next].clone();
        if extend && self.mode == SelectionMode::Multiple {
            self.extend_to(key, false, order);
        } else {
            self.select(key);
        }
    }

    /// Select every key in `order` (multiple mode only).
    pub fn select_all(&mut self, order: &[K]) {
        if self.mode == SelectionMode::Multiple {
            self.selected = order.to_vec();
            self.range.clear();
        }
    }

    /// Drop keys that no longer exist (after filtering / deletion).
    pub fn retain(&mut self, exists: impl Fn(&K) -> bool) {
        self.selected.retain(|k| exists(k));
        self.range.retain(|k| exists(k));
        if self.anchor.as_ref().is_some_and(|k| !exists(k)) {
            self.anchor = None;
        }
        if self.cursor.as_ref().is_some_and(|k| !exists(k)) {
            self.cursor = None;
        }
    }
}

/// An axis-aligned rectangle in any unit (marquee geometry).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width (≥ 0).
    pub w: f32,
    /// Height (≥ 0).
    pub h: f32,
}

impl Rect {
    /// New rectangle.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// The rectangle spanned by two corner points, in any order — the
    /// marquee between the press point and the pointer.
    pub fn from_points(a: (f32, f32), b: (f32, f32)) -> Self {
        Self {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            w: (a.0 - b.0).abs(),
            h: (a.1 - b.1).abs(),
        }
    }

    /// Overlap test (touching edges do not count).
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

/// Marquee (selection box) hit test: indices of `items` the marquee
/// touches. A marquee smaller than `threshold` on both axes is a click,
/// not a drag, and hits nothing.
pub fn marquee_hits(marquee: Rect, items: &[Rect], threshold: f32) -> Vec<usize> {
    if marquee.w < threshold && marquee.h < threshold {
        return Vec::new();
    }
    items
        .iter()
        .enumerate()
        .filter(|(_, r)| marquee.intersects(r))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORDER: [u32; 6] = [1, 2, 3, 4, 5, 6];
    const PLAIN: SelectGesture = SelectGesture {
        toggle: false,
        extend: false,
    };
    const TOGGLE: SelectGesture = SelectGesture {
        toggle: true,
        extend: false,
    };
    const SHIFT: SelectGesture = SelectGesture {
        toggle: false,
        extend: true,
    };
    const BOTH: SelectGesture = SelectGesture {
        toggle: true,
        extend: true,
    };

    #[test]
    fn click_toggle_and_ranges() {
        let mut s = SelectionModel::new(SelectionMode::Multiple);
        s.click(2, PLAIN, &ORDER);
        s.click(4, SHIFT, &ORDER);
        assert_eq!(s.selected(), &[2, 3, 4]);
        // A new Shift range from the same anchor replaces the old one.
        s.click(3, SHIFT, &ORDER);
        assert_eq!(s.selected(), &[2, 3]);
        s.click(6, TOGGLE, &ORDER);
        assert_eq!(s.selected(), &[2, 3, 6]);
        s.click(3, TOGGLE, &ORDER);
        assert_eq!(s.selected(), &[2, 6]);
        // ⌘-Shift adds a range from the new anchor (3 was toggled → anchor).
        s.click(5, BOTH, &ORDER);
        assert_eq!(s.selected(), &[2, 6, 3, 4, 5]);
    }

    #[test]
    fn keyboard_and_single_mode() {
        let mut s = SelectionModel::new(SelectionMode::Multiple);
        s.move_cursor(1, false, &ORDER);
        assert_eq!(s.selected(), &[1]);
        s.move_cursor(1, true, &ORDER);
        s.move_cursor(1, true, &ORDER);
        assert_eq!(s.selected(), &[1, 2, 3]);
        s.move_cursor(-1, true, &ORDER);
        assert_eq!(s.selected(), &[1, 2]);
        s.select_all(&ORDER);
        assert_eq!(s.selected().len(), 6);
        s.retain(|k| *k % 2 == 0);
        assert_eq!(s.selected(), &[2, 4, 6]);

        let mut one = SelectionModel::new(SelectionMode::Single);
        one.click(2, TOGGLE, &ORDER);
        one.click(5, SHIFT, &ORDER);
        assert_eq!(one.selected(), &[5]);
        one.select_all(&ORDER);
        assert_eq!(one.selected(), &[5]);
    }

    #[test]
    fn marquee_hits_intersecting_items() {
        let m = Rect::from_points((50., 50.), (10., 10.));
        assert_eq!(m, Rect::new(10., 10., 40., 40.));
        let items = [
            Rect::new(0., 0., 20., 20.),
            Rect::new(45., 45., 10., 10.),
            Rect::new(50., 0., 10., 10.),
            Rect::new(100., 100., 10., 10.),
        ];
        assert_eq!(marquee_hits(m, &items, 4.), vec![0, 1]);
        // A tiny drag is a click.
        let click = Rect::from_points((5., 5.), (7., 6.));
        assert!(marquee_hits(click, &items, 4.).is_empty());
    }
}
