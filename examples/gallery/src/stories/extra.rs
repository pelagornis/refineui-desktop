//! Stories for the components added with RefineUI web 0.5.0 (Toolbar, Button
//! Group, Command, Date / Time Input, conversation primitives, Chart,
//! Carousel, Navigation Menu, Footer, layout primitives…).

use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, px,
};
use refineui::prelude::*;
use refineui::{Axis, FrostMode, frost_mode, set_frost_mode};

fn eid(s: String) -> gpui::ElementId {
    gpui::ElementId::Name(s.into())
}

use super::{Story, StoryGroup, row, section, setter, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "button-group",
            name: "Button Group",
            group: StoryGroup::Actions,
            summary: "Related buttons joined in one tube or spaced apart.",
            shot: (640., 400.),
            render: button_group,
        },
        Story {
            id: "toolbar",
            name: "Toolbar",
            group: StoryGroup::Actions,
            summary: "A row of ghost buttons and toggles with groups and dividers.",
            shot: (640., 220.),
            render: toolbar,
        },
        Story {
            id: "date-input",
            name: "Date Input",
            group: StoryGroup::Forms,
            summary: "Field that opens a Calendar popover (single date or range).",
            shot: (760., 160.),
            render: date_input,
        },
        Story {
            id: "time-input",
            name: "Time Input",
            group: StoryGroup::Forms,
            summary: "Field that opens hour / minute wheels.",
            shot: (640., 160.),
            render: time_input,
        },
        Story {
            id: "label",
            name: "Label",
            group: StoryGroup::Forms,
            summary: "Form label in three sizes with a required marker.",
            shot: (480., 120.),
            render: label,
        },
        Story {
            id: "tag",
            name: "Tag",
            group: StoryGroup::Display,
            summary: "Tag (an alias of Chip, like the web).",
            shot: (560., 120.),
            render: tag,
        },
        Story {
            id: "result",
            name: "Result",
            group: StoryGroup::Display,
            summary: "Outcome screen: icon, title, description and actions.",
            shot: (800., 300.),
            render: result,
        },
        Story {
            id: "message",
            name: "Message",
            group: StoryGroup::Display,
            summary: "Conversation row: avatar, bubbles and a footer.",
            shot: (640., 360.),
            render: message,
        },
        Story {
            id: "bubble",
            name: "Bubble",
            group: StoryGroup::Display,
            summary: "Chat bubbles in seven variants with grouping and reactions.",
            shot: (640., 360.),
            render: bubble,
        },
        Story {
            id: "marker",
            name: "Marker",
            group: StoryGroup::Display,
            summary: "Timeline marker (inline or divider) with an optional live pulse.",
            shot: (560., 160.),
            render: marker,
        },
        Story {
            id: "code-block",
            name: "Code Block",
            group: StoryGroup::Display,
            summary: "Syntax-highlighted code (Night Owl) with title, line numbers, highlights and copy.",
            shot: (640., 300.),
            render: code_block,
        },
        Story {
            id: "chart",
            name: "Chart",
            group: StoryGroup::Display,
            summary: "Bar and monotone line charts with axis, grid and hover tooltip.",
            shot: (720., 780.),
            render: chart,
        },
        Story {
            id: "carousel",
            name: "Carousel",
            group: StoryGroup::Optional,
            summary: "Looping slides with outline buttons and a sliding-pill meter.",
            shot: (640., 360.),
            render: carousel,
        },
        Story {
            id: "navigation-menu",
            name: "Navigation Menu",
            group: StoryGroup::Optional,
            summary: "Top navigation with links and dropdown panels.",
            shot: (640., 120.),
            render: navigation_menu,
        },
        Story {
            id: "navigation-menu-mega",
            name: "Navigation Menu · Mega",
            group: StoryGroup::Optional,
            summary: "Floating mega panel centred under the bar (layout = floating).",
            shot: (900., 380.),
            render: navigation_menu_mega,
        },
        Story {
            id: "navigation-menu-overlay",
            name: "Navigation Menu · Overlay",
            group: StoryGroup::Optional,
            summary: "Full-width sheet from the bar's edge over a scrim (layout = overlay).",
            shot: (900., 380.),
            render: navigation_menu_overlay,
        },
        Story {
            id: "footer",
            name: "Footer",
            group: StoryGroup::Optional,
            summary: "Sitemap groups and a social | brand | locale meta row.",
            shot: (800., 320.),
            render: footer,
        },
        Story {
            id: "command",
            name: "Command",
            group: StoryGroup::Overlays,
            summary: "Searchable command list with groups, shortcuts and keyboard navigation.",
            shot: (520., 420.),
            render: command,
        },
        Story {
            id: "dropdown",
            name: "Dropdown",
            group: StoryGroup::Overlays,
            summary: "Button that opens a Menu; optional native macOS blur.",
            shot: (560., 320.),
            render: dropdown,
        },
        Story {
            id: "layout",
            name: "Layout",
            group: StoryGroup::Foundations,
            summary: "Stack, Grid, Container, Box and Spacer presets.",
            shot: (720., 360.),
            render: layout,
        },
    ]
}

