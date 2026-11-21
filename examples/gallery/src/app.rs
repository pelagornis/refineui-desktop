//! The gallery window.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{
    App, AppContext, Application, Bounds, Context, Entity, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowHandle, WindowKind, WindowOptions, div, prelude::*, px, size,
};
use refineui::prelude::*;
use refineui::shell::{WindowPreset, WindowSpec, traffic_light_point, window_options};
use refineui::{Appearance, Density, Theme};

use crate::stories::{Story, StoryGroup, all_stories};

/// Gallery root view — laid out like the RefineUI docs site: a navigation
/// sidebar with search, a top bar with the page breadcrumb and theme controls,
/// then the page title, lead paragraph and a bordered preview surface.
pub struct Gallery {
    stories: Vec<Story>,
    selected: SharedString,
    toaster: Entity<Toaster>,
    search: Entity<TextInput>,
    /// Open external preview windows, keyed by story id. A stale handle
    /// (window already closed) is dropped the next time the story is opened.
    external_windows: HashMap<&'static str, WindowHandle<ExternalStory>>,
    _subscription: gpui::Subscription,
}

impl Gallery {
    /// New gallery, optionally starting on a given story id.
    pub fn new(initial: Option<String>, cx: &mut Context<Self>) -> Self {
        let stories = all_stories();
        let selected = initial
            .filter(|id| stories.iter().any(|s| s.id == id))
            .unwrap_or_else(|| "button".into());
        let search = cx.new(|cx| {
            SearchField::new(cx)
                .size(ControlSize::Small)
                .placeholder("Search components")
        });
        let sub = cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify());
        Self {
            stories,
            selected: selected.into(),
            toaster: cx.new(|_| Toaster::new()),
            search,
            external_windows: HashMap::new(),
            _subscription: sub,
        }
    }

    fn story(&self) -> &Story {
        self.stories
            .iter()
            .find(|s| s.id == self.selected.as_ref())
            .unwrap_or(&self.stories[0])
    }

    fn story_window_is_open(&self, id: &str, cx: &mut Context<Self>) -> bool {
        self.external_windows
            .get(id)
            .is_some_and(|handle| handle.is_active(cx).is_some())
    }

    /// Focus an already-open external preview, or open a new one sized to
    /// the story's screenshot canvas plus comfortable padding.
    fn open_story_window(&mut self, id: &'static str, cx: &mut Context<Self>) {
        if let Some(handle) = self.external_windows.get(&id).copied() {
            if handle
                .update(cx, |_, window, _| {
                    window.activate_window();
                })
                .is_ok()
            {
                return;
            }
            self.external_windows.remove(id);
        }
        let Some(story) = self.stories.iter().find(|s| s.id == id).cloned() else {
            return;
        };
        let options = if story.fills_external_window() {
            filled_app_window_options(&story, cx)
        } else {
            let width = story.shot.0 + EXTERNAL_WINDOW_PAD;
            let height = story.shot.1 + EXTERNAL_WINDOW_PAD;
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(width.min(480.)), px(height.min(280.)))),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(story.name.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }
        };
        match cx.open_window(options, {
            let story = story.clone();
            move |window, cx| {
                window.activate_window();
                cx.new(|_| ExternalStory { story })
            }
        }) {
            Ok(handle) => {
                self.external_windows.insert(id, handle);
            }
            Err(err) => {
                eprintln!("failed to open {} window: {err}", story.name);
            }
        }
    }
}

/// Extra logical px added around [`Story::shot`] when opening an external
/// preview (24px of in-window padding plus a little breathing room).
const EXTERNAL_WINDOW_PAD: f32 = 48.;

/// Hosts a gallery story in its own window.
struct ExternalStory {
    story: Story,
}

impl Render for ExternalStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.story.fills_external_window() {
            // The app is the window: no padding, no host fill, no extra chrome.
            return refineui::root(cx)
                .size_full()
                .child(self.story.render_in_window(window, cx))
                .into_any_element();
        }
        let content = self.story.render_in_window(window, cx);
        let c = cx.theme().colors;
        refineui::root(cx)
            .size_full()
            .flex()
            .flex_col()
            .bg(c.background_primary)
            .child(
                div()
                    .id("external-story-scroll")
                    .flex_1()
                    .size_full()
                    .overflow_scroll()
                    .p(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(content),
            )
            .into_any_element()
    }
}

