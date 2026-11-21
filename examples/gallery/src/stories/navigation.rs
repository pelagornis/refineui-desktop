use gpui::{AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use refineui::prelude::*;

use super::{Story, StoryGroup, section, setter, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "tabs",
            name: "Tabs",
            group: StoryGroup::Navigation,
            summary: "Switch between views in the same context.",
            shot: (560., 120.),
            render: tabs,
        },
        Story {
            id: "breadcrumb",
            name: "Breadcrumb",
            group: StoryGroup::Navigation,
            summary: "Location within a hierarchy.",
            shot: (560., 90.),
            render: breadcrumb,
        },
        Story {
            id: "pagination",
            name: "Pagination",
            group: StoryGroup::Navigation,
            summary: "Move between pages of results.",
            shot: (560., 140.),
            render: pagination,
        },
        Story {
            id: "sidebar",
            name: "Sidebar",
            group: StoryGroup::Navigation,
            summary: "App navigation landmark with trigger, resize handle and peek.",
            shot: (744., 498.),
            render: sidebar,
        },
        Story {
            id: "tree",
            name: "Tree",
            group: StoryGroup::Navigation,
            summary: "Expandable hierarchical list.",
            shot: (560., 300.),
            render: tree,
        },
        Story {
            id: "accordion",
            name: "Accordion",
            group: StoryGroup::Navigation,
            summary: "Stacked disclosure sections with animated height.",
            shot: (560., 340.),
            render: accordion,
        },
        Story {
            id: "collapsible",
            name: "Collapsible",
            group: StoryGroup::Navigation,
            summary: "Headless show / hide region with the accordion motion.",
            shot: (520., 220.),
            render: collapsible,
        },
        Story {
            id: "calendar",
            name: "Calendar",
            group: StoryGroup::Navigation,
            summary: "Month grid date picker.",
            shot: (340., 360.),
            render: calendar,
        },
    ]
}

fn tabs(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    let panel = |t: &'static str| {
        div()
            .p(px(16.))
            .rounded(px(12.))
            .bg(c.background_surface)
            .child(Text::new(t).color(TextColor::Secondary))
    };
    // Web usage: value-keyed triggers + content panels, uncontrolled here.
    Tabs::new("tabs")
        .default_value("overview")
        .trigger(TabsTrigger::new("overview", "Overview").icon(IconName::Home))
        .trigger(TabsTrigger::new("issues", "Issues").count(12))
        .trigger(TabsTrigger::new("pulls", "Pull requests").count(3))
        .trigger(TabsTrigger::new("archived", "Archived").disabled(true))
        .content(TabsContent::new(
            "overview",
            panel("Project overview and recent activity."),
        ))
        .content(TabsContent::new("issues", panel("12 open issues.")))
        .content(TabsContent::new(
            "pulls",
            panel("3 pull requests waiting for review."),
        ))
        .into_any_element()
}

fn breadcrumb(_w: &mut Window, cx: &mut App) -> AnyElement {
    let _ = cx;
    Breadcrumb::new("bc", ["Workspace", "refineui-gpui", "crates", "button.rs"])
        .on_select(|_, _, _| {})
        .into_any_element()
}

fn pagination(w: &mut Window, cx: &mut App) -> AnyElement {
    let page = state("page", w, cx, || 4usize);
    v_stack(12.)
        .child(Pagination::new("pg", *page.read(cx), 20).on_change(setter(&page)))
        .child(Pagination::new("pg-s", 1, 5).size(ControlSize::Small))
        .into_any_element()
}

