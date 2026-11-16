use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window, div, px};
use refineui::prelude::*;

use super::{Story, StoryGroup, row, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "menu",
            name: "Menu",
            group: StoryGroup::Overlays,
            summary: "Trigger-anchored action list (web `Menu`).",
            shot: (520., 260.),
            render: menu,
        },
        Story {
            id: "context-menu",
            name: "Context Menu",
            group: StoryGroup::Overlays,
            summary: "Right-click menu at the pointer, with submenus.",
            shot: (520., 300.),
            render: context_menu,
        },
        Story {
            id: "popover",
            name: "Popover",
            group: StoryGroup::Overlays,
            summary: "Anchored floating panel with rich content.",
            shot: (520., 300.),
            render: popover,
        },
        Story {
            id: "dialog",
            name: "Dialog",
            group: StoryGroup::Overlays,
            summary: "Modal dialog for focused tasks.",
            shot: (560., 300.),
            render: dialog,
        },
        Story {
            id: "drawer",
            name: "Drawer",
            group: StoryGroup::Overlays,
            summary: "Side panel sliding over content.",
            shot: (460., 420.),
            render: drawer,
        },
    ]
}

/// Web `MenuPreview` / `ContextMenuPreview` items.
fn file_items(menu: Menu) -> Menu {
    menu.label("File")
        .item(MenuItem::new("New").icon(IconName::Add).shortcut("⌘N"))
        .item(MenuItem::new("Open…").icon(IconName::Folder).shortcut("⌘O"))
        .divider()
        .item(MenuItem::new("Copy").icon(IconName::Copy).shortcut("⌘C"))
        .item(
            MenuItem::new("Delete")
                .icon(IconName::Delete)
                .shortcut("⌘⌫"),
        )
}

fn context_items(id: &'static str) -> Menu {
    file_items(Menu::new(id)).divider().submenu(
        MenuItem::new("More"),
        Menu::new((id, 1usize))
            .item(MenuItem::new("Rename"))
            .item(MenuItem::new("Move")),
    )
}

/// Web `MenuPreview`: a secondary "Open" trigger. The panel is also shown
/// inline (as it renders when open) for the screenshot.
fn menu(_w: &mut Window, _cx: &mut App) -> AnyElement {
    row()
        .items_start()
        .gap(px(24.))
        .child(DropdownMenu::new(
            "dd",
            Button::new("dd-btn", "Open").variant(ButtonVariant::Secondary),
            file_items(Menu::new("menu-dd")),
        ))
        .child(file_items(Menu::new("menu-inline")))
        .into_any_element()
}

/// Web `ContextMenuPreview`: right click the secondary "Right click" button;
/// the menu opens at the pointer, "More" opens a submenu on hover.
fn context_menu(_w: &mut Window, _cx: &mut App) -> AnyElement {
    row()
        .items_start()
        .gap(px(24.))
        .child(ContextMenu::new(
            "ctx",
            Button::new("ctx-btn", "Right click").variant(ButtonVariant::Secondary),
            context_items("menu-ctx"),
        ))
        .child(context_items("menu-ctx-inline"))
        .into_any_element()
}

