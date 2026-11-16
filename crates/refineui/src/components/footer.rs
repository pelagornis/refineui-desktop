//! Footer — web `Footer` (`components/Footer`): a sitemap of link groups,
//! then a meta row `social | brand lockup | locale` with the brand centered.
//!
//! Root: hairline top `borderDefault`, transparent, `px-xx-large` /
//! `py-xxx-large`, sections 32px apart. Groups auto-fit at ≥ 200px with
//! 32 / 24px gaps; a group is a `bodyMd` primary label and `bodyMd`
//! tertiary links (24px min target, primary on hover, `duration-fast`).
//! Brand: logo + `bodyMd` name, `bodyXs` tertiary description, centered.
//! Social links are 24px boxes with 16px tertiary glyphs, 16px apart.

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, px,
};
use refineui_tokens::{spacing, stroke, typography::role};

use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// A titled column of links.
pub struct FooterGroup {
    label: SharedString,
    links: Vec<SharedString>,
}

impl FooterGroup {
    /// Group titled `label` with `links`.
    pub fn new(
        label: impl Into<SharedString>,
        links: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            label: label.into(),
            links: links.into_iter().map(Into::into).collect(),
        }
    }
}

/// Site / app footer.
#[derive(IntoElement)]
pub struct Footer {
    id: ElementId,
    groups: Vec<FooterGroup>,
    logo: Option<AnyElement>,
    name: Option<SharedString>,
    description: Option<SharedString>,
    social: Vec<IconName>,
    end: Option<AnyElement>,
}

impl Footer {
    /// Empty footer.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            groups: Vec::new(),
            logo: None,
            name: None,
            description: None,
            social: Vec::new(),
            end: None,
        }
    }

    /// Add a sitemap group.
    pub fn group(mut self, g: FooterGroup) -> Self {
        self.groups.push(g);
        self
    }

    /// Brand lockup: optional logo, name and description.
    pub fn brand(
        mut self,
        logo: Option<AnyElement>,
        name: impl Into<SharedString>,
        description: Option<SharedString>,
    ) -> Self {
        self.logo = logo;
        self.name = Some(name.into());
        self.description = description;
        self
    }

    /// Social glyph links (start of the meta row).
    pub fn social(mut self, icons: impl IntoIterator<Item = IconName>) -> Self {
        self.social = icons.into_iter().collect();
        self
    }

    /// End slot (locale picker, copyright…).
    pub fn end(mut self, el: impl IntoElement) -> Self {
        self.end = Some(el.into_any_element());
        self
    }
}

impl RenderOnce for Footer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let c = cx.theme().colors;
        let (tertiary, primary) = (c.foreground_tertiary, c.foreground_primary);
        let id = self.id.clone();
        let groups = self.groups.into_iter().enumerate().map(|(g, group)| {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(200.))
                .gap(px(spacing::LARGE))
                .child(
                    div()
                        .text_style_token(role::BODY_MD)
                        .text_color(primary)
                        .child(group.label),
                )
                .children(group.links.into_iter().enumerate().map(move |(l, link)| {
                    div()
                        .id(gpui::ElementId::Name(format!("footer-{g}-{l}").into()))
                        .flex()
                        .items_center()
                        .min_h(px(24.))
                        .cursor_pointer()
                        .text_style_token(role::BODY_MD)
                        .text_color(tertiary)
                        .hover(move |s| s.text_color(primary))
                        .child(link)
                }))
        });
        let social = div()
            .flex()
            .flex_1()
            .items_center()
            .gap(px(spacing::LARGE))
            .children(self.social.into_iter().enumerate().map(|(i, ic)| {
                div()
                    .id(("footer-social", i))
                    .size(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(Icon::new(ic).size(IconSize::Small).color(tertiary))
            }));
        let brand = div()
            .flex()
            .flex_none()
            .flex_wrap()
            .items_center()
            .justify_center()
            .gap(px(spacing::SMALL))
            .children(self.logo)
            .when_some(self.name, |d, n| {
                d.child(
                    div()
                        .text_style_token(role::BODY_MD)
                        .text_color(primary)
                        .child(n),
                )
            })
            .when_some(self.description, |d, t| {
                d.child(
                    div()
                        .text_style_token(role::BODY_XS)
                        .text_color(tertiary)
                        .child(t),
                )
            });
        div()
            .id(id)
            .flex()
            .flex_col()
            .w_full()
            .gap(px(spacing::XXX_LARGE))
            .border_t(px(stroke::HAIRLINE))
            .border_color(c.border_default)
            .px(px(spacing::XX_LARGE))
            .py(px(spacing::XXX_LARGE))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap_x(px(spacing::XXX_LARGE))
                    .gap_y(px(spacing::XX_LARGE))
                    .children(groups),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_center()
                    .gap(px(spacing::X_LARGE))
                    .child(social)
                    .child(brand)
                    .child(div().flex().flex_1().justify_end().children(self.end)),
            )
    }
}
