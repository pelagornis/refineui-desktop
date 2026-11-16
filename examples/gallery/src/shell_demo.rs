//! `refineui-shell`: a desktop app shell built only from RefineUI shell
//! parts — titlebar (unified toolbar), menu bar (system on macOS), activity
//! bar, sidebar, editor placeholder, bottom panel, status bar and the
//! command palette. Run with `cargo run -p refineui-gallery --bin
//! refineui-shell`, or from the gallery's "Shell demo" button.

use std::path::PathBuf;

use gpui::{
    App, AppContext, Application, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px,
};
use refineui::prelude::*;
use refineui::shell::{set_system_menus, track_window_state, window_options};
use refineui::tokens::{spacing, stroke, typography::role};

use crate::demo::{self, *};

/// Where the window placement is saved.
pub fn state_path() -> PathBuf {
    let base = match Os::current() {
        Os::MacOs => std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support"))
            .unwrap_or_else(std::env::temp_dir),
        Os::Windows => std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir),
        Os::Linux => std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(std::env::temp_dir),
    };
    base.join("refineui-shell").join("window.state")
}

/// Open the shell demo window in a running app (`refineui::init` done).
pub fn open(cx: &mut App) {
    demo::install(cx);
    if Os::current() == Os::MacOs {
        let menus = demo::menus();
        for missing in set_system_menus(&menus, cx) {
            eprintln!("refineui-shell: menu action {missing} is not registered");
        }
    }
    let path = state_path();
    let spec = WindowSpec::new(WindowPreset::Main, "RefineUI Shell")
        .app_id("dev.refineui.shell")
        .restore(WindowState::load(&path));
    let options = window_options(&spec, cx);
    let opened = cx.open_window(options, move |window, cx| {
        let view = cx.new(|cx| ShellDemo::new(path.clone(), window, cx));
        let focus = view.read(cx).editor_focus.clone();
        window.focus(&focus);
        view
    });
    if let Err(e) = opened {
        eprintln!("refineui-shell: could not open the window: {e}");
    }
}

/// Stand-alone entry point.
pub fn run() {
    Application::new()
        .with_assets(refineui::Assets)
        .run(|cx: &mut App| {
            refineui::init(cx);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            open(cx);
            cx.activate(true);
        });
}

#[derive(Clone)]
struct File {
    id: &'static str,
    name: &'static str,
    path: &'static str,
    lang: &'static str,
    text: &'static str,
}

const FILES: &[File] = &[
    File {
        id: "lib.rs",
        name: "lib.rs",
        path: "crates/refineui/src/lib.rs",
        lang: "Rust",
        text: "//! RefineUI for GPUI.\n\npub mod components;\npub mod shell;\n\n/// Initialise RefineUI.\npub fn init(cx: &mut gpui::App) {\n    Theme::init(cx);\n    root::bind_keys(cx);\n    components::init(cx);\n}\n",
    },
    File {
        id: "titlebar.rs",
        name: "titlebar.rs",
        path: "crates/refineui/src/shell/titlebar.rs",
        lang: "Rust",
        text: "//! Window titlebar and window controls.\n\n#[derive(IntoElement)]\npub struct Titlebar {\n    id: ElementId,\n    title: Option<SharedString>,\n    unified: bool,\n}\n\nimpl Titlebar {\n    /// New titlebar.\n    pub fn new(id: impl Into<ElementId>) -> Self {\n        Self { id: id.into(), title: None, unified: false }\n    }\n}\n",
    },
    File {
        id: "activity_bar.rs",
        name: "activity_bar.rs",
        path: "crates/refineui/src/shell/activity_bar.rs",
        lang: "Rust",
        text: "//! Activity Bar / Navigation Rail.\n\npub enum IndicatorStyle {\n    None,\n    Bar,\n    Dot,\n    Plate,\n}\n\nlet bar = ActivityBar::new(\"activity\")\n    .indicator(IndicatorStyle::Plate)\n    .items(items)\n    .selected(\"explorer\");\n",
    },
    File {
        id: "Cargo.toml",
        name: "Cargo.toml",
        path: "Cargo.toml",
        lang: "TOML",
        text: "[workspace]\nmembers = [\"crates/*\", \"examples/gallery\"]\nresolver = \"2\"\n\n[workspace.dependencies]\ngpui = \"0.2.2\"\n",
    },
    File {
        id: "README.md",
        name: "README.md",
        path: "README.md",
        lang: "Markdown",
        text: "# RefineUI for GPUI\n\nA desktop-native design system.\n\n```sh\ncargo run -p refineui-gallery --bin refineui-shell\n```\n",
    },
];

