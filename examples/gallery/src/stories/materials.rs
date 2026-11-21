//! Material Engine showcases: effects (lighting, compositing, shadows) and
//! recipes (desktop surfaces) over live content.

use gpui::{
    AnyElement, App, Hsla, IntoElement, ParentElement, Styled, Window, div, point, px, rgb,
};
use refineui::prelude::*;
use refineui_material::prelude::*;

use super::foundations::material_backdrop;
use super::{Story, StoryGroup};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "material-effects",
            name: "Material Effects",
            group: StoryGroup::Foundations,
            summary: "Lighting, blend modes, masks, inner and drop shadows, nesting — evaluated on the GPU over a real blur.",
            shot: (760., 620.),
            render: effects,
        },
        Story {
            id: "material-recipes",
            name: "Material Recipes",
            group: StoryGroup::Foundations,
            summary: "Ready-made materials: titlebar, sidebar, popover, menu, tooltip, HUD, sheet, toast.",
            shot: (760., 560.),
            render: recipes_story,
        },
        Story {
            id: "material-components",
            name: "Material Components",
            group: StoryGroup::Foundations,
            summary: "Components on the Material Engine: Surface, Text / Icon / Separator on surfaces, glass Button and Input, material Popover, Tooltip, Toast and Dialog.",
            shot: (760., 600.),
            render: components_story,
        },
    ]
}

fn mode(cx: &App) -> Mode {
    if cx.theme().is_dark() {
        Mode::Dark
    } else {
        Mode::Light
    }
}

fn caption(title: &str, note: &str, color: Hsla, muted: Hsla) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(color)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(muted)
                .child(note.to_string()),
        )
}

