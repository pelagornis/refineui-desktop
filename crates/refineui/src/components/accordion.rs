//! Accordion and Collapsible — web `Accordion` (`single | multiple`, `sm | md | lg`)
//! and the headless `Collapsible`.
//!
//! Item: `roundedXLarge`, `backgroundPrimaryHover` while open. Trigger:
//! `controlTouchMin` (44px) min height, `sizeMedium × sizeXSmall` padding,
//! typography `labelMd / bodyLg / subtitleLg`, pressed `backgroundPrimaryActive`
//! (no hover state, per the Web Kit), chevron rotates 180° over `duration-fast`.
//! Panel animates height (380ms) and content opacity / offset (260ms).

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Transformation, Window, div,
    prelude::*, px, radians,
};
use refineui_tokens::{radius, size::icon, spacing, typography::role};

use crate::components::disclosure::collapse_panel;
use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, use_focus_handle, use_interaction,
};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, StyledExt};

type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

struct AccordionItem {
    title: SharedString,
    icon: Option<IconName>,
    content: ContentBuilder,
    disabled: bool,
}

/// Vertically stacked disclosure sections. Uncontrolled: remembers which are open.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    items: Vec<AccordionItem>,
    multiple: bool,
    default_open: BTreeSet<usize>,
    size: ControlSize,
}

impl Accordion {
    /// New accordion (single-open, `md`).
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            multiple: false,
            default_open: BTreeSet::new(),
            size: ControlSize::Medium,
        }
    }

    /// Add a section.
    pub fn item(
        mut self,
        title: impl Into<SharedString>,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.items.push(AccordionItem {
            title: title.into(),
            icon: None,
            content: Rc::new(content),
            disabled: false,
        });
        self
    }

    /// Add a section with a leading icon.
    pub fn item_with_icon(
        mut self,
        icon: IconName,
        title: impl Into<SharedString>,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.items.push(AccordionItem {
            title: title.into(),
            icon: Some(icon),
            content: Rc::new(content),
            disabled: false,
        });
        self
    }

    /// Add a disabled section.
    pub fn disabled_item(mut self, title: impl Into<SharedString>) -> Self {
        self.items.push(AccordionItem {
            title: title.into(),
            icon: None,
            content: Rc::new(|_, _| div().into_any_element()),
            disabled: true,
        });
        self
    }

    /// Allow several open sections (web `type="multiple"`).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Sections open initially.
    pub fn default_open(mut self, ix: impl IntoIterator<Item = usize>) -> Self {
        self.default_open = ix.into_iter().collect();
        self
    }

    /// Size (`sm | md | lg`).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_open = self.default_open.clone();
        let open = window.use_keyed_state(child_id(&self.id, "open"), cx, move |_, _| default_open);
        let open_now = open.read(cx).clone();
        let multiple = self.multiple;
        let size = self.size;
        let parent = self.id.clone();
        let mut rows = Vec::with_capacity(self.items.len());
        for (ix, item) in self.items.into_iter().enumerate() {
            let state = open.clone();
            let is_open = open_now.contains(&ix);
            let item_id =
                ElementId::NamedChild(Box::new(parent.clone()), format!("item-{ix}").into());
            rows.push(disclosure_item(
                &item_id,
                item,
                is_open,
                size,
                Rc::new(move |_, cx: &mut App| {
                    state.update(cx, |set, cx| {
                        if set.contains(&ix) {
                            set.remove(&ix);
                        } else {
                            if !multiple {
                                set.clear();
                            }
                            set.insert(ix);
                        }
                        cx.notify();
                    })
                }),
                window,
                cx,
            ));
        }
        div()
            .id(self.id)
            .w_full()
            .flex()
            .flex_col()
            .gap(px(spacing::X_SMALL))
            .children(rows)
    }
}

type Toggle = Rc<dyn Fn(&mut Window, &mut App)>;

