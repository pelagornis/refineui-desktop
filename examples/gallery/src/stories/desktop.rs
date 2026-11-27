//! Desktop shell chrome and interaction stories (Phase 2): titlebar, window
//! controls, menu bar, status bar, activity bar, application shell,
//! shortcut hints, drag and drop and selection box.

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
            shot: APPLICATION_SHELL_SIZE,
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
            summary: "Top-level destinations: icon-only with tooltips, expanded with sections and folders, captions, horizontal bars, badges, disabled items, drag-to-reorder.",
            shot: (980., 600.),
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
            id: "sortable-list",
            name: "Sortable List",
            group: StoryGroup::Desktop,
            summary: "Complete drag and drop: reorder, move between lists, indicators, keyboard, Esc, auto-scroll.",
            shot: (760., 420.),
            render: sortable_list,
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
            id: "notification-center",
            name: "Notification Center",
            group: StoryGroup::Desktop,
            summary: "Notification history on a sheet material: unread badges, inline actions, per-item dismiss, Clear All and a spring slide-in.",
            shot: (820., 680.),
            render: notification_center,
        },
        Story {
            id: "preferences-window",
            name: "Preferences Window",
            group: StoryGroup::Desktop,
            summary: "Preferences layout: a chrome sidebar of sections and grouped label / control rows.",
            shot: PREFERENCES_WINDOW_SIZE,
            render: preferences_window,
        },
        Story {
            id: "shortcut-recorder",
            name: "Shortcut Recorder",
            group: StoryGroup::Desktop,
            summary: "Click and press a key combination; platform keycaps, clear, cancel with Escape and keymap conflict warnings.",
            shot: (640., 420.),
            render: shortcut_recorder,
        },
        Story {
            id: "desktop-showcase",
            name: "Desktop Showcase",
            group: StoryGroup::Desktop,
            summary: "A complete app on the Material Engine: vibrant shell chrome, sidebar, workspace, status bar and the notification center.",
            shot: (1280., 800.),
            render: desktop_showcase,
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
    let (width, height) = APPLICATION_SHELL_SIZE;
    div()
        .w(px(width))
        .h(px(height))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(shell_app(true, w, cx))
        .into_any_element()
}

/// Edge-to-edge application shell for the external window.
pub(crate) fn application_shell_app(w: &mut Window, cx: &mut App) -> AnyElement {
    shell_app(false, w, cx)
}