fn button_group(_w: &mut Window, cx: &mut App) -> AnyElement {
    let buttons = |p: &'static str| {
        [
            Button::new(eid(format!("{p}-l")), "Left"),
            Button::new(eid(format!("{p}-c")), "Center"),
            Button::new(eid(format!("{p}-r")), "Right"),
        ]
    };
    v_stack(16.)
        .child(section(
            "Attached",
            row()
                .gap(px(16.))
                .child(
                    ButtonGroup::new()
                        .buttons(buttons("a"))
                        .size(ControlSize::Small),
                )
                .child(ButtonGroup::new().buttons(buttons("b")))
                .child(
                    ButtonGroup::new()
                        .buttons(buttons("c"))
                        .size(ControlSize::Large),
                ),
            cx,
        ))
        .child(section(
            "Separate · outline",
            row().child(
                ButtonGroup::new()
                    .buttons(buttons("d"))
                    .attached(false)
                    .variant(ButtonVariant::Outline),
            ),
            cx,
        ))
        .child(section(
            "Vertical",
            row().child(
                ButtonGroup::new()
                    .buttons(buttons("e"))
                    .orientation(Axis::Vertical),
            ),
            cx,
        ))
        .into_any_element()
}

fn formatting_toolbar(
    id: &'static str,
    attached: bool,
    bold: &gpui::Entity<bool>,
    italic: &gpui::Entity<bool>,
    cx: &App,
) -> Toolbar {
    Toolbar::new(id)
        .label("Formatting")
        .attached(attached)
        .group(
            ToolbarGroup::new()
                .toggle(
                    ToggleButton::icon_only(eid(format!("{id}-b")), IconName::Edit, *bold.read(cx))
                        .on_change(setter(bold)),
                )
                .toggle(
                    ToggleButton::icon_only(
                        eid(format!("{id}-i")),
                        IconName::Textbox,
                        *italic.read(cx),
                    )
                    .on_change(setter(italic)),
                ),
        )
        .divider()
        .button(Button::icon_only(eid(format!("{id}-copy")), IconName::Copy).tooltip("Copy"))
        .button(Button::icon_only(eid(format!("{id}-link")), IconName::Link).tooltip("Link"))
        .divider()
        .button(Button::new(eid(format!("{id}-share")), "Share").icon(IconName::Open))
}

fn toolbar(w: &mut Window, cx: &mut App) -> AnyElement {
    let bold = state("tb-bold", w, cx, || true);
    let italic = state("tb-italic", w, cx, || false);
    v_stack(16.)
        .child(section(
            "Attached (default)",
            formatting_toolbar("tb-a", true, &bold, &italic, cx),
            cx,
        ))
        .child(section(
            "Plain",
            formatting_toolbar("tb-p", false, &bold, &italic, cx),
            cx,
        ))
        .into_any_element()
}

