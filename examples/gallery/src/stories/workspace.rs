//! Workspace composition stories: Split View, Tab Group, Panel Group, Dock
//! Area, Inspector and the Floating Panel frame (`refineui::workspace`).

use gpui::{AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use refineui::prelude::*;

use super::{Story, StoryGroup, section, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "split-view",
            name: "Split View",
            group: StoryGroup::Workspace,
            summary: "Panes along an axis with draggable, keyboard-operable resize handles.",
            shot: (760., 320.),
            render: split_view,
        },
        Story {
            id: "tab-group",
            name: "Tab Group",
            group: StoryGroup::Workspace,
            summary: "Document tabs (dirty, pinned, preview) over the active document.",
            shot: (760., 260.),
            render: tab_group,
        },
        Story {
            id: "panel-group",
            name: "Panel Group",
            group: StoryGroup::Workspace,
            summary: "A dock's panels as a tab strip or as stacked collapsible panels.",
            shot: (760., 340.),
            render: panel_group,
        },
        Story {
            id: "dock-area",
            name: "Dock Area",
            group: StoryGroup::Workspace,
            summary: "Center content with resizable left, right and bottom docks.",
            shot: (900., 520.),
            render: dock_area,
        },
        Story {
            id: "inspector",
            name: "Inspector",
            group: StoryGroup::Workspace,
            summary: "Property sheet of the selection, in collapsible sections.",
            shot: (720., 340.),
            render: inspector,
        },
        Story {
            id: "floating-panel",
            name: "Floating Panel",
            group: StoryGroup::Workspace,
            summary: "A panel torn off into its own always-on-top window.",
            shot: (420., 300.),
            render: floating_panel,
        },
    ]
}

/// Placeholder pane content.
fn pane(label: impl Into<SharedString>, cx: &App) -> gpui::Div {
    let c = cx.theme().colors;
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(c.background_primary)
        .text_color(c.foreground_tertiary)
        .text_style_token(refineui::tokens::typography::role::BODY_SM)
        .child(label.into())
}

/// Bordered frame for a workspace demo.
fn frame(w: f32, h: f32, cx: &App) -> gpui::Div {
    let c = cx.theme().colors;
    div()
        .w(px(w))
        .h(px(h))
        .flex()
        .overflow_hidden()
        .rounded(px(refineui::tokens::radius::LARGE))
        .border_1()
        .border_color(c.border_default)
        .bg(c.background_primary)
}

fn split_view(w: &mut Window, cx: &mut App) -> AnyElement {
    let outer = state("sv-outer", w, cx, || vec![0.25, 0.5, 0.25]);
    let inner = state("sv-inner", w, cx, || vec![0.65, 0.35]);
    let (o2, i2) = (outer.clone(), inner.clone());
    frame(712., 272., cx)
        .child(
            SplitView::horizontal("sv")
                .ratios(outer.read(cx).clone())
                .min_size(vec![120., 200., 120.])
                .on_resize(move |r, _, cx| {
                    o2.update(cx, |s, cx| {
                        *s = r.to_vec();
                        cx.notify();
                    })
                })
                .child(pane("Explorer", cx))
                .child(
                    SplitView::vertical("sv-center")
                        .ratios(inner.read(cx).clone())
                        .min_size(vec![60., 60.])
                        .on_resize(move |r, _, cx| {
                            i2.update(cx, |s, cx| {
                                *s = r.to_vec();
                                cx.notify();
                            })
                        })
                        .child(pane("Editor", cx))
                        .child(pane("Terminal", cx)),
                )
                .child(pane("Outline", cx)),
        )
        .into_any_element()
}

fn tab_group(w: &mut Window, cx: &mut App) -> AnyElement {
    let tabs = state("tg-tabs", w, cx, || {
        vec![
            ("lib", "lib.rs"),
            ("main", "main.rs"),
            ("cargo", "Cargo.toml"),
            ("readme", "README.md"),
        ]
    });
    let active = state("tg-active", w, cx, || "main".to_string());
    let list = tabs.read(cx).clone();
    let current = active.read(cx).clone();
    let active_ix = list.iter().position(|(id, _)| *id == current);
    let (sel, close) = (active.clone(), tabs.clone());
    let bar = TabBar::new("tg-bar", "main-group")
        .focused(true)
        .tabs(list.iter().map(|(id, title)| {
            let tab = DocumentTab::new(*id, *title).icon(IconName::Document);
            match *id {
                "lib" => tab.pinned(true),
                "main" => tab.dirty(true),
                "cargo" => tab.preview(true),
                _ => tab,
            }
        }))
        .active(active_ix)
        .on_select(move |id, _, cx| {
            let id = id.to_string();
            sel.update(cx, |s, cx| {
                *s = id;
                cx.notify();
            })
        })
        .on_close(move |id, _, cx| {
            let id = id.to_string();
            close.update(cx, |s, cx| {
                s.retain(|(t, _)| *t != id);
                cx.notify();
            })
        });
    let title = list
        .iter()
        .find(|(id, _)| *id == current)
        .map(|(_, t)| *t)
        .unwrap_or("No open editors");
    frame(712., 212., cx)
        .child(TabGroup::new("tg").bar(bar).child(pane(title, cx)))
        .into_any_element()
}