/// Web `PopOverDemo`: secondary "Open" trigger, "Dimensions" form content,
/// centered below the trigger with the beak.
fn popover(w: &mut Window, cx: &mut App) -> AnyElement {
    let shot = state("popover-shot", w, cx, || true);
    let width = w.use_keyed_state("pop-width", cx, |_, cx| {
        TextInput::new(cx)
            .size(ControlSize::Small)
            .default_value("100%")
    });
    let height = w.use_keyed_state("pop-height", cx, |_, cx| {
        TextInput::new(cx)
            .size(ControlSize::Small)
            .default_value("25px")
    });
    let p = Popover::new(
        "pop",
        Button::new("pop-btn", "Open").variant(ButtonVariant::Secondary),
        move |_close, _, _| {
            // Panel `min-w popoverPanelWidth` (325) minus `sizeLarge` + hairline padding.
            v_stack(16.)
                .w(px(292.))
                .child(
                    v_stack(4.)
                        .child(Text::new("Dimensions").role(TextRole::BodyMd))
                        .child(
                            Text::new("Layer size")
                                .role(TextRole::CaptionLg)
                                .color(TextColor::Secondary),
                        ),
                )
                .child(
                    v_stack(10.)
                        .w_full()
                        .child(
                            div().w(px(292.)).child(
                                Field::new()
                                    .size(ControlSize::Small)
                                    .label("Width")
                                    .control(width.clone()),
                            ),
                        )
                        .child(
                            div().w(px(292.)).child(
                                Field::new()
                                    .size(ControlSize::Small)
                                    .label("Height")
                                    .control(height.clone()),
                            ),
                        ),
                )
                .into_any_element()
        },
    );
    div()
        .w(px(472.))
        .flex()
        .justify_center()
        .child(if *shot.read(cx) { p.force_open() } else { p })
        .into_any_element()
}

fn dialog(w: &mut Window, cx: &mut App) -> AnyElement {
    let open = state("dialog-open", w, cx, || false);
    let is_open = *open.read(cx);
    let close = {
        let open = open.clone();
        move |_: &mut Window, cx: &mut App| {
            open.update(cx, |o, cx| {
                *o = false;
                cx.notify();
            })
        }
    };
    let body =
        |id: &'static str, inline: bool, close: Option<Box<dyn Fn(&mut Window, &mut App)>>| {
            let d = Dialog::new(id, "Discard changes?")
                .description("You have unsaved edits. Discarding them cannot be undone.")
                .size(DialogSize::Small)
                .footer(
                    h_stack(6.)
                        .child(Button::new((id, 0usize), "Cancel").variant(ButtonVariant::Outline))
                        .child(Button::new((id, 1usize), "Discard")),
                )
                .inline(inline);
            match close {
                Some(c) => d.on_close(move |w, cx| c(w, cx)),
                None => d,
            }
        };
    v_stack(16.)
        .items_start()
        .child(body("dlg-inline", true, None))
        .child(
            Button::new("dlg-open", "Open dialog")
                .variant(ButtonVariant::Secondary)
                .on_click({
                    let open = open.clone();
                    move |_, _, cx| {
                        open.update(cx, |o, cx| {
                            *o = true;
                            cx.notify();
                        })
                    }
                }),
        )
        .child(body("dlg-modal", false, Some(Box::new(close))).open(is_open))
        .into_any_element()
}

/// Web `DrawerDemo`: a primary "Open" trigger (`DrawerTrigger` is a
/// default Button), `sm` drawer on the right with title, supporting copy,
/// body text and a split footer (Cancel outline / Submit primary).
fn drawer(w: &mut Window, cx: &mut App) -> AnyElement {
    let open = state("drawer-open", w, cx, || false);
    let is_open = *open.read(cx);
    let content = |id: &'static str, inline: bool| {
        Drawer::new(id, "Drawer title")
            .size(DrawerSize::Small)
            .description("Supporting copy")
            .inline(inline)
            .child(Text::new("Drawer content").role(TextRole::BodyMd))
            .footer_actions([
                Button::new((id, 0usize), "Cancel")
                    .variant(ButtonVariant::Outline)
                    .full_width(true),
                Button::new((id, 1usize), "Submit").full_width(true),
            ])
    };
    let close = {
        let open = open.clone();
        move |_: &mut Window, cx: &mut App| {
            open.update(cx, |o, cx| {
                *o = false;
                cx.notify();
            })
        }
    };
    v_stack(16.)
        .items_start()
        .child(Button::new("drawer-open", "Open").on_click({
            let open = open.clone();
            move |_, _, cx| {
                open.update(cx, |o, cx| {
                    *o = true;
                    cx.notify();
                })
            }
        }))
        .child(div().h(px(372.)).child(content("drawer-inline", true)))
        .child(content("drawer-modal", false).on_close(close).open(is_open))
        .into_any_element()
}
