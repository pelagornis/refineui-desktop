//! Menu Bar — one menu model, two presentations (brief §5 Application):
//!
//! * **macOS** — the system menu bar. [`set_system_menus`] converts the
//!   model to GPUI menus (`cx.set_menus`): actions are built by name from
//!   GPUI's action registry, AppKit shows their key equivalents from the
//!   bound keymap, and standard items get their `OsAction` (Undo, Redo,
//!   Cut, Copy, Paste, Select All) so the system routes them natively.
//! * **Windows / Linux** — [`MenuBar`], an in-window row of top-level
//!   menus (`bodySm`, `roundedMedium` 28px buttons). Click opens a
//!   [`crate::Menu`] panel under the menu; while one is open, hovering
//!   another top-level menu switches to it (native menu-bar behaviour).
//!   Items dispatch their action to the focused element and show the
//!   shortcut from the installed keymap. Keyboard: F10 / Alt focuses the
//!   bar ([`FocusMenuBar`]), ← / → move between menus, ↓ / Enter / Space
//!   open, Escape closes and returns focus.
//!
//! [`crate::shell::ApplicationShell`] picks the presentation from
//! [`crate::shell::ShellSpec::in_window_menu`].

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    OsAction, ParentElement, RenderOnce, SharedString, Styled, Window, anchored, div, point, px,
};
use refineui_tokens::{radius, spacing, typography::role};

use crate::motion::{
    TrackInteraction, child_id, focus_visible, probe, use_interaction, use_layout_probe,
};
use crate::{ActiveTheme, Menu, MenuItem, StyledExt};

#[allow(missing_docs)]
mod actions {
    gpui::actions!(menu_bar, [FocusMenuBar]);
}
pub use actions::FocusMenuBar;

/// One entry of an [`AppMenu`].
#[derive(Clone, Debug, PartialEq)]
pub enum AppMenuItem {
    /// Runs a GPUI action by name.
    Action {
        /// Label.
        label: SharedString,
        /// GPUI action name (`"workspace::Save"`).
        action: SharedString,
    },
    /// Separator line.
    Separator,
    /// Nested menu.
    Submenu(AppMenu),
}

/// A top-level menu ("File", "Edit", …).
#[derive(Clone, Debug, PartialEq)]
pub struct AppMenu {
    /// Title.
    pub name: SharedString,
    /// Entries.
    pub items: Vec<AppMenuItem>,
}

impl AppMenu {
    /// Empty menu titled `name`.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            items: Vec::new(),
        }
    }

    /// Add an action item.
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        action: impl Into<SharedString>,
    ) -> Self {
        self.items.push(AppMenuItem::Action {
            label: label.into(),
            action: action.into(),
        });
        self
    }

    /// Add a separator.
    pub fn separator(mut self) -> Self {
        self.items.push(AppMenuItem::Separator);
        self
    }

    /// Add a submenu.
    pub fn submenu(mut self, menu: AppMenu) -> Self {
        self.items.push(AppMenuItem::Submenu(menu));
        self
    }

    /// Every action name in this menu (recursively).
    pub fn actions(&self) -> Vec<SharedString> {
        let mut v = Vec::new();
        for i in &self.items {
            match i {
                AppMenuItem::Action { action, .. } => v.push(action.clone()),
                AppMenuItem::Submenu(m) => v.extend(m.actions()),
                AppMenuItem::Separator => {}
            }
        }
        v
    }
}

/// The system action for a standard edit item, by label.
pub fn os_action_for(label: &str) -> Option<OsAction> {
    Some(match label {
        "Undo" => OsAction::Undo,
        "Redo" => OsAction::Redo,
        "Cut" => OsAction::Cut,
        "Copy" => OsAction::Copy,
        "Paste" => OsAction::Paste,
        "Select All" => OsAction::SelectAll,
        _ => return None,
    })
}

fn gpui_items(
    items: &[AppMenuItem],
    cx: &App,
    missing: &mut Vec<SharedString>,
) -> Vec<gpui::MenuItem> {
    items
        .iter()
        .filter_map(|i| match i {
            AppMenuItem::Separator => Some(gpui::MenuItem::separator()),
            AppMenuItem::Submenu(m) => Some(gpui::MenuItem::submenu(gpui::Menu {
                name: m.name.clone(),
                items: gpui_items(&m.items, cx, missing),
            })),
            AppMenuItem::Action { label, action } => match cx.build_action(action, None) {
                Ok(a) => Some(gpui::MenuItem::Action {
                    name: label.clone(),
                    action: a,
                    os_action: os_action_for(label),
                }),
                Err(_) => {
                    missing.push(action.clone());
                    None
                }
            },
        })
        .collect()
}

/// GPUI menus for `menus`, plus the action names that are not registered
/// (those items are left out).
pub fn system_menus(menus: &[AppMenu], cx: &App) -> (Vec<gpui::Menu>, Vec<SharedString>) {
    let mut missing = Vec::new();
    let out = menus
        .iter()
        .map(|m| gpui::Menu {
            name: m.name.clone(),
            items: gpui_items(&m.items, cx, &mut missing),
        })
        .collect();
    (out, missing)
}

