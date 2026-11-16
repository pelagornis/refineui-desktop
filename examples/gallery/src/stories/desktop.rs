//! Desktop shell chrome and interaction stories (Phase 2): titlebar, window
//! controls, menu bar, status bar, activity bar, application shell,
//! shortcut hints, drag and drop, selection box and the command palette.

use gpui::{
    AnyElement, App, AppContext, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::*, px,
};
use refineui::interaction::dnd::{DockZone, DropEdge, dock_zone, drop_edge, insertion_index};
use refineui::prelude::*;
use refineui::shell::ControlButtons;

use super::{Story, StoryGroup, row, section, setter, state};
use crate::demo;

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "application-shell",
            name: "Application Shell",
            group: StoryGroup::Desktop,
            summary: "Titlebar, menu bar, unified toolbar, activity bar, docks, workspace and status bar composed with F6 region cycling.",
            shot: (1040., 640.),
            render: application_shell,
        },
        Story {
            id: "titlebar",
            name: "Titlebar",
            group: StoryGroup::Desktop,
            summary: "Per-OS window titlebar: macOS traffic-light inset, Windows caption buttons, Linux client-side decorations.",
            shot: (900., 560.),
            render: titlebar,
        },
        Story {
            id: "window-controls",
            name: "Window Controls",
            group: StoryGroup::Desktop,
            summary: "Minimize / maximize / close drawn per OS and wired to GPUI's window-control hit areas.",
            shot: (720., 300.),
            render: window_controls,
        },
        Story {
            id: "menu-bar",
            name: "Menu Bar",
            group: StoryGroup::Desktop,
            summary: "In-window menu bar (Windows / Linux) with keyboard navigation and keymap shortcuts; the system menu bar on macOS.",
            shot: (720., 420.),
            render: menu_bar,
        },
        Story {
            id: "status-bar",
            name: "Status Bar",
            group: StoryGroup::Desktop,
            summary: "Window-bottom strip of text, buttons and progress with leading / trailing groups.",
            shot: (860., 200.),
            render: status_bar,
        },
        Story {
            id: "activity-bar",
            name: "Activity Bar",
            group: StoryGroup::Desktop,
            summary: "Vertical navigation rail: shape, size, indicator, colours, labels, header / footer slots, custom faces, drag-to-reorder.",
            shot: (980., 560.),
            render: activity_bar,
        },
        Story {
            id: "shortcut-hint",
            name: "Shortcut Hint",
            group: StoryGroup::Desktop,
            summary: "Platform-formatted key combinations, from literal keys or the installed keymap.",
            shot: (720., 360.),
            render: shortcut_hint,
        },
        Story {
            id: "drag-and-drop",
            name: "Drag and Drop",
            group: StoryGroup::Desktop,
            summary: "Drag handle, drag preview, insertion indicator and docking drop zones.",
            shot: (860., 420.),
            render: drag_and_drop,
        },
        Story {
            id: "selection-box",
            name: "Selection Box",
            group: StoryGroup::Desktop,
            summary: "Rubber-band marquee plus click, ⌘/Ctrl-click, Shift-click and keyboard selection.",
            shot: (760., 380.),
            render: selection_box,
        },
        Story {
            id: "command-palette",
            name: "Command Palette",
            group: StoryGroup::Desktop,
            summary: "⇧⌘P / Ctrl+Shift+P action search with groups, icons and keymap shortcuts.",
            shot: (760., 520.),
            render: command_palette,
        },
    ]
}

/// A framed box standing in for a window.
fn window_frame(w: f32, h: f32, child: impl IntoElement, cx: &App) -> gpui::Div {
    let c = cx.theme().colors;
    div()
        .w(px(w))
        .h(px(h))
        .flex()
        .flex_col()
        .rounded(px(10.))
        .overflow_hidden()
        .border(px(0.5))
        .border_color(c.border_strong)
        .bg(c.background_primary)
        .shadow(cx.theme().shadow(Elevation::Shadow4))
        .child(child)
}

fn os_name(os: Os) -> &'static str {
    match os {
        Os::MacOs => "macOS",
        Os::Windows => "Windows",
        Os::Linux => "Linux",
    }
}