fn effects(_: &mut Window, cx: &mut App) -> AnyElement {
    let mode = mode(cx);
    let c = cx.theme().colors;
    let (fg, muted) = (c.foreground_primary, c.foreground_secondary);
    // A neutral, unlit base so each card shows one effect.
    let base = || {
        Material::regular(mode)
            .lighting(Lighting::NONE)
            .radius(px(14.))
            .shadows(Vec::new())
    };
    let card = |m: Material, title: &str, note: &str| {
        material_surface(m)
            .w(px(128.))
            .h(px(104.))
            .p(px(10.))
            .child(caption(title, note, fg, muted))
    };
    let brand: Hsla = rgb(0x0F6CBD).into();
    let row = |top: f32| {
        div()
            .absolute()
            .left(px(20.))
            .top(px(top))
            .flex()
            .gap(px(12.))
    };
    let header = |top: f32, text: &str| {
        div()
            .absolute()
            .left(px(20.))
            .top(px(top))
            .text_size(px(11.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(fg)
            .child(text.to_string())
    };

    let lighting = row(130.)
        .child(card(
            base().lighting(Lighting::NONE.directional(0.55).angle(315.)),
            "directional",
            "sheen from top-left",
        ))
        .child(card(
            base().lighting(Lighting::NONE.ambient(0.22)),
            "ambient",
            "uniform lift",
        ))
        .child(card(
            base().lighting(Lighting::NONE.rim(0.95, px(2.5)).angle(0.)),
            "edge",
            "rim, brightest facing the light",
        ))
        .child(card(
            base().lighting(
                Lighting::NONE
                    .specular(0.7)
                    .specular_at(point(0.3, 0.3), px(24.)),
            ),
            "specular",
            "soft highlight",
        ))
        .child(card(
            base().lighting(Lighting::NONE.reflection(0.45).angle(315.)),
            "reflection",
            "glossy band",
        ));

    let mut blends = row(270.);
    for mode_ in BlendMode::ALL {
        blends = blends.child(card(
            base().tint(brand.opacity(0.55)).tint_blend(mode_),
            mode_.name(),
            "tint blend",
        ));
    }

    let shadows = row(410.)
        .child(card(
            base().inner_shadow(InnerShadow::pressed(mode)),
            "inner · pressed",
            "inset band on top",
        ))
        .child(card(
            base().inner_shadow(InnerShadow::well(mode)),
            "inner · well",
            "recessed field",
        ))
        .child(card(
            base().mask(Mask::fade_bottom(0.35)),
            "mask",
            "fades out downwards",
        ))
        .child(card(
            base().shadows(ShadowPreset::Layered(mode).shadows(mode)),
            "drop shadow",
            "outside only — not in the blur",
        ))
        .child(
            material_surface(Material::glass(mode).radius(px(14.)))
                .w(px(128.))
                .h(px(104.))
                .p(px(8.))
                .child(
                    material_surface(base().radius(px(10.)))
                        .size_full()
                        .p(px(8.))
                        .child(
                            material_surface(
                                Material::role(MaterialRole::Thin, mode)
                                    .radius(px(8.))
                                    .shadows(Vec::new()),
                            )
                            .size_full()
                            .p(px(6.))
                            .child(caption(
                                "nested ×3",
                                "glass › regular › thin",
                                fg,
                                muted,
                            )),
                        ),
                ),
        );

    let support = match BlurSupport::current() {
        BlurSupport::Real => "GPU: real blur + per-pixel light",
        BlurSupport::Fallback => "fallback: opaque fill + quad light",
    };
    div()
        .relative()
        .w(px(712.))
        .h(px(572.))
        .rounded(px(16.))
        .overflow_hidden()
        .child(material_backdrop(cx))
        .child(header(108., "Lighting"))
        .child(lighting)
        .child(header(248., "Blend modes (tint over backdrop)"))
        .child(blends)
        .child(header(388., "Shadows, masks, nesting"))
        .child(shadows)
        .child(
            div()
                .absolute()
                .right(px(16.))
                .bottom(px(12.))
                .text_size(px(11.))
                .text_color(muted)
                .child(support),
        )
        .into_any_element()
}

fn recipes_story(_: &mut Window, cx: &mut App) -> AnyElement {
    let mode = mode(cx);
    let c = cx.theme().colors;
    let fg = |r: Recipe| r.foreground(mode);
    let muted = c.foreground_secondary;
    let text = |r: Recipe, t: &str, size: f32| {
        div()
            .text_size(px(size))
            .text_color(fg(r))
            .child(t.to_string())
    };

    let titlebar = material_surface(recipes::titlebar(mode))
        .absolute()
        .left_0()
        .top_0()
        .w_full()
        .h(px(40.))
        .px(px(14.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(div().size(px(10.)).rounded_full().bg(rgb(0xFF5F57)))
        .child(div().size(px(10.)).rounded_full().bg(rgb(0xFEBC2E)))
        .child(div().size(px(10.)).rounded_full().bg(rgb(0x28C840)))
        .child(div().w(px(12.)))
        .child(text(Recipe::Titlebar, "titlebar · chrome", 12.));
    let divider = |vertical: bool| {
        let d = div().absolute().bg(c.border_subtle);
        if vertical { d.w(px(1.)) } else { d.h(px(1.)) }
    };
    let mut items = div().flex().flex_col().gap(px(2.)).p(px(10.));
    for (i, label) in ["Library", "Recents", "Shared", "Favorites", "Trash"]
        .into_iter()
        .enumerate()
    {
        let row = div()
            .px(px(8.))
            .py(px(5.))
            .rounded(px(6.))
            .text_size(px(12.))
            .text_color(fg(Recipe::Sidebar))
            .child(label);
        items = items.child(if i == 0 {
            row.bg(c.background_surface_selected.opacity(0.6))
        } else {
            row
        });
    }
    let sidebar = material_surface(recipes::sidebar(mode))
        .absolute()
        .left_0()
        .top(px(40.))
        .w(px(170.))
        .bottom_0()
        .child(items)
        .child(
            text(Recipe::Sidebar, "sidebar · chrome", 10.)
                .absolute()
                .left(px(18.))
                .bottom(px(12.)),
        );

    let mut menu_items = div().flex().flex_col().p(px(4.));
    for label in ["Cut", "Copy", "Paste", "Select All"] {
        menu_items = menu_items.child(
            div()
                .px(px(10.))
                .py(px(5.))
                .rounded(px(5.))
                .text_size(px(12.))
                .text_color(fg(Recipe::Menu))
                .child(label),
        );
    }
    let menu = material_surface(recipes::menu(mode))
        .absolute()
        .left(px(200.))
        .top(px(70.))
        .w(px(150.))
        .child(menu_items);

    let popover = material_surface(recipes::popover(mode))
        .absolute()
        .left(px(380.))
        .top(px(64.))
        .w(px(220.))
        .p(px(14.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            text(Recipe::Popover, "Share “Roadmap”", 13.).font_weight(gpui::FontWeight::SEMIBOLD),
        )
        .child(text(
            Recipe::Popover,
            "popover · regular · overlay elevation",
            10.,
        ))
        .child(
            div()
                .h(px(28.))
                .rounded(px(8.))
                .bg(c.background_primary.opacity(0.7))
                .border_1()
                .border_color(c.border_default)
                .px(px(8.))
                .flex()
                .items_center()
                .text_size(px(11.))
                .text_color(muted)
                .child("name@example.com"),
        );

    let tooltip = material_surface(recipes::tooltip(mode))
        .absolute()
        .left(px(620.))
        .top(px(86.))
        .px(px(8.))
        .py(px(4.))
        .child(text(Recipe::Tooltip, "tooltip · thin", 11.));

    let hud = material_surface(recipes::hud())
        .absolute()
        .left(px(260.))
        .top(px(250.))
        .w(px(150.))
        .h(px(130.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .child(
            Icon::new(IconName::WeatherSunny)
                .size(IconSize::Large)
                .color(gpui::white()),
        )
        .child(
            div()
                .w(px(96.))
                .h(px(4.))
                .rounded_full()
                .bg(gpui::white().opacity(0.25))
                .child(div().w(px(64.)).h_full().rounded_full().bg(gpui::white())),
        )
        .child(text(Recipe::Hud, "HUD · smoked glass", 10.));

    let toast = material_surface(recipes::toast(mode))
        .absolute()
        .left(px(190.))
        .top(px(420.))
        .w(px(230.))
        .p(px(12.))
        .flex()
        .gap(px(10.))
        .items_center()
        .child(Icon::new(IconName::Checkmark).color(c.foreground_brand))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    text(Recipe::Toast, "Export complete", 12.)
                        .font_weight(gpui::FontWeight::SEMIBOLD),
                )
                .child(text(Recipe::Toast, "toast · regular", 10.)),
        );

    let sheet = material_surface(recipes::sheet(mode))
        .absolute()
        .left(px(440.))
        .top(px(280.))
        .w(px(260.))
        .h(px(170.))
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(text(Recipe::Sheet, "Save changes?", 14.).font_weight(gpui::FontWeight::SEMIBOLD))
        .child(text(
            Recipe::Sheet,
            "sheet · thick · modal elevation · glass rim",
            10.,
        ))
        .child(
            div()
                .mt_auto()
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(Button::new("sheet-cancel", "Cancel").variant(ButtonVariant::Secondary))
                .child(Button::new("sheet-save", "Save")),
        );

    div()
        .relative()
        .w(px(712.))
        .h(px(512.))
        .rounded(px(16.))
        .overflow_hidden()
        .child(material_backdrop(cx))
        .child(sidebar)
        .child(divider(true).left(px(170.)).top(px(40.)).bottom_0())
        .child(titlebar)
        .child(divider(false).left_0().top(px(40.)).w_full())
        .child(menu)
        .child(popover)
        .child(tooltip)
        .child(hud)
        .child(toast)
        .child(sheet)
        .into_any_element()
}

/// Phase 4: RefineUI components drawing on materials (`material` feature).
fn components_story(window: &mut Window, cx: &mut App) -> AnyElement {
    use std::time::Duration;
    let active = materials_active(cx);
    let search = window.use_keyed_state("material-search", cx, |_, cx| {
        TextInput::search(cx)
            .glass(true)
            .placeholder("Search the library")
    });
    let at = |x: f32, y: f32, e: gpui::Div| e.absolute().left(px(x)).top(px(y));

    let panel = Surface::new(SurfaceKind::Panel)
        .w(px(200.))
        .p(px(14.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(Text::new("Surface · panel").role(TextRole::SubtitleMd))
        .child(
            Text::new("A div whose plate is a material.")
                .role(TextRole::BodyXs)
                .color(TextColor::Secondary),
        )
        .child(Separator::horizontal().on_surface(SurfaceKind::Panel))
        .child(
            div()
                .flex()
                .gap(px(6.))
                .child(Button::new("mc-glass", "Glass").glass(true))
                .child(
                    Button::icon_only("mc-glass-icon", IconName::Settings)
                        .glass(true)
                        .tooltip("Settings"),
                ),
        );

    let hud = Surface::new(SurfaceKind::Hud)
        .w(px(150.))
        .p(px(14.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .child(
            Icon::new(IconName::WeatherSunny)
                .size(IconSize::Large)
                .on_surface(SurfaceKind::Hud),
        )
        .child(Separator::horizontal().on_surface(SurfaceKind::Hud))
        .child(
            Text::new("Volume 60%")
                .role(TextRole::BodySm)
                .on_surface(SurfaceKind::Hud),
        )
        .child(
            Text::new("Surface · hud")
                .role(TextRole::CaptionMd)
                .color(TextColor::Secondary)
                .on_surface(SurfaceKind::Hud),
        );

    let glass = Surface::new(SurfaceKind::Glass)
        .w(px(180.))
        .h(px(120.))
        .p(px(14.))
        .flex()
        .flex_col()
        .justify_end()
        .child(Text::new("Surface · glass").role(TextRole::SubtitleMd))
        .child(
            Text::new("lit edges, reflection, grain")
                .role(TextRole::BodyXs)
                .color(TextColor::Secondary),
        );

    let popover = popover_panel(
        Some("Popover".into()),
        div()
            .text_size(px(12.))
            .child("Frosted panel on the Popover material.")
            .into_any_element(),
        None,
        PopoverAlign::Start,
        cx,
    )
    .w(px(240.));

    let toast = Toast::new(
        "mc-toast",
        ToastData::new("Export complete")
            .description("Toast::material(true)")
            .variant(ToastVariant::Success)
            .duration(Some(Duration::from_secs(4))),
    )
    .material(true);

    let dialog = Dialog::new("mc-dialog", "Save changes?")
        .description("Dialog::material(true) — the Sheet material.")
        .size(DialogSize::Small)
        .footer(
            div()
                .flex()
                .gap(px(6.))
                .child(Button::new("mc-d-0", "Cancel").variant(ButtonVariant::Secondary))
                .child(Button::new("mc-d-1", "Save")),
        )
        .material(true)
        .inline(true);

    let status = div()
        .text_size(px(10.))
        .text_color(cx.theme().colors.foreground_secondary)
        .child(if active {
            "materials active: real blur"
        } else {
            "materials inactive: opaque fallback"
        });

    div()
        .relative()
        .w(px(712.))
        .h(px(552.))
        .rounded(px(16.))
        .overflow_hidden()
        .child(material_backdrop(cx))
        .child(at(20., 110., div().child(panel)))
        .child(at(240., 110., div().child(hud)))
        .child(at(410., 110., div().child(glass)))
        .child(at(20., 300., div().w(px(240.)).child(search)))
        .child(at(
            20.,
            356.,
            div().child(tooltip_with_beak(
                "Tooltip on the thin material",
                BeakSide::Top,
                cx,
            )),
        ))
        .child(at(20., 420., div().child(toast)))
        .child(at(300., 290., div().child(popover)))
        .child(at(330., 380., div().w(px(360.)).child(dialog)))
        .child(at(540., 20., div().child(status)))
        .into_any_element()
}
