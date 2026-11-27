//! Demo actions, key bindings, menus and command entries shared by the
//! gallery stories.

use gpui::App;
use refineui::IconName;
use refineui::components::{CommandGroup, CommandItem};
use refineui::interaction::keymap::KeybindingManager;
use refineui::platform::Os;
use refineui::shell::AppMenu;

gpui::actions!(
    demo,
    [
        NewFile,
        OpenFile,
        Save,
        SaveAll,
        CloseTab,
        Quit,
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        Find,
        ToggleSidebar,
        TogglePanel,
        ToggleStatusBar,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        CycleDensity,
        ToggleAppearance,
        NextTab,
        PreviousTab,
        Run,
        Debug,
        About,
        Docs,
        Minimize,
        ZoomWindow,
        Hide,
        Preferences,
        SplitRight,
        SplitDown,
        ToggleZoom,
        FloatPanel,
        NewWindow,
        ToggleInspector,
        ResetLayout,
        ShowCommands,
    ]
);

/// The demo keymap (contexts: `Workspace` root, `Editor` for edit commands).
pub fn keymap() -> KeybindingManager {
    let mut km = KeybindingManager::new();
    km.context("Workspace", None)
        .context("Editor", Some("Workspace"));
    for (keys, action, ctx) in [
        ("secondary-n", "demo::NewFile", None),
        ("secondary-o", "demo::OpenFile", None),
        ("secondary-s", "demo::Save", None),
        ("secondary-alt-s", "demo::SaveAll", None),
        ("secondary-w", "demo::CloseTab", None),
        ("secondary-z", "demo::Undo", Some("Editor")),
        ("secondary-shift-z", "demo::Redo", Some("Editor")),
        ("secondary-x", "demo::Cut", Some("Editor")),
        ("secondary-c", "demo::Copy", Some("Editor")),
        ("secondary-v", "demo::Paste", Some("Editor")),
        ("secondary-a", "demo::SelectAll", Some("Editor")),
        ("secondary-f", "demo::Find", None),
        ("secondary-shift-p", "demo::ShowCommands", None),
        ("secondary-b", "demo::ToggleSidebar", None),
        ("secondary-j", "demo::TogglePanel", None),
        ("secondary-=", "demo::ZoomIn", None),
        ("secondary--", "demo::ZoomOut", None),
        ("secondary-0", "demo::ResetZoom", None),
        ("ctrl-tab", "demo::NextTab", None),
        ("ctrl-shift-tab", "demo::PreviousTab", None),
        ("f5", "demo::Run", None),
        ("shift-f5", "demo::Debug", None),
        ("secondary-,", "demo::Preferences", None),
        ("secondary-\\", "demo::SplitRight", None),
        ("secondary-alt-\\", "demo::SplitDown", None),
        ("secondary-shift-enter", "demo::ToggleZoom", None),
        ("secondary-shift-n", "demo::NewWindow", None),
        ("secondary-alt-b", "demo::ToggleInspector", None),
    ] {
        km.bind(keys, action, ctx);
    }
    // OS-specific: quit / hide / minimize.
    km.bind_for(Os::MacOs, "cmd-q", "demo::Quit", None)
        .bind_for(Os::MacOs, "cmd-h", "demo::Hide", None)
        .bind_for(Os::MacOs, "cmd-m", "demo::Minimize", None)
        .bind_for(Os::Windows, "alt-f4", "demo::Quit", None)
        .bind_for(Os::Linux, "ctrl-q", "demo::Quit", None);
    km
}

/// Install [`keymap`] once per app (idempotent).
pub fn install(cx: &mut App) {
    struct Installed;
    impl gpui::Global for Installed {}
    if cx.has_global::<Installed>() {
        return;
    }
    cx.set_global(Installed);
    for problem in keymap().install(cx) {
        eprintln!("refineui demo keymap: {problem}");
    }
}