fn shell_app(embedded: bool, w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let current = Os::current();
    let selected = state("shell-activity", w, cx, || SharedString::from("explorer"));
    let c = cx.theme().colors;
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
    ApplicationShell::new("story-shell")
        .os(current)
        .embedded(embedded)
        .vibrant(true)
        .title("refineui-gpui")
        .subtitle("shell.rs")
        .menus(demo::menus())
        .toolbar(toolbar)
        .activity_bar(activity)
        .left_dock(ShellDock::new(sidebar, 220.))
        .center(editor)
        .status_bar(status)
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
    let wide_sel = state("ab-wide-sel", w, cx, || SharedString::from("inbox"));
    let wide = state("ab-wide", w, cx, || true);
    let rail_sel = state("ab-rail-sel", w, cx, || SharedString::from("home"));
    let top_sel = state("ab-top-sel", w, cx, || SharedString::from("files"));
    let c = cx.theme().colors;
    let item = |id: &str| match id {
        "explorer" => ActivityItem::icon("explorer", "Explorer", IconName::Document)
            .action("workspace::ToggleExplorer"),
        "search" => ActivityItem::icon("search", "Search", IconName::Search),
        "git" => ActivityItem::icon("git", "Source Control", IconName::Git).badge(4),
        "debug" => ActivityItem::icon("debug", "Run and Debug", IconName::Bug),
        _ => ActivityItem::icon("extensions", "Extensions", IconName::Cube).dot(),
    };
    let frame = |title: &str, h: f32, bar: AnyElement| {
        v_stack(8.)
            .items_start()
            .child(
                Text::new(title.to_string())
                    .role(TextRole::CaptionMd)
                    .color(TextColor::Tertiary),
            )
            .child(
                div()
                    .h(px(h))
                    .flex()
                    .rounded(px(10.))
                    .overflow_hidden()
                    .border(px(0.5))
                    .border_color(c.border_default)
                    .child(bar),
            )
    };
    let ids: Vec<&'static str> = order.read(cx).clone();
    let icon_only = ActivityBar::new("ab-icons")
        .items(ids.iter().map(|i| item(i)))
        .divider()
        .item(
            ActivityItem::icon("issues", "Issues", IconName::ErrorCircle)
                .badge(128)
                .badge_variant(BadgeVariant::Danger),
        )
        .item(ActivityItem::icon("deploy", "Deploy (offline)", IconName::Cloud).disabled(true))
        .footer_item(ActivityItem::image(
            "me",
            "Pelagornis Official",
            super::display::portrait(),
        ))
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
    let is_wide = *wide.read(cx);
    let toggle = {
        let wide = wide.clone();
        Button::icon_only("ab-toggle", IconName::PanelLeft)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Small)
            .tooltip(if is_wide { "Collapse" } else { "Expand" })
            .on_click(move |_, _, cx| {
                wide.update(cx, |v, cx| {
                    *v = !*v;
                    cx.notify();
                })
            })
    };
    let expanded = ActivityBar::new("ab-wide")
        .expanded(is_wide)
        .header(toggle)
        .section("Mail")
        .item(ActivityItem::icon("inbox", "Inbox", IconName::Mail).badge(12))
        .item(ActivityItem::icon("starred", "Starred", IconName::Star))
        .item(ActivityItem::icon("drafts", "Drafts", IconName::Edit).unread(true))
        .section("Projects")
        .item(
            ActivityItem::folder(
                "work",
                "Work",
                [
                    ActivityItem::icon("board", "Roadmap", IconName::Board),
                    ActivityItem::icon("chat", "Design review", IconName::Chat).badge(3),
                    ActivityItem::icon("db", "Data", IconName::Database),
                ],
            )
            .default_open(true),
        )
        .item(
            ActivityItem::folder(
                "personal",
                "Personal",
                [
                    ActivityItem::icon("ideas", "Ideas", IconName::Lightbulb).dot(),
                    ActivityItem::icon("trips", "Trips", IconName::Pin),
                ],
            )
            .folder_icon(IconName::Heart),
        )
        .footer_item(ActivityItem::icon(
            "wide-settings",
            "Settings",
            IconName::Settings,
        ))
        .selected(wide_sel.read(cx).clone())
        .on_select(setter(&wide_sel));
    let captions = ActivityBar::new("ab-rail")
        .labels(ActivityLabels::Below)
        .item(ActivityItem::icon("home", "Home", IconName::Home))
        .item(ActivityItem::icon("chat", "Chat", IconName::Chat).badge(2))
        .item(ActivityItem::icon(
            "calendar",
            "Calendar",
            IconName::Calendar,
        ))
        .item(ActivityItem::icon("files", "Files", IconName::Folder).dot())
        .footer_item(ActivityItem::icon("help", "Help", IconName::QuestionCircle))
        .selected(rail_sel.read(cx).clone())
        .on_select(setter(&rail_sel));
    let top = |id: &'static str, indicator: ActiveIndicator| {
        ActivityBar::new(id)
            .side(BarSide::Top)
            .indicator(indicator)
            .item(ActivityItem::icon("files", "Files", IconName::Document))
            .item(ActivityItem::icon(
                "outline",
                "Outline",
                IconName::Navigation,
            ))
            .item(ActivityItem::icon("changes", "Changes", IconName::Git).badge(7))
            .divider()
            .item(ActivityItem::icon("problems", "Problems", IconName::Warning).dot())
            .footer_item(ActivityItem::icon("more", "More", IconName::MoreHorizontal))
            .selected(top_sel.read(cx).clone())
            .on_select(setter(&top_sel))
    };
    // Badge counts 1 / 9 / 42 / 128 in each label mode: corner badges cap at
    // `9+` (tooltip has the count), expanded rows spell out up to `99+`.
    let counts = |id: &'static str, labels: ActivityLabels| {
        ActivityBar::new(id)
            .labels(labels)
            .item(ActivityItem::icon("one", "Mentions", IconName::Mail).badge(1))
            .item(ActivityItem::icon("nine", "Reviews", IconName::Chat).badge(9))
            .item(ActivityItem::icon("many", "Changes", IconName::Git).badge(42))
            .item(
                ActivityItem::icon("lots", "Issues", IconName::ErrorCircle)
                    .badge(128)
                    .badge_variant(BadgeVariant::Danger),
            )
            .selected("one")
    };
    let badge_row = h_stack(16.)
        .items_start()
        .child(frame(
            "Badges, icon only",
            180.,
            counts("ab-count-icons", ActivityLabels::Tooltip).into_any_element(),
        ))
        .child(frame(
            "Badges, captions",
            264.,
            counts("ab-count-rail", ActivityLabels::Below).into_any_element(),
        ))
        .child(frame(
            "Badges, expanded",
            180.,
            counts("ab-count-wide", ActivityLabels::Beside).into_any_element(),
        ));
    let main_row = h_stack(20.)
        .items_start()
        .child(frame(
            "Icon only (drag to reorder)",
            440.,
            icon_only.into_any_element(),
        ))
        .child(frame(
            "Expanded, sections, folders",
            440.,
            expanded.into_any_element(),
        ))
        .child(frame("Captions", 440., captions.into_any_element()))
        .child(
            v_stack(24.)
                .child(frame(
                    "Horizontal (top), edge indicator",
                    48.,
                    div()
                        .w(px(300.))
                        .child(top("ab-top", ActiveIndicator::Edge))
                        .into_any_element(),
                ))
                .child(frame(
                    "Horizontal, plate indicator",
                    48.,
                    div()
                        .w(px(300.))
                        .child(top("ab-top-plate", ActiveIndicator::Plate))
                        .into_any_element(),
                ))
                .child(badge_row),
        );
    main_row.into_any_element()
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

