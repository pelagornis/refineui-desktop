//! **Interaction layer** — focus, keyboard, selection, pointer and drag
//! and drop (brief §3, §4, §7).
//!
//! Each submodule is *logic first*: plain state machines with no GPUI
//! types, unit-tested on their own, that components drive from GPUI
//! events. That keeps behaviour identical across components (a Tree, a
//! Table and a List select the same way) and keeps application state out
//! of transient component state (brief §8).
//!
//! * [`keymap`] — key bindings with contexts, platform-resolved
//!   `secondary-`, conflict detection and lookup for shortcut hints.
//! * [`selection`] — single / multiple selection with anchor, ⌘/Ctrl-click
//!   toggle, Shift range, keyboard extension and select-all.
//! * [`focus`] — roving focus and focus-order helpers.
//! * [`dnd`] — drop-position math for lists, trees and dock zones.

pub mod dnd;
pub mod focus;
pub mod keymap;
pub mod selection;
