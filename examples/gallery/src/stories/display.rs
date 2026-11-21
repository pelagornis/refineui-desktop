use std::time::Duration;

use gpui::prelude::*;
use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window, div, px};
use refineui::prelude::*;

use super::{Story, StoryGroup, row, section, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "text",
            name: "Text",
            group: StoryGroup::Display,
            summary: "Typography primitives: Text, Heading and Label.",
            shot: (640., 320.),
            render: text,
        },
        Story {
            id: "badge",
            name: "Badge",
            group: StoryGroup::Display,
            summary: "Small status or count indicator.",
            shot: (600., 120.),
            render: badge,
        },
        Story {
            id: "chip",
            name: "Chip",
            group: StoryGroup::Display,
            summary: "Compact, optionally removable or selectable label (also Tag).",
            shot: (640., 240.),
            render: chip,
        },
        Story {
            id: "avatar",
            name: "Avatar",
            group: StoryGroup::Display,
            summary: "Image, icon or initials with status; Avatars stack / spread.",
            shot: (520., 440.),
            render: avatar,
        },
        Story {
            id: "card",
            name: "Card",
            group: StoryGroup::Display,
            summary: "Grouped content surface; elevated or outlined, selectable.",
            shot: (720., 360.),
            render: card,
        },
        Story {
            id: "alert",
            name: "Alert",
            group: StoryGroup::Display,
            summary: "Inline message banner.",
            shot: (640., 420.),
            render: alert,
        },
        Story {
            id: "divider",
            name: "Divider",
            group: StoryGroup::Display,
            summary: "Horizontal and vertical separators.",
            shot: (520., 160.),
            render: divider,
        },
        Story {
            id: "kbd",
            name: "Kbd",
            group: StoryGroup::Display,
            summary: "Keyboard shortcut hint.",
            shot: (480., 100.),
            render: kbd,
        },
        Story {
            id: "spinner",
            name: "Spinner",
            group: StoryGroup::Display,
            summary: "Indeterminate activity indicator.",
            shot: (480., 110.),
            render: spinner,
        },
        Story {
            id: "progress",
            name: "Progress",
            group: StoryGroup::Display,
            summary: "Determinate and indeterminate progress bars.",
            shot: (560., 190.),
            render: progress,
        },
        Story {
            id: "progress-stepper",
            name: "Progress Stepper",
            group: StoryGroup::Display,
            summary: "Multi-step process completion indicator.",
            shot: (560., 180.),
            render: progress_stepper,
        },
        Story {
            id: "skeleton",
            name: "Skeleton",
            group: StoryGroup::Display,
            summary: "Loading placeholder shapes.",
            shot: (480., 150.),
            render: skeleton,
        },
        Story {
            id: "tooltip",
            name: "Tooltip",
            group: StoryGroup::Display,
            summary: "Hover hint with optional shortcut. Beak is one outline with the body.",
            shot: (720., 180.),
            render: tooltip,
        },
        Story {
            id: "stepper",
            name: "Stepper",
            group: StoryGroup::Display,
            summary: "Numbered steps with animated rails and checkmarks.",
            shot: (640., 200.),
            render: stepper,
        },
        Story {
            id: "toast",
            name: "Toast",
            group: StoryGroup::Display,
            summary: "Transient notification; push through a Toaster.",
            shot: (460., 300.),
            render: toast,
        },
    ]
}