fn date_input(w: &mut Window, cx: &mut App) -> AnyElement {
    let d = state("di-date", w, cx, || Some(Date::new(2026, 11, 10)));
    let r = state("di-range", w, cx, || {
        (Some(Date::new(2026, 11, 3)), Some(Date::new(2026, 11, 12)))
    });
    let d2 = d.clone();
    let r2 = r.clone();
    let (rs, re) = *r.read(cx);
    row()
        .items_start()
        .gap(px(16.))
        .child(
            div().w(px(220.)).child(
                DateInput::new("di", *d.read(cx))
                    .full_width(true)
                    .on_change(move |v, _, cx| {
                        d2.update(cx, |s, cx| {
                            *s = Some(*v);
                            cx.notify();
                        })
                    }),
            ),
        )
        .child(
            div().w(px(280.)).child(
                DateInput::range("di-r", rs, re)
                    .full_width(true)
                    .on_range_change(move |a, b, _, cx| {
                        r2.update(cx, |s, cx| {
                            *s = (a, b);
                            cx.notify();
                        })
                    }),
            ),
        )
        .child(
            div()
                .w(px(160.))
                .child(DateInput::new("di-e", None).full_width(true).error(true)),
        )
        .into_any_element()
}

fn time_input(w: &mut Window, cx: &mut App) -> AnyElement {
    let t = state("ti-time", w, cx, || Some(Time::new(9, 30)));
    let t2 = t.clone();
    row()
        .items_start()
        .gap(px(16.))
        .child(
            div().w(px(180.)).child(
                TimeInput::new("ti", *t.read(cx))
                    .full_width(true)
                    .on_change(move |v, _, cx| {
                        t2.update(cx, |s, cx| {
                            *s = Some(*v);
                            cx.notify();
                        })
                    }),
            ),
        )
        .child(
            div().w(px(180.)).child(
                TimeInput::new("ti-12", Some(Time::new(14, 15)))
                    .twelve_hour(true)
                    .minute_step(15)
                    .full_width(true),
            ),
        )
        .child(
            div()
                .w(px(180.))
                .child(TimeInput::new("ti-d", None).disabled(true).full_width(true)),
        )
        .into_any_element()
}

fn label(_w: &mut Window, _cx: &mut App) -> AnyElement {
    row()
        .gap(px(24.))
        .child(Label::new("Small").size(ControlSize::Small))
        .child(Label::new("Medium").required(true))
        .child(Label::new("Large").size(ControlSize::Large))
        .into_any_element()
}

fn tag(_w: &mut Window, _cx: &mut App) -> AnyElement {
    row()
        .child(Tag::tag("t1", "Design"))
        .child(Tag::tag("t2", "Rust").variant(TagVariant::Outline))
        .child(
            Tag::tag("t3", "GPUI")
                .variant(TagVariant::Filled)
                .icon(IconName::Code),
        )
        .child(Tag::tag("t4", "Small").size(ControlSize::Small))
        .into_any_element()
}

fn result(_w: &mut Window, _cx: &mut App) -> AnyElement {
    row()
        .items_start()
        .gap(px(16.))
        .child(
            div().w(px(340.)).child(
                ResultView::new(ResultVariant::Success)
                    .title("Payment received")
                    .description("Your receipt was sent to your email.")
                    .action(Button::new("r-done", "Done").variant(ButtonVariant::Primary))
                    .action(Button::new("r-view", "View receipt")),
            ),
        )
        .child(
            div().w(px(340.)).child(
                ResultView::new(ResultVariant::Error)
                    .title("Something went wrong")
                    .description("Check your connection and try again.")
                    .action(Button::new("r-retry", "Try again")),
            ),
        )
        .into_any_element()
}

