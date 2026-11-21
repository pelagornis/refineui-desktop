//! Serializable workspace layout model.
//!
//! ```text
//! WorkspaceLayout
//! ├─ center: PaneNode            // editor area: splits of tab groups
//! │   ├─ Split { axis, children, ratios }
//! │   └─ Tabs(TabGroupState { id, tabs, active, pinned, preview })
//! ├─ docks: left / right / bottom DockState { visible, size, panels, active }
//! ├─ floating: [FloatingPanelState { panel, bounds }]
//! └─ zoomed: Option<ZoomTarget>  // a dock or tab group filling the window
//! ```
//!
//! Sizes are logical px (docks) or fractions summing to 1 (split ratios),
//! so a layout restores at any window size; [`WorkspaceLayout::normalize`]
//! repairs anything a stale file or a removed document left behind.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Current [`WorkspaceLayout::version`].
pub const LAYOUT_VERSION: u32 = 1;
/// Minimum split ratio (keeps every pane grabbable).
pub const MIN_RATIO: f32 = 0.05;

/// Split direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SplitAxis {
    /// Children side by side.
    Horizontal,
    /// Children stacked.
    Vertical,
}

/// A node of the center area.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaneNode {
    /// Children along `axis`, sized by `ratios` (same length, sum 1).
    Split {
        /// Direction.
        axis: SplitAxis,
        /// Children.
        children: Vec<PaneNode>,
        /// Fractions of the split's extent.
        ratios: Vec<f32>,
    },
    /// A tab group (leaf).
    Tabs(TabGroupState),
}

/// A tab group: ordered document ids.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TabGroupState {
    /// Stable group id.
    pub id: String,
    /// Document ids in tab order.
    pub tabs: Vec<String>,
    /// Active tab index.
    pub active: Option<usize>,
    /// The first `pinned` tabs are pinned.
    pub pinned: usize,
    /// The preview tab (opened by a single click, replaced by the next
    /// preview; promoted by editing or double-clicking).
    #[cfg_attr(feature = "serde", serde(default))]
    pub preview: Option<String>,
}

/// One of the three dock areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DockSide {
    /// Leading (primary sidebar).
    Left,
    /// Trailing (secondary sidebar, inspector).
    Right,
    /// Bottom (terminal, output).
    Bottom,
}

impl DockSide {
    /// All sides.
    pub const ALL: [DockSide; 3] = [DockSide::Left, DockSide::Right, DockSide::Bottom];
}

/// What fills the window when zoomed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ZoomTarget {
    /// A dock fills the workspace.
    Dock(DockSide),
    /// A tab group fills the editor area.
    Group(String),
}

/// Where a panel currently lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelLocation {
    /// In a dock at this index.
    Dock(DockSide, usize),
    /// Floating (index into [`WorkspaceLayout::floating`]).
    Floating(usize),
}

/// A dock area on one window edge.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DockState {
    /// Shown.
    pub visible: bool,
    /// Width (left / right) or height (bottom), logical px.
    pub size: f32,
    /// Panel ids in tab order.
    pub panels: Vec<String>,
    /// Active panel index.
    pub active: Option<usize>,
}

impl DockState {
    /// Hidden dock of `size` px.
    pub fn new(size: f32) -> Self {
        Self {
            visible: false,
            size,
            panels: Vec::new(),
            active: None,
        }
    }
}

/// The three dock areas.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Docks {
    /// Leading dock (primary sidebar).
    pub left: DockState,
    /// Trailing dock (inspector / secondary sidebar).
    pub right: DockState,
    /// Bottom dock (terminal, output, problems).
    pub bottom: DockState,
}

/// A panel floating above the workspace.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FloatingPanelState {
    /// Panel id.
    pub panel: String,
    /// `[x, y, width, height]`, screen logical px (floating panels are
    /// their own windows).
    pub bounds: [f32; 4],
}

/// Everything the user arranged in one window.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WorkspaceLayout {
    /// Format version ([`LAYOUT_VERSION`]).
    pub version: u32,
    /// Editor area.
    pub center: PaneNode,
    /// Dock areas.
    pub docks: Docks,
    /// Floating panels, back to front.
    pub floating: Vec<FloatingPanelState>,
    /// Focused tab group id.
    pub active_group: Option<String>,
    /// Zoomed dock or group (`None` = normal layout).
    #[cfg_attr(feature = "serde", serde(default))]
    pub zoomed: Option<ZoomTarget>,
}

impl Default for WorkspaceLayout {
    /// One empty tab group, docks hidden (left 256, right 320, bottom 240).
    fn default() -> Self {
        Self {
            version: LAYOUT_VERSION,
            center: PaneNode::Tabs(TabGroupState {
                id: "main".into(),
                ..TabGroupState::default()
            }),
            docks: Docks {
                left: DockState::new(refineui_foundation::size::component::SIDEBAR_WIDTH),
                right: DockState::new(320.0),
                bottom: DockState::new(240.0),
            },
            floating: Vec::new(),
            active_group: Some("main".into()),
            zoomed: None,
        }
    }
}

