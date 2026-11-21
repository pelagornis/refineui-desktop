//! Multi-window session persistence (`serde` feature).
//!
//! A [`Session`] is every open window's placement ([`WindowState`]) and
//! [`WorkspaceLayout`], keyed by a stable window id, saved as one JSON file.
//! Apps load it at launch, open one window per entry and call
//! [`Session::upsert`] + [`Session::save`] whenever a window moves or its
//! layout changes; [`Session::remove`] when the user closes a window (but
//! not when the app quits, so every window comes back).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::shell::WindowState;
use crate::workspace::layout::WorkspaceLayout;

/// Current [`Session::version`].
pub const SESSION_VERSION: u32 = 1;

/// One window of a session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowSession {
    /// Stable window id (`window-1`, …).
    pub id: String,
    /// Placement, if known.
    #[serde(default)]
    pub window: Option<WindowState>,
    /// Workspace layout.
    pub layout: WorkspaceLayout,
}

/// Every window's placement and layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// Format version ([`SESSION_VERSION`]).
    #[serde(default)]
    pub version: u32,
    /// Windows, in opening order.
    #[serde(default)]
    pub windows: Vec<WindowSession>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: SESSION_VERSION,
            windows: Vec::new(),
        }
    }
}

impl Session {
    /// Parse a session; layouts are normalized and duplicate ids dropped.
    /// Corrupt input gives an empty session.
    pub fn from_json(json: &str) -> Self {
        let mut s: Session = serde_json::from_str(json).unwrap_or_default();
        s.version = SESSION_VERSION;
        let mut seen = std::collections::HashSet::new();
        s.windows
            .retain(|w| !w.id.is_empty() && seen.insert(w.id.clone()));
        for w in &mut s.windows {
            w.layout.normalize();
        }
        s
    }

    /// Serialize (pretty JSON).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Load `path`; a missing or unreadable file gives an empty session.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|s| Self::from_json(&s))
            .unwrap_or_default()
    }

    /// Save to `path` atomically (temp file + rename), creating folders.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(tmp, path)
    }

    /// The window `id`.
    pub fn get(&self, id: &str) -> Option<&WindowSession> {
        self.windows.iter().find(|w| w.id == id)
    }

    /// Insert or replace window `id`. A `None` placement keeps the saved
    /// one.
    pub fn upsert(&mut self, id: &str, window: Option<WindowState>, layout: WorkspaceLayout) {
        match self.windows.iter_mut().find(|w| w.id == id) {
            Some(w) => {
                if window.is_some() {
                    w.window = window;
                }
                w.layout = layout;
            }
            None => self.windows.push(WindowSession {
                id: id.to_string(),
                window,
                layout,
            }),
        }
    }

    /// Forget window `id` (the user closed it).
    pub fn remove(&mut self, id: &str) {
        self.windows.retain(|w| w.id != id);
    }

    /// A window id not used yet.
    pub fn fresh_id(&self) -> String {
        (1..)
            .map(|n| format!("window-{n}"))
            .find(|c| self.get(c).is_none())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::layout::DockSide;

    #[test]
    fn session_round_trips_and_repairs() {
        let mut s = Session::default();
        let mut a = WorkspaceLayout::default();
        a.open("main", "lib.rs", false);
        a.move_panel("files", DockSide::Left, None);
        let place = WindowState {
            x: 10.0,
            y: 20.0,
            width: 900.0,
            height: 600.0,
            maximized: false,
            fullscreen: false,
        };
        let id = s.fresh_id();
        assert_eq!(id, "window-1");
        s.upsert(&id, Some(place), a.clone());
        let id2 = s.fresh_id();
        s.upsert(&id2, None, WorkspaceLayout::default());
        // Upsert without a placement keeps the saved one.
        s.upsert(&id, None, a.clone());
        assert_eq!(s.get(&id).unwrap().window, Some(place));
        let back = Session::from_json(&s.to_json());
        assert_eq!(back, s);
        s.remove(&id2);
        assert_eq!(s.windows.len(), 1);

        // Duplicates and bad layouts are repaired; garbage gives a default.
        let dup = r#"{"windows":[{"id":"w","layout":LAYOUT},{"id":"w","layout":LAYOUT}]}"#
            .replace("LAYOUT", &serde_json::to_string(&a).unwrap());
        assert_eq!(Session::from_json(&dup).windows.len(), 1);
        assert_eq!(Session::from_json("not json"), Session::default());

        let dir = std::env::temp_dir().join(format!("refineui-session-{}", std::process::id()));
        let path = dir.join("session.json");
        back.save(&path).unwrap();
        assert_eq!(Session::load(&path), back);
        let _ = std::fs::remove_dir_all(dir);
    }
}
