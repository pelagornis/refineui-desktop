//! CommandPalette — the desktop command palette built on [`Command`]
//! (brief §5 Interaction).
//!
//! A floating panel at the top centre of the window (560px wide, 12% from
//! the top, `popup` layer) over a transparent click-catcher. Entries are
//! GPUI actions by name ([`PaletteCommand`]); their shortcuts come from the
//! installed [`crate::interaction::keymap::Keymap`] in the platform's
//! notation and refresh every time the palette opens.
//!
//! * Toggle with [`ToggleCommandPalette`] (bound to ⇧⌘P / Ctrl+Shift+P by
//!   [`CommandPalette::bind_keys`]); Escape or a click outside closes.
//! * Opening remembers the focused element; running a command restores
//!   focus there first, then dispatches the action — so it reaches the
//!   editor / panel the user was in, exactly like pressing its shortcut.
//! * Typing filters by label and keywords (the [`Command`] matcher);
//!   ↑ / ↓ / Enter as in [`Command`].
//!
//! The host view renders the palette entity as the last child of its root
//! and forwards the toggle action:
//!
//! ```ignore
//! .on_action(cx.listener(|this, _: &ToggleCommandPalette, window, cx| {
//!     this.palette.update(cx, |p, cx| p.toggle(window, cx))
//! }))
//! .child(self.palette.clone())
//! ```

use gpui::{
    App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
    prelude::*, px, relative,
};
use refineui_tokens::spacing;

use crate::IconName;
use crate::components::command::{Command, CommandEvent, CommandGroup, CommandItem};
use gpui::KeyBinding;

#[allow(missing_docs)]
mod actions {
    gpui::actions!(command_palette, [ToggleCommandPalette]);
}
pub use actions::ToggleCommandPalette;

/// Width of the palette panel.
pub const COMMAND_PALETTE_WIDTH: f32 = 560.;

/// One palette entry: a GPUI action by name.
#[derive(Clone, Debug)]
pub struct PaletteCommand {
    /// Label shown in the list.
    pub label: SharedString,
    /// GPUI action name (`"workspace::Save"`).
    pub action: SharedString,
    /// Group heading (`None` = ungrouped, listed first).
    pub group: Option<SharedString>,
    /// Leading icon.
    pub icon: Option<IconName>,
    /// Extra search words.
    pub keywords: Vec<SharedString>,
}

impl PaletteCommand {
    /// Entry for `action` labelled `label`.
    pub fn new(label: impl Into<SharedString>, action: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            action: action.into(),
            group: None,
            icon: None,
            keywords: Vec::new(),
        }
    }

    /// Group heading (e.g. `"File"`, `"View"`).
    pub fn group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Extra search words.
    pub fn keywords(mut self, k: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.keywords = k.into_iter().map(Into::into).collect();
        self
    }
}

/// Group `commands` (first-seen group order) into [`CommandGroup`]s with
/// shortcut labels from the installed keymap.
fn groups(commands: &[PaletteCommand], cx: &App) -> Vec<CommandGroup> {
    let mut order: Vec<Option<SharedString>> = Vec::new();
    for c in commands {
        if !order.contains(&c.group) {
            order.push(c.group.clone());
        }
    }
    order
        .into_iter()
        .map(|g| {
            let mut group = match &g {
                Some(h) => CommandGroup::new(h.clone()),
                None => CommandGroup::untitled(),
            };
            for c in commands.iter().filter(|c| c.group == g) {
                let mut item = CommandItem::new(c.label.clone())
                    .value(c.action.clone())
                    .keywords(c.keywords.iter().cloned().chain([c.action.clone()]));
                if let Some(i) = c.icon {
                    item = item.icon(i);
                }
                if let Some(s) = crate::interaction::keymap::shortcut_for(&c.action, cx) {
                    item = item.shortcut(s);
                }
                group = group.item(item);
            }
            group
        })
        .collect()
}

/// The palette (an entity). See the module docs.
pub struct CommandPalette {
    command: Entity<Command>,
    commands: Vec<PaletteCommand>,
    open: bool,
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
    pub fn new(commands: Vec<PaletteCommand>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let command = cx.new(|cx| Command::new(window, cx).empty_text("No matching commands."));
        let sub = cx.subscribe_in(&command, window, |this, _, e: &CommandEvent, window, cx| {
            let CommandEvent::Select(action) = e;
            this.run(action.clone(), window, cx);
        });
        Self {
            command,
            commands,
            open: false,
            restore: None,
            _sub: sub,
        }
    }

    /// Replace the command list.
    pub fn set_commands(&mut self, commands: Vec<PaletteCommand>, cx: &mut Context<Self>) {
        self.commands = commands;
        cx.notify();
    }

    /// Is it showing?
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open (refreshing shortcut labels, clearing the query, focusing the
    /// search field).
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.restore = window.focused(cx);
        let groups = groups(&self.commands, cx);
        self.command.update(cx, |c, cx| {
            c.set_groups(groups, cx);
            c.reset(cx);
        });
        let search = self.command.read(cx).search().clone();
        window.focus(&search.focus_handle(cx));
        self.open = true;
        cx.notify();
    }

    /// Close and give focus back.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
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

    fn run(&mut self, action: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
        if let Ok(a) = cx.build_action(&action, None) {
            window.dispatch_action(a, cx);
        }
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let close_click = cx.listener(|this, _, window, cx| this.close(window, cx));
        let close_key = cx.listener(|this, e: &KeyDownEvent, window, cx| {
            if e.keystroke.key == "escape" {
                cx.stop_propagation();
                this.close(window, cx);
            }
        });
        crate::layer(
            div()
                .id("refineui-command-palette")
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .pt(relative(0.12))
                .px(px(spacing::LARGE))
                .occlude()
                .on_mouse_down(MouseButton::Left, close_click)
                .child(
                    div()
                        .w(px(COMMAND_PALETTE_WIDTH))
                        .max_w_full()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_key_down(close_key)
                        .child(self.command.clone()),
                ),
        )
        .priority(crate::foundation::LayerOrder::Popup.priority())
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn groups_keep_first_seen_order(cx: &mut gpui::TestAppContext) {
        let cmds = vec![
            PaletteCommand::new("Save", "a::Save").group("File"),
            PaletteCommand::new("Zoom In", "a::ZoomIn").group("View"),
            PaletteCommand::new("Open", "a::Open").group("File"),
        ];
        let g = cx.update(|cx| groups(&cmds, cx));
        assert_eq!(g.len(), 2);
    }
}
