use gpui::{AnyElement, App, Entity, IntoElement, ParentElement, Styled, Window, div, px};
use refineui::prelude::*;

use super::{Story, StoryGroup, row, section, setter, state};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "input",
            name: "Input",
            group: StoryGroup::Forms,
            summary: "Single-line text input with selection, clipboard, IME and placeholder.",
            shot: (720., 400.),
            render: input,
        },
        Story {
            id: "textarea",
            name: "Textarea",
            group: StoryGroup::Forms,
            summary: "Multi-line text input with soft wrap.",
            shot: (720., 200.),
            render: textarea,
        },
        Story {
            id: "search-field",
            name: "Search Field",
            group: StoryGroup::Forms,
            summary: "Filled search input with clear button; Esc clears.",
            shot: (720., 300.),
            render: search_field,
        },
        Story {
            id: "input-otp",
            name: "Input OTP",
            group: StoryGroup::Forms,
            summary: "One-time code cells with a skeleton pulse on empty slots.",
            shot: (560., 440.),
            render: input_otp,
        },
        Story {
            id: "field",
            name: "Field",
            group: StoryGroup::Forms,
            summary: "Label, hint and validation message around a control.",
            shot: (720., 230.),
            render: field,
        },
        Story {
            id: "checkbox",
            name: "Checkbox",
            group: StoryGroup::Forms,
            summary: "Checked, unchecked and indeterminate checkboxes.",
            shot: (620., 200.),
            render: checkbox,
        },
        Story {
            id: "radio",
            name: "Radio",
            group: StoryGroup::Forms,
            summary: "Single choice from a small set.",
            shot: (620., 200.),
            render: radio,
        },
        Story {
            id: "switch",
            name: "Switch",
            group: StoryGroup::Forms,
            summary: "Immediate on/off setting.",
            shot: (560., 150.),
            render: switch,
        },
        Story {
            id: "slider",
            name: "Slider",
            group: StoryGroup::Forms,
            summary: "Pick a value from a continuous or stepped range.",
            shot: (560., 250.),
            render: slider,
        },
        Story {
            id: "spin-button",
            name: "Spin Button",
            group: StoryGroup::Forms,
            summary: "Numeric stepper with keyboard increments.",
            shot: (560., 130.),
            render: spin_button,
        },
        Story {
            id: "select",
            name: "Select",
            group: StoryGroup::Forms,
            summary: "Pick one option from a grouped popup list (web Select). The popup covers the trigger with the selected row on the value, shifting and scrolling to stay in the window.",
            shot: (720., 560.),
            render: select,
        },
    ]
}

fn text_input(
    key: &'static str,
    w: &mut Window,
    cx: &mut App,
    f: impl FnOnce(TextInput) -> TextInput,
) -> Entity<TextInput> {
    w.use_keyed_state(key, cx, |_, cx| f(TextInput::new(cx)))
}

/// Web `InputDemo`: a 325px `lg` outline input ("Email address"), then
/// sizes and states.
fn input(w: &mut Window, cx: &mut App) -> AnyElement {
    let email = text_input("in-email", w, cx, |i| i.placeholder("Email address"));
    let filled = text_input("in-filled", w, cx, |i| {
        i.placeholder("Filled appearance")
            .appearance(InputAppearance::Filled)
    });
    let password = text_input("in-pass", w, cx, |i| {
        i.default_value("hunter2")
            .masked(true)
            .leading_icon(IconName::LockClosed)
    });
    let small = text_input("in-small", w, cx, |i| {
        i.placeholder("Small").size(ControlSize::Small)
    });
    let medium = text_input("in-md", w, cx, |i| {
        i.placeholder("Medium").size(ControlSize::Medium)
    });
    let large = text_input("in-large", w, cx, |i| i.placeholder("Large"));
    let invalid = text_input("in-invalid", w, cx, |i| {
        i.default_value("not-an-email").invalid(true)
    });
    let disabled = text_input("in-disabled", w, cx, |i| {
        i.placeholder("Disabled").disabled(true)
    });
    let wide = |e: Entity<TextInput>| div().w(px(325.)).child(e);
    let cell = |e: Entity<TextInput>| div().w(px(210.)).child(e);
    row()
        .items_start()
        .gap(px(32.))
        .child(
            v_stack(16.)
                .child(section("Default", wide(email), cx))
                .child(section(
                    "Sizes",
                    v_stack(8.)
                        .child(wide(small))
                        .child(wide(medium))
                        .child(wide(large)),
                    cx,
                )),
        )
        .child(section(
            "Appearance & states",
            v_stack(8.)
                .child(cell(filled))
                .child(cell(password))
                .child(cell(invalid))
                .child(cell(disabled)),
            cx,
        ))
        .into_any_element()
}