fn text(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .child(section(
            "Roles",
            v_stack(6.)
                .child(Text::new("Title").role(TextRole::TitleMd))
                .child(Text::new("Subtitle").role(TextRole::SubtitleLg))
                .child(Text::new("Body — the primary reading size.").role(TextRole::BodyLg))
                .child(
                    Text::new("Secondary body for supporting copy.")
                        .role(TextRole::BodyMd)
                        .color(TextColor::Secondary),
                )
                .child(
                    Text::new("Caption")
                        .role(TextRole::CaptionMd)
                        .color(TextColor::Tertiary),
                ),
            cx,
        ))
        .child(section(
            "Colors",
            row()
                .gap(px(16.))
                .child(Text::new("Brand").color(TextColor::Brand))
                .child(Text::new("Success").color(TextColor::Success))
                .child(Text::new("Warning").color(TextColor::Warning))
                .child(Text::new("Error").color(TextColor::Error))
                .child(Text::new("Disabled").color(TextColor::Disabled)),
            cx,
        ))
        .child(section(
            "Label",
            row()
                .gap(px(16.))
                .child(Label::new("Label"))
                .child(Label::new("Required").required(true))
                .child(Label::new("Disabled").disabled(true)),
            cx,
        ))
        .into_any_element()
}

fn badge(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .child(section(
            "Variants",
            row().children(
                BadgeVariant::ALL
                    .into_iter()
                    .map(|v| Badge::new(v.label()).variant(v)),
            ),
            cx,
        ))
        .child(section(
            "Interactive & count",
            row()
                .children(
                    BadgeVariant::ALL
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| Badge::new(v.label()).variant(v).id(("badge", i))),
                )
                .child(Badge::count(3))
                .child(Badge::count(120).variant(BadgeVariant::Danger)),
            cx,
        ))
        .into_any_element()
}

fn chip(w: &mut Window, cx: &mut App) -> AnyElement {
    let sel = state("chip-sel", w, cx, || 0usize);
    let removed = state("chip-removed", w, cx, Vec::<usize>::new);
    let s = *sel.read(cx);
    let gone = removed.read(cx).clone();
    v_stack(16.)
        .child(section(
            "Variants",
            row()
                .children(
                    ChipVariant::ALL
                        .into_iter()
                        .map(|v| Chip::new(v.label(), v.label()).variant(v)),
                )
                .child(Chip::new("disabled", "Disabled").disabled(true)),
            cx,
        ))
        .child(section(
            "Sizes",
            row().children(
                [ControlSize::Small, ControlSize::Medium, ControlSize::Large]
                    .into_iter()
                    .enumerate()
                    .map(|(i, z)| {
                        Chip::new(("size", i), ["Small", "Medium", "Large"][i])
                            .size(z)
                            .icon(IconName::Star)
                    }),
            ),
            cx,
        ))
        .child(section(
            "Selectable",
            row().children(
                ["All", "Open", "Closed"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let sel = sel.clone();
                        Chip::new(("filter", i), l)
                            .variant(ChipVariant::Outline)
                            .selected(s == i)
                            .on_click(move |_, _, cx| {
                                sel.update(cx, |v, cx| {
                                    *v = i;
                                    cx.notify();
                                })
                            })
                    }),
            ),
            cx,
        ))
        .child(section(
            "Removable",
            row().children(
                ["rust", "gpui", "design"]
                    .into_iter()
                    .enumerate()
                    .filter(|(i, _)| !gone.contains(i))
                    .map(|(i, l)| {
                        let removed = removed.clone();
                        Chip::tag(("tag", i), l).on_remove(move |_, _, cx| {
                            removed.update(cx, |v, cx| {
                                v.push(i);
                                cx.notify();
                            })
                        })
                    }),
            ),
            cx,
        ))
        .into_any_element()
}

/// The web demo portrait (`avatars.githubusercontent.com/u/108743931`).
pub(super) fn portrait() -> gpui::ImageSource {
    static BYTES: &[u8] = include_bytes!("../../assets/pelagornis.png");
    thread_local! {
        static IMG: std::sync::Arc<gpui::Image> = std::sync::Arc::new(
            gpui::Image::from_bytes(gpui::ImageFormat::Png, BYTES.to_vec()),
        );
    }
    IMG.with(|i| gpui::ImageSource::from(i.clone()))
}

fn pic(name: &str) -> Avatar {
    Avatar::new(name.to_string()).image(portrait())
}

