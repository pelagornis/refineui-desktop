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
        Story {
            id: "materials",
            name: "Materials",
            group: StoryGroup::Foundations,
            summary: "Material Engine: real backdrop blur, glass, lighting and grain over live content.",
            shot: (760., 520.),
            render: materials,
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

/// A busy backdrop so blur is easy to judge: color fields, stripes and text.
pub(super) fn material_backdrop(cx: &App) -> gpui::Div {
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
    let mut stripes = div()
        .absolute()
        .left(px(0.))
        .top(px(300.))
        .flex()
        .gap(px(10.));
    for i in 0..36 {
        let c = [0xF25022, 0x7FBA00, 0x00A4EF, 0xFFB900][i % 4];
        stripes = stripes.child(div().w(px(10.)).h(px(170.)).bg(rgb(c)));
    }
    let mut lines = div()
        .absolute()
        .left(px(24.))
        .top(px(16.))
        .flex()
        .flex_col()
        .gap(px(2.));
    for t in [
        "RefineUI Material Engine",
        "Real backdrop blur — sampled from the pixels behind",
        "The quick brown fox jumps over the lazy dog 0123456789",
    ] {
        lines = lines.child(
            div()
                .text_size(px(22.))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(if dark { gpui::white() } else { gpui::black() })
                .child(t),
        );
    }
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .bg(linear_gradient(
            135.,
            linear_color_stop(rgb(if dark { 0x1E2A4A } else { 0xDDE7FF }), 0.),
            linear_color_stop(rgb(if dark { 0x3A1F3D } else { 0xFFE3EC }), 1.),
        ))
        .child(blob(40., 110., 170., 0x0F6CBD))
        .child(blob(250., 70., 140., 0xE3008C))
        .child(blob(430., 150., 190., 0xFFB900))
        .child(blob(600., 60., 120., 0x13A10E))
        .child(stripes)
        .child(lines)
}

fn materials(_: &mut Window, cx: &mut App) -> AnyElement {
    use refineui_material::prelude::*;
    let mode = if cx.theme().is_dark() {
        Mode::Dark
    } else {
        Mode::Light
    };
    let c = cx.theme().colors;
    let label = |title: &str, note: &str| {
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.foreground_primary)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(c.foreground_secondary)
                    .child(note.to_string()),
            )
    };
    let tile = |role: MaterialRole, note: &str| {
        material_surface(Material::role(role, mode))
            .w(px(160.))
            .h(px(96.))
            .p(px(12.))
            .child(label(role.name(), note))
    };
    let support = match BlurSupport::current() {
        BlurSupport::Real => "real backdrop blur",
        BlurSupport::Fallback => "fallback fill (no blur in this renderer)",
    };
    let row1 = div()
        .absolute()
        .left(px(24.))
        .top(px(118.))
        .flex()
        .gap(px(16.))
        .child(tile(MaterialRole::Thin, "blur 12 · tint 45%"))
        .child(tile(MaterialRole::Regular, "blur 16 · tint 62%"))
        .child(tile(MaterialRole::Thick, "blur 28 · tint 78%"))
        .child(tile(MaterialRole::Chrome, "titlebars, sidebars"));
    let glass = material_surface(Material::glass(mode).radius(px(24.)))
        .absolute()
        .left(px(24.))
        .top(px(250.))
        .w(px(330.))
        .h(px(200.))
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(label(
            "glass",
            "blur 18 · sat 1.9 · rim · reflection · specular · grain",
        ))
        .child(
            material_surface(Material::role(MaterialRole::Regular, mode).radius(px(14.)))
                .w(px(220.))
                .h(px(84.))
                .p(px(12.))
                .child(label("nested regular", "blurs the glass behind it")),
        );
    let fallback = material_surface(Material::regular(mode).blur(px(0.)))
        .absolute()
        .left(px(378.))
        .top(px(250.))
        .w(px(160.))
        .h(px(96.))
        .p(px(12.))
        .child(label("fallback", "no blur: opaque fill"));
    let lit = material_surface(
        Material::glass(mode)
            .lighting(Lighting::edge(0.9).angle(90.).specular(0.25))
            .shadows(ShadowPreset::Layered(mode).shadows(mode)),
    )
    .absolute()
    .left(px(562.))
    .top(px(250.))
    .w(px(150.))
    .h(px(200.))
    .p(px(12.))
    .child(label("side light", "light from the right · layered shadow"));
    let scrim = material_surface(Material::role(MaterialRole::Scrim, mode).radius(px(12.)))
        .absolute()
        .left(px(378.))
        .top(px(362.))
        .w(px(160.))
        .h(px(88.))
        .p(px(12.))
        .child(label("scrim", "modal backdrop"));
    div()
        .relative()
        .w(px(712.))
        .h(px(472.))
        .rounded(px(16.))
        .overflow_hidden()
        .child(material_backdrop(cx))
        .child(row1)
        .child(glass)
        .child(fallback)
        .child(lit)
        .child(scrim)
        .child(
            div()
                .absolute()
                .right(px(16.))
                .top(px(16.))
                .text_size(px(11.))
                .text_color(c.foreground_secondary)
                .child(format!("Renderer: {support}")),
        )
        .into_any_element()
}