/// Web `SidebarPreview`.
fn sidebar_rail(id: &'static str, sel: &gpui::Entity<SharedString>, cx: &App) -> Sidebar {
    Sidebar::new(id)
        .fill()
        .brand("RefineUI")
        .group(
            Some("Product"),
            [
                SidebarItem::new("overview", "Overview").icon(IconName::Search),
                SidebarItem::new("components", "Components").icon(IconName::Folder),
                SidebarItem::new("tokens", "Tokens").icon(IconName::Calendar),
            ],
        )
        .group(
            Some("Account"),
            [SidebarItem::new("settings", "Settings").icon(IconName::Settings)],
        )
        .footer_item(SidebarItem::new("account", "Pelagornis").icon(IconName::Person))
        .selected(sel.read(cx).clone())
        .on_select(setter(sel))
}

/// Web `SidebarPreview`: a 450px app frame. Sidebar 32% (18–48%) in a
/// resizable group; the trigger collapses it to 0 over `duration-panel`
/// (panel easing) with the rail frozen; collapsed, the left edge peeks.
fn sidebar(w: &mut Window, cx: &mut App) -> AnyElement {
    const FRAME_W: f32 = 696.;
    const DEFAULT: f32 = 32.;
    let c = cx.theme().colors;
    let sel = state("sb-sel", w, cx, || SharedString::from("overview"));
    let collapsed = state("sb-collapsed", w, cx, || false);
    let restored = state("sb-size", w, cx, || DEFAULT);
    let is_collapsed = *collapsed.read(cx);
    let size = *restored.read(cx);
    let anim = refineui::motion::animate(
        "sb-anim",
        if is_collapsed { 0. } else { size },
        refineui::motion::Motion::PANEL,
        w,
        cx,
    );
    let animating = (anim - if is_collapsed { 0. } else { size }).abs() > 0.01;
    let avail = FRAME_W - 2. - 12.;
    let rail_open = !is_collapsed || animating;
    let rail = sidebar_rail("sb", &sel, cx);
    let rail = if animating {
        rail.width(px(size / 100. * avail))
    } else {
        rail
    };
    let toggle = collapsed.clone();
    let pin = collapsed.clone();
    let layout = restored.clone();
    let mut side = ResizablePanel::new()
        .size(anim)
        .min_size(0.)
        .max_size(if is_collapsed && !animating { 0. } else { 48. });
    if rail_open {
        side = side.child(rail);
    }
    let mut group =
        ResizablePanelGroup::new("sb-group")
            .panel(side)
            .on_layout(move |sizes, _, cx| {
                if let Some(next) = sizes.first() {
                    layout.update(cx, |s, cx| {
                        *s = next.clamp(18., 48.);
                        cx.notify();
                    });
                }
            });
    if rail_open {
        group = group.handle(ResizableHandle::new().with_handle(true).disabled(animating));
    }
    let group = group.panel(
        ResizablePanel::new()
            .size(100. - anim)
            .min_size(if is_collapsed && !animating {
                100.
            } else {
                30.
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .child(
                        v_stack(6.)
                            .flex_1()
                            .min_h_0()
                            .p(px(16.))
                            // Clear the floating sidebar trigger (top-left).
                            .pt(px(44.))
                            .bg(c.surface_sunken)
                            .child(Text::new("Overview").role(TextRole::BodySm))
                            .child(
                                Text::new("Main")
                                    .role(TextRole::BodySm)
                                    .color(TextColor::Secondary),
                            ),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(6.))
                            .left(px(6.))
                            .child(sidebar_trigger(
                                "sb-trigger",
                                !is_collapsed,
                                move |open, _, cx| {
                                    toggle.update(cx, |s, cx| {
                                        *s = !open;
                                        cx.notify();
                                    })
                                },
                            )),
                    )
                    .child(
                        SidebarPeek::new(
                            "sb-peek",
                            is_collapsed && !animating,
                            px(size / 100. * (FRAME_W - 2.)),
                        )
                        .panel(sidebar_rail("sb-peek-rail", &sel, cx).embedded(true))
                        .on_pin(move |_, cx| {
                            pin.update(cx, |s, cx| {
                                *s = false;
                                cx.notify();
                            })
                        }),
                    ),
            ),
    );
    div()
        .w(px(FRAME_W))
        .h(px(450.))
        .flex()
        .border_1()
        .border_color(c.border_default)
        .rounded(px(8.))
        .overflow_hidden()
        .bg(c.background_primary)
        .child(group)
        .into_any_element()
}