/// The demo menu set (File / Edit / View / Run / Window / Help). On macOS
/// the first menu becomes the application menu.
pub fn menus() -> Vec<AppMenu> {
    let mut v = Vec::new();
    if Os::current() == Os::MacOs {
        v.push(
            AppMenu::new("RefineUI Shell")
                .action("About RefineUI Shell", "demo::About")
                .separator()
                .action("Settings…", "demo::Preferences")
                .separator()
                .action("Hide RefineUI Shell", "demo::Hide")
                .action("Quit RefineUI Shell", "demo::Quit"),
        );
    }
    let mut file = AppMenu::new("File")
        .action("New File", "demo::NewFile")
        .action("Open…", "demo::OpenFile")
        .submenu(
            AppMenu::new("Open Recent")
                .action("refineui-gpui", "demo::OpenFile")
                .action("refineui-system-icons", "demo::OpenFile"),
        )
        .separator()
        .action("Save", "demo::Save")
        .action("Save All", "demo::SaveAll")
        .separator()
        .action("Close Tab", "demo::CloseTab");
    if Os::current() != Os::MacOs {
        file = file
            .separator()
            .action("Settings", "demo::Preferences")
            .separator()
            .action("Exit", "demo::Quit");
    }
    v.push(file);
    v.push(
        AppMenu::new("Edit")
            .action("Undo", "demo::Undo")
            .action("Redo", "demo::Redo")
            .separator()
            .action("Cut", "demo::Cut")
            .action("Copy", "demo::Copy")
            .action("Paste", "demo::Paste")
            .action("Select All", "demo::SelectAll")
            .separator()
            .action("Find", "demo::Find"),
    );
    v.push(
        AppMenu::new("View")
            .action("Command Palette…", "demo::ShowCommands")
            .separator()
            .action("Toggle Sidebar", "demo::ToggleSidebar")
            .action("Toggle Panel", "demo::TogglePanel")
            .action("Toggle Status Bar", "demo::ToggleStatusBar")
            .action("Toggle Inspector", "demo::ToggleInspector")
            .separator()
            .submenu(
                AppMenu::new("Editor Layout")
                    .action("Split Right", "demo::SplitRight")
                    .action("Split Down", "demo::SplitDown")
                    .action("Toggle Maximized Group", "demo::ToggleZoom")
                    .separator()
                    .action("Float Panel", "demo::FloatPanel")
                    .action("Reset Layout", "demo::ResetLayout"),
            )
            .separator()
            .submenu(
                AppMenu::new("Appearance")
                    .action("Toggle Light / Dark", "demo::ToggleAppearance")
                    .action("Cycle Density", "demo::CycleDensity"),
            )
            .action("Zoom In", "demo::ZoomIn")
            .action("Zoom Out", "demo::ZoomOut")
            .action("Actual Size", "demo::ResetZoom"),
    );
    v.push(
        AppMenu::new("Run")
            .action("Run", "demo::Run")
            .action("Start Debugging", "demo::Debug"),
    );
    v.push(
        AppMenu::new("Window")
            .action("New Window", "demo::NewWindow")
            .separator()
            .action("Minimize", "demo::Minimize")
            .action("Zoom", "demo::ZoomWindow")
            .separator()
            .action("Next Tab", "demo::NextTab")
            .action("Previous Tab", "demo::PreviousTab"),
    );
    let mut help = AppMenu::new("Help").action("Documentation", "demo::Docs");
    if Os::current() != Os::MacOs {
        help = help.separator().action("About", "demo::About");
    }
    v.push(help);
    v
}

/// Command entries, grouped in first-seen order (each runs a demo action).
pub fn commands() -> Vec<CommandGroup> {
    let items = [
        (
            "File",
            CommandItem::new("New File")
                .action("demo::NewFile")
                .icon(IconName::Document),
        ),
        (
            "File",
            CommandItem::new("Open…")
                .action("demo::OpenFile")
                .icon(IconName::FolderOpen),
        ),
        (
            "File",
            CommandItem::new("Save")
                .action("demo::Save")
                .icon(IconName::Save),
        ),
        ("File", CommandItem::new("Save All").action("demo::SaveAll")),
        (
            "File",
            CommandItem::new("Close Tab").action("demo::CloseTab"),
        ),
        (
            "View",
            CommandItem::new("Toggle Sidebar")
                .action("demo::ToggleSidebar")
                .icon(IconName::PanelLeft)
                .keywords(["explorer", "panel"]),
        ),
        (
            "View",
            CommandItem::new("Toggle Panel")
                .action("demo::TogglePanel")
                .keywords(["terminal", "output", "problems"]),
        ),
        (
            "View",
            CommandItem::new("Toggle Status Bar").action("demo::ToggleStatusBar"),
        ),
        (
            "View",
            CommandItem::new("Toggle Inspector")
                .action("demo::ToggleInspector")
                .icon(IconName::Options),
        ),
        (
            "Layout",
            CommandItem::new("Split Editor Right")
                .action("demo::SplitRight")
                .icon(IconName::SquareMultiple)
                .keywords(["split", "group", "pane"]),
        ),
        (
            "Layout",
            CommandItem::new("Split Editor Down")
                .action("demo::SplitDown")
                .keywords(["split", "group", "pane"]),
        ),
        (
            "Layout",
            CommandItem::new("Toggle Maximized Editor Group")
                .action("demo::ToggleZoom")
                .icon(IconName::FullScreenMaximize)
                .keywords(["zoom", "maximize"]),
        ),
        (
            "Layout",
            CommandItem::new("Float Panel")
                .action("demo::FloatPanel")
                .keywords(["detach", "window", "undock"]),
        ),
        (
            "Layout",
            CommandItem::new("Reset Layout").action("demo::ResetLayout"),
        ),
        (
            "Window",
            CommandItem::new("New Window")
                .action("demo::NewWindow")
                .icon(IconName::Desktop),
        ),
        (
            "View",
            CommandItem::new("Toggle Light / Dark")
                .action("demo::ToggleAppearance")
                .icon(IconName::Moon)
                .keywords(["theme", "appearance", "dark", "light"]),
        ),
        (
            "View",
            CommandItem::new("Cycle Density")
                .action("demo::CycleDensity")
                .keywords(["compact", "comfortable", "spacious"]),
        ),
        (
            "Run",
            CommandItem::new("Run")
                .action("demo::Run")
                .icon(IconName::Play),
        ),
        (
            "Run",
            CommandItem::new("Start Debugging")
                .action("demo::Debug")
                .icon(IconName::Bug),
        ),
        (
            "Preferences",
            CommandItem::new("Settings")
                .action("demo::Preferences")
                .icon(IconName::Settings),
        ),
        (
            "Help",
            CommandItem::new("Documentation")
                .action("demo::Docs")
                .icon(IconName::QuestionCircle),
        ),
    ];
    let mut order: Vec<&str> = Vec::new();
    for (heading, _) in &items {
        if !order.contains(heading) {
            order.push(heading);
        }
    }
    order
        .into_iter()
        .map(|heading| {
            items
                .iter()
                .filter(|(h, _)| *h == heading)
                .fold(CommandGroup::new(heading), |g, (_, item)| {
                    g.item(item.clone())
                })
        })
        .collect()
}
