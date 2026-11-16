//! Every gallery story must render in a debug build.
//!
//! GPUI guards several element APIs with `debug_assert!`s that only fire while
//! an element is built (e.g. a second `on_hover` / `tooltip` / `on_drag` on one
//! element). Release-mode screenshots never hit them, so a story can look fine
//! in the docs yet panic the moment it opens in `cargo run` — the Command
//! crash (Command rows combined `track_interaction`, which installs
//! `on_hover`, with a second `on_hover`). This opens each story in a test
//! window in both appearances and draws a few frames.

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window};
use refineui::{Appearance, Theme};
use refineui_gallery::{Story, all_stories};

struct Host {
    story: Story,
}

impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = (self.story.render)(window, cx);
        refineui::root(cx).size_full().child(content)
    }
}

#[gpui::test]
fn every_story_renders_in_both_appearances(cx: &mut TestAppContext) {
    cx.update(refineui::init);
    for appearance in [Appearance::Light, Appearance::Dark] {
        cx.update(|cx| Theme::set_appearance(appearance, cx));
        for story in all_stories() {
            let window = cx.add_window(|_, _| Host {
                story: story.clone(),
            });
            // A few more frames: presence / probe-driven layouts re-render.
            for _ in 0..3 {
                window.update(cx, |_, window, _| window.refresh()).unwrap();
                cx.run_until_parked();
            }
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap();
            cx.run_until_parked();
        }
    }
}

/// The `refineui-shell` demo view (every shell part at once) renders and
/// survives its view toggles and the command palette in a debug build.
#[gpui::test]
fn shell_demo_renders(cx: &mut TestAppContext) {
    use refineui_gallery::{demo, shell_demo::ShellDemo};
    cx.update(|cx| {
        refineui::init(cx);
        demo::install(cx);
    });
    let path = std::env::temp_dir().join("refineui-shell-test/window.state");
    let window = cx.add_window(|window, cx| ShellDemo::new(path, window, cx));
    cx.run_until_parked();
    for action in [
        "demo::ToggleSidebar",
        "demo::TogglePanel",
        "demo::ToggleStatusBar",
        "demo::NextTab",
        "demo::CloseTab",
        "demo::CycleDensity",
        "demo::ToggleAppearance",
        "command_palette::ToggleCommandPalette",
        "command_palette::ToggleCommandPalette",
        "demo::ToggleSidebar",
        "demo::TogglePanel",
    ] {
        window
            .update(cx, |_, window, cx| {
                let a = cx.build_action(action, None).expect(action);
                window.dispatch_action(a, cx);
                window.refresh();
            })
            .unwrap();
        cx.run_until_parked();
    }
    window
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
}