fn sortable_list(w: &mut Window, cx: &mut App) -> AnyElement {
    type Lists = Vec<(SharedString, Vec<(SharedString, &'static str)>)>;
    let lists = state("sortable-lists", w, cx, || -> Lists {
        vec![
            (
                "todo".into(),
                (1..=9)
                    .map(|i| {
                        (
                            SharedString::from(format!("t{i}")),
                            [
                                "Design tokens",
                                "Material blur",
                                "Activity Bar",
                                "Command Palette",
                                "Docs",
                                "Icons",
                                "Tabs",
                                "Search",
                                "Status bar",
                            ][i - 1],
                        )
                    })
                    .collect(),
            ),
            (
                "doing".into(),
                vec![
                    ("d1".into(), "Foundation crate"),
                    ("d2".into(), "Phase 0 report"),
                ],
            ),
            ("done".into(), vec![("x1".into(), "Input OTP")]),
        ]
    });
    let data = lists.read(cx).clone();
    let names = ["To do", "In progress", "Done"];
    let keys: Vec<SharedString> = data.iter().map(|(k, _)| k.clone()).collect();
    let mut cols = Vec::new();
    for (ix, (key, items)) in data.into_iter().enumerate() {
        let lists = lists.clone();
        cols.push(
            v_stack(8.)
                .w(px(230.))
                .child(
                    Text::new(names[ix])
                        .role(TextRole::CaptionLg)
                        .color(TextColor::Tertiary),
                )
                .child(
                    SortableList::new(
                        SharedString::from(format!("sortable-{key}")),
                        key.clone(),
                        "board",
                    )
                    .items(
                        items
                            .iter()
                            .map(|(id, l)| SortableItem::new(id.clone(), *l)),
                    )
                    .max_height(px(300.))
                    .neighbors(
                        ix.checked_sub(1).map(|i| keys[i].clone()),
                        keys.get(ix + 1).cloned(),
                    )
                    .on_move(move |m, _, cx| {
                        lists.update(cx, |l, cx| {
                            m.apply(l);
                            cx.notify();
                        })
                    }),
                ),
        );
    }
    v_stack(12.)
        .child(
            Text::new("Drag rows within or across lists. Keyboard: focus a list, Space lifts, ↑ / ↓ / ← / → move, Enter drops, Esc cancels; Alt+↑ / ↓ moves at once.")
                .role(TextRole::BodySm)
                .color(TextColor::Secondary),
        )
        .child(h_stack(16.).items_start().children(cols))
        .into_any_element()
}

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

/// Soft desktop wallpaper for material stories: two-tone gradient and a few
/// large colour fields the chrome materials blur.
fn wallpaper(cx: &App) -> gpui::Div {
    use gpui::{linear_color_stop, linear_gradient, rgb};
    let dark = cx.theme().is_dark();
    let blob = |x: f32, y: f32, d: f32, color: u32| {
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .size(px(d))
            .rounded_full()
            .bg(rgb(color))
    };
    let (a, b) = if dark {
        (0x101833, 0x2A1636)
    } else {
        (0xDCE6FF, 0xF7E4F0)
    };
    let tones: [u32; 4] = if dark {
        [0x3B4FD8, 0xB0368A, 0x0F8F86, 0xD9822B]
    } else {
        [0x6E86FF, 0xFF8FC8, 0x5FD6C4, 0xFFC56E]
    };
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .bg(linear_gradient(
            150.,
            linear_color_stop(rgb(a), 0.),
            linear_color_stop(rgb(b), 1.),
        ))
        .child(blob(-80., -120., 420., tones[0]))
        .child(blob(520., 380., 460., tones[1]))
        .child(blob(160., 420., 260., tones[2]))
        .child(blob(820., -60., 300., tones[3]))
}

fn sample_notifications(cx: &App) -> Vec<Notification> {
    let _ = cx;
    vec![
        Notification::new("Build finished")
            .id("build")
            .source("Pipelines")
            .time("now")
            .body("release-2.4 passed 128 checks in 3m 12s.")
            .icon(IconName::Checkmark, NotificationTone::Success)
            .unread(true),
        Notification::new("Mina mentioned you")
            .id("mention")
            .source("Comments · Q4 Launch Plan")
            .time("4m")
            .body("Can you check the pricing table before Friday?")
            .icon(IconName::Chat, NotificationTone::Brand)
            .unread(true),
        Notification::new("Storage almost full")
            .id("storage")
            .source("Sync")
            .time("1h")
            .body("92% of 50 GB used. Old snapshots can be removed.")
            .icon(IconName::Cloud, NotificationTone::Warning),
        Notification::new("Update installed")
            .id("update")
            .source("RefineUI")
            .time("Yesterday")
            .body("Version 2.4 adds materials to every window.")
            .icon(IconName::Star, NotificationTone::Neutral),
    ]
}

fn notification_center(w: &mut Window, cx: &mut App) -> AnyElement {
    let open = state("nc-open", w, cx, || true);
    let cleared = state("nc-cleared", w, cx, || false);
    let dismissed = state("nc-dismissed", w, cx, Vec::<usize>::new);
    let is_open = *open.read(cx);
    let gone = dismissed.read(cx).clone();
    let items: Vec<Notification> = if *cleared.read(cx) {
        Vec::new()
    } else {
        sample_notifications(cx)
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !gone.contains(i))
            .map(|(_, n)| n)
            .collect()
    };
    let map: Vec<usize> = (0..4).filter(|i| !gone.contains(i)).collect();
    let center = NotificationCenter::new("story-nc")
        .open(is_open)
        .max_height(px(600.))
        .notifications(items)
        .on_dismiss({
            let dismissed = dismissed.clone();
            move |ix, _, cx| {
                let real = map.get(*ix).copied();
                dismissed.update(cx, |v, cx| {
                    v.extend(real);
                    cx.notify();
                })
            }
        })
        .on_clear_all({
            let cleared = cleared.clone();
            move |_, _, cx| {
                cleared.update(cx, |v, cx| {
                    *v = true;
                    cx.notify();
                })
            }
        })
        .on_close({
            let open = open.clone();
            move |_, _, cx| {
                open.update(cx, |v, cx| {
                    *v = false;
                    cx.notify();
                })
            }
        });
    let empty = NotificationCenter::new("story-nc-empty")
        .inline(true)
        .width(px(300.));
    let toggle = Button::new(
        "nc-toggle",
        if is_open {
            "Hide Center"
        } else {
            "Show Center"
        },
    )
    .size(ControlSize::Small)
    .variant(ButtonVariant::Secondary)
    .on_click({
        let open = open.clone();
        let cleared = cleared.clone();
        let dismissed = dismissed.clone();
        move |_, _, cx| {
            open.update(cx, |v, cx| {
                *v = !*v;
                cx.notify();
            });
            cleared.update(cx, |v, _| *v = false);
            dismissed.update(cx, |v, _| v.clear());
        }
    });
    let mid = NotificationCenter::new("story-nc-mid")
        .sample_progress(0.45)
        .width(px(360.))
        .max_height(px(520.))
        .notifications(sample_notifications(cx));
    div()
        .relative()
        .w(px(772.))
        .h(px(632.))
        .rounded(px(16.))
        .overflow_hidden()
        .child(wallpaper(cx))
        .child(div().absolute().left(px(16.)).top(px(16.)).child(toggle))
        .child(
            div()
                .absolute()
                .left(px(16.))
                .top(px(56.))
                .w(px(300.))
                .h(px(360.))
                .overflow_hidden()
                .child(mid),
        )
        .child(div().absolute().left(px(16.)).bottom(px(16.)).child(empty))
        .child(
            div()
                .absolute()
                .top(px(16.))
                .right(px(16.))
                .bottom(px(16.))
                .child(center),
        )
        .into_any_element()
}

fn preferences_window_body(w: &mut Window, cx: &mut App) -> PreferencesWindow {
    demo::install(cx);
    let section = state("prefs-section", w, cx, || 0usize);
    let launch = state("prefs-launch", w, cx, || true);
    let sync = state("prefs-sync", w, cx, || false);
    let recorder = w.use_keyed_state("prefs-recorder", cx, |_, cx| {
        ShortcutRecorder::new(cx)
            .with_keys("secondary-shift-p")
            .action("demo::ShowCommands")
    });
    let toggle = |e: &gpui::Entity<bool>, id: &'static str, cx: &App| {
        let e2 = e.clone();
        Switch::new(id, *e.read(cx)).on_change(move |v, _, cx| {
            e2.update(cx, |s, cx| {
                *s = *v;
                cx.notify();
            })
        })
    };
    let ix = *section.read(cx);
    PreferencesWindow::new("story-prefs")
        .section(
            PreferencesSection::new("General", IconName::Settings)
                .description("How the app starts, saves and updates."),
        )
        .section(
            PreferencesSection::new("Appearance", IconName::WeatherSunny)
                .description("Theme, density and materials."),
        )
        .section(
            PreferencesSection::new("Shortcuts", IconName::Keyboard)
                .description("Record your own key combinations."),
        )
        .section(PreferencesSection::new("Accounts", IconName::Person))
        .section(PreferencesSection::new("Privacy", IconName::LockClosed))
        .selected(ix)
        .on_select(setter(&section))
        .footer(
            Text::new("Version 2.4.0")
                .role(TextRole::CaptionMd)
                .color(TextColor::Tertiary),
        )
        .child(
            PreferencesGroup::new()
                .title("Startup")
                .row(
                    PreferencesRow::new("Open at login")
                        .description("Start in the menu bar when you sign in.")
                        .control(toggle(&launch, "prefs-launch-sw", cx)),
                )
                .row(
                    PreferencesRow::new("Restore windows")
                        .description("Reopen the documents from your last session.")
                        .control(Switch::new("prefs-restore", true)),
                ),
        )
        .child(
            PreferencesGroup::new()
                .title("Sync")
                .description("Sync uses end-to-end encryption.")
                .row(
                    PreferencesRow::new("Sync settings across devices").control(toggle(
                        &sync,
                        "prefs-sync-sw",
                        cx,
                    )),
                )
                .row(PreferencesRow::new("Command Palette").control(recorder)),
        )
}