fn textarea(w: &mut Window, cx: &mut App) -> AnyElement {
    let a = w.use_keyed_state("ta-a", cx, |_, cx| {
        Textarea::new(cx).placeholder("Write a message…")
    });
    let b = w.use_keyed_state("ta-b", cx, |_, cx| {
        Textarea::new(cx).default_value(
            "RefineUI for GPUI renders the same tokens, states and motion as the web kit.",
        )
    });
    let c = w.use_keyed_state("ta-c", cx, |_, cx| {
        Textarea::new(cx).placeholder("Disabled").disabled(true)
    });
    row()
        .items_start()
        .gap(px(16.))
        .child(div().w(px(210.)).child(a))
        .child(div().w(px(210.)).child(b))
        .child(div().w(px(210.)).child(c))
        .into_any_element()
}

/// Web `SearchFieldDemo`: a 325px default (`md`) field, then sm / md / lg
/// stacked.
fn search_field(w: &mut Window, cx: &mut App) -> AnyElement {
    let default = w.use_keyed_state("sf-default", cx, |_, cx| {
        SearchField::new(cx).placeholder("Search")
    });
    let sizes = [
        (ControlSize::Small, "Small"),
        (ControlSize::Medium, "Medium"),
        (ControlSize::Large, "Large"),
    ];
    let fields: Vec<_> = sizes
        .into_iter()
        .enumerate()
        .map(|(i, (z, label))| {
            w.use_keyed_state(("sf", i), cx, move |_, cx| {
                SearchField::new(cx).size(z).placeholder(label)
            })
        })
        .collect();
    let disabled = w.use_keyed_state("sf-dis", cx, |_, cx| {
        SearchField::new(cx).placeholder("Disabled").disabled(true)
    });
    row()
        .items_start()
        .gap(px(32.))
        .child(
            v_stack(16.)
                .child(section("Default", div().w(px(325.)).child(default), cx))
                .child(section(
                    "Sizes",
                    v_stack(8.).children(fields.into_iter().map(|f| div().w(px(325.)).child(f))),
                    cx,
                )),
        )
        .child(section("Disabled", div().w(px(210.)).child(disabled), cx))
        .into_any_element()
}

fn input_otp(_w: &mut Window, cx: &mut App) -> AnyElement {
    v_stack(16.)
        .child(section(
            "Default (lg, empty slots pulse)",
            InputOtp::new("otp-default"),
            cx,
        ))
        .child(section(
            "Sizes",
            v_stack(10.)
                .child(
                    InputOtp::new("otp-sm")
                        .size(ControlSize::Small)
                        .length(4)
                        .default_value("12"),
                )
                .child(
                    InputOtp::new("otp-md")
                        .size(ControlSize::Medium)
                        .length(4)
                        .default_value("20"),
                )
                .child(
                    InputOtp::new("otp-lg")
                        .size(ControlSize::Large)
                        .length(4)
                        .default_value("2026"),
                ),
            cx,
        ))
        .child(section(
            "Disabled",
            InputOtp::new("otp-dis")
                .length(4)
                .default_value("42")
                .disabled(true),
            cx,
        ))
        .into_any_element()
}

fn field(w: &mut Window, cx: &mut App) -> AnyElement {
    let name = text_input("field-name", w, cx, |i| i.placeholder("Ada Lovelace"));
    let email = text_input("field-email", w, cx, |i| {
        i.default_value("ada@").invalid(true)
    });
    row()
        .items_start()
        .gap(px(24.))
        .child(
            div().w(px(300.)).child(
                Field::new()
                    .label("Full name")
                    .required(true)
                    .hint("Shown on your profile.")
                    .control(name),
            ),
        )
        .child(
            div().w(px(300.)).child(
                Field::new()
                    .label("Email")
                    .error("Enter a valid email address.")
                    .control(email),
            ),
        )
        .into_any_element()
}

fn checkbox(w: &mut Window, cx: &mut App) -> AnyElement {
    let a = state("cb-a", w, cx, || true);
    let b = state("cb-b", w, cx, || false);
    let c = state("cb-c", w, cx, || true);
    v_stack(16.)
        .child(section(
            "States",
            row()
                .gap(px(20.))
                .child(
                    Checkbox::new("a", *a.read(cx))
                        .label("Checked")
                        .on_change(setter(&a)),
                )
                .child(
                    Checkbox::new("b", *b.read(cx))
                        .label("Unchecked")
                        .on_change(setter(&b)),
                )
                .child(Checkbox::new("i", CheckState::Indeterminate).label("Indeterminate"))
                .child(Checkbox::new("d", true).label("Disabled").disabled(true)),
            cx,
        ))
        .child(section(
            "With description",
            Checkbox::new("c", *c.read(cx))
                .circular(true)
                .label("Sync settings")
                .description("Keep preferences in sync across devices.")
                .on_change(setter(&c)),
            cx,
        ))
        .into_any_element()
}

fn radio(w: &mut Window, cx: &mut App) -> AnyElement {
    let v = state("radio-v", w, cx, || 1usize);
    let h = state("radio-h", w, cx, || 0usize);
    let sel = *v.read(cx);
    let hs = *h.read(cx);
    row()
        .items_start()
        .gap(px(48.))
        .child(section(
            "Vertical",
            RadioGroup::new("rg-v", ["System", "Light", "Dark"], Some(sel)).on_change(setter(&v)),
            cx,
        ))
        .child(section(
            "Horizontal",
            RadioGroup::new("rg-h", ["Small", "Medium", "Large"], Some(hs))
                .horizontal()
                .on_change(setter(&h)),
            cx,
        ))
        .child(section(
            "Disabled",
            RadioGroup::new("rg-d", ["Option A", "Option B"], Some(0)).disabled(true),
            cx,
        ))
        .into_any_element()
}

