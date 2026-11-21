//! Component stories: one entry per component, rendered by the gallery and by
//! `refineui-screenshots` (which captures each story's showcase in isolation).

use gpui::{
    AnyElement, App, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
};
use refineui::prelude::*;

mod actions;
mod data;
mod desktop;
mod display;
mod extra;
mod forms;
mod foundations;
mod materials;
mod navigation;
mod overlays;

pub(crate) use desktop::DESKTOP_SHOWCASE_SIZE;

/// Sidebar group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoryGroup {
    /// Tokens.
    Foundations,
    /// Buttons and friends.
    Actions,
    /// Form controls.
    Forms,
    /// Status and content display.
    Display,
    /// Navigation.
    Navigation,
    /// Popups and modals.
    Overlays,
    /// Data.
    Data,
    /// Desktop shell chrome and interaction (Phase 2).
    Desktop,
}

impl StoryGroup {
    /// All groups in sidebar order.
    pub const ALL: [StoryGroup; 8] = [
        Self::Foundations,
        Self::Actions,
        Self::Forms,
        Self::Display,
        Self::Navigation,
        Self::Overlays,
        Self::Data,
        Self::Desktop,
    ];

    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Foundations => "Foundations",
            Self::Actions => "Actions",
            Self::Forms => "Forms",
            Self::Display => "Display",
            Self::Navigation => "Navigation",
            Self::Overlays => "Overlays",
            Self::Data => "Data",
            Self::Desktop => "Desktop shell",
        }
    }
}

/// One component page.
#[derive(Clone)]
pub struct Story {
    /// Kebab-case id; also the screenshot / docs slug.
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Sidebar group.
    pub group: StoryGroup,
    /// One-line summary.
    pub summary: &'static str,
    /// Screenshot canvas size in logical px (content is padded by 24px).
    pub shot: (f32, f32),
    /// Showcase renderer (variants and states).
    pub render: fn(&mut Window, &mut App) -> AnyElement,
}

/// Stories whose fixed showcase is wider than the gallery Preview card in
/// the default 1200×820 window (sidebar 260, body padding 48, preview
/// padding 40: ~764px inner). The gallery shows a launch button that
/// opens the story in a separate window; `refineui-screenshots` and the
/// visual-regression tests still call [`Story::render`] directly.
///
/// Measured content widths (not screenshot canvas):
/// `desktop-showcase` / `application-shell` 1280, `preferences-window` 720,
/// `titlebar` 840, `status-bar` 820, `notification-center` 772.
/// Not included: `activity-bar` (three rails, comfortably under 764),
/// `menu-bar` (660), `drag-and-drop` (340+24+400 = 764, flush with the
/// preview), `result` (696).
impl Story {
    /// Whether the gallery should open this story in an external window.
    pub fn opens_in_window(&self) -> bool {
        Self::EXTERNAL_WINDOW_IDS.contains(&self.id)
    }

    /// These stories are real app windows: they fill the OS window
    /// edge-to-edge (no padded host chrome, no fake frame). Screenshots
    /// use [`Story::render`] at the story's content size without the
    /// usual 24px shot padding.
    pub fn fills_external_window(&self) -> bool {
        matches!(
            self.id,
            "desktop-showcase" | "preferences-window" | "application-shell"
        )
    }

    /// Render this story as the root of an external window.
    pub fn render_in_window(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        match self.id {
            "desktop-showcase" => desktop::desktop_showcase_app(window, cx),
            "preferences-window" => desktop::preferences_window_app(window, cx),
            "application-shell" => desktop::application_shell_app(window, cx),
            _ => (self.render)(window, cx),
        }
    }

    /// Story ids that use the external-window Preview button.
    pub const EXTERNAL_WINDOW_IDS: &'static [&'static str] = &[
        "application-shell",
        "titlebar",
        "status-bar",
        "notification-center",
        "preferences-window",
        "desktop-showcase",
    ];
}

/// Every story, in sidebar order.
pub fn all_stories() -> Vec<Story> {
    let mut v = Vec::new();
    v.extend(foundations::stories());
    v.extend(materials::stories());
    v.extend(actions::stories());
    v.extend(forms::stories());
    v.extend(display::stories());
    v.extend(navigation::stories());
    v.extend(overlays::stories());
    v.extend(data::stories());
    v.extend(desktop::stories());
    v.extend(extra::stories());
    v
}

// ---- helpers shared by stories ------------------------------------------

/// A labelled row of examples.
pub(crate) fn section(
    title: impl Into<SharedString>,
    content: impl IntoElement,
    cx: &App,
) -> gpui::Div {
    v_stack(8.)
        .child(
            div()
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(cx.theme().colors.foreground_tertiary)
                .child(title.into()),
        )
        .child(content)
}

/// Wrapping row.
pub(crate) fn row() -> gpui::Div {
    div().flex().flex_wrap().items_center().gap(px(8.))
}

/// Keyed state helper.
pub(crate) fn state<T: 'static>(
    key: &'static str,
    window: &mut Window,
    cx: &mut App,
    init: impl FnOnce() -> T,
) -> Entity<T> {
    window.use_keyed_state(key, cx, |_, _| init())
}

/// Setter callback for keyed state.
pub(crate) fn setter<T: Clone + 'static>(
    e: &Entity<T>,
) -> impl Fn(&T, &mut Window, &mut App) + 'static {
    let e = e.clone();
    move |v: &T, _: &mut Window, cx: &mut App| {
        e.update(cx, |s, cx| {
            *s = v.clone();
            cx.notify();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_ids_are_unique_kebab_case() {
        let stories = all_stories();
        let mut seen = std::collections::HashSet::new();
        for s in &stories {
            assert!(seen.insert(s.id), "duplicate story id {}", s.id);
            assert!(
                s.id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "story id {} must be kebab-case (it is the docs slug)",
                s.id
            );
            assert!(s.shot.0 > 0. && s.shot.1 > 0.);
        }
        assert!(stories.len() >= 60);
        let ids: Vec<_> = stories.iter().map(|s| s.id).collect();
        for id in Story::EXTERNAL_WINDOW_IDS {
            assert!(ids.contains(id), "external-window story id {id} is missing");
        }
        assert_eq!(
            stories.iter().filter(|s| s.opens_in_window()).count(),
            Story::EXTERNAL_WINDOW_IDS.len()
        );
    }
}