fn file(id: &str) -> &'static File {
    FILES.iter().find(|f| f.id == id).unwrap_or(&FILES[0])
}

/// The demo's root view.
pub struct ShellDemo {
    activity: SharedString,
    order: Vec<&'static str>,
    sidebar: bool,
    panel: bool,
    status: bool,
    tabs: Vec<&'static str>,
    active: usize,
    message: SharedString,
    zoom: i32,
    palette: Entity<CommandPalette>,
    editor_focus: FocusHandle,
    _state: gpui::Subscription,
}

impl ShellDemo {
    /// New demo view saving its window placement to `path` (call
    /// [`demo::install`] first so menus and hints find their shortcuts).
    pub fn new(path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let palette = cx.new(|cx| CommandPalette::new(demo::commands(), window, cx));
        let state = track_window_state(window, cx, path);
        Self {
            activity: "explorer".into(),
            order: vec!["explorer", "search", "git", "run", "extensions"],
            sidebar: true,
            panel: true,
            status: true,
            tabs: vec!["lib.rs", "titlebar.rs", "activity_bar.rs"],
            active: 1,
            message: "Ready".into(),
            zoom: 0,
            palette,
            editor_focus: cx.focus_handle(),
            _state: state,
        }
    }

    fn say(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.message = msg.into();
        cx.notify();
    }

    fn open_file(&mut self, id: &str, cx: &mut Context<Self>) {
        let f = file(id);
        match self.tabs.iter().position(|t| *t == f.id) {
            Some(ix) => self.active = ix,
            None => {
                self.tabs.push(f.id);
                self.active = self.tabs.len() - 1;
            }
        }
        cx.notify();
    }