fn message(_w: &mut Window, _cx: &mut App) -> AnyElement {
    v_stack(16.)
        .w(px(560.))
        .child(
            Message::new()
                .avatar(Avatar::new("Ada Lovelace"))
                .child(BubbleGroup::new([
                    Bubble::new("Did the new tokens land?"),
                    Bubble::new("The toolbar looks great in dark mode."),
                ]))
                .footer("Ada · 9:41"),
        )
        .child(
            Message::new()
                .align(BubbleAlign::End)
                .child(BubbleGroup::new([
                    Bubble::new("Yes — shipped in 0.5.0.")
                        .variant(BubbleVariant::Tinted)
                        .align(BubbleAlign::End),
                    Bubble::new("Screenshots are in the docs.")
                        .variant(BubbleVariant::Tinted)
                        .align(BubbleAlign::End)
                        .reactions(["👍", "🎉"]),
                ]))
                .footer("Delivered"),
        )
        .into_any_element()
}

fn bubble(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .w(px(592.))
        .child(section(
            "Variants",
            // Bubbles fill their row (`w_full`), so give each a cell width.
            row().children(
                BubbleVariant::ALL
                    .into_iter()
                    .map(|v| div().w(px(140.)).child(Bubble::new(v.label()).variant(v))),
            ),
            cx,
        ))
        .child(section(
            "Group & reactions",
            row()
                .items_start()
                .gap(px(24.))
                .child(div().w(px(284.)).child(BubbleGroup::new([
                    Bubble::new("First"),
                    Bubble::new("Second in a run"),
                    Bubble::new("Last").reactions(["❤️"]),
                ])))
                .child(
                    div().w(px(284.)).child(BubbleGroup::new([
                        Bubble::new("Sent")
                            .variant(BubbleVariant::Secondary)
                            .align(BubbleAlign::End),
                        Bubble::new("And another")
                            .variant(BubbleVariant::Secondary)
                            .align(BubbleAlign::End),
                    ])),
                ),
            cx,
        ))
        .into_any_element()
}

fn marker(_w: &mut Window, _cx: &mut App) -> AnyElement {
    v_stack(16.)
        .w(px(480.))
        .child(
            row()
                .gap(px(16.))
                .child(Marker::new("m1", "Edited").icon(IconName::Edit))
                .child(Marker::new("m2", "Live").status(true)),
        )
        .child(Marker::new("m3", "Today").variant(MarkerVariant::Divider))
        .into_any_element()
}

fn code_block(_w: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .w(px(560.))
        .child(
            CodeBlock::new(
                "cb",
                "use refineui::prelude::*;\n\nfn main() {\n    Button::new(\"save\", \"Save\")\n        .variant(ButtonVariant::Primary);\n}",
            )
            .title("main.rs")
            .icon(IconName::Code)
            .language("rust")
            .line_numbers(true)
            .highlight([4, 5]),
        )
        .into_any_element()
}

fn chart(_w: &mut Window, _cx: &mut App) -> AnyElement {
    let labels = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
    v_stack(16.)
        .w(px(640.))
        .child(
            Card::new()
                .title("Revenue")
                .description("Last 6 months")
                .child(Chart::bars(
                    "chart-bars",
                    [186., 305., 237., 273., 209., 214.],
                    labels,
                )),
        )
        .child(
            Card::new()
                .title("Visitors")
                .description("Last 6 months")
                .child(
                    Chart::line("chart-line", [86., 125., 97., 173., 149., 194.], labels)
                        .series(ChartSeries::Info),
                ),
        )
        .into_any_element()
}

fn carousel(_w: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .w(px(560.))
        .child(
            Carousel::new("carousel", 4, |ix, _, cx| {
                let c = cx.theme().colors;
                let fills = [
                    c.background_brand_subtle,
                    c.background_info_subtle,
                    c.background_success_subtle,
                    c.background_warning_subtle,
                ];
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(fills[ix % 4])
                    .child(Text::new(format!("Slide {}", ix + 1)).role(TextRole::TitleMd))
                    .into_any_element()
            })
            .height(220.),
        )
        .into_any_element()
}

