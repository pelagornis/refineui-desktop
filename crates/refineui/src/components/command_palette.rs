//! Desktop host for [`Command`](super::Command) in a [`CommandDialog`](super::CommandDialog).
//!
//! This is not a second command UI. The search field, list, grouping, fuzzy
//! match, indicator and empty state are all [`super::Command`]. The host
//! only adds what the web `CommandDialog` does not: a global shortcut
//! ([`ToggleCommandPalette`], ⇧⌘P / Ctrl+Shift+P), recent commands, nested
//! [`CommandPage`]s, keymap shortcut labels, and dispatching a GPUI action
//! after restoring focus.
//!
//! Build items with [`super::CommandItem`] (`action`, `group`,
//! [`CommandItem::page`](super::CommandItem::page)); render the entity as the
//! last child of the window root:
//!
//! ```ignore
//! .on_action(cx.listener(|this, _: &ToggleCommandPalette, window, cx| {
//!     this.palette.update(cx, |p, cx| p.toggle(window, cx))
//! }))
//! .child(self.palette.clone())
//! ```

use gpui::{
    App, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, Subscription, Window, div, prelude::*, px,
};
use refineui_foundation::spacing;

use super::{Command, CommandDialog, CommandEvent, CommandGroup, CommandItem};
use crate::motion::{Motion, animate_from};
use gpui::KeyBinding;

#[allow(missing_docs)]
mod actions {
    gpui::actions!(command_palette, [ToggleCommandPalette]);
}
pub use actions::ToggleCommandPalette;

/// Width of the palette panel.
pub const COMMAND_PALETTE_WIDTH: f32 = 600.;
/// How many recent commands are remembered.
pub const RECENT_LIMIT: usize = 5;

/// A nested page of commands (desktop; the web Command has no pages).
#[derive(Clone)]
pub struct CommandPage {
    id: SharedString,
    title: SharedString,
    placeholder: SharedString,
    commands: Vec<CommandItem>,
}

impl CommandPage {
    /// Page `id` titled `title`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        commands: Vec<CommandItem>,
    ) -> Self {
        let title = title.into();
        Self {
            id: id.into(),
            placeholder: format!("Search {}…", title.to_lowercase()).into(),
            title,
            commands,
        }
    }

    /// Search placeholder on this page.
    pub fn placeholder(mut self, p: impl Into<SharedString>) -> Self {
        self.placeholder = p.into();
        self
    }
}

/// Deprecated name for [`CommandPage`].
pub type PalettePage = CommandPage;

fn item(c: &CommandItem, cx: &App) -> CommandItem {
    let mut item = c.clone();
    if !item.keywords.iter().any(|k| k == &item.value) {
        item.keywords.push(item.value.clone());
    }
    if item.page.is_none()
        && let Some(s) = crate::interaction::keymap::shortcut_for(&item.value, cx)
    {
        item = item.shortcut(s);
    }
    item
}

/// Groups in first-seen order, with an optional `Recent` group first.
pub(crate) fn groups(
    commands: &[CommandItem],
    recent: &[SharedString],
    cx: &App,
) -> Vec<CommandGroup> {
    let mut out = Vec::new();
    let recent_items: Vec<&CommandItem> = recent
        .iter()
        .filter_map(|v| commands.iter().find(|c| &c.value == v))
        .collect();
    if !recent_items.is_empty() {
        let mut g = CommandGroup::new("Recent");
        for c in recent_items {
            // Distinct value so the same command can also appear in its group.
            g = g.item(item(c, cx).value(format!("recent:{}", c.value)));
        }
        out.push(g);
    }
    let mut order: Vec<Option<SharedString>> = Vec::new();
    for c in commands {
        if !order.contains(&c.group) {
            order.push(c.group.clone());
        }
    }
    out.extend(order.into_iter().map(|g| {
        let mut group = match &g {
            Some(h) => CommandGroup::new(h.clone()),
            None => CommandGroup::untitled(),
        };
        for c in commands.iter().filter(|c| c.group == g) {
            group = group.item(item(c, cx));
        }
        group
    }));
    out
}