fn application_shell(w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let os = state("shell-os", w, cx, Os::current);
    let current = *os.read(cx);
    let selected = state("shell-activity", w, cx, || SharedString::from("explorer"));
    let c = cx.theme().colors;
    let oses = [Os::MacOs, Os::Windows, Os::Linux];
    let toolbar = Toolbar::new("story-shell-tb")
        .fill(true)
        .size(ControlSize::Small)
        .button(
            Button::icon_only("s-sidebar", IconName::PanelLeft)
                .variant(ButtonVariant::Ghost)
                .tooltip("Toggle Sidebar")
                .tooltip_action("demo::ToggleSidebar"),
        )
        .spacer()
        .child(
            div()
                .w(px(300.))
                .h(px(24.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(6.))
                .bg(c.background_surface)
                .border(px(0.5))
                .border_color(c.border_subtle)
                .child(
                    Icon::new(IconName::Search)
                        .size(IconSize::XSmall)
                        .color(c.foreground_tertiary),
                )
                .child(
                    div().flex_1().child(
                        Text::new("Search commands")
                            .role(TextRole::BodySm)
                            .color(TextColor::Tertiary),
                    ),
                )
                .child(ShortcutHint::keys("secondary-shift-p").os(current)),
        )
        .spacer()
        .button(
            Button::icon_only("s-run", IconName::Play)
                .variant(ButtonVariant::Ghost)
                .tooltip("Run")
                .tooltip_action("demo::Run"),
        );
    let activity = ActivityBar::new("story-shell-activity")
        .items([
            ActivityItem::icon("explorer", "Explorer", IconName::Document),
            ActivityItem::icon("search", "Search", IconName::Search),
            ActivityItem::icon("git", "Source Control", IconName::Git).badge(3),
            ActivityItem::icon("extensions", "Extensions", IconName::Cube),
        ])
        .footer_item(ActivityItem::icon(
            "settings",
            "Settings",
            IconName::Settings,
        ))
        .selected(selected.read(cx).clone())
        .on_select(setter(&selected));
    let sidebar = v_stack(2.)
        .p(px(8.))
        .child(
            Text::new("EXPLORER")
                .role(TextRole::CaptionMd)
                .color(TextColor::Secondary),
        )
        .child(
            Tree::new(
                "story-shell-tree",
                vec![
                    TreeNode::branch(
                        "src",
                        "src",
                        vec![
                            TreeNode::leaf("lib", "lib.rs").icon(IconName::Code),
                            TreeNode::leaf("shell", "shell.rs").icon(IconName::Code),
                        ],
                    ),
                    TreeNode::leaf("cargo", "Cargo.toml").icon(IconName::Document),
                ],
            )
            .size(ControlSize::Small)
            .default_expanded(["src"]),
        );
    let editor = v_stack(0.)
        .size_full()
        .child(
            h_stack(0.)
                .h(px(32.))
                .bg(c.surface_sunken)
                .border_b(px(0.5))
                .border_color(c.border_subtle)
                .child(
                    h_stack(6.)
                        .h_full()
                        .px(px(12.))
                        .bg(c.background_primary)
                        .border_t(px(2.))
                        .border_color(c.border_focus)
                        .child(
                            Icon::new(IconName::Code)
                                .size(IconSize::XSmall)
                                .color(c.foreground_brand),
                        )
                        .child(Text::new("shell.rs").role(TextRole::BodySm)),
                )
                .child(
                    h_stack(6.).h_full().px(px(12.)).child(
                        Text::new("lib.rs")
                            .role(TextRole::BodySm)
                            .color(TextColor::Secondary),
                    ),
                ),
        )
        .child(
            div()
                .flex_1()
                .p(px(12.))
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(12.))
                .text_color(c.foreground_secondary)
                .child("ApplicationShell::new(\"app\")")
                .child("    .menus(menus)")
                .child("    .toolbar(toolbar)")
                .child("    .activity_bar(activity)")
                .child("    .left_dock(ShellDock::new(sidebar, 220.))")
                .child("    .center(editor)")
                .child("    .status_bar(status)"),
        );
    let status = StatusBar::new("story-shell-status")
        .leading(StatusItem::text("branch", "main").icon(IconName::Git))
        .leading(
            StatusItem::text("problems", "0 errors")
                .icon(IconName::Checkmark)
                .tone(StatusTone::Success),
        )
        .trailing(StatusItem::text("pos", "Ln 4, Col 12"))
        .trailing(StatusItem::text("lang", "Rust"));
    let shell = ApplicationShell::new("story-shell")
        .os(current)
        .embedded(true)
        .title("refineui-gpui")
        .subtitle("shell.rs")
        .menus(demo::menus())
        .toolbar(toolbar)
        .activity_bar(activity)
        .left_dock(ShellDock::new(sidebar, 220.))
        .center(editor)
        .status_bar(status);
    v_stack(12.)
        .child(
            row()
                .child(
                    SegmentedControl::new(
                        "shell-os-pick",
                        oses.map(|o| Segment::from(os_name(o))),
                        oses.iter().position(|o| *o == current).unwrap_or(0),
                    )
                    .size(ControlSize::Small)
                    .on_change({
                        let os = os.clone();
                        move |ix, _, cx| {
                            os.update(cx, |o, cx| {
                                *o = oses[(*ix).min(2)];
                                cx.notify();
                            })
                        }
                    }),
                )
                .child(
                    Button::new("open-shell-demo", "Open Shell demo window")
                        .icon(IconName::Open)
                        .variant(ButtonVariant::Outline)
                        .size(ControlSize::Small)
                        .on_click(|_, _, cx| crate::shell_demo::open(cx)),
                ),
        )
        .child(window_frame(980., 560., shell, cx))
        .into_any_element()
}

