//! Serializable workspace layout model.
//!
//! ```text
//! WorkspaceLayout
//! ├─ center: PaneNode            // editor area: splits of tab groups
//! │   ├─ Split { axis, children, ratios }
//! │   └─ Tabs(TabGroupState { id, tabs, active, pinned })
//! ├─ docks: left / right / bottom DockState { visible, size, panels, active }
//! └─ floating: [FloatingPanelState { panel, bounds }]
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
    /// `[x, y, width, height]`, window-local logical px.
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
                left: DockState::new(refineui_tokens::size::component::SIDEBAR_WIDTH),
                right: DockState::new(320.0),
                bottom: DockState::new(240.0),
            },
            floating: Vec::new(),
            active_group: Some("main".into()),
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
    }
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
        }
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
