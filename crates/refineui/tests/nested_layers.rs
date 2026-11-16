//! Regression test: overlays nested in overlays (a submenu inside a context
//! menu, a Select inside a Dialog) used `gpui::deferred` twice and panicked
//! with "cannot call defer_draw during deferred drawing".

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};

struct Nested;

impl Render for Nested {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(200.)).child(
            refineui::layer(
                div()
                    .size(px(100.))
                    .child(refineui::layer(div().size(px(50.)).child("inner")).priority(2)),
            )
            .priority(1),
        )
    }
}

#[gpui::test]
fn nested_layers_draw_without_panicking(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| Nested);
    // Force a couple of full draws.
    window.update(cx, |_, window, _| window.refresh()).unwrap();
    cx.run_until_parked();
    window.update(cx, |_, window, _| window.refresh()).unwrap();
    cx.run_until_parked();
}

#[test]
#[should_panic(expected = "defer_draw during deferred drawing")]
fn nested_gpui_deferred_still_panics() {
    // Documents the root cause: plain nested `deferred` is unsupported by GPUI.
    struct Bad;
    impl Render for Bad {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child(gpui::deferred(
                div().child(gpui::deferred(div().child("x"))),
            ))
        }
    }
    let cx = TestAppContext::single();
    let mut cx = cx;
    let window = cx.add_window(|_, _| Bad);
    window
        .update(&mut cx, |_, window, _| window.refresh())
        .unwrap();
    cx.run_until_parked();
}
