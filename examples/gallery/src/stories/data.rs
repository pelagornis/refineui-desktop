use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window, div, px};
use refineui::prelude::*;

use super::{Story, StoryGroup, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "table",
            name: "Table",
            group: StoryGroup::Data,
            summary: "Sortable, selectable data table.",
            shot: (640., 260.),
            render: table,
        },
        Story {
            id: "scroll-area",
            name: "Scroll Area",
            group: StoryGroup::Data,
            summary: "Overlay scrollbar with edge blur on scrolled-away rows.",
            shot: (420., 304.),
            render: scroll_area,
        },
        Story {
            id: "resizable",
            name: "Resizable",
            group: StoryGroup::Data,
            summary: "Panel group with draggable separators.",
            shot: (640., 304.),
            render: resizable,
        },
    ]
}

fn table(w: &mut Window, cx: &mut App) -> AnyElement {
    let sel = state("table-sel", w, cx, || Some(1usize));
    let asc = state("table-asc", w, cx, || true);
    let mut rows = vec![
        ("button.rs", "Actions", "412"),
        ("input.rs", "Forms", "890"),
        ("menu.rs", "Overlays", "355"),
        ("table.rs", "Data", "270"),
        ("tree.rs", "Navigation", "198"),
    ];
    if !*asc.read(cx) {
        rows.reverse();
    }
    let set_sel = {
        let sel = sel.clone();
        move |ix: &usize, _: &mut Window, cx: &mut App| {
            sel.update(cx, |s, cx| {
                *s = Some(*ix);
                cx.notify();
            })
        }
    };
    let flip = {
        let asc = asc.clone();
        move |_: &usize, _: &mut Window, cx: &mut App| {
            asc.update(cx, |a, cx| {
                *a = !*a;
                cx.notify();
            })
        }
    };
    div()
        .w(px(580.))
        .child(
            Table::new(
                "table",
                [
                    Column::new("File").fraction(0.45).sorted(*asc.read(cx)),
                    Column::new("Group"),
                    Column::new("Lines").width(80.).align(Align::End),
                ],
            )
            .text_rows(rows.into_iter().map(|(a, b, c)| [a, b, c]))
            .striped(true)
            .selected(*sel.read(cx))
            .on_select(set_sel)
            .on_sort(flip),
        )
        .into_any_element()
}

const TAGS: [&str; 38] = [
    "Accordion",
    "Alert",
    "Avatar",
    "Badge",
    "Bubble",
    "Button",
    "Calendar",
    "Card",
    "Carousel",
    "Checkbox",
    "Dialog",
    "Divider",
    "Drawer",
    "Dropdown",
    "Field",
    "Input",
    "Label",
    "Link",
    "Menu",
    "Pagination",
    "PopOver",
    "Progress",
    "ProgressStepper",
    "Radio",
    "Resizable",
    "ScrollArea",
    "Select",
    "Skeleton",
    "Slider",
    "Spinner",
    "Stepper",
    "Switch",
    "SegmentedControl",
    "Tabs",
    "Tag",
    "Textarea",
    "Toast",
    "Tooltip",
];

/// Web `ScrollAreaDemo`: 256px tall tag list, hover scrollbar, edge blur.
fn scroll_area(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    div()
        .w(px(372.))
        .h(px(256.))
        .border_1()
        .border_color(c.border_default)
        .rounded(px(8.))
        .overflow_hidden()
        .child(ScrollArea::new("scroll").children(TAGS.iter().map(|name| {
            // Web demo rows carry `data-scroll-edge-item`.
            scroll_edge_item(
                *name,
                div()
                    .px(px(16.))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(c.border_default)
                    .child(Text::new(*name).role(TextRole::BodySm)),
            )
        })))
        .into_any_element()
}

fn panel_label(label: &'static str, cx: &App) -> impl IntoElement {
    let c = cx.theme().colors;
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p(px(16.))
        .bg(c.background_surface)
        .child(
            Text::new(label)
                .role(TextRole::BodySm)
                .color(TextColor::Secondary),
        )
}

/// Web `ResizableDemo`: 35 / 65 split, min 20 / 30, grip handle, 256px tall.
fn resizable(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    div()
        .w(px(592.))
        .h(px(256.))
        .border_1()
        .border_color(c.border_default)
        .rounded(px(8.))
        .overflow_hidden()
        .child(
            ResizablePanelGroup::new("resizable")
                .panel(
                    ResizablePanel::new()
                        .default_size(35.)
                        .min_size(20.)
                        .child(panel_label("Sidebar", cx)),
                )
                .handle(ResizableHandle::new().with_handle(true))
                .panel(
                    ResizablePanel::new()
                        .default_size(65.)
                        .min_size(30.)
                        .child(panel_label("Main", cx)),
                ),
        )
        .into_any_element()
}