/// Web `AvatarPreview` + `AvatarsPreview`.
fn avatar(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .child(
            row()
                .gap(px(10.))
                .child(pic("User"))
                .child(
                    Avatar::new("Guest")
                        .color(AvatarColor::Neutral)
                        .layout(AvatarLayout::Icon),
                )
                .child(Avatar::new("Pelagornis Labs").color(AvatarColor::Orange)),
        )
        .child(section(
            "Sizes",
            row().gap(px(10.)).children(
                [
                    AvatarSize::XSmall,
                    AvatarSize::Small,
                    AvatarSize::Medium,
                    AvatarSize::Large,
                    AvatarSize::XLarge,
                ]
                .into_iter()
                .map(|s| pic("Size").size(s)),
            ),
            cx,
        ))
        .child(section(
            "Status",
            row().gap(px(10.)).children(
                [
                    Presence::Online,
                    Presence::Away,
                    Presence::Unavailable,
                    Presence::Offline,
                ]
                .into_iter()
                .map(|p| pic("Status").presence(p)),
            ),
            cx,
        ))
        .child(section(
            "Stack",
            Avatars::from_avatars((1..=5).map(|i| pic(&format!("Member {i}")))).max(3),
            cx,
        ))
        .child(section(
            "Spread",
            Avatars::from_avatars((1..=4).map(|i| pic(&format!("Member {i}"))))
                .layout(AvatarsLayout::Spread),
            cx,
        ))
        .into_any_element()
}

/// Web `CardPreview` (elevated) plus an outlined, selectable card.
fn card(w: &mut Window, cx: &mut App) -> AnyElement {
    let sel = state("card-sel", w, cx, || false);
    let selected = *sel.read(cx);
    row()
        .items_start()
        .gap(px(16.))
        .child(
            div().w(px(326.)).child(
                Card::new()
                    .eyebrow(
                        h_stack(6.).child(Badge::new("New")).child(
                            Text::new("Design system")
                                .role(TextRole::CaptionMd)
                                .color(TextColor::Tertiary),
                        ),
                    )
                    .title("RefineUI is now available")
                    .description("Ship product UI with tokens, React, and Web Kit parity.")
                    .action(
                        Button::icon_only("more", IconName::MoreHorizontal)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Small),
                    )
                    .child(
                        v_stack(10.)
                            .child(
                                Text::new(
                                    "Utilities, React components, and shared foundations for \
                                     building consistent web applications.",
                                )
                                .role(TextRole::BodyMd)
                                .color(TextColor::Secondary),
                            )
                            .child(
                                h_stack(6.)
                                    .child(
                                        Avatar::new("Pelagornis")
                                            .size(AvatarSize::XSmall)
                                            .color(AvatarColor::Blue),
                                    )
                                    .child(
                                        Text::new("Pelagornis Inc · 7 hours ago")
                                            .role(TextRole::CaptionLg)
                                            .color(TextColor::Tertiary),
                                    ),
                            ),
                    )
                    .footer(
                        h_stack(6.)
                            .child(
                                Button::new("c2", "Learn more")
                                    .variant(ButtonVariant::Outline)
                                    .size(ControlSize::Small),
                            )
                            .child(Button::new("c1", "Get started").size(ControlSize::Small)),
                    ),
            ),
        )
        .child(
            div().w(px(300.)).child(
                Card::new()
                    .variant(CardVariant::Outlined)
                    .id("card-select")
                    .selected(selected)
                    .on_click(move |_, _, cx| {
                        sel.update(cx, |v, cx| {
                            *v = !*v;
                            cx.notify();
                        })
                    })
                    .title("Outlined · selectable")
                    .description("Click to toggle the 2px borderFocus selection ring.")
                    .child(
                        Progress::new("card-progress", 0.62)
                            .label("Storage")
                            .show_value(true),
                    ),
            ),
        )
        .into_any_element()
}