impl PaneNode {
    fn groups<'a>(&'a self, out: &mut Vec<&'a TabGroupState>) {
        match self {
            PaneNode::Tabs(g) => out.push(g),
            PaneNode::Split { children, .. } => children.iter().for_each(|c| c.groups(out)),
        }
    }

    fn groups_mut<'a>(&'a mut self, out: &mut Vec<&'a mut TabGroupState>) {
        match self {
            PaneNode::Tabs(g) => out.push(g),
            PaneNode::Split { children, .. } => children.iter_mut().for_each(|c| c.groups_mut(out)),
        }
    }

    /// The node at `path` (child indices from this node).
    pub fn at(&self, path: &[usize]) -> Option<&PaneNode> {
        match (path.split_first(), self) {
            (None, n) => Some(n),
            (Some((i, rest)), PaneNode::Split { children, .. }) => children.get(*i)?.at(rest),
            _ => None,
        }
    }

    /// Mutable [`Self::at`].
    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut PaneNode> {
        match path.split_first() {
            None => Some(self),
            Some((i, rest)) => match self {
                PaneNode::Split { children, .. } => children.get_mut(*i)?.at_mut(rest),
                PaneNode::Tabs(_) => None,
            },
        }
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut PaneNode> {
        match self {
            PaneNode::Tabs(g) if g.id == id => Some(self),
            PaneNode::Tabs(_) => None,
            PaneNode::Split { children, .. } => children.iter_mut().find_map(|c| c.find_mut(id)),
        }
    }

    /// Remove empty groups / single-child splits and repair ratios.
    /// Returns `None` when nothing is left.
    fn normalized(self) -> Option<PaneNode> {
        match self {
            PaneNode::Tabs(mut g) => {
                g.pinned = g.pinned.min(g.tabs.len());
                if g.preview.as_ref().is_some_and(|p| {
                    g.tabs
                        .iter()
                        .position(|t| t == p)
                        .is_none_or(|i| i < g.pinned)
                }) {
                    g.preview = None;
                }
                g.active = match (g.active, g.tabs.len()) {
                    (_, 0) => None,
                    (Some(a), n) => Some(a.min(n - 1)),
                    (None, _) => Some(0),
                };
                Some(PaneNode::Tabs(g))
            }
            PaneNode::Split {
                axis,
                children,
                ratios,
            } => {
                let mut kept: Vec<(PaneNode, f32)> = children
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, c)| {
                        let r = ratios.get(i).copied().unwrap_or(0.0);
                        c.normalized().map(|c| (c, r))
                    })
                    // Flatten same-axis nesting.
                    .flat_map(|(c, r)| match c {
                        PaneNode::Split {
                            axis: a,
                            children,
                            ratios,
                        } if a == axis => children
                            .into_iter()
                            .zip(ratios.into_iter().map(move |x| x * r))
                            .collect::<Vec<_>>(),
                        c => vec![(c, r)],
                    })
                    .collect();
                match kept.len() {
                    0 => None,
                    1 => kept.pop().map(|(c, _)| c),
                    n => {
                        let mut ratios: Vec<f32> = kept
                            .iter()
                            .map(|(_, r)| if r.is_finite() { r.max(0.0) } else { 0.0 })
                            .collect();
                        let sum: f32 = ratios.iter().sum();
                        if sum <= f32::EPSILON {
                            ratios = vec![1.0 / n as f32; n];
                        } else {
                            ratios.iter_mut().for_each(|r| *r /= sum);
                        }
                        // Clamp to the minimum, then renormalize.
                        ratios.iter_mut().for_each(|r| *r = r.max(MIN_RATIO));
                        let sum: f32 = ratios.iter().sum();
                        ratios.iter_mut().for_each(|r| *r /= sum);
                        Some(PaneNode::Split {
                            axis,
                            children: kept.into_iter().map(|(c, _)| c).collect(),
                            ratios,
                        })
                    }
                }
            }
        }
    }
}

impl WorkspaceLayout {
    /// Every tab group, in reading order.
    pub fn tab_groups(&self) -> Vec<&TabGroupState> {
        let mut v = Vec::new();
        self.center.groups(&mut v);
        v
    }

    /// The tab group `id`.
    pub fn group(&self, id: &str) -> Option<&TabGroupState> {
        self.tab_groups().into_iter().find(|g| g.id == id)
    }

    /// Split group `id` along `axis`, moving its active tab into a new group
    /// `new_id` placed after it (`before` = place it first). Returns false
    /// when `id` does not exist.
    pub fn split(&mut self, id: &str, axis: SplitAxis, new_id: &str, before: bool) -> bool {
        let Some(node) = self.center.find_mut(id) else {
            return false;
        };
        let PaneNode::Tabs(g) = node else {
            return false;
        };
        let mut old = std::mem::take(g);
        let mut fresh = TabGroupState {
            id: new_id.into(),
            ..TabGroupState::default()
        };
        if let Some(a) = old.active.filter(|a| *a < old.tabs.len())
            && old.tabs.len() > 1
        {
            fresh.tabs.push(old.tabs.remove(a));
            fresh.active = Some(0);
        }
        let (first, second) = if before {
            (PaneNode::Tabs(fresh), PaneNode::Tabs(old))
        } else {
            (PaneNode::Tabs(old), PaneNode::Tabs(fresh))
        };
        *node = PaneNode::Split {
            axis,
            children: vec![first, second],
            ratios: vec![0.5, 0.5],
        };
        self.active_group = Some(new_id.into());
        self.normalize();
        true
    }