/// OS window options for a story that fills its window edge-to-edge.
fn filled_app_window_options(story: &Story, cx: &App) -> WindowOptions {
    match story.id {
        "preferences-window" => {
            window_options(&WindowSpec::new(WindowPreset::Settings, "Preferences"), cx)
        }
        "application-shell" => {
            window_options(&WindowSpec::new(WindowPreset::Main, "refineui-gpui"), cx)
        }
        _ => desktop_showcase_window_options(cx),
    }
}

/// OS window for the Desktop Showcase: `ApplicationShell` fills the window
/// edge-to-edge. The titlebar is transparent so the shell's own `Titlebar`
/// hosts the traffic lights. An opaque window background keeps the app
/// reading as one surface (a blurred backdrop would show the desktop
/// around or through the shell and make it look like a card).
fn desktop_showcase_window_options(cx: &App) -> WindowOptions {
    let density = cx.theme().density;
    let adapter = refineui::platform::current();
    let titlebar_h = adapter.titlebar_height(true, density);
    let (width, height) = crate::stories::DESKTOP_SHOWCASE_SIZE;
    let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: traffic_light_point(titlebar_h),
        }),
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: true,
        is_minimizable: true,
        window_background: WindowBackgroundAppearance::Opaque,
        window_min_size: Some(size(px(800.), px(500.))),
        window_decorations: Some(if adapter.client_decorations() {
            WindowDecorations::Client
        } else {
            WindowDecorations::Server
        }),
        ..Default::default()
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let story = self.story().clone();
        let query = self.search.read(cx).text().to_lowercase();

        let mut sidebar = Sidebar::new("gallery-nav")
            .width(px(260.))
            .header(
                v_stack(12.)
                    .w_full()
                    .child(
                        h_stack(8.)
                            .child(
                                div()
                                    .size(px(28.))
                                    .rounded(px(8.))
                                    .bg(c.background_brand)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new(IconName::Board)
                                            .size(IconSize::XSmall)
                                            .color(c.foreground_on_brand),
                                    ),
                            )
                            .child(
                                v_stack(0.)
                                    .child(Text::new("RefineUI").role(TextRole::SubtitleMd))
                                    .child(
                                        Text::new("for GPUI · desktop")
                                            .role(TextRole::CaptionMd)
                                            .color(TextColor::Tertiary),
                                    ),
                            ),
                    )
                    .child(self.search.clone()),
            )
            .selected(self.selected.clone())
            .on_select(cx.listener(|this, id: &SharedString, _, cx| {
                this.selected = id.clone();
                cx.notify();
            }));
        for group in StoryGroup::ALL {
            let items: Vec<_> = self
                .stories
                .iter()
                .filter(|s| s.group == group)
                .filter(|s| query.is_empty() || s.name.to_lowercase().contains(&query))
                .map(|s| SidebarItem::new(s.id, s.name))
                .collect();
            if !items.is_empty() {
                sidebar = sidebar.group(Some(group.label()), items);
            }
        }

        let dark = theme.is_dark();
        let toolbar = h_stack(8.)
            .child(
                SegmentedControl::new(
                    "appearance",
                    [
                        Segment::with_icon("Light", IconName::WeatherSunny),
                        Segment::with_icon("Dark", IconName::Moon),
                    ],
                    usize::from(dark),
                )
                .size(ControlSize::Small)
                .on_change(|ix, _, cx| {
                    Theme::set_appearance(
                        if *ix == 1 {
                            Appearance::Dark
                        } else {
                            Appearance::Light
                        },
                        cx,
                    )
                }),
            )
            .child(
                Select::new(
                    "density",
                    ["Compact", "Comfortable", "Spacious"],
                    Density::ALL.iter().position(|d| *d == theme.density),
                )
                .size(ControlSize::Small)
                .appearance(SelectAppearance::Ghost)
                .width(px(120.))
                .on_change(|ix, _, cx| {
                    Theme::set_density(Density::ALL[(*ix).min(Density::ALL.len() - 1)], cx)
                }),
            )
            .child(
                Button::new("toast-demo", "Show toast")
                    .icon(IconName::Alert)
                    .variant(ButtonVariant::Outline)
                    .size(ControlSize::Small)
                    .on_click({
                        let toaster = self.toaster.clone();
                        move |_, _, cx| {
                            toaster.update(cx, |t, cx| {
                                t.push(
                                    ToastData::new("Saved to gallery")
                                        .variant(ToastVariant::Success)
                                        .duration(Some(Duration::from_secs(4))),
                                    cx,
                                );
                            })
                        }
                    }),
            );

        let topbar = h_stack(16.)
            .h(px(56.))
            .flex_none()
            .px(px(32.))
            .border_b(px(0.5))
            .border_color(c.border_default)
            .bg(c.background_primary)
            .child(div().flex_1().child(Breadcrumb::new(
                "page-crumb",
                ["Components", story.group.label(), story.name],
            )))
            .child(toolbar);

        let showcase = if story.opens_in_window() {
            let id = story.id;
            let already_open = self.story_window_is_open(id, cx);
            let label = if already_open {
                format!("Show {}", story.name)
            } else {
                format!("Open {}", story.name)
            };
            ResultView::new(ResultVariant::Empty)
                .icon(IconName::SquareMultiple)
                .title(story.name)
                .description(format!(
                    "{} × {} — this preview is larger than the gallery window.",
                    story.shot.0 as u32, story.shot.1 as u32
                ))
                .action(
                    Button::new(SharedString::from(format!("open-story-{id}")), label)
                        .icon(IconName::Open)
                        .variant(ButtonVariant::Primary)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_story_window(id, cx);
                        })),
                )
                .into_any_element()
        } else {
            (story.render)(window, cx)
        };
        let page = v_stack(0.)
            .w_full()
            .max_w(px(960.))
            .child(
                v_stack(8.)
                    .pb(px(24.))
                    .child(
                        Text::new(story.group.label())
                            .role(TextRole::LabelMd)
                            .color(TextColor::Brand),
                    )
                    .child(Text::new(story.name).role(TextRole::TitleMd))
                    .child(
                        Text::new(story.summary)
                            .role(TextRole::BodyLg)
                            .color(TextColor::Secondary),
                    ),
            )
            .child(
                v_stack(10.)
                    .child(Text::new("Preview").role(TextRole::SubtitleMd))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .min_h(px(240.))
                            .p(px(40.))
                            .rounded(px(16.))
                            .border_1()
                            .border_color(c.border_default)
                            .bg(c.background_primary)
                            .child(showcase),
                    ),
            );
        let body = div()
            .id(SharedString::from(format!("story-scroll-{}", story.id)))
            .flex_1()
            .overflow_y_scroll()
            .px(px(48.))
            .py(px(40.))
            .flex()
            .flex_col()
            .items_center()
            .child(page);

        refineui::root(cx)
            .flex()
            .flex_row()
            .child(sidebar)
            .child(
                v_stack(0.)
                    .flex_1()
                    .h_full()
                    .bg(c.background_primary)
                    .child(topbar)
                    .child(body),
            )
            .child(self.toaster.clone())
    }
}

/// Parse `--story <id>` and `--dark` from argv.
fn parse_args() -> (Option<String>, bool) {
    let mut story = None;
    let mut dark = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--story" => story = args.next(),
            "--dark" => dark = true,
            _ => {}
        }
    }
    (story, dark)
}

/// Run the gallery application.
pub fn run() {
    let (story, dark) = parse_args();
    Application::new()
        .with_assets(refineui::Assets)
        .run(move |cx: &mut App| {
            refineui::init(cx);
            if dark {
                Theme::set_appearance(Appearance::Dark, cx);
            }
            let showcase = story.as_deref().and_then(|id| {
                all_stories()
                    .into_iter()
                    .find(|s| s.id == id && s.fills_external_window())
            });
            if let Some(s) = showcase {
                cx.open_window(filled_app_window_options(&s, cx), move |window, cx| {
                    window.activate_window();
                    cx.new(|_| ExternalStory { story: s })
                })
                .expect("failed to open story window");
            } else {
                let bounds = Bounds::centered(None, size(px(1200.), px(820.)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(800.), px(520.))),
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some("RefineUI Gallery".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        window.activate_window();
                        cx.new(|cx| Gallery::new(story, cx))
                    },
                )
                .expect("failed to open gallery window");
            }
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}