/// Pushes `action` to the front of the recent list (deduplicated, capped).
pub(crate) fn remember(recent: &mut Vec<SharedString>, action: SharedString) {
    recent.retain(|r| r != &action);
    recent.insert(0, action);
    recent.truncate(RECENT_LIMIT);
}

/// The palette (an entity). See the module docs.
pub struct CommandPalette {
    command: Entity<Command>,
    commands: Vec<CommandItem>,
    pages: Vec<CommandPage>,
    /// Open page ids, root excluded.
    stack: Vec<SharedString>,
    recent: Vec<SharedString>,
    open: bool,
    /// Bumped on every page change (keys the page transition).
    page_epoch: usize,
    restore: Option<FocusHandle>,
    _sub: Subscription,
}

impl CommandPalette {
    /// Bind ⇧⌘P / Ctrl+Shift+P to [`ToggleCommandPalette`].
    pub fn bind_keys(cx: &mut App) {
        cx.bind_keys([KeyBinding::new(
            "secondary-shift-p",
            ToggleCommandPalette,
            None,
        )]);
    }

    /// New (closed) palette over `commands`.
    pub fn new(commands: Vec<CommandItem>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let command = cx.new(|cx| {
            let mut command = Command::new(window, cx)
                .size(crate::ControlSize::Large)
                .hints(true)
                .empty_text("No matching commands.");
            command.set_material(true, cx);
            command
        });
        let sub = cx.subscribe_in(
            &command,
            window,
            |this, _, e: &CommandEvent, window, cx| match e {
                CommandEvent::Select(v) => this.run(v.clone(), window, cx),
                CommandEvent::Back if !this.stack.is_empty() => this.back(cx),
                _ => {}
            },
        );
        Self {
            command,
            commands,
            pages: Vec::new(),
            stack: Vec::new(),
            recent: Vec::new(),
            open: false,
            page_epoch: 0,
            restore: None,
            _sub: sub,
        }
    }

    /// Adds a nested page (opened by [`CommandItem::page`]).
    pub fn page(mut self, page: CommandPage) -> Self {
        self.pages.push(page);
        self
    }

    /// Replace the root command list.
    pub fn set_commands(&mut self, commands: Vec<CommandItem>, cx: &mut Context<Self>) {
        self.commands = commands;
        cx.notify();
    }

    /// Replace the nested pages.
    pub fn set_pages(&mut self, pages: Vec<CommandPage>, cx: &mut Context<Self>) {
        self.pages = pages;
        cx.notify();
    }

    /// Recently run actions, newest first.
    pub fn recent(&self) -> &[SharedString] {
        &self.recent
    }

    /// Is it showing?
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open page ids (root excluded).
    pub fn page_stack(&self) -> &[SharedString] {
        &self.stack
    }

    fn current_page(&self) -> Option<&CommandPage> {
        let id = self.stack.last()?;
        self.pages.iter().find(|p| &p.id == id)
    }

