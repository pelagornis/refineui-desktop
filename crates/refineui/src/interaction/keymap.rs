//! Keybinding manager: contexts, platform resolution, conflict detection
//! and shortcut-hint lookup (brief §5 Keybinding Manager, §7 "shortcut
//! conflicts").
//!
//! Bindings are data (`keys` in GPUI keystroke syntax, an action name and
//! an optional context). Contexts form a tree — e.g. `Workspace` ⊃
//! `Editor` ⊃ `Completion` — mirroring GPUI `key_context`s: when several
//! bindings match, the one in the **innermost** active context wins.
//!
//! [`KeybindingManager::conflicts`] reports, per platform:
//! * [`ConflictKind::Duplicate`] — same keys, same context, different
//!   actions (one of them can never fire: a real bug);
//! * [`ConflictKind::Shadowed`] — same keys in an ancestor and a descendant
//!   context (the outer one is unreachable while the inner is active —
//!   often intended, listed so it is a conscious choice);
//! * [`ConflictKind::Prefix`] — a single keystroke that is also the first
//!   stroke of a chord in an overlapping context (the chord forces a
//!   pending-key delay on the single binding).
//!
//! [`KeybindingManager::install`] registers the bindings with GPUI
//! (`cx.bind_keys`, actions built by name — `"workspace::Save"` — from
//! GPUI's action registry, contexts parsed as `key_context` predicates) and
//! keeps the manager as the app-wide [`Keymap`] global, the single source of
//! truth for [`crate::ShortcutHint`], [`crate::Tooltip::for_action`],
//! menu-item shortcuts and the [`crate::CommandPalette`].

use std::collections::HashMap;

use crate::platform::{Modifier, Os};

/// The installed keymap (see [`KeybindingManager::install`]).
#[derive(Clone, Debug, Default)]
pub struct Keymap(pub KeybindingManager);

impl gpui::Global for Keymap {}

/// Keys (GPUI syntax as written, e.g. `secondary-s`) of the outermost
/// binding of `action` for the chrome OS in the installed [`Keymap`].
pub fn keys_for_action(action: &str, cx: &gpui::App) -> Option<String> {
    let os = crate::platform::chrome_os(cx);
    let km = cx.try_global::<Keymap>()?;
    km.0.keys_for(action, os).first().map(|b| b.keys.clone())
}

/// Platform label (`⇧⌘P` / `Ctrl+Shift+P`) for `action`'s shortcut in the
/// installed [`Keymap`], if bound.
pub fn shortcut_for(action: &str, cx: &gpui::App) -> Option<String> {
    keys_for_action(action, cx).map(|k| crate::platform::active(cx).shortcut_label(&k))
}

/// One binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// Keystrokes as written (`"secondary-shift-p"`, `"ctrl-k ctrl-s"`).
    pub keys: String,
    /// Action name (`"workspace::ToggleCommandPalette"`).
    pub action: String,
    /// Context name, `None` = global.
    pub context: Option<String>,
    /// Restrict to one OS (`None` = all).
    pub os: Option<Os>,
}

/// A detected conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// What kind.
    pub kind: ConflictKind,
    /// Normalized keys.
    pub keys: String,
    /// The binding that loses / is affected.
    pub a: Binding,
    /// The other binding.
    pub b: Binding,
}

/// Conflict kinds (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConflictKind {
    /// Same keys and context, different actions.
    Duplicate,
    /// Inner context hides an outer binding.
    Shadowed,
    /// Single stroke is the prefix of a chord.
    Prefix,
}

/// Contexts + bindings.
#[derive(Clone, Debug, Default)]
pub struct KeybindingManager {
    parents: HashMap<String, Option<String>>,
    bindings: Vec<Binding>,
}

