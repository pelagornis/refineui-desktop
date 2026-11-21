//! CodeBlock — web `CodeBlock` (`components/CodeBlock/style.ts`).
//!
//! Root: `roundedXLarge` (12px), hairline `borderSubtle`, `surfaceSunken`,
//! clipped. Header: `px-x-large` / `pt-x-large` / `pb-large` (20 / 20 / 16px),
//! `gap-large`, title `label2` in `foregroundPrimary` (with an optional
//! icon, `gap-x-small`), actions on the end (`gap-xxx-small`) — the copy
//! button is a small ghost icon Button that shows a check for 1.6s.
//! Content: `body1` monospace with relaxed leading (1.625), `py-x-large`;
//! each line `px-x-large` with `gap-large`; highlighted lines use
//! `backgroundBrandSubtle`; line numbers are 20px wide, end-aligned,
//! `foregroundTertiary`. `wrap(false)` (default) scrolls horizontally.

use std::time::{Duration, Instant};

use gpui::{
    App, ClipboardItem, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, px,
};
use refineui_foundation::{radius, spacing, stroke, typography::role};

use crate::motion::child_id;
use crate::{ActiveTheme, Button, ButtonVariant, ControlSize, Icon, IconName, IconSize, StyledExt};

/// How long the copy button shows its "copied" check (web 1600ms).
pub const CODE_BLOCK_COPIED_FOR: Duration = Duration::from_millis(1600);

/// A titled, copyable block of code.
#[derive(IntoElement)]
pub struct CodeBlock {
    id: ElementId,
    code: SharedString,
    title: Option<SharedString>,
    icon: Option<IconName>,
    line_numbers: bool,
    highlighted: Vec<usize>,
    wrap: bool,
    copy: bool,
    language: Option<SharedString>,
}

impl CodeBlock {
    /// New block for `code`.
    pub fn new(id: impl Into<ElementId>, code: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            code: code.into(),
            title: None,
            icon: None,
            line_numbers: false,
            highlighted: Vec::new(),
            wrap: false,
            copy: true,
            language: None,
        }
    }

    /// Header title (file name, language).
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Header icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Show 1-based line numbers.
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Highlight these 1-based lines (web `CodeBlockLine variant="highlighted"`).
    pub fn highlight(mut self, lines: impl IntoIterator<Item = usize>) -> Self {
        self.highlighted = lines.into_iter().collect();
        self
    }

    /// Wrap long lines instead of scrolling.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Syntax-highlight as `lang` (`rust`, `ts`, `tsx`, `js`, `sh`, `json`,
    /// `css`, `html`, `py`, `yaml`, `md`, …) with the web docs' Night Owl
    /// Light / Night Owl colours. Unknown languages render plain.
    pub fn language(mut self, lang: impl Into<SharedString>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Show the copy button (default `true`).
    pub fn copy_button(mut self, on: bool) -> Self {
        self.copy = on;
        self
    }
}

