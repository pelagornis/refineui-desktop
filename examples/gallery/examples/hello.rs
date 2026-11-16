//! Minimal RefineUI app: `cargo run -p refineui-gallery --example hello`.

use gpui::{
    App, AppContext, Application, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowOptions, px,
};
use refineui::prelude::*;

struct Hello;

impl Render for Hello {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        refineui::root(cx)
            .p(px(24.))
            .child(Button::new("hello", "Hello RefineUI").on_click(|_, _, _| println!("hi")))
    }
}

fn main() {
    Application::new()
        .with_assets(refineui::Assets) // icons + spinner
        .run(|cx: &mut App| {
            refineui::init(cx); // fonts, theme, key bindings
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Hello)).unwrap();
            cx.activate(true);
        });
}