fn disclosure_item(
    id: &ElementId,
    item: AccordionItem,
    is_open: bool,
    size: ControlSize,
    toggle: Toggle,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let c = theme.colors;
    let trigger_id = child_id(id, "trigger");
    let (istate, i) = use_interaction(&trigger_id, window, cx);
    let focus = use_focus_handle(&trigger_id, 0, window, cx);
    let focused = focus.is_focused(window) && focus_visible(cx);
    let (trigger_text, content_text, icon_px) = match size {
        ControlSize::Small => (role::LABEL_MD, role::BODY_SM, icon::X_SMALL),
        ControlSize::Medium => (role::BODY_LG, role::BODY_MD, icon::SMALL),
        ControlSize::Large => (role::SUBTITLE_LG, role::BODY_LG, icon::MEDIUM),
    };
    // Pressed only; open items share the item's hover plate.
    let trigger_bg = if !item.disabled && i.pressed {
        c.background_primary_active
    } else {
        c.background_primary.opacity(0.)
    };
    let trigger_bg = animate(
        child_id(id, "tbg"),
        trigger_bg,
        Motion::FAST_EASE,
        window,
        cx,
    );
    let item_bg = if is_open {
        c.background_primary_hover
    } else {
        c.background_primary_hover.opacity(0.)
    };
    let item_bg = animate(child_id(id, "ibg"), item_bg, Motion::FAST_EASE, window, cx);
    let chevron = animate(
        child_id(id, "chev"),
        if is_open { 1.0f32 } else { 0.0 },
        Motion::FAST,
        window,
        cx,
    );
    let fg = if item.disabled {
        c.foreground_disabled
    } else {
        c.foreground_primary
    };
    let content = item.content.clone();
    let panel = collapse_panel(id, is_open, window, cx, move |w, cx| {
        let c = cx.theme().colors;
        div()
            .px(px(spacing::MEDIUM))
            .pb(px(spacing::MEDIUM))
            .text_style_token(content_text)
            .text_color(c.foreground_secondary)
            .child(content(w, cx))
            .into_any_element()
    });
    let key_toggle = toggle.clone();
    div()
        .w_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(px(radius::X_LARGE))
        .bg(item_bg)
        .child(
            div()
                .id(trigger_id)
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(spacing::MEDIUM))
                .min_h(px(refineui_tokens::size::component::TOUCH_MIN))
                .px(px(spacing::MEDIUM))
                .py(px(spacing::X_SMALL))
                .rounded(px(radius::X_LARGE))
                .bg(trigger_bg)
                .text_color(fg)
                .when(focused, |d| d.shadow(theme.focus_ring()))
                .when(!item.disabled, |d| {
                    d.cursor_action()
                        .track_focus(&focus)
                        .track_interaction(&istate)
                        .on_click(move |_, w, cx| toggle(w, cx))
                        .on_key_down(move |e: &KeyDownEvent, w, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                cx.stop_propagation();
                                key_toggle(w, cx);
                            }
                        })
                })
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap(px(spacing::MEDIUM))
                        .text_style_token(trigger_text)
                        .when_some(item.icon, |d, ic| {
                            d.child(Icon::new(ic).size(IconSize::Custom(px(icon_px))).color(fg))
                        })
                        .child(item.title),
                )
                .child(
                    Icon::new(IconName::ChevronDown)
                        .size(IconSize::Custom(px(icon_px)))
                        .color(fg)
                        .transform(Transformation::rotate(radians(
                            chevron * std::f32::consts::PI,
                        ))),
                ),
        )
        .children(panel)
        .into_any_element()
}

/// Headless show / hide region (web `Collapsible`): click `trigger` to toggle
/// `content`, which expands with the accordion panel motion.
#[derive(IntoElement)]
pub struct Collapsible {
    id: ElementId,
    trigger: AnyElement,
    content: ContentBuilder,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Collapsible {
    /// New collapsible region.
    pub fn new(
        id: impl Into<ElementId>,
        trigger: impl IntoElement,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: trigger.into_any_element(),
            content: Rc::new(content),
            open: None,
            default_open: false,
            on_open_change: None,
        }
    }

    /// Controlled open state.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Initial state when uncontrolled.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Called with the requested state.
    pub fn on_open_change(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_open_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Collapsible {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let initial = self.default_open;
        let state = window.use_keyed_state(child_id(&self.id, "open"), cx, move |_, _| initial);
        let is_open = self.open.unwrap_or(*state.read(cx));
        let content = self.content.clone();
        let panel = collapse_panel(&self.id, is_open, window, cx, move |w, cx| content(w, cx));
        let on_change = self.on_open_change.clone();
        let controlled = self.open.is_some();
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id(child_id(&self.id, "trigger"))
                    .cursor_action()
                    .on_click(move |_, w, cx| {
                        let next = !is_open;
                        if !controlled {
                            state.update(cx, |s, cx| {
                                *s = next;
                                cx.notify();
                            });
                        }
                        if let Some(f) = &on_change {
                            f(&next, w, cx);
                        }
                    })
                    .child(self.trigger),
            )
            .children(panel)
    }
}