fn preferences_window(w: &mut Window, cx: &mut App) -> AnyElement {
    let (width, height) = PREFERENCES_WINDOW_SIZE;
    div()
        .w(px(width))
        .h(px(height))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(preferences_app(true, w, cx))
        .into_any_element()
}

/// Edge-to-edge Preferences window for the external OS window.
pub(crate) fn preferences_window_app(w: &mut Window, cx: &mut App) -> AnyElement {
    preferences_app(false, w, cx)
}

fn preferences_app(embedded: bool, w: &mut Window, cx: &mut App) -> AnyElement {
    let prefs = preferences_window_body(w, cx);
    let body = div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            Titlebar::new("prefs-title")
                .title("Preferences")
                .buttons(WindowPreset::Settings)
                .interactive(!embedded),
        )
        .child(div().flex_1().min_h_0().child(prefs));
    if embedded {
        body.into_any_element()
    } else {
        WindowFrame::new(body).into_any_element()
    }
}

fn shortcut_recorder(w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let empty = w.use_keyed_state("sr-empty", cx, |_, cx| ShortcutRecorder::new(cx));
    let set = w.use_keyed_state("sr-set", cx, |_, cx| {
        ShortcutRecorder::new(cx)
            .with_keys("secondary-shift-p")
            .action("demo::ShowCommands")
    });
    let conflict = w.use_keyed_state("sr-conflict", cx, |_, cx| {
        ShortcutRecorder::new(cx)
            .with_keys("secondary-s")
            .action("demo::Export")
    });
    let chord = w.use_keyed_state("sr-chord", cx, |_, cx| {
        ShortcutRecorder::new(cx)
            .with_keys("secondary-k secondary-s")
            .chords(true)
    });
    let disabled = w.use_keyed_state("sr-disabled", cx, |_, cx| {
        ShortcutRecorder::new(cx).with_keys("f5").disabled(true)
    });
    let line = |label: &'static str, e: gpui::AnyView| {
        h_stack(16.)
            .items_start()
            .child(
                div().w(px(140.)).pt(px(6.)).child(
                    Text::new(label)
                        .role(TextRole::BodySm)
                        .color(TextColor::Secondary),
                ),
            )
            .child(div().w(px(260.)).child(e))
    };
    v_stack(16.)
        .child(line("Empty", empty.into()))
        .child(line("Recorded", set.into()))
        .child(line("Conflict", conflict.into()))
        .child(line("Chord", chord.into()))
        .child(line("Disabled", disabled.into()))
        .child(
            Text::new("Click a field, then press a combination. Esc cancels, Backspace clears; chord fields wait 1s for a second key.")
                .role(TextRole::CaptionMd)
                .color(TextColor::Tertiary),
        )
        .into_any_element()
}