fn navigation_menu(_w: &mut Window, _cx: &mut App) -> AnyElement {
    NavigationMenu::new("nav")
        .link("Overview", true, |_, _| {})
        .menu(
            "Products",
            [
                NavigationLink::new("Design system")
                    .description("Tokens and components")
                    .icon(IconName::Board),
                NavigationLink::new("Icons")
                    .description("RefineUI System Icons")
                    .icon(IconName::Star),
                NavigationLink::new("Desktop")
                    .description("GPUI for macOS apps")
                    .icon(IconName::Code),
            ],
        )
        .menu(
            "Resources",
            [
                NavigationLink::new("Docs"),
                NavigationLink::new("Changelog"),
            ],
        )
        .link("Pricing", false, |_, _| {})
        .disabled_link("Careers")
        .into_any_element()
}

fn mega_column(
    title: &'static str,
    links: [(&'static str, &'static str, IconName); 3],
    cx: &App,
) -> Div {
    let c = cx.theme().colors;
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .w(px(260.))
        .child(
            div()
                .px(px(12.))
                .py(px(6.))
                .text_size(px(12.))
                .text_color(c.foreground_tertiary)
                .child(title),
        )
        .children(links.into_iter().map(|(label, desc, icon)| {
            div()
                .flex()
                .gap(px(12.))
                .items_start()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .hover(move |s| s.bg(c.background_primary_hover))
                .child(Icon::new(icon).color(c.foreground_secondary))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.))
                                .text_color(c.foreground_primary)
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(c.foreground_tertiary)
                                .child(desc),
                        ),
                )
        }))
}

fn mega_content(_w: &mut Window, cx: &mut App) -> AnyElement {
    div()
        .flex()
        .gap(px(8.))
        .p(px(8.))
        .child(mega_column(
            "Build",
            [
                ("Components", "Accessible building blocks", IconName::Board),
                ("Tokens", "Colour, type and motion", IconName::Star),
                ("Icons", "RefineUI System Icons", IconName::Heart),
            ],
            cx,
        ))
        .child(mega_column(
            "Ship",
            [
                ("Desktop", "GPUI for macOS apps", IconName::Code),
                ("Web", "React 19 + Tailwind", IconName::Link),
                ("Docs", "Guides and references", IconName::Mail),
            ],
            cx,
        ))
        .child(mega_column(
            "Learn",
            [
                ("Changelog", "What shipped lately", IconName::Calendar),
                ("Examples", "Full app templates", IconName::Board),
                ("Community", "Discussions and help", IconName::Heart),
            ],
            cx,
        ))
        .into_any_element()
}

fn nav_layout(id: &'static str, layout: NavigationLayout) -> AnyElement {
    div()
        .w(px(900.))
        .h(px(380.))
        .child(
            NavigationMenu::new(id)
                .link("Overview", true, |_, _| {})
                .panel("Products", layout, mega_content)
                .menu(
                    "Resources",
                    [
                        NavigationLink::new("Docs"),
                        NavigationLink::new("Changelog"),
                    ],
                )
                .link("Pricing", false, |_, _| {})
                .default_open(1),
        )
        .into_any_element()
}

fn navigation_menu_mega(_w: &mut Window, _cx: &mut App) -> AnyElement {
    nav_layout("nav-mega", NavigationLayout::Floating)
}

fn navigation_menu_overlay(_w: &mut Window, _cx: &mut App) -> AnyElement {
    nav_layout("nav-overlay", NavigationLayout::Overlay)
}

fn footer(_w: &mut Window, _cx: &mut App) -> AnyElement {
    div()
        .w(px(760.))
        .child(
            Footer::new("footer")
                .group(FooterGroup::new(
                    "Product",
                    ["Components", "Tokens", "Icons"],
                ))
                .group(FooterGroup::new(
                    "Resources",
                    ["Docs", "Changelog", "GitHub"],
                ))
                .group(FooterGroup::new("Company", ["About", "Blog", "Contact"]))
                .brand(None, "RefineUI", Some("© 2026 Pelagornis".into()))
                .social([IconName::Mail, IconName::Link, IconName::Heart])
                .end(
                    Button::new("locale", "English")
                        .variant(ButtonVariant::Outline)
                        .size(ControlSize::Small)
                        .icon_end(IconName::ChevronDown),
                ),
        )
        .into_any_element()
}