fn alert(_w: &mut Window, _cx: &mut App) -> AnyElement {
    v_stack(10.)
        .w(px(560.))
        .children(
            AlertVariant::ALL
                .into_iter()
                .filter(|v| *v != AlertVariant::Custom)
                .enumerate()
                .map(|(i, v)| {
                    let a = Alert::new(("alert", i), v.label())
                        .variant(v)
                        .description("Something happened that you should know about.")
                        .on_close(|_, _, _| {});
                    if i == 0 {
                        a.action(
                            Button::new("undo", "Undo")
                                .size(ControlSize::Small)
                                .variant(ButtonVariant::Outline),
                        )
                    } else {
                        a
                    }
                }),
        )
        .into_any_element()
}

fn divider(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .w(px(440.))
        .child(Divider::horizontal())
        .child(Divider::horizontal().label("Center label"))
        .child(
            Divider::horizontal()
                .label("Left")
                .align(DividerAlign::Left),
        )
        .child(Divider::horizontal().icon())
        .child(section(
            "Vertical",
            h_stack(12.)
                .h(px(24.))
                .child(Text::new("Left"))
                .child(Divider::vertical())
                .child(Text::new("Right"))
                .child(Divider::vertical().strong(true))
                .child(Text::new("Strong")),
            cx,
        ))
        .into_any_element()
}

fn kbd(_w: &mut Window, cx: &mut App) -> AnyElement {
    row()
        .child(Kbd::new("⌘K"))
        .child(Kbd::new("⇧⌘P"))
        .child(Kbd::new("⌥⏎"))
        .child(Kbd::new("Esc"))
        .child(
            div()
                .px(px(8.))
                .py(px(4.))
                .rounded(px(6.))
                .bg(cx.theme().colors.surface_tooltip)
                .child(
                    h_stack(6.)
                        .text_size(px(12.))
                        .text_color(cx.theme().colors.foreground_inversed)
                        .child("Save")
                        .child(Kbd::new("⌘S").inverted(true)),
                ),
        )
        .into_any_element()
}

fn spinner(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .child(section(
            "Sizes",
            row()
                .gap(px(20.))
                .children(SpinnerSize::ALL.into_iter().map(|s| Spinner::new().size(s))),
            cx,
        ))
        .child(section("With label", Spinner::new().label("Loading…"), cx))
        .into_any_element()
}

fn progress(_: &mut Window, _: &mut App) -> AnyElement {
    v_stack(16.)
        .w(px(460.))
        .child(Progress::new("p1", 0.3).label("Uploading").show_value(true))
        .child(Progress::new("p2", 0.75).large(true))
        .child(Progress::indeterminate("p3").label("Indexing"))
        .into_any_element()
}

/// Web `ProgressStepperPreview`: checkout steps, `Shipping` current.
fn progress_stepper(w: &mut Window, cx: &mut App) -> AnyElement {
    let step = state("progress-step", w, cx, || 1usize);
    let current = *step.read(cx);
    v_stack(24.)
        .w(px(460.))
        .child(
            ProgressStepper::new("ps", ["Cart", "Shipping", "Payment", "Review"], current)
                .on_change(super::setter(&step)),
        )
        .child(section(
            "Disabled step",
            ProgressStepper::new("ps-disabled", ["Account", "Profile", "Complete"], 0)
                .disabled_step(2)
                .on_change(|_, _, _| {}),
            cx,
        ))
        .into_any_element()
}