    fn close_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.tabs.len() {
            self.tabs.remove(ix);
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len().saturating_sub(1);
            }
            cx.notify();
        }
    }

    fn activity_item(id: &str) -> ActivityItem {
        match id {
            "explorer" => ActivityItem::icon("explorer", "Explorer", IconName::Document)
                .action("demo::ToggleSidebar"),
            "search" => {
                ActivityItem::icon("search", "Search", IconName::Search).action("demo::Find")
            }
            "git" => ActivityItem::icon("git", "Source Control", IconName::Git).badge(3),
            "run" => {
                ActivityItem::icon("run", "Run and Debug", IconName::Bug).action("demo::Debug")
            }
            _ => ActivityItem::icon("extensions", "Extensions", IconName::Cube).dot(),
        }
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = cx.theme().colors;
        let header = |title: &str| {
            div()
                .flex_none()
                .h(px(36.))
                .px(px(spacing::MEDIUM))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    Text::new(title.to_uppercase())
                        .role(TextRole::CaptionMd)
                        .color(TextColor::Secondary),
                )
        };
        let body: gpui::AnyElement = match self.activity.as_ref() {
            "explorer" => {
                let this = cx.entity().downgrade();
                Tree::new(
                    "files",
                    vec![
                        TreeNode::branch(
                            "crates",
                            "crates",
                            vec![TreeNode::branch(
                                "refineui",
                                "refineui",
                                vec![TreeNode::branch(
                                    "src",
                                    "src",
                                    vec![
                                        TreeNode::leaf("lib.rs", "lib.rs").icon(IconName::Code),
                                        TreeNode::branch(
                                            "shell",
                                            "shell",
                                            vec![
                                                TreeNode::leaf("titlebar.rs", "titlebar.rs")
                                                    .icon(IconName::Code),
                                                TreeNode::leaf(
                                                    "activity_bar.rs",
                                                    "activity_bar.rs",
                                                )
                                                .icon(IconName::Code),
                                            ],
                                        ),
                                    ],
                                )],
                            )],
                        ),
                        TreeNode::leaf("Cargo.toml", "Cargo.toml").icon(IconName::Document),
                        TreeNode::leaf("README.md", "README.md").icon(IconName::Document),
                    ],
                )
                .size(ControlSize::Small)
                .default_expanded(["crates", "refineui", "src", "shell"])
                .on_select(move |id, _, cx| {
                    if FILES.iter().any(|f| f.id == id.as_ref()) {
                        let id = id.clone();
                        this.update(cx, |this, cx| this.open_file(&id, cx)).ok();
                    }
                })
                .into_any_element()
            }
            "git" => v_stack(4.)
                .px(px(spacing::MEDIUM))
                .child(Text::new("Changes").role(TextRole::LabelMd))
                .children(
                    ["titlebar.rs  M", "activity_bar.rs  M", "shell_demo.rs  A"].map(|t| {
                        Text::new(t)
                            .role(TextRole::BodySm)
                            .color(TextColor::Secondary)
                    }),
                )
                .into_any_element(),
            other => div()
                .px(px(spacing::MEDIUM))
                .child(
                    Text::new(format!("Nothing in {other} yet."))
                        .role(TextRole::BodySm)
                        .color(TextColor::Tertiary),
                )
                .into_any_element(),
        };
        let title = Self::activity_item(&self.activity).label().clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(c.foreground_primary)
            .child(header(&title))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px(px(spacing::X_SMALL))
                    .child(body),
            )
    }

    fn editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let hair = px(stroke::HAIRLINE);
        let mut strip = div()
            .id("tab-strip")
            .flex_none()
            .h(px(36.))
            .flex()
            .items_end()
            .bg(c.surface_sunken)
            .border_b(hair)
            .border_color(c.border_subtle)
            .overflow_x_scroll();
        for (ix, id) in self.tabs.iter().enumerate() {
            let f = file(id);
            let active = ix == self.active;
            strip = strip.child(
                div()
                    .id(("tab", ix))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(spacing::X_SMALL))
                    .pl(px(spacing::MEDIUM))
                    .pr(px(spacing::X_SMALL))
                    .border_r(hair)
                    .border_color(c.border_subtle)
                    .cursor_action()
                    .when(active, |d| {
                        d.bg(c.background_primary)
                            .border_t(px(stroke::THICK))
                            .border_color(c.border_focus)
                    })
                    .when(!active, |d| d.hover(|d| d.bg(c.background_surface_hover)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.active = ix;
                        cx.notify();
                    }))
                    .child(
                        Icon::new(IconName::Code)
                            .size(IconSize::XSmall)
                            .color(if active {
                                c.foreground_brand
                            } else {
                                c.foreground_tertiary
                            }),
                    )
                    .child(Text::new(f.name).role(TextRole::BodySm).color(if active {
                        TextColor::Primary
                    } else {
                        TextColor::Secondary
                    }))
                    .child(
                        Button::icon_only(("close-tab", ix), IconName::Dismiss)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Small)
                            .icon_size(IconSize::XSmall)
                            .tooltip("Close Tab")
                            .tooltip_action("demo::CloseTab")
                            .on_click(cx.listener(move |this, _, _, cx| this.close_tab(ix, cx))),
                    ),
            );
        }
        let content: gpui::AnyElement = match self.tabs.get(self.active) {
            Some(id) => {
                let f = file(id);
                let size = (13 + self.zoom).clamp(9, 24) as f32;
                let line_h = (size * 1.6).round();
                let lines: Vec<&str> = f.text.lines().collect();
                v_stack(0.)
                    .size_full()
                    .child(
                        div()
                            .flex_none()
                            .h(px(26.))
                            .px(px(spacing::MEDIUM))
                            .flex()
                            .items_center()
                            .child(
                                Text::new(f.path.replace('/', "  ›  "))
                                    .role(TextRole::CaptionMd)
                                    .color(TextColor::Tertiary),
                            ),
                    )
                    .child(
                        div()
                            .id("code")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .font_family(theme.mono_font_family.clone())
                            .text_size(px(size))
                            .line_height(px(line_h))
                            .children(lines.into_iter().enumerate().map(|(n, l)| {
                                div()
                                    .flex()
                                    .when(n == 6, |d| d.bg(c.background_surface_hover))
                                    .child(
                                        div()
                                            .flex_none()
                                            .w(px(48.))
                                            .pr(px(spacing::MEDIUM))
                                            .flex()
                                            .justify_end()
                                            .text_color(c.foreground_tertiary)
                                            .child(format!("{}", n + 1)),
                                    )
                                    .child(div().text_color(c.foreground_primary).child(
                                        if l.is_empty() {
                                            " ".to_string()
                                        } else {
                                            l.to_string()
                                        },
                                    ))
                            })),
                    )
                    .into_any_element()
            }
            None => v_stack(8.)
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::Board)
                        .size(IconSize::XLarge)
                        .color(c.foreground_disabled),
                )
                .child(
                    Text::new("No open editors")
                        .role(TextRole::SubtitleMd)
                        .color(TextColor::Secondary),
                )
                .child(
                    h_stack(8.)
                        .child(
                            Text::new("Show all commands")
                                .role(TextRole::BodySm)
                                .color(TextColor::Tertiary),
                        )
                        .child(ShortcutHint::action(
                            "command_palette::ToggleCommandPalette",
                        )),
                )
                .into_any_element(),
        };
        div()
            .key_context("Editor")
            .track_focus(&self.editor_focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(c.background_primary)
            .child(strip)
            .child(div().flex_1().min_h_0().child(content))
    }

    fn panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        v_stack(0.)
            .size_full()
            .bg(c.background_primary)
            .child(
                h_stack(16.)
                    .flex_none()
                    .h(px(32.))
                    .px(px(spacing::MEDIUM))
                    .child(
                        Text::new("PROBLEMS")
                            .role(TextRole::CaptionMd)
                            .color(TextColor::Tertiary),
                    )
                    .child(
                        Text::new("OUTPUT")
                            .role(TextRole::CaptionMd)
                            .color(TextColor::Tertiary),
                    )
                    .child(Text::new("TERMINAL").role(TextRole::CaptionMd))
                    .child(div().flex_1())
                    .child(
                        Button::icon_only("close-panel", IconName::Dismiss)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Small)
                            .tooltip("Close Panel")
                            .tooltip_action("demo::TogglePanel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.panel = false;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .px(px(spacing::MEDIUM))
                    .font_family(theme.mono_font_family.clone())
                    .text_size(px(role::BODY_SM.size))
                    .text_color(c.foreground_secondary)
                    .child("$ cargo run -p refineui-gallery --bin refineui-shell")
                    .child("   Compiling refineui v0.2.0")
                    .child("    Finished `dev` profile in 4.21s")
                    .child(
                        div()
                            .text_color(c.foreground_success)
                            .child("     Running `target/debug/refineui-shell`"),
                    ),
            )
    }
}