impl RenderOnce for CodeBlock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let copied_at =
            window.use_keyed_state(child_id(&self.id, "copied"), cx, |_, _| None::<Instant>);
        let copied = copied_at
            .read(cx)
            .is_some_and(|t| t.elapsed() < CODE_BLOCK_COPIED_FOR);
        if copied {
            window.request_animation_frame();
        }
        let text = role::BODY_LG;
        let leading = (text.size * 1.625).round();
        let code = self.code.clone();
        let header = (self.title.is_some() || self.copy).then(|| {
            div()
                .flex()
                .min_w_0()
                .flex_none()
                .items_center()
                .gap(px(spacing::LARGE))
                .px(px(spacing::X_LARGE))
                .pt(px(spacing::X_LARGE))
                .pb(px(spacing::LARGE))
                .text_color(c.foreground_secondary)
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap(px(spacing::X_SMALL))
                        .text_style_token(role::LABEL_MD)
                        .text_color(c.foreground_primary)
                        .when_some(self.icon, |d, i| {
                            d.child(
                                Icon::new(i)
                                    .size(IconSize::XSmall)
                                    .color(c.foreground_secondary),
                            )
                        })
                        .children(self.title.clone()),
                )
                .when(self.copy, |d| {
                    let state = copied_at.clone();
                    d.child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap(px(spacing::XXX_SMALL))
                            .child(
                                Button::icon_only(
                                    child_id(&self.id, "copy"),
                                    if copied {
                                        IconName::Checkmark
                                    } else {
                                        IconName::Copy
                                    },
                                )
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Small)
                                .tooltip(if copied { "Copied" } else { "Copy code" })
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        code.to_string(),
                                    ));
                                    state.update(cx, |t, cx| {
                                        *t = Some(Instant::now());
                                        cx.notify();
                                    });
                                }),
                            ),
                    )
                })
        });
        let numbers = self.line_numbers;
        let wrap = self.wrap;
        let dark = theme.is_dark();
        let runs = self
            .language
            .as_ref()
            .and_then(|lang| cached_highlight(&self.code, lang, dark));
        let code_fg = runs
            .as_ref()
            .map(|_| crate::components::code_highlight::code_foreground(dark));
        let lines = self.code.lines().enumerate().map(|(i, line)| {
            let styled = runs.as_ref().and_then(|r| r.get(i)).map(|r| {
                gpui::StyledText::new(SharedString::from(line.to_string()))
                    .with_highlights(r.iter().cloned())
            });
            let n = i + 1;
            div()
                .flex()
                .min_w_full()
                .gap(px(spacing::LARGE))
                .px(px(spacing::X_LARGE))
                .when(self.highlighted.contains(&n), |d| {
                    d.bg(c.background_brand_subtle)
                })
                .when(numbers, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .w(px(spacing::X_LARGE))
                            .text_right()
                            .text_color(c.foreground_tertiary)
                            .child(n.to_string()),
                    )
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .when(!wrap, |d| d.whitespace_nowrap())
                        // Keep empty lines one line tall.
                        .when_some(code_fg, |d, fg| d.text_color(fg))
                        .map(|d| match styled {
                            Some(t) if !line.is_empty() => d.child(t),
                            // Keep empty lines one line tall.
                            _ => d.child(if line.is_empty() {
                                " ".to_string()
                            } else {
                                line.to_string()
                            }),
                        }),
                )
        });
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .overflow_hidden()
            .rounded(px(radius::X_LARGE))
            .border(px(stroke::HAIRLINE))
            .border_color(c.border_subtle)
            .bg(c.surface_sunken)
            .text_color(c.foreground_primary)
            .children(header)
            .child(
                div()
                    .id(child_id(&self.id, "content"))
                    .when(!wrap, |d| d.overflow_x_scroll())
                    .py(px(spacing::X_LARGE))
                    .font_family(theme.mono_font_family.clone())
                    .text_size(px(text.size))
                    .line_height(px(leading))
                    .child(div().flex().flex_col().min_w_full().children(lines)),
            )
    }
}

type Runs = std::rc::Rc<Vec<Vec<crate::components::code_highlight::CodeRun>>>;

/// Highlighting is a full grammar parse; cache the last few blocks per thread
/// (keyed by code, language and appearance) so re-renders are free.
fn cached_highlight(code: &str, lang: &str, dark: bool) -> Option<Runs> {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    thread_local! {
        static CACHE: RefCell<HashMap<u64, Option<Runs>>> = RefCell::new(HashMap::new());
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (code, lang, dark).hash(&mut h);
    let key = h.finish();
    CACHE.with(|c| {
        if let Some(hit) = c.borrow().get(&key) {
            return hit.clone();
        }
        let value =
            crate::components::code_highlight::highlight(code, lang, dark).map(std::rc::Rc::new);
        let mut c = c.borrow_mut();
        if c.len() > 64 {
            c.clear();
        }
        c.insert(key, value.clone());
        value
    })
}