/// Install `menus` as the system menu bar (macOS; elsewhere GPUI keeps
/// them for `cx.get_menus()`). Returns unregistered action names.
pub fn set_system_menus(menus: &[AppMenu], cx: &mut App) -> Vec<SharedString> {
    let (m, missing) = system_menus(menus, cx);
    cx.set_menus(m);
    missing
}

fn cid(id: &ElementId, s: String) -> ElementId {
    ElementId::NamedChild(Box::new(id.clone()), s.into())
}

fn panel(menu: &AppMenu, id: ElementId) -> Menu {
    let mut p = Menu::new(id.clone()).width(px(240.));
    for (ix, i) in menu.items.iter().enumerate() {
        p = match i {
            AppMenuItem::Action { label, action } => {
                p.item(MenuItem::new(label.clone()).action(action.clone()))
            }
            AppMenuItem::Separator => p.divider(),
            AppMenuItem::Submenu(m) => p.submenu(
                MenuItem::new(m.name.clone()),
                panel(m, cid(&id, format!("sub-{ix}"))),
            ),
        };
    }
    p
}

pub(crate) struct BarState {
    open: Option<usize>,
    cursor: usize,
    restore: Option<FocusHandle>,
}

/// In-window menu bar. See the module docs.
#[derive(IntoElement)]
pub struct MenuBar {
    id: ElementId,
    menus: Vec<AppMenu>,
}

impl MenuBar {
    /// Menu bar showing `menus`.
    pub fn new(id: impl Into<ElementId>, menus: Vec<AppMenu>) -> Self {
        Self {
            id: id.into(),
            menus,
        }
    }

    /// Bind F10 to [`FocusMenuBar`] (Windows / Linux convention).
    pub fn bind_keys(cx: &mut App) {
        cx.bind_keys([gpui::KeyBinding::new("f10", FocusMenuBar, None)]);
    }
}