fn titlebar(_w: &mut Window, cx: &mut App) -> AnyElement {
    let mut col = v_stack(20.);
    for os in [Os::MacOs, Os::Windows, Os::Linux] {
        let bar = Titlebar::new(SharedString::from(format!("tb-{}", os_name(os))))
            .os(os)
            .interactive(false)
            .title("refineui-gpui")
            .subtitle("main.rs");
        let unified = Titlebar::new(SharedString::from(format!("tbu-{}", os_name(os))))
            .os(os)
            .interactive(false)
            .unified(true)
            .content(
                Toolbar::new(SharedString::from(format!("tbu-tools-{}", os_name(os))))
                    .fill(true)
                    .size(ControlSize::Small)
                    .button(
                        Button::icon_only("u-side", IconName::PanelLeft)
                            .variant(ButtonVariant::Ghost),
                    )
                    .spacer()
                    .child(Text::new("Unified toolbar").role(TextRole::LabelMd))
                    .spacer()
                    .button(
                        Button::icon_only("u-share", IconName::Share).variant(ButtonVariant::Ghost),
                    ),
            );
        col = col.child(section(
            os_name(os),
            v_stack(8.)
                .child(window_frame(840., 44., bar, cx).h_auto())
                .child(window_frame(840., 52., unified, cx).h_auto()),
            cx,
        ));
    }
    col.into_any_element()
}