    /// Close group `id` (its documents are dropped from the layout).
    pub fn close_group(&mut self, id: &str) {
        fn remove(node: PaneNode, id: &str) -> Option<PaneNode> {
            match node {
                PaneNode::Tabs(g) if g.id == id => None,
                PaneNode::Split {
                    axis,
                    children,
                    ratios,
                } => {
                    let mut kept = Vec::new();
                    let mut kept_r = Vec::new();
                    for (c, r) in children.into_iter().zip(ratios) {
                        if let Some(c) = remove(c, id) {
                            kept.push(c);
                            kept_r.push(r);
                        }
                    }
                    (!kept.is_empty()).then_some(PaneNode::Split {
                        axis,
                        children: kept,
                        ratios: kept_r,
                    })
                }
                other => Some(other),
            }
        }
        let center = std::mem::replace(&mut self.center, PaneNode::Tabs(TabGroupState::default()));
        self.center = remove(center, id).unwrap_or_else(|| WorkspaceLayout::default().center);
        if self.active_group.as_deref() == Some(id) {
            self.active_group = None;
        }
        self.normalize();
    }

    /// Repair the layout: version, ratios (≥ [`MIN_RATIO`], sum 1), empty
    /// splits, same-axis nesting, active indices, dock sizes, a valid
    /// active group.
    pub fn normalize(&mut self) {
        self.version = LAYOUT_VERSION;
        let center = std::mem::replace(&mut self.center, PaneNode::Tabs(TabGroupState::default()));
        self.center = center
            .normalized()
            .unwrap_or_else(|| WorkspaceLayout::default().center);
        for d in [
            &mut self.docks.left,
            &mut self.docks.right,
            &mut self.docks.bottom,
        ] {
            if !d.size.is_finite() || d.size < 0.0 {
                d.size = 0.0;
            }
            d.active = match (d.active, d.panels.len()) {
                (_, 0) => None,
                (Some(a), n) => Some(a.min(n - 1)),
                (None, _) => Some(0),
            };
        }
        let ids: Vec<String> = self.tab_groups().iter().map(|g| g.id.clone()).collect();
        if !self.active_group.as_ref().is_some_and(|a| ids.contains(a)) {
            self.active_group = ids.first().cloned();
        }
        let zoom_ok = match &self.zoomed {
            None => true,
            Some(ZoomTarget::Dock(side)) => {
                let d = self.dock(*side);
                d.visible && !d.panels.is_empty()
            }
            Some(ZoomTarget::Group(id)) => ids.contains(id),
        };
        if !zoom_ok {
            self.zoomed = None;
        }
    }

    // ---- tab groups ------------------------------------------------------

    /// The tab group `id`, mutably.
    pub fn group_mut(&mut self, id: &str) -> Option<&mut TabGroupState> {
        let mut v = Vec::new();
        self.center.groups_mut(&mut v);
        v.into_iter().find(|g| g.id == id)
    }

    /// The group holding `doc`, if any.
    pub fn group_of(&self, doc: &str) -> Option<&TabGroupState> {
        self.tab_groups()
            .into_iter()
            .find(|g| g.tabs.iter().any(|t| t == doc))
    }

    /// A group id not used yet (`prefix`, `prefix-2`, …).
    pub fn fresh_group_id(&self, prefix: &str) -> String {
        let ids: Vec<String> = self.tab_groups().iter().map(|g| g.id.clone()).collect();
        if !ids.iter().any(|i| i == prefix) {
            return prefix.to_string();
        }
        (2..)
            .map(|n| format!("{prefix}-{n}"))
            .find(|c| !ids.contains(c))
            .unwrap_or_default()
    }

    /// Open `doc` in `group` and activate it. Already open: activate it
    /// (and promote it when `preview` is false). A `preview` open replaces
    /// the group's current preview tab; otherwise the tab is inserted after
    /// the active one (never among pinned tabs). Returns false when the
    /// group does not exist.
    pub fn open(&mut self, group: &str, doc: &str, preview: bool) -> bool {
        let Some(g) = self.group_mut(group) else {
            return false;
        };
        if let Some(i) = g.tabs.iter().position(|t| t == doc) {
            g.active = Some(i);
            if !preview && g.preview.as_deref() == Some(doc) {
                g.preview = None;
            }
        } else {
            let replace = if preview {
                g.preview
                    .take()
                    .and_then(|p| g.tabs.iter().position(|t| *t == p))
                    .filter(|i| *i >= g.pinned)
            } else {
                None
            };
            let ix = match replace {
                Some(i) => {
                    g.tabs[i] = doc.to_string();
                    i
                }
                None => {
                    let at = g
                        .active
                        .map(|a| a + 1)
                        .unwrap_or(g.tabs.len())
                        .max(g.pinned)
                        .min(g.tabs.len());
                    g.tabs.insert(at, doc.to_string());
                    at
                }
            };
            g.active = Some(ix);
            if preview {
                g.preview = Some(doc.to_string());
            }
        }
        self.active_group = Some(group.to_string());
        true
    }