impl Focusable for ShellDemo {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.editor_focus.clone()
    }
}

impl Render for ShellDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let file_id = self.tabs.get(self.active).copied();
        let subtitle = file_id.map(|f| file(f).name).unwrap_or("Welcome");
        let lang = file_id.map(|f| file(f).lang).unwrap_or("Plain Text");

        let toolbar = Toolbar::new("shell-toolbar")
            .fill(true)
            .size(ControlSize::Small)
            .button(
                Button::icon_only("tb-sidebar", IconName::PanelLeft)
                    .variant(ButtonVariant::Ghost)
                    .selected(self.sidebar)
                    .tooltip("Toggle Sidebar")
                    .tooltip_action("demo::ToggleSidebar")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sidebar = !this.sidebar;
                        cx.notify();
                    })),
            )
            .button(
                Button::icon_only("tb-back", IconName::ChevronLeft)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Previous Tab")
                    .tooltip_action("demo::PreviousTab")
                    .on_click(|_, w, cx| w.dispatch_action(Box::new(PreviousTab), cx)),
            )
            .button(
                Button::icon_only("tb-forward", IconName::ChevronRight)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Next Tab")
                    .tooltip_action("demo::NextTab")
                    .on_click(|_, w, cx| w.dispatch_action(Box::new(NextTab), cx)),
            )
            .spacer()
            .child(search_pill(cx))
            .spacer()
            .button(
                Button::icon_only("tb-run", IconName::Play)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Run")
                    .tooltip_action("demo::Run")
                    .on_click(|_, w, cx| w.dispatch_action(Box::new(Run), cx)),
            )
            .button(
                Button::icon_only("tb-debug", IconName::Bug)
                    .variant(ButtonVariant::Ghost)
                    .tooltip("Start Debugging")
                    .tooltip_action("demo::Debug")
                    .on_click(|_, w, cx| w.dispatch_action(Box::new(Debug), cx)),
            )
            .button(
                Button::icon_only("tb-panel", IconName::Layer)
                    .variant(ButtonVariant::Ghost)
                    .selected(self.panel)
                    .tooltip("Toggle Panel")
                    .tooltip_action("demo::TogglePanel")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.panel = !this.panel;
                        cx.notify();
                    })),
            );

        let order = self.order.clone();
        let activity = ActivityBar::new("activity")
            .items(order.iter().map(|id| Self::activity_item(id)))
            .footer_item(ActivityItem::initials("account", "Pelagornis").tint(AvatarColor::Blue))
            .footer_item(
                ActivityItem::icon("settings", "Settings", IconName::Settings)
                    .action("demo::Preferences"),
            )
            .selected(if self.sidebar {
                self.activity.clone()
            } else {
                "".into()
            })
            .on_select(cx.listener(|this, id: &SharedString, _, cx| {
                if id.as_ref() == "settings" || id.as_ref() == "account" {
                    this.say(format!("{} — not part of the demo", id), cx);
                    return;
                }
                if this.activity == *id {
                    this.sidebar = !this.sidebar;
                } else {
                    this.activity = id.clone();
                    this.sidebar = true;
                }
                cx.notify();
            }))
            .on_reorder({
                let this = cx.entity().downgrade();
                move |from, to, _, cx| {
                    this.update(cx, |this, cx| {
                        if from < this.order.len() {
                            let item = this.order.remove(from);
                            let to = to.min(this.order.len());
                            this.order.insert(to, item);
                            cx.notify();
                        }
                    })
                    .ok();
                }
            });

        let density = theme.density;
        let status = StatusBar::new("status")
            .leading(
                StatusItem::button("branch", "main", |_, _| {})
                    .icon(IconName::Git)
                    .tooltip("Checkout Branch…", None),
            )
            .leading(
                StatusItem::text("problems", "0 errors · 2 warnings")
                    .icon(IconName::Warning)
                    .tone(StatusTone::Warning),
            )
            .leading(StatusItem::text("message", self.message.clone()))
            .trailing(StatusItem::text("cursor", "Ln 7, Col 5"))
            .trailing(StatusItem::text("indent", "Spaces: 4"))
            .trailing(StatusItem::text("encoding", "UTF-8"))
            .trailing(StatusItem::text("lang", lang))
            .trailing(
                StatusItem::button("density", format!("{density:?}"), |w, cx| {
                    w.dispatch_action(Box::new(CycleDensity), cx)
                })
                .icon(IconName::Grid)
                .tooltip("Cycle Density", Some("demo::CycleDensity")),
            )
            .trailing(
                StatusItem::button(
                    "appearance",
                    if theme.is_dark() { "Dark" } else { "Light" },
                    |w, cx| w.dispatch_action(Box::new(ToggleAppearance), cx),
                )
                .icon(if theme.is_dark() {
                    IconName::Moon
                } else {
                    IconName::WeatherSunny
                })
                .tooltip("Toggle Light / Dark", Some("demo::ToggleAppearance")),
            );

        let mut shell = ApplicationShell::new("shell")
            .spec(ShellSpec {
                status_bar: self.status,
                ..ShellSpec::default()
            })
            .title("refineui-gpui")
            .subtitle(subtitle)
            .menus(demo::menus())
            .toolbar(toolbar)
            .activity_bar(activity)
            .center(self.editor(cx))
            .status_bar(status)
            .overlay(self.palette.clone());
        if self.sidebar {
            shell = shell.left_dock(ShellDock::new(self.sidebar(cx), 248.));
        }
        if self.panel {
            shell = shell.bottom_dock(ShellDock::new(self.panel(cx), 180.));
        }
        let _ = window;

        refineui::root(cx)
            .key_context("Workspace")
            .on_action(cx.listener(|this, _: &ToggleCommandPalette, window, cx| {
                this.palette.update(cx, |p, cx| p.toggle(window, cx));
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.sidebar = !this.sidebar;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &TogglePanel, _, cx| {
                this.panel = !this.panel;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleStatusBar, _, cx| {
                this.status = !this.status;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &NewFile, _, cx| this.say("New file (demo)", cx)))
            .on_action(cx.listener(|this, _: &OpenFile, _, cx| this.say("Open… (demo)", cx)))
            .on_action(cx.listener(|this, _: &Save, _, cx| {
                let name = this.tabs.get(this.active).copied().unwrap_or("nothing");
                this.say(format!("Saved {name}"), cx)
            }))
            .on_action(cx.listener(|this, _: &SaveAll, _, cx| this.say("Saved all files", cx)))
            .on_action(cx.listener(|this, _: &CloseTab, _, cx| {
                let ix = this.active;
                this.close_tab(ix, cx)
            }))
            .on_action(cx.listener(|this, _: &NextTab, _, cx| {
                if !this.tabs.is_empty() {
                    this.active = (this.active + 1) % this.tabs.len();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &PreviousTab, _, cx| {
                if !this.tabs.is_empty() {
                    this.active = (this.active + this.tabs.len() - 1) % this.tabs.len();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.say("Undo", cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.say("Redo", cx)))
            .on_action(cx.listener(|this, _: &Cut, _, cx| this.say("Cut", cx)))
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.say("Copied", cx)))
            .on_action(cx.listener(|this, _: &Paste, _, cx| this.say("Pasted", cx)))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| this.say("Selected all", cx)))
            .on_action(cx.listener(|this, _: &Find, _, cx| {
                this.activity = "search".into();
                this.sidebar = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| {
                this.zoom += 1;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| {
                this.zoom -= 1;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ResetZoom, _, cx| {
                this.zoom = 0;
                cx.notify();
            }))
            .on_action(|_: &CycleDensity, _, cx| {
                let d = cx.theme().density;
                let ix = Density::ALL.iter().position(|x| *x == d).unwrap_or(0);
                Theme::set_density(Density::ALL[(ix + 1) % Density::ALL.len()], cx);
            })
            .on_action(|_: &ToggleAppearance, _, cx| Theme::toggle_appearance(cx))
            .on_action(cx.listener(|this, _: &Run, _, cx| this.say("Running refineui-shell…", cx)))
            .on_action(
                cx.listener(|this, _: &Debug, _, cx| this.say("Debugger attached (demo)", cx)),
            )
            .on_action(cx.listener(|this, _: &Preferences, _, cx| this.say("Settings (demo)", cx)))
            .on_action(cx.listener(|this, _: &About, _, cx| {
                this.say("RefineUI Shell — built from RefineUI shell parts", cx)
            }))
            .on_action(
                cx.listener(|this, _: &Docs, _, cx| this.say("Docs: docs/ (bun run dev)", cx)),
            )
            .on_action(|_: &Minimize, w, _| w.minimize_window())
            .on_action(|_: &ZoomWindow, w, _| w.zoom_window())
            .on_action(|_: &Hide, _, cx| cx.hide())
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(shell)
    }
}

fn search_pill(cx: &mut App) -> impl IntoElement {
    let c = cx.theme().colors;
    div()
        .id("search-pill")
        .w(px(360.))
        .h(px(26.))
        .px(px(spacing::SMALL))
        .flex()
        .items_center()
        .gap(px(spacing::X_SMALL))
        .rounded(px(6.))
        .bg(c.background_surface)
        .border(px(stroke::HAIRLINE))
        .border_color(c.border_subtle)
        .cursor_action()
        .hover(|d| {
            d.bg(c.background_surface_hover)
                .border_color(c.border_default)
        })
        .on_click(|_, w, cx| w.dispatch_action(Box::new(ToggleCommandPalette), cx))
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
        .child(ShortcutHint::action(
            "command_palette::ToggleCommandPalette",
        ))
}
