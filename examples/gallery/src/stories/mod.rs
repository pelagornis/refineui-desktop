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
mod navigation;
mod overlays;

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
    /// Web-oriented components behind the `web-extras` feature (not in
    /// Desktop Core).
    Optional,
}

impl StoryGroup {
    /// All groups in sidebar order.
    pub const ALL: [StoryGroup; 9] = [
        Self::Foundations,
        Self::Actions,
        Self::Forms,
        Self::Display,
        Self::Navigation,
        Self::Overlays,
        Self::Data,
        Self::Desktop,
        Self::Optional,
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
            Self::Optional => "Optional (web-extras)",
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

/// Every story, in sidebar order.
pub fn all_stories() -> Vec<Story> {
    let mut v = Vec::new();
    v.extend(foundations::stories());
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
    }
}
