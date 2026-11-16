//! Focus order and roving focus.
//!
//! Rules (see the *Interaction model* architecture page):
//!
//! 1. Tab order follows reading order of **regions** (titlebar → toolbar →
//!    activity bar → sidebars → workspace → panels → status bar); inside a
//!    region it follows document order (GPUI `tab_index`).
//! 2. Composite widgets (lists, trees, tabs, toolbars, menus, radio groups)
//!    are **one** Tab stop with roving focus inside ([`RovingFocus`]).
//! 3. Modal surfaces trap focus and restore it to the opener on close.
//! 4. The focus ring shows only for keyboard focus (`:focus-visible`,
//!    `refineui::motion::focus_visible`).
//! 5. `F6` / `Shift-F6` cycle regions (brief §7: keyboard-first).

/// Next index for roving focus over `len` items, skipping `disabled` ones.
///
/// `delta` is ±1 (arrows) or ±`len` (Home / End use [`RovingFocus::first`]
/// / [`RovingFocus::last`]). Returns `None` when nothing is focusable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RovingFocus {
    /// Wrap from the last item to the first (menus) or stop (lists, trees).
    pub wrap: bool,
}

impl RovingFocus {
    /// Move from `current` by `delta`, skipping disabled items.
    pub fn step(
        self,
        len: usize,
        current: Option<usize>,
        delta: isize,
        disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let Some(cur) = current else {
            return if delta >= 0 {
                self.first(len, &disabled)
            } else {
                self.last(len, &disabled)
            };
        };
        let mut i = cur as isize;
        for _ in 0..len {
            i += delta.signum();
            if i < 0 || i >= len as isize {
                if !self.wrap {
                    return Some(cur);
                }
                i = i.rem_euclid(len as isize);
            }
            if !disabled(i as usize) {
                return Some(i as usize);
            }
        }
        Some(cur)
    }

    /// First enabled item.
    pub fn first(self, len: usize, disabled: impl Fn(usize) -> bool) -> Option<usize> {
        (0..len).find(|i| !disabled(*i))
    }

    /// Last enabled item.
    pub fn last(self, len: usize, disabled: impl Fn(usize) -> bool) -> Option<usize> {
        (0..len).rev().find(|i| !disabled(*i))
    }
}

/// F6 region cycling (brief §6): the next region after `current` among
/// `len` regions, wrapping; `None` (focus outside every region) starts at
/// the first (forward) or last (backward) region.
pub fn cycle_region(current: Option<usize>, len: usize, forward: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match (current, forward) {
        (None, true) => 0,
        (None, false) => len - 1,
        (Some(i), true) => (i + 1) % len,
        (Some(i), false) => (i + len - 1) % len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roving_skips_disabled_and_wraps_only_when_asked() {
        let off = |i: usize| i == 2;
        let list = RovingFocus { wrap: false };
        let menu = RovingFocus { wrap: true };
        assert_eq!(list.step(4, Some(1), 1, off), Some(3));
        assert_eq!(list.step(4, Some(3), 1, off), Some(3));
        assert_eq!(menu.step(4, Some(3), 1, off), Some(0));
        assert_eq!(menu.step(4, Some(0), -1, off), Some(3));
        assert_eq!(list.step(4, None, 1, off), Some(0));
        assert_eq!(list.step(0, None, 1, off), None);
        assert_eq!(list.last(4, |i| i == 3), Some(2));
    }

    #[test]
    fn regions_cycle_and_wrap() {
        assert_eq!(cycle_region(None, 3, true), Some(0));
        assert_eq!(cycle_region(None, 3, false), Some(2));
        assert_eq!(cycle_region(Some(2), 3, true), Some(0));
        assert_eq!(cycle_region(Some(0), 3, false), Some(2));
        assert_eq!(cycle_region(Some(0), 0, true), None);
    }
}