    /// Turn the preview tab `doc` into a normal tab.
    pub fn promote(&mut self, group: &str, doc: &str) {
        if let Some(g) = self.group_mut(group)
            && g.preview.as_deref() == Some(doc)
        {
            g.preview = None;
        }
    }

    /// Remove `doc` from `group` without normalizing.
    fn take_tab(&mut self, group: &str, doc: &str) -> Option<(usize, bool, bool)> {
        let g = self.group_mut(group)?;
        let i = g.tabs.iter().position(|t| t == doc)?;
        g.tabs.remove(i);
        let pinned = i < g.pinned;
        if pinned {
            g.pinned -= 1;
        }
        let preview = g.preview.as_deref() == Some(doc);
        if preview {
            g.preview = None;
        }
        g.active = match g.active {
            _ if g.tabs.is_empty() => None,
            // Closing the active tab activates its right neighbour (or the
            // new last tab).
            Some(a) if a == i => Some(i.min(g.tabs.len() - 1)),
            Some(a) if a > i => Some(a - 1),
            a => a,
        };
        Some((i, pinned, preview))
    }

    /// Drop `group` when it is empty and not the only group.
    fn drop_if_empty(&mut self, group: &str) {
        if self.group(group).is_some_and(|g| g.tabs.is_empty()) && self.tab_groups().len() > 1 {
            self.close_group(group);
        }
    }

    /// Close one tab. An emptied group disappears unless it is the last.
    pub fn close_tab(&mut self, group: &str, doc: &str) -> bool {
        let ok = self.take_tab(group, doc).is_some();
        self.drop_if_empty(group);
        self.normalize();
        ok
    }

    /// Close every unpinned tab of `group` except `doc`.
    pub fn close_others(&mut self, group: &str, doc: &str) {
        if let Some(g) = self.group_mut(group) {
            let pinned = g.pinned;
            let keep: Vec<String> = g
                .tabs
                .iter()
                .enumerate()
                .filter(|(i, t)| *i < pinned || *t == doc)
                .map(|(_, t)| t.clone())
                .collect();
            g.tabs = keep;
            g.active = g.tabs.iter().position(|t| t == doc);
        }
        self.normalize();
    }

