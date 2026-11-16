//! Demo actions, key bindings, menus and palette commands shared by the
//! gallery stories and the `refineui-shell` demo window.

use gpui::App;
use refineui::IconName;
use refineui::components::PaletteCommand;
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
        (
            "secondary-shift-p",
            "command_palette::ToggleCommandPalette",
            None,
        ),
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
            .action("Command Palette…", "command_palette::ToggleCommandPalette")
            .separator()
            .action("Toggle Sidebar", "demo::ToggleSidebar")
            .action("Toggle Panel", "demo::TogglePanel")
            .action("Toggle Status Bar", "demo::ToggleStatusBar")
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

/// Command palette entries.
pub fn commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand::new("New File", "demo::NewFile")
            .group("File")
            .icon(IconName::Document),
        PaletteCommand::new("Open…", "demo::OpenFile")
            .group("File")
            .icon(IconName::FolderOpen),
        PaletteCommand::new("Save", "demo::Save")
            .group("File")
            .icon(IconName::Save),
        PaletteCommand::new("Save All", "demo::SaveAll").group("File"),
        PaletteCommand::new("Close Tab", "demo::CloseTab").group("File"),
        PaletteCommand::new("Toggle Sidebar", "demo::ToggleSidebar")
            .group("View")
            .icon(IconName::PanelLeft)
            .keywords(["explorer", "panel"]),
        PaletteCommand::new("Toggle Panel", "demo::TogglePanel")
            .group("View")
            .keywords(["terminal", "output", "problems"]),
        PaletteCommand::new("Toggle Status Bar", "demo::ToggleStatusBar").group("View"),
        PaletteCommand::new("Toggle Light / Dark", "demo::ToggleAppearance")
            .group("View")
            .icon(IconName::Moon)
            .keywords(["theme", "appearance", "dark", "light"]),
        PaletteCommand::new("Cycle Density", "demo::CycleDensity")
            .group("View")
            .keywords(["compact", "comfortable", "spacious"]),
        PaletteCommand::new("Run", "demo::Run")
            .group("Run")
            .icon(IconName::Play),
        PaletteCommand::new("Start Debugging", "demo::Debug")
            .group("Run")
            .icon(IconName::Bug),
        PaletteCommand::new("Settings", "demo::Preferences")
            .group("Preferences")
            .icon(IconName::Settings),
        PaletteCommand::new("Documentation", "demo::Docs")
            .group("Help")
            .icon(IconName::QuestionCircle),
    ]
}
