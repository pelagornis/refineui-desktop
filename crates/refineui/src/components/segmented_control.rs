//! SegmentedControl — web `SegmentedControl`.

use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use gpui::CursorStyle;
use refineui_tokens::{spacing, typography::role};

use crate::motion::{
    Motion, TrackInteraction, animate, child_id, focus_visible, probe, use_interaction,
    use_layout_probe,
};
use crate::{ActiveTheme, ControlSize, Elevation, Icon, IconName, IconSize, StyledExt};

/// One segment.
#[derive(Clone)]
pub struct Segment {
    label: SharedString,
    icon: Option<IconName>,
}

impl<T: Into<SharedString>> From<T> for Segment {
    fn from(label: T) -> Self {
        Self {
            label: label.into(),
            icon: None,
        }
    }
}

impl Segment {
    /// Segment with an icon.
    pub fn with_icon(label: impl Into<SharedString>, icon: IconName) -> Self {
        Self {
            label: label.into(),
            icon: Some(icon),
        }
    }
}

/// A row of mutually exclusive segments (web `SegmentedControl`).
///
/// Pill track (`backgroundBrandSubtle`, 1px `borderDefault`) with a brand
/// indicator (`shadow-2`) that slides to the selected item — transform over
/// `duration-medium`, width over `duration-slow`, both emphasized. Selected text
/// is `foregroundOnBrand`; others `foregroundSecondary`, hover `foregroundPrimary`.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    segments: Vec<Segment>,
    selected: usize,
    size: ControlSize,
    full_width: bool,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl SegmentedControl {
    /// New control.
    pub fn new(
        id: impl Into<ElementId>,
        segments: impl IntoIterator<Item = impl Into<Segment>>,
        selected: usize,
    ) -> Self {
        Self {
            id: id.into(),
            segments: segments.into_iter().map(Into::into).collect(),
            selected,
            size: ControlSize::Medium,
            full_width: false,
            disabled: false,
            on_change: None,
        }
    }

    /// Size (`md` is the web control; `sm` / `lg` adjust block padding).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Stretch segments to fill the width.
    pub fn full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }

    /// Disable every segment.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the chosen index.
    pub fn on_change(mut self, f: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for SegmentedControl {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let parent = self.id.clone();
        let probe_state = use_layout_probe(&self.id, window, cx);
        let measured = probe_state.read(cx).relative(self.selected);
        let py = match self.size {
            ControlSize::Small => spacing::XX_SMALL,
            ControlSize::Medium => spacing::X_SMALL,
            ControlSize::Large => spacing::MEDIUM,
        };
        // Indicator hover → `selectedHoverBackground`.
        let selected_id = child_id(&self.id, "seg").clone();
        let sel_item_id =
            ElementId::NamedChild(Box::new(selected_id), format!("{}", self.selected).into());
        let (_, sel_i) = use_interaction(&sel_item_id, window, cx);
        let indicator_bg = if self.disabled {
            c.background_surface_disabled
        } else if sel_i.hovered {
            c.background_brand_hover
        } else {
            c.background_brand
        };
        let indicator_bg = animate(
            child_id(&self.id, "ind-bg"),
            indicator_bg,
            Motion::FAST,
            window,
            cx,
        );
        let indicator = measured.map(|b| {
            let left = animate(
                child_id(&self.id, "ind-x"),
                b.left(),
                Motion::INDICATOR,
                window,
                cx,
            );
            let width = animate(
                child_id(&self.id, "ind-w"),
                b.size.width,
                Motion::INDICATOR_WIDTH,
                window,
                cx,
            );
            (left, width)
        });
        let ready = indicator.is_some();
        let track = probe_state.read(cx).container;
        let focus_ring = theme.focus_ring_color(c.border_focus);
        let show_focus = focus_visible(cx);
        let mut items = Vec::new();
        for (ix, seg) in self.segments.into_iter().enumerate() {
            let selected = ix == self.selected;
            let item_id =
                ElementId::NamedChild(Box::new(child_id(&parent, "seg")), format!("{ix}").into());
            let (state, i) = use_interaction(&item_id, window, cx);
            let fg = if self.disabled {
                c.foreground_disabled
            } else if selected {
                c.foreground_on_brand
            } else if i.hovered {
                c.foreground_primary
            } else {
                c.foreground_secondary
            };
            let fg = animate(child_id(&item_id, "fg"), fg, Motion::FAST, window, cx);
            let handler = self.on_change.clone().filter(|_| !self.disabled);
            let ring = focus_ring.clone();
            items.push(
                div()
                    .id(item_id)
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(spacing::XX_SMALL))
                    .when(self.full_width, |d| d.flex_1())
                    .px(px(spacing::MEDIUM))
                    .py(px(py))
                    .rounded_full()
                    .text_style_token(role::CAPTION_LG)
                    .whitespace_nowrap()
                    .text_color(fg)
                    // Pre-measure fallback (CSS `:not([data-indicator-ready])`).
                    .when(selected && !ready, |d| {
                        d.bg(indicator_bg).shadow(theme.shadow(Elevation::SURFACE))
                    })
                    .when(!self.disabled, |d| {
                        d.cursor_action()
                            .track_interaction(&state)
                            .tab_index(0)
                            .when(show_focus, |d| d.focus(move |s| s.shadow(ring)))
                    })
                    .when(self.disabled, |d| d.cursor(CursorStyle::OperationNotAllowed))
                    .when_some(seg.icon, |d, icon| {
                        d.child(Icon::new(icon).size(IconSize::XSmall).color(fg))
                    })
                    .child(seg.label)
                    .child(probe(&probe_state, Some(ix)))
                    .when_some(handler, |d, h| d.on_click(move |_, w, cx| h(&ix, w, cx))),
            );
        }
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_none()
            .when(self.full_width, |d| d.w_full())
            .rounded_full()
            .border_1()
            .border_color(c.border_default)
            .bg(c.background_brand_subtle)
            .overflow_hidden()
            .child(probe(&probe_state, None))
            .when_some(indicator, |d, (left, width)| {
                // GPUI clips `overflow_hidden` to the track's *rectangle*, not
                // its rounded shape (the web clips to the pill). A thumb shadow
                // at either end would bleed past the rounded ends as a gray
                // sliver, so it fades out as the thumb reaches an end (where
                // the web clips it away anyway).
                let track_w = track.map(|b| f32::from(b.size.width)).unwrap_or(f32::MAX);
                let track_h = track
                    .map(|b| f32::from(b.size.height))
                    .unwrap_or(1.)
                    .max(1.);
                let l = f32::from(left);
                let gap = l.min(track_w - l - f32::from(width)).max(0.);
                let fade = (gap / (track_h / 2.)).clamp(0., 1.);
                let shadow: Vec<_> = theme
                    .shadow(Elevation::SURFACE)
                    .into_iter()
                    .map(|mut s| {
                        s.color.a *= fade;
                        s
                    })
                    .filter(|s| s.color.a > 0.001)
                    .collect();
                d.child(
                    div()
                        // Web `absolute inset-y-0 start-0 h-full` at the item's
                        // `offsetLeft`: both are padding-box relative, so the
                        // thumb sits inside the 1px border.
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(left)
                        .w(width)
                        .rounded_full()
                        .bg(indicator_bg)
                        .shadow(shadow),
                )
            })
            .children(items)
    }
}