fn switch(w: &mut Window, cx: &mut App) -> AnyElement {
    let a = state("sw-a", w, cx, || true);
    let b = state("sw-b", w, cx, || false);
    row()
        .gap(px(24.))
        .child(
            Switch::new("a", *a.read(cx))
                .label("Wi-Fi")
                .on_change(setter(&a)),
        )
        .child(
            Switch::new("b", *b.read(cx))
                .label("Bluetooth")
                .on_change(setter(&b)),
        )
        .child(Switch::new("c", true).label("Disabled").disabled(true))
        .into_any_element()
}

fn slider(w: &mut Window, cx: &mut App) -> AnyElement {
    let a = state("sl-a", w, cx, || 40.0f32);
    let b = state("sl-b", w, cx, || 3.0f32);
    let av = *a.read(cx);
    let bv = *b.read(cx);
    v_stack(16.)
        .w(px(420.))
        .child(section(
            format!("Continuous · {av:.0}"),
            Slider::new("a", av).range(0., 100.).on_change(setter(&a)),
            cx,
        ))
        .child(section(
            format!("Stepped · {bv:.0}"),
            Slider::new("b", bv)
                .range(0., 10.)
                .step(1.)
                .small(true)
                .on_change(setter(&b)),
            cx,
        ))
        .child(section(
            "Disabled",
            Slider::new("c", 60.).range(0., 100.).disabled(true),
            cx,
        ))
        .into_any_element()
}

fn spin_button(w: &mut Window, cx: &mut App) -> AnyElement {
    let a = state("spin-a", w, cx, || 12.0f64);
    let b = state("spin-b", w, cx, || 1.5f64);
    row()
        .gap(px(16.))
        .child(
            SpinButton::new("a", *a.read(cx))
                .range(0., 99.)
                .on_change(setter(&a)),
        )
        .child(
            SpinButton::new("b", *b.read(cx))
                .step(0.25)
                .precision(2)
                .suffix(" s")
                .on_change(setter(&b)),
        )
        .child(SpinButton::new("c", 4.).disabled(true))
        .into_any_element()
}

/// Web `SelectPreview` groups (North America / Asia / Europe).
fn timezone_select(id: &'static str, selected: Option<usize>) -> Select {
    Select::new(id, [] as [&str; 0], selected)
        .placeholder("Timezone")
        .group(
            "North America",
            [
                "Eastern Standard Time",
                "Central Standard Time",
                "Mountain Standard Time",
                "Pacific Standard Time",
                "Alaska Standard Time",
                "Hawaii Standard Time",
            ],
        )
        .group(
            "Asia",
            [
                "India Standard Time",
                "China Standard Time",
                "Japan Standard Time",
                "Korea Standard Time",
                "Indonesia Central Standard Time",
                "Singapore Standard Time",
                "Philippine Standard Time",
            ],
        )
        .group(
            "Europe",
            [
                "Greenwich Mean Time",
                "Central European Time",
                "Eastern European Time",
                "Western European Summer Time",
                "Turkey Time",
                "Moscow Time",
            ],
        )
}

fn select(w: &mut Window, cx: &mut App) -> AnyElement {
    let a = state("sel-a", w, cx, || Some(1usize));
    let b = state("sel-b", w, cx, || None::<usize>);
    let set_a = {
        let a = a.clone();
        move |ix: &usize, _: &mut Window, cx: &mut App| {
            a.update(cx, |s, cx| {
                *s = Some(*ix);
                cx.notify();
            })
        }
    };
    let set_b = {
        let b = b.clone();
        move |ix: &usize, _: &mut Window, cx: &mut App| {
            b.update(cx, |s, cx| {
                *s = Some(*ix);
                cx.notify();
            })
        }
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            row()
                .items_start()
                .gap(px(16.))
                .child(
                    // Web docs preview: grouped timezones, placeholder, no
                    // default value. The popup covers the trigger with the
                    // aligned row on the value (web item-aligned) and
                    // shifts / shrinks / scrolls when space is tight.
                    timezone_select("open", None)
                        .width(px(280.))
                        .default_open(true),
                )
                .child(
                    timezone_select("font", *a.read(cx))
                        .width(px(260.))
                        .on_change(set_a),
                )
                .child(
                    Select::new(
                        "weight",
                        ["Regular", "Medium", "Semibold", "Bold"],
                        *b.read(cx),
                    )
                    .placeholder("Weight")
                    .appearance(SelectAppearance::Ghost)
                    .width(px(160.))
                    .on_change(set_b),
                )
                .child(
                    timezone_select("dis", Some(0))
                        .disabled(true)
                        .width(px(160.)),
                ),
        )
        .into_any_element()
}
