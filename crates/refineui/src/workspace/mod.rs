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
//! Phase 3 builds the GPUI views (Split View, Panel, Panel Group, Dock
//! Area, Tab Group, Floating Panel, Resizable Handle) on this model.

pub mod layout;
