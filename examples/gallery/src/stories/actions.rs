use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window};
use refineui::prelude::*;

use super::{Story, StoryGroup, row, section, setter, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "button",
            name: "Button",
            group: StoryGroup::Actions,
            summary: "Primary, secondary, outline and ghost buttons in three sizes.",
            shot: (720., 300.),
            render: button,
        },
        Story {
            id: "toggle-button",
            name: "Toggle Button",
            group: StoryGroup::Actions,
            summary: "Two-state toolbar button (pressed = selected plate).",
            shot: (560., 140.),
            render: toggle,
        },
        Story {
            id: "segmented-control",
            name: "Segmented Control",
            group: StoryGroup::Actions,
            summary: "Mutually exclusive segments.",
            shot: (560., 180.),
            render: segmented,
        },
        Story {
            id: "link",
            name: "Link",
            group: StoryGroup::Actions,
            summary: "Inline text link.",
            shot: (480., 100.),
            render: link,
        },
    ]
}

fn button(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .child(section(
            "Variants",
            row().children(
                ButtonVariant::ALL
                    .into_iter()
                    .map(|v| Button::new(v.label(), v.label()).variant(v)),
            ),
            cx,
        ))
        .child(section(
            "Sizes",
            row()
                .child(Button::new("sm", "Small").size(ControlSize::Small))
                .child(Button::new("md", "Medium").size(ControlSize::Medium))
                .child(Button::new("lg", "Large").size(ControlSize::Large)),
            cx,
        ))
        .child(section(
            "Icons & states",
            row()
                .child(
                    Button::new("icon", "New file")
                        .icon(IconName::Add)
                        .variant(ButtonVariant::Secondary),
                )
                .child(
                    Button::new("menu", "Options")
                        .icon_end(IconName::ChevronDown)
                        .variant(ButtonVariant::Outline),
                )
                .child(
                    Button::icon_only("settings", IconName::Settings)
                        .variant(ButtonVariant::Ghost)
                        .tooltip("Settings"),
                )
                .child(
                    Button::icon_only("delete", IconName::Delete)
                        .variant(ButtonVariant::Outline)
                        .tooltip("Delete"),
                )
                .child(
                    Button::icon_only("close-sm", IconName::Dismiss)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Small)
                        .icon_size(IconSize::Small),
                )
                .child(Button::new("loading", "Saving").loading(true))
                .child(Button::new("disabled", "Disabled").disabled(true))
                .child(
                    Button::new("disabled-o", "Disabled")
                        .variant(ButtonVariant::Outline)
                        .disabled(true),
                ),
            cx,
        ))
        .into_any_element()
}

fn toggle(w: &mut Window, cx: &mut App) -> AnyElement {
    let bold = state("toggle-bold", w, cx, || true);
    let italic = state("toggle-italic", w, cx, || false);
    let grid = state("toggle-grid", w, cx, || false);
    row()
        .child(ToggleButton::new("bold", "Bold", *bold.read(cx)).on_change(setter(&bold)))
        .child(ToggleButton::new("italic", "Italic", *italic.read(cx)).on_change(setter(&italic)))
        .child(
            ToggleButton::icon_only("grid", IconName::Grid, *grid.read(cx))
                .tooltip("Grid")
                .on_change(setter(&grid)),
        )
        .child(ToggleButton::new("disabled", "Disabled", false).disabled(true))
        .into_any_element()
}

fn segmented(w: &mut Window, cx: &mut App) -> AnyElement {
    let view = state("seg-view", w, cx, || 0usize);
    let size = state("seg-size", w, cx, || 1usize);
    v_stack(16.)
        .child(
            SegmentedControl::new(
                "seg1",
                [
                    Segment::with_icon("List", IconName::Document),
                    Segment::with_icon("Grid", IconName::Grid),
                    Segment::with_icon("Board", IconName::Board),
                ],
                *view.read(cx),
            )
            .on_change(setter(&view)),
        )
        .child(
            SegmentedControl::new("seg2", ["Day", "Week", "Month", "Year"], *size.read(cx))
                .size(ControlSize::Small)
                .on_change(setter(&size)),
        )
        .into_any_element()
}

fn link(_w: &mut Window, cx: &mut App) -> AnyElement {
    let _ = cx;
    row()
        .gap(gpui::px(24.))
        .child(
            Link::new("docs", "Read the docs")
                .href("https://github.com/pelagornis/refineui-desktop"),
        )
        .child(
            Link::new("ext", "GPUI")
                .href("https://gpui.rs")
                .external(true),
        )
        .child(Link::new("disabled-link", "Disabled").disabled(true))
        .into_any_element()
}