fn command(w: &mut Window, cx: &mut App) -> AnyElement {
    let cmd = w.use_keyed_state("command-story", cx, |window, cx| {
        Command::new(window, cx)
            .group(
                CommandGroup::new("Suggestions")
                    .item(CommandItem::new("Calendar").icon(IconName::Calendar))
                    .item(
                        CommandItem::new("Search files")
                            .icon(IconName::Search)
                            .shortcut("⌘P"),
                    )
                    .item(CommandItem::new("Mail").icon(IconName::Mail).disabled(true)),
            )
            .group(
                CommandGroup::new("Settings")
                    .item(
                        CommandItem::new("Profile")
                            .icon(IconName::Person)
                            .shortcut("⌘,"),
                    )
                    .item(
                        CommandItem::new("Appearance")
                            .icon(IconName::Moon)
                            .keywords(["theme", "dark"]),
                    )
                    .item(CommandItem::new("Keyboard shortcuts").icon(IconName::Keyboard)),
            )
    });
    div().w(px(440.)).child(cmd).into_any_element()
}

fn dropdown_items(menu: Menu) -> Menu {
    menu.label("File")
        .item(MenuItem::new("New").icon(IconName::Add).shortcut("⌘N"))
        .item(MenuItem::new("Open…").icon(IconName::Folder).shortcut("⌘O"))
        .divider()
        .item(MenuItem::new("Delete").icon(IconName::Delete).danger(true))
}

fn dropdown(_w: &mut Window, cx: &mut App) -> AnyElement {
    let native = frost_mode(cx) == FrostMode::Native;
    v_stack(16.)
        .child(section(
            "Dropdown",
            row()
                .gap(px(16.))
                .child(DropdownMenu::new(
                    "dd-a",
                    Button::new("dd-a-btn", "Options")
                        .icon_end(IconName::ChevronDown)
                        .variant(ButtonVariant::Outline),
                    dropdown_items(Menu::new("dd-a-menu")),
                ))
                .child(
                    DropdownMenu::new(
                        "dd-b",
                        Button::icon_only("dd-b-btn", IconName::MoreHorizontal)
                            .variant(ButtonVariant::Ghost),
                        dropdown_items(Menu::new("dd-b-menu")),
                    )
                    .align_end(),
                ),
            cx,
        ))
        .child(section(
            "Backdrop (prototype)",
            Switch::new("frost", native)
                .label(if refineui::vibrancy::native_frost_supported() {
                    "Native blur (pop-up window)"
                } else {
                    "Native blur — macOS only (opaque here)"
                })
                .on_change(|on, _, cx| {
                    set_frost_mode(
                        if *on {
                            FrostMode::Native
                        } else {
                            FrostMode::Opaque
                        },
                        cx,
                    )
                }),
            cx,
        ))
        .into_any_element()
}

fn layout(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    let tile = move |t: &'static str| {
        token_box(
            10.,
            Some(c.background_brand_subtle),
            8.,
            Some((1., c.border_default)),
        )
        .child(Text::new(t).role(TextRole::CaptionMd))
    };
    v_stack(16.)
        .w(px(640.))
        .child(section(
            "Stack (row / column)",
            h_stack(8.)
                .child(tile("A"))
                .child(tile("B"))
                .child(spacer())
                .child(tile("End")),
            cx,
        ))
        .child(section(
            "Grid · 3 columns",
            grid(3, 8.)
                .child(tile("1"))
                .child(tile("2"))
                .child(tile("3"))
                .child(tile("4").col_span(2))
                .child(tile("5")),
            cx,
        ))
        .child(section(
            "Container (24px inline padding) · Box",
            container().child(
                token_box(
                    16.,
                    Some(c.background_surface),
                    12.,
                    Some((1., c.border_subtle)),
                )
                .child(Text::new("Box with padding, background, radius and border")),
            ),
            cx,
        ))
        .into_any_element()
}