fn tree_nodes() -> Vec<TreeNode> {
    vec![
        TreeNode::branch(
            "src",
            "src",
            vec![
                TreeNode::branch(
                    "components",
                    "components",
                    vec![
                        TreeNode::leaf("button", "button.rs"),
                        TreeNode::leaf("input", "input.rs"),
                    ],
                ),
                TreeNode::leaf("lib", "lib.rs"),
            ],
        ),
        TreeNode::branch("assets", "assets", vec![TreeNode::leaf("icons", "icons")]),
        TreeNode::leaf("cargo", "Cargo.toml"),
    ]
}

fn tree(_w: &mut Window, cx: &mut App) -> AnyElement {
    h_stack(24.)
        .items_start()
        .child(div().w(px(240.)).child(section(
            "Medium",
            Tree::new("tree", tree_nodes()).default_expanded(["src", "components"]),
            cx,
        )))
        .child(
            div().w(px(240.)).child(section(
                "Small",
                Tree::new("tree-sm", tree_nodes())
                    .size(ControlSize::Small)
                    .default_expanded(["src"]),
                cx,
            )),
        )
        .into_any_element()
}

fn answer(text: &'static str) -> impl Fn(&mut Window, &mut App) -> AnyElement + 'static {
    move |_, _| {
        Text::new(text)
            .color(TextColor::Secondary)
            .into_any_element()
    }
}

fn accordion(_w: &mut Window, cx: &mut App) -> AnyElement {
    div()
        .w(px(500.))
        .child(section(
            "Default",
            Accordion::new("acc")
                .item_with_icon(
                    IconName::Info,
                    "What is RefineUI GPUI?",
                    answer("A desktop design system for GPUI applications."),
                )
                .item_with_icon(
                    IconName::Moon,
                    "Does it support dark mode?",
                    answer("Yes — every token has a light and dark value."),
                )
                .item_with_icon(
                    IconName::Keyboard,
                    "Is it accessible?",
                    answer("Controls are focusable and keyboard operable."),
                )
                .disabled_item("Coming soon")
                .default_open([0]),
            cx,
        ))
        .into_any_element()
}

fn collapsible(w: &mut Window, cx: &mut App) -> AnyElement {
    let open = state("collapsible-open", w, cx, || true);
    let is_open = *open.read(cx);
    div()
        .w(px(440.))
        .child(
            Collapsible::new(
                "collapsible",
                h_stack(8.)
                    .child(Text::new("Starred repositories").role(TextRole::LabelMd))
                    .child(Badge::count(3).variant(BadgeVariant::Neutral))
                    .child(
                        Icon::new(if is_open {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .size(IconSize::XSmall),
                    ),
                |_, _| {
                    v_stack(6.)
                        .pt(px(10.))
                        .child(Text::new("pelagornis/refineui"))
                        .child(Text::new("pelagornis/refineui-gpui"))
                        .child(Text::new("pelagornis/starlight-theme-page"))
                        .into_any_element()
                },
            )
            .open(is_open)
            .on_open_change(setter(&open)),
        )
        .into_any_element()
}

fn calendar(w: &mut Window, cx: &mut App) -> AnyElement {
    let sel = state("cal-sel", w, cx, || Some(Date::new(2026, 11, 18)));
    let month = state("cal-month", w, cx, || Date::new(2026, 11, 1));
    let set = {
        let sel = sel.clone();
        move |d: &Date, _: &mut Window, cx: &mut App| {
            sel.update(cx, |s, cx| {
                *s = Some(*d);
                cx.notify();
            })
        }
    };
    Calendar::new("cal", *month.read(cx))
        .today(Date::new(2026, 11, 10))
        .selected(*sel.read(cx))
        .on_select(set)
        .into_any_element()
}