    /// Rebuild the command list for the current page.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let (cmds, recent, crumbs, placeholder): (Vec<CommandItem>, Vec<SharedString>, _, _) =
            match self.current_page() {
                Some(p) => (
                    p.commands.clone(),
                    Vec::new(),
                    self.stack
                        .iter()
                        .filter_map(|id| self.pages.iter().find(|p| &p.id == id))
                        .map(|p| p.title.clone())
                        .collect(),
                    p.placeholder.clone(),
                ),
                None => (
                    self.commands.clone(),
                    self.recent.clone(),
                    Vec::new(),
                    SharedString::from("Type a command or search…"),
                ),
            };
        let groups = groups(&cmds, &recent, cx);
        self.command.update(cx, |c, cx| {
            c.set_groups(groups, cx);
            c.set_breadcrumb(crumbs, cx);
            c.search()
                .update(cx, |s, cx| s.set_placeholder(placeholder, cx));
            c.reset(cx);
        });
        self.page_epoch += 1;
        cx.notify();
    }

    /// Open on the root page (refreshing shortcut labels, clearing the
    /// query, focusing the search field).
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.restore = window.focused(cx);
        self.stack.clear();
        self.refresh(cx);
        let search = self.command.read(cx).search().clone();
        window.focus(&search.focus_handle(cx));
        self.open = true;
        cx.notify();
    }

    /// Opens nested page `id`.
    pub fn push_page(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = id.into();
        if self.pages.iter().any(|p| p.id == id) {
            self.stack.push(id);
            self.refresh(cx);
        }
    }

    /// Goes back one page.
    pub fn back(&mut self, cx: &mut Context<Self>) {
        self.stack.pop();
        self.refresh(cx);
    }

    /// Escape: back one page, or close on the root.
    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.stack.is_empty() {
            self.close(window, cx);
        } else {
            self.back(cx);
        }
    }

    /// Close and give focus back.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.stack.clear();
        if let Some(f) = self.restore.take() {
            window.focus(&f);
        }
        cx.notify();
    }

    /// Open or close.
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(window, cx);
        } else {
            self.open(window, cx);
        }
    }

    fn run(&mut self, value: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let action: SharedString = value
            .strip_prefix("recent:")
            .map(|s| SharedString::from(s.to_string()))
            .unwrap_or(value);
        if let Some(page) = action.strip_prefix("page:") {
            let page = SharedString::from(page.to_string());
            self.push_page(page, cx);
            return;
        }
        remember(&mut self.recent, action.clone());
        self.close(window, cx);
        if let Ok(a) = cx.build_action(&action, None) {
            window.dispatch_action(a, cx);
        }
    }
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        // Page transition: the command card fades and slides in from the
        // trailing edge each time the page changes.
        let t = animate_from(
            ("refineui-palette-page", self.page_epoch),
            0.0f32,
            1.0,
            Motion::PANEL,
            window,
            cx,
        );
        let body = div()
            .w(px(COMMAND_PALETTE_WIDTH))
            .max_w_full()
            .opacity(0.4 + 0.6 * t)
            .ml(px((1. - t) * spacing::MEDIUM))
            .child(self.command.clone());
        div().child(
            CommandDialog::new("refineui-command-palette", self.command.clone())
                .open(self.open)
                .on_close(move |w, cx| {
                    this.update(cx, |p, cx| p.dismiss(w, cx)).ok();
                })
                .content(body),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::command::fuzzy_match;

    #[gpui::test]
    fn groups_keep_first_seen_order_and_recent_first(cx: &mut gpui::TestAppContext) {
        let cmds = vec![
            CommandItem::new("Save").action("a::Save").group("File"),
            CommandItem::new("Zoom In")
                .action("a::ZoomIn")
                .group("View"),
            CommandItem::new("Open").action("a::Open").group("File"),
        ];
        let g = cx.update(|cx| groups(&cmds, &[], cx));
        assert_eq!(g.len(), 2);
        let g = cx.update(|cx| groups(&cmds, &["a::ZoomIn".into()], cx));
        assert_eq!(g.len(), 3);
    }

    #[test]
    fn recent_is_deduplicated_and_capped() {
        let mut r = Vec::new();
        for a in ["a", "b", "a", "c", "d", "e", "f"] {
            remember(&mut r, a.into());
        }
        assert_eq!(r.len(), RECENT_LIMIT);
        assert_eq!(r[0], SharedString::from("f"));
        assert_eq!(r.iter().filter(|x| x.as_ref() == "a").count(), 1);
    }

    #[test]
    fn palette_ranks_with_fuzzy_match() {
        assert!(fuzzy_match("tgl", "Toggle Sidebar").is_some());
    }
}