/// Normalize one keystroke sequence for `os`: lower-case, resolve
/// `secondary`, canonical modifier order `ctrl-alt-shift-cmd-key`.
pub fn normalize(keys: &str, os: Os) -> String {
    let primary = match os {
        Os::MacOs => Modifier::Cmd,
        _ => Modifier::Ctrl,
    };
    keys.split_whitespace()
        .map(|stroke| {
            let lower = stroke.to_lowercase();
            let parts: Vec<&str> = lower.split('-').collect();
            let (mods, key) = parts.split_at(parts.len().saturating_sub(1));
            let mut flags = [false; 4];
            for m in mods {
                match *m {
                    "ctrl" | "control" => flags[0] = true,
                    "alt" | "option" => flags[1] = true,
                    "shift" => flags[2] = true,
                    "cmd" | "super" | "win" | "platform" => flags[3] = true,
                    "secondary" => match primary {
                        Modifier::Cmd => flags[3] = true,
                        Modifier::Ctrl => flags[0] = true,
                    },
                    _ => {}
                }
            }
            let mut out: Vec<&str> = ["ctrl", "alt", "shift", "cmd"]
                .iter()
                .zip(flags)
                .filter_map(|(n, on)| on.then_some(*n))
                .collect();
            out.push(key.first().copied().unwrap_or(""));
            out.join("-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl KeybindingManager {
    /// Empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare `name` as a context nested in `parent` (`None` = root).
    pub fn context(&mut self, name: impl Into<String>, parent: Option<&str>) -> &mut Self {
        self.parents.insert(name.into(), parent.map(str::to_string));
        self
    }

    /// Add a binding for every OS.
    pub fn bind(&mut self, keys: &str, action: &str, context: Option<&str>) -> &mut Self {
        self.bindings.push(Binding {
            keys: keys.into(),
            action: action.into(),
            context: context.map(str::to_string),
            os: None,
        });
        self
    }

    /// Add a binding for one OS only.
    pub fn bind_for(
        &mut self,
        os: Os,
        keys: &str,
        action: &str,
        context: Option<&str>,
    ) -> &mut Self {
        self.bindings.push(Binding {
            keys: keys.into(),
            action: action.into(),
            context: context.map(str::to_string),
            os: Some(os),
        });
        self
    }

    /// All bindings.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Depth of `ctx` (global = 0); unknown contexts count as roots.
    fn depth(&self, ctx: Option<&str>) -> usize {
        let mut d = 0;
        let mut cur = ctx.map(str::to_string);
        while let Some(c) = cur {
            d += 1;
            cur = self.parents.get(&c).cloned().flatten();
            if d > 64 {
                break;
            }
        }
        d
    }

    /// Is `outer` an ancestor of (or equal to) `inner`? Global is the
    /// ancestor of everything.
    fn contains(&self, outer: Option<&str>, inner: Option<&str>) -> bool {
        let Some(outer) = outer else { return true };
        let mut cur = inner.map(str::to_string);
        let mut guard = 0;
        while let Some(c) = cur {
            if c == outer {
                return true;
            }
            cur = self.parents.get(&c).cloned().flatten();
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        false
    }

    fn active_on(&self, os: Os) -> impl Iterator<Item = &Binding> {
        self.bindings
            .iter()
            .filter(move |b| b.os.is_none_or(|o| o == os))
    }

    /// The binding `keys` triggers on `os` when `active` (innermost first,
    /// e.g. `["Completion", "Editor", "Workspace"]`) are the focused
    /// contexts.
    pub fn resolve(&self, keys: &str, os: Os, active: &[&str]) -> Option<&Binding> {
        let k = normalize(keys, os);
        let rank = |b: &Binding| match &b.context {
            None => Some(active.len()),
            Some(c) => active.iter().position(|a| a == c),
        };
        self.active_on(os)
            .filter(|b| normalize(&b.keys, os) == k)
            .filter_map(|b| rank(b).map(|r| (r, b)))
            .min_by_key(|(r, _)| *r)
            .map(|(_, b)| b)
    }

    /// Bindings of `action` on `os` (for shortcut hints), outermost first.
    pub fn keys_for(&self, action: &str, os: Os) -> Vec<&Binding> {
        let mut v: Vec<&Binding> = self.active_on(os).filter(|b| b.action == action).collect();
        v.sort_by_key(|b| self.depth(b.context.as_deref()));
        v
    }

    /// GPUI key bindings for `os`. Actions are built by name from GPUI's
    /// action registry; bindings whose action is unknown, whose keys do not
    /// parse or whose context is not a valid predicate are skipped and
    /// reported as messages.
    pub fn to_gpui(&self, os: Os, cx: &gpui::App) -> (Vec<gpui::KeyBinding>, Vec<String>) {
        let mut ok = Vec::new();
        let mut errors = Vec::new();
        for b in self.active_on(os) {
            let action = match cx.build_action(&b.action, None) {
                Ok(a) => a,
                Err(e) => {
                    errors.push(format!("{}: unknown action `{}` ({e})", b.keys, b.action));
                    continue;
                }
            };
            let predicate = match b.context.as_deref() {
                None => None,
                Some(c) => match gpui::KeyBindingContextPredicate::parse(c) {
                    Ok(p) => Some(std::rc::Rc::new(p)),
                    Err(e) => {
                        errors.push(format!("{}: bad context `{c}` ({e})", b.keys));
                        continue;
                    }
                },
            };
            match gpui::KeyBinding::load(
                &normalize(&b.keys, os),
                action,
                predicate,
                false,
                None,
                &gpui::DummyKeyboardMapper,
            ) {
                Ok(k) => ok.push(k),
                Err(e) => errors.push(format!("{}: {e}", b.keys)),
            }
        }
        (ok, errors)
    }

    /// Register every binding for the host OS with GPUI and keep this
    /// manager as the [`Keymap`] global (shortcut hints read it). Returns
    /// the problems found (unknown actions, bad keys) plus every
    /// [`ConflictKind::Duplicate`] — log or assert on them in debug builds.
    pub fn install(self, cx: &mut gpui::App) -> Vec<String> {
        let os = Os::current();
        let (bindings, mut errors) = self.to_gpui(os, cx);
        errors.extend(
            self.conflicts(os)
                .into_iter()
                .filter(|c| c.kind == ConflictKind::Duplicate)
                .map(|c| format!("{}: `{}` and `{}` collide", c.keys, c.a.action, c.b.action)),
        );
        cx.bind_keys(bindings);
        cx.set_global(Keymap(self));
        errors
    }

    /// Every conflict on `os`.
    pub fn conflicts(&self, os: Os) -> Vec<Conflict> {
        let list: Vec<(&Binding, String)> = self
            .active_on(os)
            .map(|b| (b, normalize(&b.keys, os)))
            .collect();
        let mut out = Vec::new();
        for (i, (a, ka)) in list.iter().enumerate() {
            for (b, kb) in list.iter().skip(i + 1) {
                let (ca, cb) = (a.context.as_deref(), b.context.as_deref());
                if ka == kb {
                    if a.action == b.action {
                        continue;
                    }
                    if ca == cb {
                        out.push(Conflict {
                            kind: ConflictKind::Duplicate,
                            keys: ka.clone(),
                            a: (*b).clone(),
                            b: (*a).clone(),
                        });
                    } else if self.contains(ca, cb) || self.contains(cb, ca) {
                        let (outer, inner) = if self.contains(ca, cb) {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        out.push(Conflict {
                            kind: ConflictKind::Shadowed,
                            keys: ka.clone(),
                            a: (*outer).clone(),
                            b: (*inner).clone(),
                        });
                    }
                } else if self.contains(ca, cb) || self.contains(cb, ca) {
                    let prefix = |single: &str, chord: &str| {
                        !single.contains(' ')
                            && chord.contains(' ')
                            && chord.split(' ').next() == Some(single)
                    };
                    if prefix(ka, kb) || prefix(kb, ka) {
                        let (single, chord) = if prefix(ka, kb) { (a, b) } else { (b, a) };
                        out.push(Conflict {
                            kind: ConflictKind::Prefix,
                            keys: normalize(&single.keys, os),
                            a: (*single).clone(),
                            b: (*chord).clone(),
                        });
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> KeybindingManager {
        let mut m = KeybindingManager::new();
        m.context("Workspace", None)
            .context("Editor", Some("Workspace"))
            .context("Terminal", Some("Workspace"));
        m.bind(
            "secondary-shift-p",
            "command_palette::Toggle",
            Some("Workspace"),
        )
        .bind("secondary-w", "pane::CloseActiveItem", Some("Workspace"))
        .bind("secondary-w", "editor::CloseSplit", Some("Editor"))
        .bind("ctrl-k", "editor::KillLine", Some("Editor"))
        .bind("ctrl-k ctrl-s", "keymap::Open", Some("Workspace"))
        .bind_for(Os::MacOs, "cmd-k", "terminal::Clear", Some("Terminal"));
        m
    }

    #[test]
    fn normalize_resolves_secondary_and_orders_modifiers() {
        assert_eq!(normalize("shift-secondary-P", Os::MacOs), "shift-cmd-p");
        assert_eq!(normalize("secondary-shift-p", Os::Windows), "ctrl-shift-p");
        assert_eq!(normalize("Ctrl-K  ctrl-s", Os::Linux), "ctrl-k ctrl-s");
    }

    #[test]
    fn innermost_context_wins() {
        let m = manager();
        let hit = m
            .resolve("cmd-w", Os::MacOs, &["Editor", "Workspace"])
            .unwrap();
        assert_eq!(hit.action, "editor::CloseSplit");
        let hit = m
            .resolve("ctrl-w", Os::Linux, &["Terminal", "Workspace"])
            .unwrap();
        assert_eq!(hit.action, "pane::CloseActiveItem");
        assert!(
            m.resolve("cmd-k", Os::Windows, &["Terminal", "Workspace"])
                .is_none()
        );
        assert_eq!(m.keys_for("command_palette::Toggle", Os::MacOs).len(), 1);
    }

    #[test]
    fn conflicts_are_classified() {
        let mut m = manager();
        m.bind("secondary-shift-p", "file::Print", Some("Workspace"));
        let c = m.conflicts(Os::Windows);
        let kinds: Vec<ConflictKind> = c.iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&ConflictKind::Duplicate), "{c:#?}");
        assert!(kinds.contains(&ConflictKind::Shadowed));
        assert!(kinds.contains(&ConflictKind::Prefix));
        let dup = c
            .iter()
            .find(|c| c.kind == ConflictKind::Duplicate)
            .unwrap();
        assert_eq!(dup.keys, "ctrl-shift-p");
        let shadow = c.iter().find(|c| c.kind == ConflictKind::Shadowed).unwrap();
        assert_eq!(shadow.a.action, "pane::CloseActiveItem");
        // macOS-only binding does not exist on Windows.
        assert!(
            !c.iter()
                .any(|c| c.a.action == "terminal::Clear" || c.b.action == "terminal::Clear")
        );
    }

    mod gpui_wiring {
        gpui::actions!(keymap_test, [Save, Close]);
    }

    #[gpui::test]
    fn install_binds_known_actions_and_reports_unknown(cx: &mut gpui::TestAppContext) {
        let _ = gpui_wiring::Save;
        let mut km = KeybindingManager::new();
        km.context("Workspace", None)
            .bind("secondary-s", "keymap_test::Save", None)
            .bind("secondary-w", "keymap_test::Close", Some("Workspace"))
            .bind("secondary-q", "nope::Missing", None)
            .bind("secondary-k", "keymap_test::Save", Some("Bad Context ("));
        let errors = cx.update(|cx| km.install(cx));
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("nope::Missing"));
        cx.update(|cx| {
            let expected = crate::platform::current().shortcut_label("secondary-s");
            assert_eq!(
                super::shortcut_for("keymap_test::Save", cx).as_deref(),
                Some(expected.as_str())
            );
            assert_eq!(super::shortcut_for("keymap_test::Unbound", cx), None);
        });
    }
}
