use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled, Window, div, px};
use refineui::prelude::*;
use refineui::tokens;

use super::{Story, StoryGroup, row, section};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "colors",
            name: "Colors",
            group: StoryGroup::Foundations,
            summary: "Semantic color aliases resolved for the current appearance.",
            shot: (760., 560.),
            render: colors,
        },
        Story {
            id: "typography",
            name: "Typography",
            group: StoryGroup::Foundations,
            summary: "RefineUI type scale (Inter).",
            shot: (760., 760.),
            render: typography,
        },
        Story {
            id: "elevation",
            name: "Elevation",
            group: StoryGroup::Foundations,
            summary: "Key + ambient shadow pairs and radius steps.",
            shot: (760., 300.),
            render: elevation,
        },
        Story {
            id: "icon",
            name: "Icon",
            group: StoryGroup::Foundations,
            summary: "RefineUI System Icons (regular / filled).",
            shot: (760., 300.),
            render: icons,
        },
    ]
}

fn swatch(name: &'static str, color: Hsla, cx: &App) -> impl IntoElement {
    let c = cx.theme().colors;
    div()
        .w(px(168.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .size(px(28.))
                .rounded(px(6.))
                .bg(color)
                .border_1()
                .border_color(c.border_subtle),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(c.foreground_secondary)
                .child(name.replace('_', " ")),
        )
}

fn colors(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    let groups: [(&str, Vec<(&'static str, Hsla)>); 4] = [
        (
            "Background",
            vec![
                ("background_primary", c.background_primary),
                ("background_surface", c.background_surface),
                ("background_brand", c.background_brand),
                ("background_brand_subtle", c.background_brand_subtle),
                ("background_info", c.background_info),
                ("background_success", c.background_success),
                ("background_warning", c.background_warning),
                ("background_error", c.background_error),
                ("background_discovery", c.background_discovery),
                ("surface_sunken", c.surface_sunken),
                ("surface_popover", c.surface_popover),
                ("surface_tooltip", c.surface_tooltip),
            ],
        ),
        (
            "Foreground",
            vec![
                ("foreground_primary", c.foreground_primary),
                ("foreground_secondary", c.foreground_secondary),
                ("foreground_tertiary", c.foreground_tertiary),
                ("foreground_disabled", c.foreground_disabled),
                ("foreground_link", c.foreground_link),
                ("foreground_info", c.foreground_info),
                ("foreground_success", c.foreground_success),
                ("foreground_warning", c.foreground_warning),
                ("foreground_error", c.foreground_error),
                ("foreground_discovery", c.foreground_discovery),
            ],
        ),
        (
            "Border",
            vec![
                ("border_subtle", c.border_subtle),
                ("border_default", c.border_default),
                ("border_strong", c.border_strong),
                ("border_hover", c.border_hover),
                ("border_focus", c.border_focus),
                ("border_error", c.border_error),
            ],
        ),
        (
            "Palette (blue)",
            vec![
                ("blue_100", to(tokens::palette::BLUE_100)),
                ("blue_300", to(tokens::palette::BLUE_300)),
                ("blue_500", to(tokens::palette::BLUE_500)),
                ("blue_700", to(tokens::palette::BLUE_700)),
                ("blue_900", to(tokens::palette::BLUE_900)),
            ],
        ),
    ];
    v_stack(16.)
        .children(groups.into_iter().map(|(title, items)| {
            let content = row()
                .gap(px(10.))
                .children(items.into_iter().map(|(n, col)| swatch(n, col, cx)));
            section(title, content, cx)
        }))
        .into_any_element()
}

fn to(c: tokens::Color) -> Hsla {
    gpui::rgba(c.0).into()
}

fn typography(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    v_stack(6.)
        .children(
            TextVariant::ALL
                .into_iter()
                .filter(|v| *v != TextVariant::Heading1)
                .map(|v| {
                    let t = v.token();
                    h_stack(16.)
                        .child(
                            div()
                                .w(px(150.))
                                .text_size(px(11.))
                                .text_color(c.foreground_tertiary)
                                .child(format!("{} · {}/{}", t.name, t.size, t.line_height)),
                        )
                        .child(Text::new("The quick brown fox").variant(v))
                }),
        )
        .into_any_element()
}

fn elevation(_w: &mut Window, cx: &mut App) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let steps = [
        ("surface", Elevation::SURFACE),
        ("raised", Elevation::RAISED),
        ("floating", Elevation::FLOATING),
        ("overlay", Elevation::OVERLAY),
        ("modal", Elevation::MODAL),
    ];
    v_stack(24.)
        .child(section(
            "Elevation",
            row()
                .gap(px(24.))
                .children(steps.into_iter().map(|(name, e)| {
                    div()
                        .size(px(96.))
                        .rounded(px(12.))
                        .bg(c.surface_elevated)
                        .shadow(theme.shadow(e))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(c.foreground_secondary)
                        .child(name)
                })),
            cx,
        ))
        .child(section(
            "Radius",
            row()
                .gap(px(16.))
                .children([0., 2., 4., 6., 8., 12., 16.].into_iter().map(|r| {
                    div()
                        .size(px(48.))
                        .rounded(px(r))
                        .border_2()
                        .border_color(c.border_strong)
                        .bg(c.background_surface_hover)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.))
                        .child(format!("{r}"))
                })),
            cx,
        ))
        .into_any_element()
}

fn icons(_w: &mut Window, cx: &mut App) -> AnyElement {
    let c = cx.theme().colors;
    v_stack(12.)
        .child(section(
            "Regular",
            row().gap(px(14.)).children(IconName::ALL.iter().map(|i| {
                Icon::new(*i)
                    .size(IconSize::Small)
                    .color(c.foreground_primary)
            })),
            cx,
        ))
        .child(section(
            "Filled",
            row()
                .gap(px(14.))
                .children(IconName::ALL.iter().take(24).map(|i| {
                    Icon::new(*i)
                        .style(IconStyle::Filled)
                        .size(IconSize::Small)
                        .color(c.foreground_primary)
                })),
            cx,
        ))
        .into_any_element()
}