    /// Close the unpinned tabs right of `doc`.
    pub fn close_to_right(&mut self, group: &str, doc: &str) {
        if let Some(g) = self.group_mut(group)
            && let Some(i) = g.tabs.iter().position(|t| t == doc)
        {
            let pinned = g.pinned;
            let keep: Vec<String> = g
                .tabs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j <= i || *j < pinned)
                .map(|(_, t)| t.clone())
                .collect();
            g.tabs = keep;
            if g.active.is_some_and(|a| a > i) {
                g.active = Some(i);
            }
        }
        self.normalize();
    }

    /// Close every unpinned tab of `group`.
    pub fn close_all(&mut self, group: &str) {
        if let Some(g) = self.group_mut(group) {
            g.tabs.truncate(g.pinned);
            g.active = g.active.filter(|a| *a < g.tabs.len());
        }
        self.drop_if_empty(group);
        self.normalize();
    }

    /// Pin (moves to the end of the pinned run) or unpin (moves to the
    /// first unpinned slot) `doc`. A pinned tab is never a preview.
    pub fn set_pinned(&mut self, group: &str, doc: &str, pin: bool) {
        let Some(g) = self.group_mut(group) else {
            return;
        };
        let Some(i) = g.tabs.iter().position(|t| t == doc) else {
            return;
        };
        let was = i < g.pinned;
        if was == pin {
            return;
        }
        let active_doc = g.active.and_then(|a| g.tabs.get(a).cloned());
        let t = g.tabs.remove(i);
        if pin {
            g.tabs.insert(g.pinned, t);
            g.pinned += 1;
            if g.preview.as_deref() == Some(doc) {
                g.preview = None;
            }
        } else {
            g.pinned -= 1;
            g.tabs.insert(g.pinned, t);
        }
        g.active = active_doc.and_then(|d| g.tabs.iter().position(|t| *t == d));
    }

    /// Move `doc` from group `from` to group `to` at insertion `index` (as
    /// shown before the move, e.g. from [`crate::interaction::dnd::insertion_index`]).
    /// A pinned tab stays pinned only if it lands in the pinned run; an
    /// unpinned tab never lands inside it. An emptied source group closes.
    pub fn move_tab(&mut self, from: &str, doc: &str, to: &str, index: usize) -> bool {
        if self.group(to).is_none() {
            return false;
        }
        let same = from == to;
        let Some((old, was_pinned, was_preview)) = self.take_tab(from, doc) else {
            return false;
        };
        let Some(g) = self.group_mut(to) else {
            return false;
        };
        let mut at = if same && old < index {
            index - 1
        } else {
            index
        }
        .min(g.tabs.len());
        let pinned = was_pinned && at <= g.pinned;
        if pinned {
            g.pinned += 1;
        } else {
            at = at.max(g.pinned);
        }
        g.tabs.insert(at, doc.to_string());
        g.active = Some(at);
        if was_preview && !pinned && g.preview.is_none() {
            g.preview = Some(doc.to_string());
        }
        self.active_group = Some(to.to_string());
        if !same {
            self.drop_if_empty(from);
        }
        self.normalize();
        true
    }

    /// Drop `doc` (from group `from`) on group `target` at `zone`:
    /// `Center` moves it into the group; an edge splits `target` and puts
    /// `doc` in a new group `new_id` on that side. Splitting a group with
    /// itself as its only tab is refused (false).
    pub fn split_with(
        &mut self,
        target: &str,
        doc: &str,
        from: &str,
        zone: crate::interaction::dnd::DockZone,
        new_id: &str,
    ) -> bool {
        use crate::interaction::dnd::DockZone;
        let (axis, before) = match zone {
            DockZone::Center => {
                let len = self.group(target).map(|g| g.tabs.len()).unwrap_or(0);
                return self.move_tab(from, doc, target, len);
            }
            DockZone::Left => (SplitAxis::Horizontal, true),
            DockZone::Right => (SplitAxis::Horizontal, false),
            DockZone::Top => (SplitAxis::Vertical, true),
            DockZone::Bottom => (SplitAxis::Vertical, false),
        };
        if from == target && self.group(from).is_some_and(|g| g.tabs.len() <= 1) {
            return false;
        }
        if self.group(target).is_none() || self.take_tab(from, doc).is_none() {
            return false;
        }
        let fresh = PaneNode::Tabs(TabGroupState {
            id: new_id.into(),
            tabs: vec![doc.into()],
            active: Some(0),
            ..TabGroupState::default()
        });
        if let Some(node) = self.center.find_mut(target) {
            let old = std::mem::replace(node, PaneNode::Tabs(TabGroupState::default()));
            let children = if before {
                vec![fresh, old]
            } else {
                vec![old, fresh]
            };
            *node = PaneNode::Split {
                axis,
                children,
                ratios: vec![0.5, 0.5],
            };
        }
        self.active_group = Some(new_id.into());
        if from != target {
            self.drop_if_empty(from);
        }
        self.normalize();
        true
    }

    /// Set the ratios of the split at `path` (renormalized, clamped to
    /// [`MIN_RATIO`]).
    pub fn set_ratios(&mut self, path: &[usize], ratios: &[f32]) -> bool {
        match self.center.at_mut(path) {
            Some(PaneNode::Split { ratios: r, .. }) if r.len() == ratios.len() => {
                r.copy_from_slice(ratios);
                self.normalize();
                true
            }
            _ => false,
        }
    }

    // ---- docks -----------------------------------------------------------

    /// The dock on `side`.
    pub fn dock(&self, side: DockSide) -> &DockState {
        match side {
            DockSide::Left => &self.docks.left,
            DockSide::Right => &self.docks.right,
            DockSide::Bottom => &self.docks.bottom,
        }
    }

    /// The dock on `side`, mutably.
    pub fn dock_mut(&mut self, side: DockSide) -> &mut DockState {
        match side {
            DockSide::Left => &mut self.docks.left,
            DockSide::Right => &mut self.docks.right,
            DockSide::Bottom => &mut self.docks.bottom,
        }
    }

    /// Show / hide a dock (a dock without panels stays hidden).
    pub fn toggle_dock(&mut self, side: DockSide) {
        let d = self.dock_mut(side);
        d.visible = !d.visible && !d.panels.is_empty();
        self.normalize();
    }

    /// Resize a dock, clamped to `min..=max` px.
    pub fn set_dock_size(&mut self, side: DockSide, size: f32, min: f32, max: f32) {
        if size.is_finite() {
            self.dock_mut(side).size = size.clamp(min, max.max(min));
        }
    }

    /// Where `panel` is.
    pub fn panel_location(&self, panel: &str) -> Option<PanelLocation> {
        for side in DockSide::ALL {
            if let Some(i) = self.dock(side).panels.iter().position(|p| p == panel) {
                return Some(PanelLocation::Dock(side, i));
            }
        }
        self.floating
            .iter()
            .position(|f| f.panel == panel)
            .map(PanelLocation::Floating)
    }

    /// Remove `panel` from wherever it is.
    fn take_panel(&mut self, panel: &str) -> Option<PanelLocation> {
        let loc = self.panel_location(panel)?;
        match loc {
            PanelLocation::Dock(side, i) => {
                let d = self.dock_mut(side);
                d.panels.remove(i);
                d.active = match d.active {
                    _ if d.panels.is_empty() => None,
                    Some(a) if a > i || (a == i && a == d.panels.len()) => Some(a - 1),
                    a => a,
                };
                if d.panels.is_empty() {
                    d.visible = false;
                }
            }
            PanelLocation::Floating(i) => {
                self.floating.remove(i);
            }
        }
        Some(loc)
    }

    /// Show `panel` in the dock on `to` at `index` (end when `None`),
    /// taking it from its current dock or from floating. Activates it and
    /// shows the dock.
    pub fn move_panel(&mut self, panel: &str, to: DockSide, index: Option<usize>) {
        let from = self.take_panel(panel);
        let d = self.dock_mut(to);
        let mut at = index.unwrap_or(d.panels.len());
        if let Some(PanelLocation::Dock(side, i)) = from
            && side == to
            && i < at
        {
            at -= 1;
        }
        let at = at.min(d.panels.len());
        d.panels.insert(at, panel.to_string());
        d.active = Some(at);
        d.visible = true;
        self.normalize();
    }

    /// Activate `panel` in its dock (showing the dock).
    pub fn activate_panel(&mut self, panel: &str) -> bool {
        match self.panel_location(panel) {
            Some(PanelLocation::Dock(side, i)) => {
                let d = self.dock_mut(side);
                d.active = Some(i);
                d.visible = true;
                true
            }
            _ => false,
        }
    }

    /// Detach `panel` into a floating window at `bounds` (screen px).
    pub fn float_panel(&mut self, panel: &str, bounds: [f32; 4]) {
        self.take_panel(panel);
        self.floating.push(FloatingPanelState {
            panel: panel.to_string(),
            bounds,
        });
        self.normalize();
    }

    /// Record a floating panel's new bounds.
    pub fn set_floating_bounds(&mut self, panel: &str, bounds: [f32; 4]) {
        if let Some(f) = self.floating.iter_mut().find(|f| f.panel == panel) {
            f.bounds = bounds;
        }
    }

    /// Zoom `target`, or restore when it is already zoomed.
    pub fn toggle_zoom(&mut self, target: ZoomTarget) {
        self.zoomed = if self.zoomed.as_ref() == Some(&target) {
            None
        } else {
            Some(target)
        };
        self.normalize();
    }
}