/// Logical size of the Application Shell story (screenshot and external window).
pub(crate) const APPLICATION_SHELL_SIZE: (f32, f32) = WindowPreset::Main.default_size();

/// Logical size of the Preferences window (screenshot and external window).
pub(crate) const PREFERENCES_WINDOW_SIZE: (f32, f32) = WindowPreset::Settings.default_size();

/// Logical size of the Atlas app (screenshot canvas and external window).
pub(crate) const DESKTOP_SHOWCASE_SIZE: (f32, f32) = (1280., 800.);

fn desktop_showcase(w: &mut Window, cx: &mut App) -> AnyElement {
    let (width, height) = DESKTOP_SHOWCASE_SIZE;
    div()
        .w(px(width))
        .h(px(height))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(atlas_app(true, w, cx))
        .into_any_element()
}

/// Edge-to-edge Atlas app for the external showcase window.
pub(crate) fn desktop_showcase_app(w: &mut Window, cx: &mut App) -> AnyElement {
    atlas_app(false, w, cx)
}

fn atlas_app(embedded: bool, w: &mut Window, cx: &mut App) -> AnyElement {
    demo::install(cx);
    let c = cx.theme().colors;
    let selected = state("show-activity", w, cx, || SharedString::from("notes"));
    let nav = state("show-nav", w, cx, || 1usize);
    let show_nc = state("show-nc", w, cx, || true);
    let nc_open = *show_nc.read(cx);
    let current = Os::current();

    let toolbar = Toolbar::new("show-tb")
        .fill(true)
        .size(ControlSize::Small)
        .button(
            Button::icon_only("show-side", IconName::PanelLeft)
                .variant(ButtonVariant::Ghost)
                .tooltip("Toggle Sidebar"),
        )
        .spacer()
        .child(
            div()
                .w(px(280.))
                .h(px(26.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(8.))
                .bg(c.background_surface.opacity(0.6))
                .border(px(0.5))
                .border_color(c.border_subtle)
                .child(
                    Icon::new(IconName::Search)
                        .size(IconSize::XSmall)
                        .color(c.foreground_tertiary),
                )
                .child(
                    div().flex_1().child(
                        Text::new("Search notes")
                            .role(TextRole::BodySm)
                            .color(TextColor::Tertiary),
                    ),
                )
                .child(ShortcutHint::keys("secondary-k").os(current)),
        )
        .spacer()
        .button(
            Button::icon_only("show-share", IconName::Share)
                .variant(ButtonVariant::Ghost)
                .tooltip("Share"),
        )
        .button(
            Button::icon_only("show-bell", IconName::Alert)
                .variant(ButtonVariant::Ghost)
                .tooltip("Notifications")
                .on_click({
                    let show_nc = show_nc.clone();
                    move |_, _, cx| {
                        show_nc.update(cx, |v, cx| {
                            *v = !*v;
                            cx.notify();
                        })
                    }
                }),
        );
    let activity = ActivityBar::new("show-activity")
        .items([
            ActivityItem::icon("notes", "Notes", IconName::Document),
            ActivityItem::icon("search", "Search", IconName::Search),
            ActivityItem::folder(
                "apps",
                "Apps",
                [
                    ActivityItem::icon("mail", "Mail", IconName::Mail),
                    ActivityItem::icon("photo", "Photos", IconName::Grid),
                    ActivityItem::icon("music", "Music", IconName::Play),
                    ActivityItem::icon("code", "Code", IconName::Code),
                    ActivityItem::icon("docs", "Docs", IconName::Document),
                ],
            ),
            ActivityItem::folder(
                "projects",
                "Projects",
                [
                    ActivityItem::icon("calendar", "Calendar", IconName::Calendar),
                    ActivityItem::icon("people", "People", IconName::Person).badge(2),
                ],
            )
            .default_open(true),
            ActivityItem::image("workspace", "Workspace", super::display::portrait()),
        ])
        .footer_item(ActivityItem::icon(
            "settings",
            "Settings",
            IconName::Settings,
        ))
        .selected(selected.read(cx).clone())
        .on_select(setter(&selected));

    let sel = *nav.read(cx);
    let nav_row = |ix: usize, icon: IconName, label: &'static str, count: Option<&'static str>| {
        let nav = nav.clone();
        let on = ix == sel;
        h_stack(8.)
            .id(SharedString::from(format!("show-nav-{ix}")))
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .when(on, |d| d.bg(c.background_surface_selected))
            .when(!on, |d| d.hover(|s| s.bg(c.background_surface_hover)))
            .on_click(move |_, _, cx| {
                nav.update(cx, |v, cx| {
                    *v = ix;
                    cx.notify();
                })
            })
            .child(Icon::new(icon).size(IconSize::XSmall).color(if on {
                c.foreground_brand
            } else {
                c.foreground_secondary
            }))
            .child(
                div()
                    .flex_1()
                    .child(Text::new(label).role(TextRole::BodySm).color(if on {
                        TextColor::Primary
                    } else {
                        TextColor::Secondary
                    })),
            )
            .children(count.map(|n| {
                Text::new(n)
                    .role(TextRole::CaptionMd)
                    .color(TextColor::Tertiary)
            }))
    };
    let project = |color: gpui::Hsla, label: &'static str| {
        h_stack(8.)
            .h(px(26.))
            .px(px(8.))
            .child(div().size(px(8.)).rounded_full().bg(color))
            .child(
                Text::new(label)
                    .role(TextRole::BodySm)
                    .color(TextColor::Secondary),
            )
    };
    let heading = |t: &'static str| {
        div().px(px(8.)).pt(px(12.)).pb(px(4.)).child(
            Text::new(t)
                .role(TextRole::CaptionMd)
                .color(TextColor::Tertiary),
        )
    };
    let sidebar = v_stack(2.)
        .p(px(8.))
        .child(heading("LIBRARY"))
        .child(nav_row(0, IconName::Home, "Home", None))
        .child(nav_row(1, IconName::Document, "All Notes", Some("128")))
        .child(nav_row(2, IconName::Star, "Favorites", Some("12")))
        .child(nav_row(3, IconName::Share, "Shared", Some("7")))
        .child(nav_row(4, IconName::Delete, "Trash", None))
        .child(heading("PROJECTS"))
        .child(project(c.foreground_brand, "Q4 Launch"))
        .child(project(c.foreground_success, "Design System"))
        .child(project(c.foreground_warning, "Research"));

    let stat = |label: &'static str, value: &'static str, delta: &'static str, good: bool| {
        v_stack(4.)
            .flex_1()
            .p(px(14.))
            .rounded(px(12.))
            .border(px(0.5))
            .border_color(c.border_subtle)
            .bg(c.background_surface)
            .child(
                Text::new(label)
                    .role(TextRole::CaptionMd)
                    .color(TextColor::Secondary),
            )
            .child(Text::new(value).role(TextRole::TitleMd))
            .child(Text::new(delta).role(TextRole::CaptionMd).color(if good {
                TextColor::Success
            } else {
                TextColor::Warning
            }))
    };
    let task = |done: bool, label: &'static str, who: &'static str| {
        h_stack(10.)
            .h(px(36.))
            .px(px(12.))
            .border_b(px(0.5))
            .border_color(c.border_subtle)
            .child(
                Icon::new(if done {
                    IconName::Checkmark
                } else {
                    IconName::Circle
                })
                .size(IconSize::XSmall)
                .color(if done {
                    c.foreground_success
                } else {
                    c.foreground_tertiary
                }),
            )
            .child(
                div()
                    .flex_1()
                    .child(Text::new(label).role(TextRole::BodySm).color(if done {
                        TextColor::Tertiary
                    } else {
                        TextColor::Primary
                    })),
            )
            .child(
                Text::new(who)
                    .role(TextRole::CaptionMd)
                    .color(TextColor::Secondary),
            )
    };
    let document = v_stack(16.)
        .size_full()
        .px(px(32.))
        .py(px(24.))
        .child(
            v_stack(4.)
                .child(
                    Text::new("Q4 LAUNCH · UPDATED 4 MIN AGO")
                        .role(TextRole::CaptionMd)
                        .color(TextColor::Brand),
                )
                .child(Text::new("Launch Plan").role(TextRole::TitleLg))
                .child(
                    Text::new("Ship RefineUI 2.4 with materials across every desktop surface.")
                        .role(TextRole::BodyMd)
                        .color(TextColor::Secondary),
                ),
        )
        .child(
            h_stack(12.)
                .child(stat("Readiness", "86%", "+12% this week", true))
                .child(stat("Open issues", "14", "3 blockers", false))
                .child(stat("Reviews", "9 / 11", "2 waiting", true)),
        )
        .child(
            v_stack(0.)
                .rounded(px(12.))
                .border(px(0.5))
                .border_color(c.border_subtle)
                .overflow_hidden()
                .child(task(true, "Material engine on Metal and Vulkan", "Hana"))
                .child(task(true, "Shell chrome on chrome materials", "Joon"))
                .child(task(false, "Notification center polish", "Mina"))
                .child(task(false, "Preferences window copy review", "Alex")),
        );

    let status = StatusBar::new("show-status")
        .leading(
            StatusItem::text("sync", "Synced")
                .icon(IconName::Cloud)
                .tone(StatusTone::Success),
        )
        .leading(StatusItem::text("words", "1,248 words"))
        .trailing(StatusItem::text("collab", "3 editing").icon(IconName::Person));

    let nc = NotificationCenter::new("show-nc")
        .open(nc_open)
        .width(px(340.))
        .max_height(px(470.))
        .notifications(sample_notifications(cx).into_iter().take(3))
        .on_close({
            let show_nc = show_nc.clone();
            move |_, _, cx| {
                show_nc.update(cx, |v, cx| {
                    *v = false;
                    cx.notify();
                })
            }
        })
        .on_clear_all(|_, _, _| {});

    ApplicationShell::new("show-shell")
        .os(current)
        .embedded(embedded)
        .vibrant(true)
        .title("Atlas")
        .subtitle("Launch Plan")
        .toolbar(toolbar)
        .activity_bar(activity)
        .left_dock(ShellDock::new(sidebar, 210.))
        .center(document)
        .status_bar(status)
        .overlay(div().absolute().top(px(60.)).right(px(12.)).child(nc))
        .into_any_element()
}