fn stepper(w: &mut Window, cx: &mut App) -> AnyElement {
    let step = state("stepper-step", w, cx, || 1usize);
    let current = *step.read(cx);
    v_stack(16.)
        .w(px(580.))
        .child(
            Stepper::new("stepper", current)
                .step_with_description("Account", "Create login")
                .step_with_description("Profile", "Tell us about you")
                .step_with_description("Billing", "Choose a plan")
                .step("Done")
                .on_change(super::setter(&step)),
        )
        .child(
            h_stack(6.)
                .child({
                    let s = step.clone();
                    Button::new("stepper-back", "Back")
                        .variant(ButtonVariant::Outline)
                        .size(ControlSize::Small)
                        .disabled(current == 0)
                        .on_click(move |_, _, cx| {
                            s.update(cx, |v, cx| {
                                *v = v.saturating_sub(1);
                                cx.notify();
                            })
                        })
                })
                .child({
                    let s = step.clone();
                    Button::new("stepper-next", "Next")
                        .size(ControlSize::Small)
                        .disabled(current >= 4)
                        .on_click(move |_, _, cx| {
                            s.update(cx, |v, cx| {
                                *v = (*v + 1).min(4);
                                cx.notify();
                            })
                        })
                }),
        )
        .into_any_element()
}

fn skeleton(_w: &mut Window, _cx: &mut App) -> AnyElement {
    h_stack(12.)
        .child(
            Skeleton::new()
                .shape(SkeletonShape::Circle)
                .width(px(48.))
                .height(px(48.)),
        )
        .child(
            v_stack(8.)
                .child(Skeleton::new().shape(SkeletonShape::Text).width(px(260.)))
                .child(Skeleton::new().shape(SkeletonShape::Text).width(px(180.)))
                .child(Skeleton::new().width(px(320.)).height(px(32.))),
        )
        .into_any_element()
}

/// Web `TooltipDemo`: a secondary "Label" trigger; the bubble opens below,
/// start-aligned, with the 16×8 beak (shown statically for the screenshot).
fn tooltip(_w: &mut Window, cx: &mut App) -> AnyElement {
    row()
        .gap(px(32.))
        .items_start()
        .child(
            v_stack(4.)
                .items_start()
                .child(Button::new("tt-static", "Label").variant(ButtonVariant::Secondary))
                .child(tooltip_with_beak("Below", BeakSide::Top, cx)),
        )
        .child(
            v_stack(4.)
                .items_start()
                .child(tooltip_with_beak("Above", BeakSide::Bottom, cx))
                .child(Button::new("tt-above", "Label").variant(ButtonVariant::Secondary)),
        )
        .child(
            row()
                .items_center()
                .gap(px(8.))
                .child(Button::new("tt-right", "Label").variant(ButtonVariant::Secondary))
                .child(tooltip_with_beak("Right", BeakSide::Left, cx)),
        )
        .child(
            row()
                .items_center()
                .gap(px(8.))
                .child(tooltip_with_beak("Left", BeakSide::Right, cx))
                .child(Button::new("tt-left", "Label").variant(ButtonVariant::Secondary)),
        )
        .child(
            div()
                .id("tt-target")
                .child(Button::new("tt-live", "Hover me").variant(ButtonVariant::Secondary))
                .tooltip(Tooltip::text("Tooltip")),
        )
        .child(
            div()
                .id("tt-target-2")
                .child(Button::new("save", "Save").variant(ButtonVariant::Outline))
                .tooltip(Tooltip::with_shortcut("Save", "⌘S")),
        )
        .into_any_element()
}

fn toast(_w: &mut Window, _cx: &mut App) -> AnyElement {
    v_stack(10.)
        .items_center()
        .w(px(400.))
        .children(ToastVariant::ALL.into_iter().enumerate().map(|(i, v)| {
            let data = ToastData::new(match v {
                ToastVariant::Default => "Settings saved",
                ToastVariant::Success => "Export complete",
                ToastVariant::Error => "Upload failed",
                ToastVariant::Warning => "Low disk space",
            })
            .variant(v)
            .duration(Some(Duration::from_secs(4)));
            let data = if i == 2 {
                data.action("Retry", |_, _| {})
            } else {
                data
            };
            Toast::new(("toast", i), data)
        }))
        .into_any_element()
}