fn window_controls(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    let mut col = v_stack(20.);
    for os in [Os::MacOs, Os::Windows, Os::Linux] {
        let frame = |id: &str, b: ControlButtons| {
            div()
                .h(px(40.))
                .px(px(8.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .bg(c.surface_sunken)
                .border(px(0.5))
                .border_color(c.border_subtle)
                .child(
                    WindowControls::new(SharedString::from(format!("{id}-{}", os_name(os))))
                        .os(os)
                        .interactive(false)
                        .buttons(b)
                        .height(40.),
                )
        };
        col = col.child(section(
            os_name(os),
            row()
                .child(frame("all", ControlButtons::default()))
                .child(frame("settings", WindowPreset::Settings.into())),
            cx,
        ));
    }
    col.into_any_element()
}

fn menu_bar(_w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let c = cx.theme().colors;
    let menus: Vec<AppMenu> = demo::menus()
        .into_iter()
        .filter(|m| m.name.as_ref() != "RefineUI Shell")
        .collect();
    v_stack(16.)
        .child(section(
            "In-window menu bar · click or press F10 / Alt, then arrows",
            div()
                .w(px(660.))
                .h(px(32.))
                .px(px(4.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .bg(c.surface_sunken)
                .border(px(0.5))
                .border_color(c.border_subtle)
                .child(MenuBar::new("story-menubar", menus)),
            cx,
        ))
        .child(section(
            "On macOS the same AppMenu model becomes the system menu bar (set_system_menus)",
            Text::new("File ▸ Save  ⌘S   ·   Edit ▸ Copy  ⌘C   ·   View ▸ Toggle Sidebar  ⌘B")
                .role(TextRole::BodySm)
                .color(TextColor::Secondary),
            cx,
        ))
        .into_any_element()
}

fn status_bar(w: &mut Window, cx: &mut App) -> AnyElement {
    let progress = state("status-progress", w, cx, || 0.62f32);
    let p = *progress.read(cx);
    let bar = |id: &'static str, accent: bool| {
        StatusBar::new(id)
            .accent(accent)
            .leading(
                StatusItem::button("branch", "main", |_, _| {})
                    .icon(IconName::Git)
                    .tooltip("Checkout Branch…", None),
            )
            .leading(
                StatusItem::text("errors", "2 warnings")
                    .icon(IconName::Warning)
                    .tone(StatusTone::Warning),
            )
            .leading(StatusItem::progress("index", "Indexing", p))
            .trailing(StatusItem::text("pos", "Ln 128, Col 9"))
            .trailing(StatusItem::text("enc", "UTF-8"))
            .trailing(
                StatusItem::button("bell", "", |_, _| {})
                    .icon(IconName::Alert)
                    .tooltip("Notifications", None),
            )
    };
    v_stack(16.)
        .w(px(820.))
        .child(section("Default", bar("status-default", false), cx))
        .child(section(
            "Accent (debugging / remote)",
            bar("status-accent", true),
            cx,
        ))
        .into_any_element()
}

fn activity_bar(w: &mut Window, cx: &mut App) -> AnyElement {
    let sel = state("ab-sel", w, cx, || SharedString::from("explorer"));
    let order = state("ab-order", w, cx, || {
        vec!["explorer", "search", "git", "debug", "extensions"]
    });
    let rail_sel = state("ab-rail-sel", w, cx, || SharedString::from("refineui"));
    let c = cx.theme().colors;
    let item = |id: &str| match id {
        "explorer" => ActivityItem::icon("explorer", "Explorer", IconName::Document),
        "search" => ActivityItem::icon("search", "Search", IconName::Search),
        "git" => ActivityItem::icon("git", "Source Control", IconName::Git).badge(4),
        "debug" => ActivityItem::icon("debug", "Run and Debug", IconName::Bug),
        _ => ActivityItem::icon("extensions", "Extensions", IconName::Cube).dot(),
    };
    let frame = |title: &str, bar: AnyElement| {
        v_stack(8.)
            .items_start()
            .child(
                Text::new(title.to_string())
                    .role(TextRole::CaptionMd)
                    .color(TextColor::Tertiary),
            )
            .child(
                div()
                    .h(px(440.))
                    .flex()
                    .rounded(px(10.))
                    .overflow_hidden()
                    .border(px(0.5))
                    .border_color(c.border_subtle)
                    .child(bar),
            )
    };
    let ids: Vec<&'static str> = order.read(cx).clone();
    let refine = ActivityBar::new("ab-refine")
        .items(ids.iter().map(|i| item(i)))
        .footer_item(ActivityItem::initials("me", "Pelagornis Official").tint(AvatarColor::Blue))
        .footer_item(ActivityItem::icon(
            "settings",
            "Settings",
            IconName::Settings,
        ))
        .selected(sel.read(cx).clone())
        .on_select(setter(&sel))
        .on_reorder({
            let order = order.clone();
            move |from, to, _, cx| {
                order.update(cx, |o, cx| {
                    if from < o.len() {
                        let it = o.remove(from);
                        let to = to.min(o.len());
                        o.insert(to, it);
                        cx.notify();
                    }
                })
            }
        });
    let nav = ActivityBar::navigation_rail("ab-nav")
        .items(ids.iter().take(4).map(|i| item(i)))
        .selected(sel.read(cx).clone())
        .on_select(setter(&sel));
    let expanded = ActivityBar::new("ab-expanded")
        .expanded(true)
        .header(Text::new("Workspace").role(TextRole::SubtitleMd))
        .items(ids.iter().map(|i| item(i)))
        .footer_item(ActivityItem::icon(
            "settings",
            "Settings",
            IconName::Settings,
        ))
        .selected(sel.read(cx).clone())
        .on_select(setter(&sel));
    let dots = ActivityBar::new("ab-dots")
        .shape(ItemShape::Circle)
        .indicator(IndicatorStyle::Dot)
        .restyle(|s| {
            s.background = Some(c.surface_sunken);
            s.selected_ink = Some(c.foreground_brand);
        })
        .items(ids.iter().take(4).map(|i| item(i)))
        .selected(sel.read(cx).clone())
        .on_select(setter(&sel));
    let rail = ActivityBar::rail("ab-rail")
        .item(ActivityItem::icon("home", "Direct Messages", IconName::Mail).badge(2))
        .divider()
        .item(ActivityItem::image("refineui", "RefineUI", super::display::portrait()).unread(true))
        .item(
            ActivityItem::initials("rust", "Rust Korea")
                .tint(AvatarColor::Orange)
                .badge(12),
        )
        .item(ActivityItem::folder(
            "games",
            "Game Night",
            [
                ActivityItem::initials("lol", "League Club").tint(AvatarColor::Blue),
                ActivityItem::initials("mc", "Minecraft Hub").tint(AvatarColor::Green),
            ],
        ))
        .item(ActivityItem::initials("design", "Design Systems").tint(AvatarColor::Purple))
        .footer_item(ActivityItem::icon("add", "Add", IconName::Add))
        .selected(rail_sel.read(cx).clone())
        .on_select(setter(&rail_sel));
    let custom = ActivityBar::new("ab-custom")
        .indicator(IndicatorStyle::None)
        .render_item(|item, st, _, cx| {
            let c = cx.theme().colors;
            div()
                .size(px(36.))
                .rounded(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .border(px(1.))
                .border_color(if st.selected {
                    c.border_focus
                } else {
                    c.border_subtle
                })
                .bg(if st.selected {
                    c.background_brand_subtle
                } else {
                    c.background_primary
                })
                .child(
                    Text::new(item.label().chars().next().unwrap_or('?').to_string())
                        .role(TextRole::LabelMd)
                        .color(if st.selected {
                            TextColor::Brand
                        } else {
                            TextColor::Secondary
                        }),
                )
                .into_any_element()
        })
        .items(ids.iter().take(4).map(|i| item(i)))
        .selected(sel.read(cx).clone())
        .on_select(setter(&sel));
    h_stack(20.)
        .items_start()
        .child(frame("Refine (default)", refine.into_any_element()))
        .child(frame("Navigation rail", nav.into_any_element()))
        .child(frame("Expanded", expanded.into_any_element()))
        .child(frame("Circle · Dot", dots.into_any_element()))
        .child(frame("Rail preset", rail.into_any_element()))
        .child(frame("Custom render", custom.into_any_element()))
        .into_any_element()
}

fn shortcut_hint(_w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let examples = [
        ("Save", "secondary-s"),
        ("Command Palette", "secondary-shift-p"),
        ("Find", "secondary-f"),
        ("Close Tab", "secondary-w"),
        ("Next Tab", "ctrl-tab"),
        ("Focus Next Region", "f6"),
    ];
    let mut grid = v_stack(10.);
    for (label, keys) in examples {
        let mut r = h_stack(16.).child(
            div()
                .w(px(160.))
                .child(Text::new(label).role(TextRole::BodySm)),
        );
        for os in [Os::MacOs, Os::Windows, Os::Linux] {
            r = r.child(div().w(px(150.)).child(ShortcutHint::keys(keys).os(os)));
        }
        grid = grid.child(r);
    }
    let mut head = h_stack(16.).child(div().w(px(160.)));
    for os in [Os::MacOs, Os::Windows, Os::Linux] {
        head = head.child(
            div().w(px(150.)).child(
                Text::new(os_name(os))
                    .role(TextRole::CaptionMd)
                    .color(TextColor::Tertiary),
            ),
        );
    }
    v_stack(20.)
        .child(section("Keycaps", v_stack(10.).child(head).child(grid), cx))
        .child(section(
            "Text style · from the keymap · in a tooltip",
            row()
                .child(ShortcutHint::action("demo::Save").style(ShortcutStyle::Text))
                .child(ShortcutHint::action("demo::ToggleSidebar"))
                .child(
                    Button::new("hint-tooltip", "Hover me")
                        .variant(ButtonVariant::Outline)
                        .size(ControlSize::Small)
                        .tooltip("Save")
                        .tooltip_action("demo::Save"),
                ),
            cx,
        ))
        .into_any_element()
}

#[derive(Clone)]
struct RowDrag {
    from: usize,
    label: SharedString,
}

impl Render for RowDragView {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        self.0.element(cx)
    }
}

struct RowDragView(DragPreview);

fn drag_and_drop(w: &mut Window, cx: &mut App) -> AnyElement {
    let items = state("dnd-items", w, cx, || {
        vec![
            "Design tokens",
            "Window chrome",
            "Keymap",
            "Command palette",
            "Docs & screenshots",
        ]
    });
    let target = state("dnd-target", w, cx, || None::<(usize, DropEdge)>);
    let zone = state("dnd-zone", w, cx, || None::<DockZone>);
    let c = cx.theme().colors;
    let list_items = items.read(cx).clone();
    let tgt = *target.read(cx);
    let mut list = v_stack(0.).w(px(340.));
    for (ix, label) in list_items.iter().enumerate() {
        let label: SharedString = (*label).into();
        if tgt == Some((ix, DropEdge::Before)) {
            list = list.child(DropIndicator::between_rows());
        }
        let t_move = target.clone();
        let t_drop = target.clone();
        let items_drop = items.clone();
        list = list.child(
            div()
                .id(("dnd-row", ix))
                .h(px(36.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(px(6.))
                .hover(|d| d.bg(c.background_surface_hover))
                .child(DragHandle::new(("dnd-handle", ix)))
                .child(
                    Icon::new(IconName::Document)
                        .size(IconSize::XSmall)
                        .color(c.foreground_tertiary),
                )
                .child(Text::new(label.clone()).role(TextRole::BodySm))
                .on_drag(
                    RowDrag {
                        from: ix,
                        label: label.clone(),
                    },
                    |d, _, _, cx| {
                        cx.new(|_| {
                            RowDragView(DragPreview::new(d.label.clone()).icon(IconName::Document))
                        })
                    },
                )
                .on_drag_move::<RowDrag>(move |e, _, cx| {
                    if !e.bounds.contains(&e.event.position) {
                        return;
                    }
                    let off = f32::from(e.event.position.y - e.bounds.top());
                    let edge = drop_edge(off, f32::from(e.bounds.size.height), false);
                    if *t_move.read(cx) != Some((ix, edge)) {
                        t_move.update(cx, |t, cx| {
                            *t = Some((ix, edge));
                            cx.notify();
                        });
                    }
                })
                .on_drop(move |d: &RowDrag, _, cx| {
                    let edge = t_drop.read(cx).map(|(_, e)| e).unwrap_or(DropEdge::Before);
                    t_drop.update(cx, |t, cx| {
                        *t = None;
                        cx.notify();
                    });
                    items_drop.update(cx, |v, cx| {
                        let to = insertion_index(ix, edge, v.len());
                        refineui::interaction::dnd::reorder(v, d.from, to);
                        cx.notify();
                    });
                }),
        );
        if tgt == Some((ix, DropEdge::After)) {
            list = list.child(DropIndicator::between_rows());
        }
    }
    let z_move = zone.clone();
    let z_drop = zone.clone();
    let pane = div()
        .id("dnd-pane")
        .relative()
        .w(px(400.))
        .h(px(280.))
        .rounded(px(10.))
        .border(px(0.5))
        .border_color(c.border_default)
        .bg(c.surface_sunken)
        .flex()
        .items_center()
        .justify_center()
        .child(Text::new("Drag a row over this pane").role(TextRole::BodySm).color(TextColor::Tertiary))
        .on_drag_move::<RowDrag>(move |e, _, cx| {
            let inside = e.bounds.contains(&e.event.position);
            let z = inside.then(|| {
                let p = e.event.position - e.bounds.origin;
                dock_zone(
                    f32::from(p.x),
                    f32::from(p.y),
                    f32::from(e.bounds.size.width),
                    f32::from(e.bounds.size.height),
                    0.25,
                )
            });
            if *z_move.read(cx) != z {
                z_move.update(cx, |s, cx| {
                    *s = z;
                    cx.notify();
                });
            }
        })
        .on_drop(move |_: &RowDrag, _, cx| {
            z_drop.update(cx, |s, cx| {
                *s = None;
                cx.notify();
            })
        })
        // Static preview for screenshots: the Right zone until a drag starts.
        .child(DropZone::new(zone.read(cx).or(if cx.has_active_drag() { None } else { Some(DockZone::Right) })));
    h_stack(24.)
        .items_start()
        .child(section(
            "Reorder (drag a row)",
            v_stack(12.).child(list).child(
                div().child(
                    DragPreview::new("Window chrome")
                        .icon(IconName::Document)
                        .count(3)
                        .element(cx),
                ),
            ),
            cx,
        ))
        .child(section("Drop zones (split / join)", pane, cx))
        .into_any_element()
}

fn selection_box(w: &mut Window, cx: &mut App) -> AnyElement {
    let count = state("sel-count", w, cx, || 2usize);
    let c = cx.theme().colors;
    let names = [
        "Brand.png",
        "Hero.png",
        "Icons",
        "Tokens.json",
        "Shell.fig",
        "Docs",
        "Notes.md",
        "Release.zip",
        "Logo.svg",
        "Palette.ase",
        "Specs.pdf",
        "Archive",
    ];
    let mut area = SelectionArea::new("sel-area")
        .selected(["Hero.png", "Tokens.json"])
        .on_change({
            let count = count.clone();
            move |sel, _, cx| {
                let n = sel.len();
                count.update(cx, |c, cx| {
                    *c = n;
                    cx.notify();
                })
            }
        });
    for name in names {
        let folder = !name.contains('.');
        area = area.item(
            name,
            v_stack(6.)
                .w(px(96.))
                .p(px(8.))
                .items_center()
                .child(
                    Icon::new(if folder {
                        IconName::Folder
                    } else {
                        IconName::Document
                    })
                    .size(IconSize::Large)
                    .color(if folder {
                        c.foreground_brand
                    } else {
                        c.foreground_secondary
                    }),
                )
                .child(Text::new(name).role(TextRole::CaptionMd)),
        );
    }
    v_stack(12.)
        .w(px(700.))
        .child(
            h_stack(8.)
                .child(Text::new(format!("{} selected", count.read(cx))).role(TextRole::LabelMd))
                .child(
                    Text::new("Drag on empty space · ⌘/Ctrl-click · Shift-click · ⌘/Ctrl-A")
                        .role(TextRole::BodySm)
                        .color(TextColor::Tertiary),
                ),
        )
        .child(
            div()
                .h(px(280.))
                .rounded(px(10.))
                .border(px(0.5))
                .border_color(c.border_subtle)
                .child(area),
        )
        .into_any_element()
}

fn command_palette(w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let palette = w.use_keyed_state("story-palette", cx, |window, cx| {
        CommandPalette::new(demo::commands(), window, cx)
    });
    let c = cx.theme().colors;
    // Inline preview of the palette body (the real one is a window-level
    // overlay opened with the button or ⇧⌘P / Ctrl+Shift+P).
    let preview_cmd = w.use_keyed_state("story-palette-preview", cx, |window, cx| {
        let mut cmd = Command::new(window, cx);
        let groups = vec![
            CommandGroup::new("File")
                .item(
                    CommandItem::new("Save").icon(IconName::Save).shortcut(
                        ShortcutHint::keys("secondary-s")
                            .label(cx)
                            .unwrap_or_default(),
                    ),
                )
                .item(
                    CommandItem::new("Open…")
                        .icon(IconName::FolderOpen)
                        .shortcut(
                            ShortcutHint::keys("secondary-o")
                                .label(cx)
                                .unwrap_or_default(),
                        ),
                ),
            CommandGroup::new("View")
                .item(
                    CommandItem::new("Toggle Sidebar")
                        .icon(IconName::PanelLeft)
                        .shortcut(
                            ShortcutHint::keys("secondary-b")
                                .label(cx)
                                .unwrap_or_default(),
                        ),
                )
                .item(CommandItem::new("Toggle Light / Dark").icon(IconName::Moon)),
            CommandGroup::new("Run").item(
                CommandItem::new("Run")
                    .icon(IconName::Play)
                    .shortcut(ShortcutHint::keys("f5").label(cx).unwrap_or_default()),
            ),
        ];
        cmd.set_groups(groups, cx);
        cmd
    });
    v_stack(16.)
        .child(
            row()
                .child(
                    Button::new("open-palette", "Open Command Palette")
                        .icon(IconName::Search)
                        .variant(ButtonVariant::Outline)
                        .size(ControlSize::Small)
                        .on_click({
                            let palette = palette.clone();
                            move |_, window, cx| palette.update(cx, |p, cx| p.toggle(window, cx))
                        }),
                )
                .child(ShortcutHint::action(
                    "command_palette::ToggleCommandPalette",
                )),
        )
        .child(
            div()
                .w(px(560.))
                .h(px(360.))
                .rounded(px(12.))
                .overflow_hidden()
                .border(px(0.5))
                .border_color(c.border_default)
                .shadow(cx.theme().shadow(Elevation::Shadow8))
                .child(preview_cmd),
        )
        .child(palette)
        .into_any_element()
}