/// New ratios after dragging the handle between children `ix` and `ix + 1`
/// to `boundary` (a fraction of the split's extent). Only that pair
/// changes; each keeps at least its `mins` fraction (missing = 0). If the
/// pair cannot honour both minimums nothing changes.
pub fn resize_at(ratios: &[f32], ix: usize, boundary: f32, mins: &[f32]) -> Vec<f32> {
    let mut out = ratios.to_vec();
    if ix + 1 >= ratios.len() || !boundary.is_finite() {
        return out;
    }
    let before: f32 = ratios[..ix].iter().sum();
    let pair = ratios[ix] + ratios[ix + 1];
    let (lo, hi) = (
        mins.get(ix).copied().unwrap_or(0.0),
        mins.get(ix + 1).copied().unwrap_or(0.0),
    );
    if lo + hi > pair {
        return out;
    }
    let left = (boundary - before).clamp(lo, pair - hi);
    out[ix] = left;
    out[ix + 1] = pair - left;
    out
}

/// Equal ratios for `n` children (double-click on a handle).
pub fn equal_ratios(n: usize) -> Vec<f32> {
    vec![1.0 / n.max(1) as f32; n]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs(id: &str, tabs: &[&str]) -> TabGroupState {
        TabGroupState {
            id: id.into(),
            tabs: tabs.iter().map(|s| s.to_string()).collect(),
            active: Some(tabs.len().saturating_sub(1)),
            pinned: 0,
            preview: None,
        }
    }

    fn tabs(l: &WorkspaceLayout, id: &str) -> Vec<String> {
        l.group(id).map(|g| g.tabs.clone()).unwrap_or_default()
    }

    #[test]
    fn open_preview_pin_and_close() {
        let mut l = WorkspaceLayout::default();
        assert!(l.open("main", "a", false));
        assert!(l.open("main", "b", true));
        // A second preview replaces the first.
        assert!(l.open("main", "c", true));
        assert_eq!(tabs(&l, "main"), ["a", "c"]);
        assert_eq!(l.group("main").unwrap().preview.as_deref(), Some("c"));
        // Opening it for real promotes it; the next preview is a new tab.
        l.open("main", "c", false);
        assert_eq!(l.group("main").unwrap().preview, None);
        l.open("main", "d", true);
        assert_eq!(tabs(&l, "main"), ["a", "c", "d"]);
        // Pin d: it moves to the pinned run and stops being a preview.
        l.set_pinned("main", "d", true);
        let g = l.group("main").unwrap();
        assert_eq!(
            (g.tabs.clone(), g.pinned, g.preview.clone()),
            (vec!["d".into(), "a".into(), "c".into()], 1, None)
        );
        assert_eq!(g.active, Some(0));
        // New tabs never go among pinned ones.
        l.open("main", "e", false);
        assert_eq!(tabs(&l, "main"), ["d", "e", "a", "c"]);
        l.close_others("main", "a");
        assert_eq!(tabs(&l, "main"), ["d", "a"]);
        l.close_all("main");
        assert_eq!(tabs(&l, "main"), ["d"]);
        l.set_pinned("main", "d", false);
        assert!(l.close_tab("main", "d"));
        // The last group survives empty.
        assert_eq!(l.tab_groups().len(), 1);
        assert_eq!(l.group("main").unwrap().active, None);
    }

    #[test]
    fn closing_the_active_tab_activates_the_right_neighbour() {
        let mut l = WorkspaceLayout {
            center: PaneNode::Tabs(docs("main", &["a", "b", "c"])),
            ..WorkspaceLayout::default()
        };
        l.group_mut("main").unwrap().active = Some(1);
        l.close_tab("main", "b");
        let g = l.group("main").unwrap();
        assert_eq!(g.tabs[g.active.unwrap()], "c");
        l.close_to_right("main", "a");
        assert_eq!(tabs(&l, "main"), ["a"]);
    }

    #[test]
    fn move_tabs_within_and_between_groups() {
        let mut l = WorkspaceLayout {
            center: PaneNode::Split {
                axis: SplitAxis::Horizontal,
                children: vec![
                    PaneNode::Tabs(docs("one", &["a", "b", "c"])),
                    PaneNode::Tabs(docs("two", &["x"])),
                ],
                ratios: vec![0.5, 0.5],
            },
            ..WorkspaceLayout::default()
        };
        // Drag a to after c (insertion index 3 as displayed).
        assert!(l.move_tab("one", "a", "one", 3));
        assert_eq!(tabs(&l, "one"), ["b", "c", "a"]);
        assert!(l.move_tab("one", "c", "two", 0));
        assert_eq!(tabs(&l, "two"), ["c", "x"]);
        assert_eq!(l.active_group.as_deref(), Some("two"));
        // Moving x out empties nothing; moving the rest of `one` closes it.
        l.move_tab("one", "b", "two", 9);
        l.move_tab("one", "a", "two", 9);
        assert!(l.group("one").is_none());
        assert!(matches!(&l.center, PaneNode::Tabs(g) if g.id == "two"));
    }

    #[test]
    fn pinned_run_is_respected_when_moving() {
        let mut l = WorkspaceLayout {
            center: PaneNode::Tabs(docs("main", &["p", "a", "b"])),
            ..WorkspaceLayout::default()
        };
        l.set_pinned("main", "p", true);
        // An unpinned tab dropped before the pinned one lands after it.
        l.move_tab("main", "b", "main", 0);
        assert_eq!(tabs(&l, "main"), ["p", "b", "a"]);
        // A pinned tab dragged past the run is unpinned.
        l.move_tab("main", "p", "main", 3);
        let g = l.group("main").unwrap();
        assert_eq!(
            (g.tabs.clone(), g.pinned),
            (vec!["b".into(), "a".into(), "p".into()], 0)
        );
    }

    #[test]
    fn drag_to_an_edge_splits() {
        use crate::interaction::dnd::DockZone;
        let mut l = WorkspaceLayout {
            center: PaneNode::Tabs(docs("main", &["a", "b"])),
            ..WorkspaceLayout::default()
        };
        assert!(l.split_with("main", "b", "main", DockZone::Bottom, "g2"));
        let PaneNode::Split { axis, children, .. } = &l.center else {
            panic!()
        };
        assert_eq!(*axis, SplitAxis::Vertical);
        assert!(matches!(&children[1], PaneNode::Tabs(g) if g.id == "g2" && g.tabs == ["b"]));
        // A group's only tab cannot split off its own group.
        assert!(!l.split_with("g2", "b", "g2", DockZone::Left, "g3"));
        // Left of `main` from g2: g2 empties and closes, main is split.
        assert!(l.split_with("main", "b", "g2", DockZone::Left, "g3"));
        assert!(l.group("g2").is_none());
        let PaneNode::Split { axis, children, .. } = &l.center else {
            panic!()
        };
        assert_eq!(*axis, SplitAxis::Horizontal);
        assert!(matches!(&children[0], PaneNode::Tabs(g) if g.id == "g3"));
        assert_eq!(l.fresh_group_id("g3"), "g3-2");
        // Center = move into the group.
        assert!(l.split_with("main", "b", "g3", DockZone::Center, "unused"));
        assert_eq!(tabs(&l, "main"), ["a", "b"]);
    }

    #[test]
    fn resizing_ratios() {
        let r = resize_at(&[0.25, 0.25, 0.5], 1, 0.65, &[0.1, 0.1, 0.1]);
        assert!((r[1] - 0.4).abs() < 1e-6 && (r[2] - 0.35).abs() < 1e-6 && r[0] == 0.25);
        // Clamped by the right neighbour's minimum.
        let r = resize_at(&[0.5, 0.5], 0, 0.99, &[0.1, 0.2]);
        assert!((r[0] - 0.8).abs() < 1e-6);
        assert_eq!(resize_at(&[0.5, 0.5], 1, 0.3, &[]), vec![0.5, 0.5]);
        assert_eq!(equal_ratios(4), vec![0.25; 4]);
        let mut l = WorkspaceLayout::default();
        l.split("main", SplitAxis::Horizontal, "b", false);
        assert!(l.set_ratios(&[], &[0.7, 0.3]));
        let PaneNode::Split { ratios, .. } = &l.center else {
            panic!()
        };
        assert!((ratios[0] - 0.7).abs() < 1e-6);
        assert!(!l.set_ratios(&[0], &[1.0]));
    }

    #[test]
    fn panels_move_between_docks_float_and_zoom() {
        let mut l = WorkspaceLayout::default();
        l.move_panel("files", DockSide::Left, None);
        l.move_panel("outline", DockSide::Left, None);
        l.move_panel("terminal", DockSide::Bottom, None);
        assert!(l.docks.left.visible && l.docks.bottom.visible);
        assert_eq!(l.docks.left.active, Some(1));
        // Reorder within a dock.
        l.move_panel("outline", DockSide::Left, Some(0));
        assert_eq!(l.docks.left.panels, ["outline", "files"]);
        // Across docks.
        l.move_panel("terminal", DockSide::Right, Some(0));
        assert!(!l.docks.bottom.visible);
        assert_eq!(
            l.panel_location("terminal"),
            Some(PanelLocation::Dock(DockSide::Right, 0))
        );
        l.float_panel("outline", [10.0, 20.0, 300.0, 400.0]);
        assert_eq!(
            l.panel_location("outline"),
            Some(PanelLocation::Floating(0))
        );
        l.set_floating_bounds("outline", [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(l.floating[0].bounds, [1.0, 2.0, 3.0, 4.0]);
        l.move_panel("outline", DockSide::Left, None);
        assert!(l.floating.is_empty());
        l.toggle_zoom(ZoomTarget::Dock(DockSide::Left));
        assert_eq!(l.zoomed, Some(ZoomTarget::Dock(DockSide::Left)));
        l.toggle_zoom(ZoomTarget::Dock(DockSide::Left));
        assert_eq!(l.zoomed, None);
        // Zooming a hidden dock is dropped by normalize.
        l.toggle_zoom(ZoomTarget::Dock(DockSide::Bottom));
        assert_eq!(l.zoomed, None);
        l.toggle_dock(DockSide::Left);
        assert!(!l.docks.left.visible);
        l.set_dock_size(DockSide::Left, 9999.0, 160.0, 600.0);
        assert_eq!(l.docks.left.size, 600.0);
        assert!(l.activate_panel("files"));
        assert!(l.docks.left.visible);
    }

    #[test]
    fn split_and_close_round_trip() {
        let mut l = WorkspaceLayout {
            center: PaneNode::Tabs(docs("main", &["a.rs", "b.rs"])),
            ..WorkspaceLayout::default()
        };
        assert!(l.split("main", SplitAxis::Horizontal, "right", false));
        let g: Vec<_> = l
            .tab_groups()
            .iter()
            .map(|g| (g.id.clone(), g.tabs.clone()))
            .collect();
        assert_eq!(
            g,
            vec![
                ("main".to_string(), vec!["a.rs".to_string()]),
                ("right".to_string(), vec!["b.rs".to_string()])
            ]
        );
        assert_eq!(l.active_group.as_deref(), Some("right"));
        // Same-axis split is flattened into one 3-way split.
        assert!(l.split("right", SplitAxis::Horizontal, "far", false));
        let PaneNode::Split {
            children, ratios, ..
        } = &l.center
        else {
            panic!("expected a split")
        };
        assert_eq!(children.len(), 3);
        assert!((ratios.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        l.close_group("right");
        l.close_group("far");
        assert!(matches!(&l.center, PaneNode::Tabs(g) if g.id == "main"));
        assert_eq!(l.active_group.as_deref(), Some("main"));
    }

    #[test]
    fn normalize_repairs_bad_input() {
        let mut l = WorkspaceLayout {
            version: 0,
            center: PaneNode::Split {
                axis: SplitAxis::Vertical,
                children: vec![
                    PaneNode::Tabs(TabGroupState {
                        id: "x".into(),
                        tabs: vec!["one".into()],
                        active: Some(9),
                        pinned: 5,
                        preview: None,
                    }),
                    PaneNode::Tabs(docs("y", &[])),
                ],
                ratios: vec![f32::NAN, 0.0],
            },
            active_group: Some("gone".into()),
            ..WorkspaceLayout::default()
        };
        l.docks.left.size = -3.0;
        l.normalize();
        assert_eq!(l.version, LAYOUT_VERSION);
        let PaneNode::Split { ratios, .. } = &l.center else {
            panic!()
        };
        assert!(ratios.iter().all(|r| *r >= MIN_RATIO - 1e-6));
        let x = l.group("x").unwrap();
        assert_eq!((x.active, x.pinned), (Some(0), 1));
        assert_eq!(l.active_group.as_deref(), Some("x"));
        assert_eq!(l.docks.left.size, 0.0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn layout_serializes() {
        let mut l = WorkspaceLayout::default();
        l.split("main", SplitAxis::Vertical, "below", false);
        l.floating.push(FloatingPanelState {
            panel: "colors".into(),
            bounds: [10.0, 20.0, 300.0, 200.0],
        });
        let json = serde_json::to_string(&l).unwrap();
        let back: WorkspaceLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back, l);
    }
}