fn panel_group(w: &mut Window, cx: &mut App) -> AnyElement {
    let active = state("pg-active", w, cx, || 0usize);
    let set = active.clone();
    let ids = ["explorer", "search", "scm"];
    let tabs = PanelGroup::new("pg-tabs")
        .mode(PanelGroupMode::Tabs)
        .side(DockSide::Left)
        .entry(
            PanelEntry::new("explorer", "Explorer")
                .icon(IconName::Folder)
                .child(pane("Files", cx)),
        )
        .entry(
            PanelEntry::new("search", "Search")
                .icon(IconName::Search)
                .child(pane("Search results", cx)),
        )
        .entry(
            PanelEntry::new("scm", "Source Control")
                .icon(IconName::Git)
                .child(pane("Changes", cx)),
        )
        .active(Some(*active.read(cx)))
        .on_activate(move |id, _, cx| {
            let ix = ids.iter().position(|x| *x == id).unwrap_or(0);
            set.update(cx, |s, cx| {
                *s = ix;
                cx.notify();
            })
        });
    let stack = PanelGroup::new("pg-stack")
        .mode(PanelGroupMode::Stack)
        .side(DockSide::Left)
        .entry(PanelEntry::new("open", "Open Editors").child(pane("2 files", cx)))
        .entry(PanelEntry::new("outline", "Outline").child(pane("No symbols", cx)))
        .entry(PanelEntry::new("timeline", "Timeline").child(pane("No history", cx)));
    div()
        .flex()
        .gap(px(24.))
        .child(section("Tabs", frame(344., 260., cx).child(tabs), cx))
        .child(section("Stack", frame(344., 260., cx).child(stack), cx))
        .into_any_element()
}

fn dock_area(w: &mut Window, cx: &mut App) -> AnyElement {
    let sizes = state("da-sizes", w, cx, || (200f32, 220f32, 140f32));
    let (l, r, b) = *sizes.read(cx);
    let resize = |side: DockSide| {
        let sizes = sizes.clone();
        move |v: f32, _: &mut Window, cx: &mut App| {
            sizes.update(cx, |s, cx| {
                match side {
                    DockSide::Left => s.0 = v,
                    DockSide::Right => s.1 = v,
                    DockSide::Bottom => s.2 = v,
                }
                cx.notify();
            })
        }
    };
    let group = |id: &'static str, side: DockSide, title: &'static str, body: &'static str| {
        PanelGroup::new(id)
            .side(side)
            .entry(PanelEntry::new(id, title).child(pane(body, cx)))
            .active(Some(0))
    };
    frame(852., 472., cx)
        .child(
            DockArea::new("da")
                .center(pane("Editor", cx))
                .dock(
                    Dock::new("da-left", DockSide::Left)
                        .size(l)
                        .limits(140., 320.)
                        .group(group("da-files", DockSide::Left, "Explorer", "Files"))
                        .on_resize(resize(DockSide::Left)),
                )
                .dock(
                    Dock::new("da-right", DockSide::Right)
                        .size(r)
                        .limits(160., 320.)
                        .group(group(
                            "da-props",
                            DockSide::Right,
                            "Inspector",
                            "Properties",
                        ))
                        .on_resize(resize(DockSide::Right)),
                )
                .dock(
                    Dock::new("da-bottom", DockSide::Bottom)
                        .size(b)
                        .limits(80., 260.)
                        .group(group(
                            "da-term",
                            DockSide::Bottom,
                            "Terminal",
                            "$ cargo run",
                        ))
                        .on_resize(resize(DockSide::Bottom)),
                ),
        )
        .into_any_element()
}

fn inspector(w: &mut Window, cx: &mut App) -> AnyElement {
    let visible = state("in-visible", w, cx, || true);
    let v2 = visible.clone();
    let sheet = Inspector::new("in")
        .section(
            InspectorSection::new("Layout")
                .row("Width", Text::new("320 px"))
                .row("Height", Text::new("Auto"))
                .row("Padding", Text::new("16 px")),
        )
        .section(InspectorSection::new("Appearance").row(
            "Visible",
            Switch::new("in-switch", *visible.read(cx)).on_change(move |on, _, cx| {
                v2.update(cx, |s, cx| {
                    *s = *on;
                    cx.notify();
                })
            }),
        ))
        .section(InspectorSection::new("Advanced").collapsed(true));
    div()
        .flex()
        .gap(px(24.))
        .child(section("Selection", frame(320., 280., cx).child(sheet), cx))
        .child(section(
            "Empty",
            frame(320., 280., cx).child(Inspector::new("in-empty")),
            cx,
        ))
        .into_any_element()
}

fn floating_panel(_w: &mut Window, cx: &mut App) -> AnyElement {
    div()
        .w(px(372.))
        .h(px(252.))
        .child(
            FloatingPanelFrame::new("fp", "Color Picker")
                .blurred(false)
                .child(pane("Panel content", cx)),
        )
        .into_any_element()
}