/// The bar's focus handle and state (shared with the shell's
/// [`FocusMenuBar`] handler).
pub(crate) fn bar_handles(
    id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> (FocusHandle, gpui::Entity<BarState>) {
    let state = window.use_keyed_state(child_id(id, "state"), cx, |_, _| BarState {
        open: None,
        cursor: 0,
        restore: None,
    });
    let focus = window
        .use_keyed_state(child_id(id, "focus"), cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    (focus, state)
}

/// Focus the bar (F10 / Alt): remember where focus was, put the cursor on
/// the first menu.
pub(crate) fn enter_bar(
    focus: &FocusHandle,
    state: &gpui::Entity<BarState>,
    window: &mut Window,
    cx: &mut App,
) {
    let prev = window.focused(cx);
    state.update(cx, |s, cx| {
        if !focus.contains_focused(window, cx) {
            s.restore = prev;
        }
        s.cursor = 0;
        cx.notify();
    });
    crate::root::set_focus_visible(true, window, cx);
    window.focus(focus);
}

impl RenderOnce for MenuBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let (focus, state) = bar_handles(&self.id, window, cx);
        let focused = focus.contains_focused(window, cx);
        let host = use_layout_probe(&self.id, window, cx);
        let (open, cursor) = {
            let s = state.read(cx);
            (s.open, s.cursor)
        };
        let n = self.menus.len();
        let ring = focused && focus_visible(cx);
        let h = theme.density.desktop().tab_height.min(28.);

        let buttons = self
            .menus
            .iter()
            .enumerate()
            .map(|(ix, m)| {
                let id = cid(&self.id, format!("menu-{ix}"));
                let (istate, i) = use_interaction(&id, window, cx);
                let active = open == Some(ix);
                let bg = if active || i.pressed {
                    c.background_primary_active
                } else if i.hovered || (ring && cursor == ix && open.is_none()) {
                    c.background_primary_hover
                } else {
                    gpui::transparent_black()
                };
                let click = state.clone();
                let hover = state.clone();
                let f = focus.clone();
                div()
                    .id(id)
                    .relative()
                    .flex_none()
                    .h(px(h))
                    .px(px(spacing::X_SMALL))
                    .flex()
                    .items_center()
                    .rounded(px(radius::MEDIUM))
                    .bg(bg)
                    .text_style_token(role::BODY_SM)
                    .text_color(c.foreground_primary)
                    .cursor_action()
                    .child(probe(&host, Some(ix)))
                    .track_interaction(&istate)
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.stop_propagation();
                        let prev = window.focused(cx);
                        click.update(cx, |s, cx| {
                            s.open = if s.open == Some(ix) { None } else { Some(ix) };
                            s.cursor = ix;
                            if s.restore.is_none() && !f.contains_focused(window, cx) {
                                s.restore = prev;
                            }
                            cx.notify();
                        });
                        window.focus(&f);
                    })
                    // `track_interaction` owns `on_hover`; a mouse-move over
                    // the title (hit-tested) switches the open menu.
                    .on_mouse_move(move |_, _, cx| {
                        hover.update(cx, |s, cx| {
                            if s.open.is_some() && s.open != Some(ix) {
                                s.open = Some(ix);
                                s.cursor = ix;
                                cx.notify();
                            }
                        });
                    })
                    .child(m.name.clone())
            })
            .collect::<Vec<_>>();

        let open_panel = open.and_then(|ix| {
            let menu = self.menus.get(ix)?;
            let b = host.read(cx).items.get(ix).copied().flatten()?;
            let close = state.clone();
            let close_out = state.clone();
            let dismiss: crate::components::menu::MenuHandler =
                std::rc::Rc::new(move |window, cx| {
                    close.update(cx, |s, cx| {
                        s.open = None;
                        if let Some(r) = s.restore.take() {
                            window.focus(&r);
                        }
                        cx.notify();
                    });
                });
            let sub = crate::components::menu::submenu_state(
                &cid(&self.id, format!("panel-{ix}")),
                window,
                cx,
            );
            Some(
                crate::layer(
                    anchored()
                        .position(point(b.left(), b.bottom() + px(spacing::XXX_SMALL)))
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .id(child_id(&self.id, "panel"))
                                .occlude()
                                .on_mouse_down_out(move |e, _, cx| {
                                    // Clicks on the bar itself are handled by
                                    // the menu buttons.
                                    if sub.read(cx).hovered {
                                        return;
                                    }
                                    let _ = e;
                                    close_out.update(cx, |s, cx| {
                                        s.open = None;
                                        cx.notify();
                                    });
                                })
                                .child(
                                    panel(menu, cid(&self.id, format!("panel-{ix}")))
                                        .on_dismiss(dismiss),
                                ),
                        ),
                )
                .priority(crate::foundation::LayerOrder::Popup.priority()),
            )
        });

        let keys = state.clone();
        let focus_action = focus.clone();
        let focus_state = state.clone();
        div()
            .id(self.id.clone())
            .key_context("MenuBar")
            .track_focus(&focus)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(spacing::XXX_SMALL))
            .relative()
            .child(probe(&host, None))
            .children(buttons)
            .children(open_panel)
            .on_action(move |_: &FocusMenuBar, window, cx| {
                enter_bar(&focus_action, &focus_state, window, cx)
            })
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                if n == 0 {
                    return;
                }
                let k = e.keystroke.key.as_str();
                let mut handled = true;
                keys.update(cx, |s, cx| {
                    match k {
                        "left" => {
                            s.cursor = (s.cursor + n - 1) % n;
                            if s.open.is_some() {
                                s.open = Some(s.cursor);
                            }
                        }
                        "right" => {
                            s.cursor = (s.cursor + 1) % n;
                            if s.open.is_some() {
                                s.open = Some(s.cursor);
                            }
                        }
                        "down" | "enter" | "space" => s.open = Some(s.cursor),
                        "escape" => {
                            if s.open.take().is_none()
                                && let Some(r) = s.restore.take()
                            {
                                window.focus(&r);
                            }
                        }
                        _ => handled = false,
                    }
                    cx.notify();
                });
                if handled {
                    cx.stop_propagation();
                }
            })
    }
}

/// Dispatch [`FocusMenuBar`] (for apps that bind Alt alone themselves).
/// The [`crate::shell::ApplicationShell`] handles it from anywhere in the
/// window.
pub fn focus_menu_bar(window: &mut Window, cx: &mut App) {
    window.dispatch_action(Box::new(FocusMenuBar), cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    gpui::actions!(menu_test, [Save, Quit]);

    fn model() -> Vec<AppMenu> {
        vec![
            AppMenu::new("File")
                .action("Save", "menu_test::Save")
                .separator()
                .submenu(AppMenu::new("Recent").action("Missing", "menu_test::Missing"))
                .action("Quit", "menu_test::Quit"),
            AppMenu::new("Edit").action("Copy", "menu_test::Save"),
        ]
    }

    #[test]
    fn actions_are_collected_recursively() {
        assert_eq!(
            model()[0].actions(),
            vec!["menu_test::Save", "menu_test::Missing", "menu_test::Quit"]
        );
        assert!(matches!(os_action_for("Copy"), Some(OsAction::Copy)));
        assert!(os_action_for("Save").is_none());
    }

    #[gpui::test]
    fn system_menus_skip_unknown_actions(cx: &mut gpui::TestAppContext) {
        let _ = (Save, Quit);
        let (menus, missing) = cx.update(|cx| system_menus(&model(), cx));
        assert_eq!(menus.len(), 2);
        assert_eq!(missing, vec![SharedString::from("menu_test::Missing")]);
        // File: Save, separator, Recent (empty submenu), Quit.
        assert_eq!(menus[0].items.len(), 4);
    }
}
