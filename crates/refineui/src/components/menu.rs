//! Menu and DropdownMenu — web `Menu` / `Dropdown`. The right-click
//! [`crate::ContextMenu`] lives in its own module and reuses [`Menu`] panels.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Corner, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
    anchored, div, prelude::*, px,
};

use gpui::CursorStyle;
use refineui_tokens::{radius, spacing, stroke, typography::role};

use crate::components::plate::row_plate;
use crate::motion::{
    Interaction, Motion, TrackInteraction, animate, child_id, presence, press_scale, probe,
    scaled_plate, use_interaction, use_layout_probe,
};
use crate::{ActiveTheme, Icon, IconName, IconSize, StyledExt};

/// Action callback for menu items.
pub type MenuHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// One entry in a [`Menu`].
#[derive(Clone)]
pub enum MenuEntry {
    /// Clickable item.
    Item(MenuItem),
    /// Group label.
    Label(SharedString),
    /// Divider line (web `MenuDivider`, formerly `MenuSeparator`).
    Divider,
    /// Deprecated name of [`MenuEntry::Divider`].
    #[deprecated(note = "renamed to `MenuEntry::Divider` (web `MenuDivider`)")]
    Separator,
    /// Row that opens a nested panel on hover (web `ContextMenuSub`).
    Submenu(MenuItem, Box<Menu>),
}

/// Clickable menu row.
#[derive(Clone)]
pub struct MenuItem {
    label: SharedString,
    description: Option<SharedString>,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    checked: bool,
    disabled: bool,
    danger: bool,
    handler: Option<MenuHandler>,
    action: Option<SharedString>,
}

impl MenuItem {
    /// New item.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            icon: None,
            shortcut: None,
            checked: false,
            disabled: false,
            danger: false,
            handler: None,
            action: None,
        }
    }

    /// Run a GPUI action by name (`"workspace::Save"`, built from GPUI's
    /// action registry and dispatched to the focused element) and show its
    /// shortcut from the installed [`crate::interaction::keymap::Keymap`]
    /// unless [`Self::shortcut`] overrides it. An explicit
    /// [`Self::on_click`] runs instead of the dispatch.
    pub fn action(mut self, name: impl Into<SharedString>) -> Self {
        self.action = Some(name.into());
        self
    }

    /// Fill the shortcut from the keymap and the click handler from the
    /// bound action.
    pub(crate) fn resolve(mut self, cx: &App) -> Self {
        if let Some(name) = self.action.clone() {
            if self.shortcut.is_none() {
                self.shortcut = crate::interaction::keymap::shortcut_for(&name, cx).map(Into::into);
            }
            if self.handler.is_none() {
                self.handler = Some(Rc::new(move |window, cx| {
                    if let Ok(a) = cx.build_action(&name, None) {
                        window.dispatch_action(a, cx);
                    }
                }));
            }
        }
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Secondary line under the label (`bodyXs`, `foregroundTertiary`).
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Shortcut hint (`⌘C`).
    pub fn shortcut(mut self, s: impl Into<SharedString>) -> Self {
        self.shortcut = Some(s.into());
        self
    }

    /// Check mark.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Destructive styling.
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// Activation handler.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.handler = Some(Rc::new(f));
        self
    }
}

/// A menu panel (static). Use [`DropdownMenu`] / [`ContextMenu`](crate::ContextMenu) for popups.
#[derive(IntoElement, Clone)]
pub struct Menu {
    id: ElementId,
    entries: Vec<MenuEntry>,
    width: Pixels,
    on_dismiss: Option<MenuHandler>,
    native_frost: bool,
}

impl Menu {
    /// Empty menu.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            width: px(refineui_tokens::size::component::MENU_PANEL_WIDTH),
            on_dismiss: None,
            native_frost: false,
        }
    }

    /// Add an item.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.entries.push(MenuEntry::Item(item));
        self
    }

    /// Add a group label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(MenuEntry::Label(label.into()));
        self
    }

    /// Add a divider line (web `MenuDivider`).
    pub fn divider(mut self) -> Self {
        self.entries.push(MenuEntry::Divider);
        self
    }

    /// Deprecated name of [`Self::divider`].
    #[deprecated(note = "renamed to `divider()` (web `MenuDivider`)")]
    pub fn separator(self) -> Self {
        self.divider()
    }

    /// Add a row that opens `menu` to its side on hover (web
    /// `ContextMenuSub` + `SubTrigger` + `SubContent`): placed at the row's
    /// right edge, flipping left when it would leave the window.
    pub fn submenu(mut self, item: MenuItem, menu: Menu) -> Self {
        self.entries.push(MenuEntry::Submenu(item, Box::new(menu)));
        self
    }

    /// Panel width.
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Inside a native-frost pop-up window ([`crate::vibrancy`]): no panel
    /// fill / border / shadow of its own — the window provides them.
    pub(crate) fn native_frost(mut self, on: bool) -> Self {
        self.native_frost = on;
        self
    }

    pub(crate) fn on_dismiss(mut self, f: MenuHandler) -> Self {
        self.on_dismiss = Some(f);
        self
    }

    pub(crate) fn panel_width(&self) -> Pixels {
        self.width
    }

    pub(crate) fn id(&self) -> &ElementId {
        &self.id
    }
}

/// Which submenu is open and whether the pointer is over it.
#[derive(Default, Clone, Copy)]
pub(crate) struct SubmenuState {
    open: Option<usize>,
    /// Pointer is over the open submenu panel — outside-click dismissal of
    /// the parent popup ignores presses there.
    pub(crate) hovered: bool,
    /// When the pointer left both the open row and its panel.
    left_at: Option<std::time::Instant>,
    /// Pointer is over the open submenu's native-frost pop-up window.
    native_hovered: bool,
}

/// Web `ContextMenuSub`: hide 150ms after the pointer leaves.
const SUBMENU_CLOSE_DELAY: std::time::Duration = std::time::Duration::from_millis(150);

pub(crate) fn submenu_state(
    menu_id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> Entity<SubmenuState> {
    window.use_keyed_state(child_id(menu_id, "submenu"), cx, |_, _| {
        SubmenuState::default()
    })
}

struct RowParts {
    bg: gpui::Hsla,
    fg: gpui::Hsla,
    sub_fg: gpui::Hsla,
    scale: f32,
    inset: f32,
}

/// One menu row (web `menuStyles.itemBase` + `row`): `p-x-small`,
/// `roundedLarge`, 16px icon column, title `bodyMd`, description / shortcut
/// `bodyXs` tertiary. The fill sits on a press-scaled plate.
fn menu_row(
    id: ElementId,
    item: &MenuItem,
    parts: RowParts,
    trailing_chevron: bool,
) -> gpui::Stateful<gpui::Div> {
    let RowParts {
        bg,
        fg,
        sub_fg,
        scale,
        inset,
    } = parts;
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap(px(spacing::X_SMALL))
        .p(px(spacing::X_SMALL))
        .py(px(spacing::X_SMALL + inset))
        .rounded(px(radius::LARGE))
        .text_color(fg)
        .child(
            scaled_plate(scale)
                .rounded(px(radius::LARGE * scale))
                .bg(bg),
        )
        .when(item.checked || item.icon.is_some(), |d| {
            d.child(
                div()
                    .flex_none()
                    .size(px(refineui_tokens::size::icon::X_SMALL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .map(|d| {
                        if item.checked {
                            d.child(
                                Icon::new(IconName::Checkmark)
                                    .size(IconSize::XSmall)
                                    .color(fg),
                            )
                        } else if let Some(icon) = item.icon {
                            d.child(Icon::new(icon).size(IconSize::XSmall).color(fg))
                        } else {
                            d
                        }
                    }),
            )
        })
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(spacing::XXX_SMALL))
                .px(px(spacing::XXX_SMALL))
                .child(
                    div()
                        .text_style_token(role::BODY_MD)
                        .child(item.label.clone()),
                )
                .when_some(item.description.clone(), |d, t| {
                    d.child(
                        div()
                            .text_style_token(role::BODY_XS)
                            .text_color(sub_fg)
                            .child(t),
                    )
                }),
        )
        .when_some(item.shortcut.clone(), |d, s| {
            d.child(
                div()
                    .flex_none()
                    .text_style_token(role::BODY_XS)
                    .text_color(sub_fg)
                    .child(s),
            )
        })
        .when(trailing_chevron, |d| {
            d.child(
                Icon::new(IconName::ChevronRight)
                    .size(IconSize::XSmall)
                    .color(fg),
            )
        })
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let parent = self.id.clone();
        let dismiss = self.on_dismiss.clone();
        let sub_state = submenu_state(&self.id, window, cx);
        let (sub_open, sub_hovered, sub_native_hovered) = {
            let s = *sub_state.read(cx);
            (s.open, s.hovered, s.native_hovered)
        };
        let mut close_sub = false;
        let mut open_sub = None;
        let mut panel_hovered = false;
        let mut open_row_hovered = false;
        // The open submenu panel, appended after every row so it paints on
        // top of the parent panel.
        let mut sub_panel: Option<AnyElement> = None;
        let mut rows: Vec<AnyElement> = Vec::new();
        // Sliding hover plate (web `menuStyles.highlight`): rows report their
        // bounds to `host`; the plate glides to the hovered row.
        let host = use_layout_probe(&child_id(&self.id, "highlight"), window, cx);
        let mut hovered_row: Option<usize> = None;
        let entries: Vec<MenuEntry> = self
            .entries
            .into_iter()
            .map(|e| match e {
                MenuEntry::Item(i) => MenuEntry::Item(i.resolve(cx)),
                MenuEntry::Submenu(i, m) => MenuEntry::Submenu(i.resolve(cx), m),
                other => other,
            })
            .collect();
        for (ix, entry) in entries.into_iter().enumerate() {
            #[allow(deprecated)]
            let entry = match entry {
                MenuEntry::Separator => MenuEntry::Divider,
                other => other,
            };
            rows.push(match entry {
                #[allow(deprecated)]
                MenuEntry::Divider | MenuEntry::Separator => div()
                    .flex()
                    .items_center()
                    .px(px(spacing::X_SMALL))
                    .py(px(spacing::XXX_SMALL))
                    .child(div().w_full().h(px(stroke::THIN)).bg(c.border_default))
                    .into_any_element(),
                MenuEntry::Label(l) => div()
                    .px(px(spacing::X_SMALL))
                    .py(px(spacing::XXX_SMALL))
                    .text_style_token(role::CAPTION_LG)
                    .text_color(c.foreground_tertiary)
                    .child(l)
                    .into_any_element(),
                MenuEntry::Item(item) => {
                    let id = ElementId::NamedChild(
                        Box::new(parent.clone()),
                        format!("item-{ix}").into(),
                    );
                    let (state, i) = use_interaction(&id, window, cx);
                    // Hovering any plain row closes an open submenu.
                    if i.hovered && sub_open.is_some() && !sub_hovered {
                        close_sub = true;
                    }
                    let rest = if item.danger {
                        c.foreground_error
                    } else {
                        c.foreground_primary
                    };
                    let (mut bg, mut fg) = row_plate(i, false, item.disabled, rest, cx);
                    if item.danger && !item.disabled {
                        fg = c.foreground_error;
                    }
                    if i.hovered && !item.disabled {
                        hovered_row = Some(ix);
                    }
                    // The sliding plate paints the hover fill; a row keeps
                    // only its pressed fill.
                    if !i.pressed {
                        bg = gpui::transparent_black();
                    }
                    let bg = animate(child_id(&id, "bg"), bg, Motion::FAST_EASE, window, cx);
                    let fg = animate(child_id(&id, "fg"), fg, Motion::FAST_EASE, window, cx);
                    let scale = press_scale(
                        child_id(&id, "press"),
                        i.pressed && !item.disabled,
                        window,
                        cx,
                    );
                    let handler = item.handler.clone();
                    let dismiss = dismiss.clone();
                    let sub_fg = if item.disabled {
                        c.foreground_disabled
                    } else {
                        c.foreground_tertiary
                    };
                    let disabled = item.disabled;
                    menu_row(
                        id,
                        &item,
                        RowParts {
                            bg,
                            fg,
                            sub_fg,
                            scale,
                            inset: theme.row_inset(),
                        },
                        false,
                    )
                    .child(probe(&host, Some(ix)))
                    .when(!disabled, |d| {
                        d.cursor_action()
                            .track_interaction(&state)
                            .on_click(move |_, w, cx| {
                                if let Some(h) = &handler {
                                    h(w, cx);
                                }
                                if let Some(d) = &dismiss {
                                    d(w, cx);
                                }
                            })
                    })
                    .when(disabled, |d| d.cursor(CursorStyle::OperationNotAllowed))
                    .into_any_element()
                }
                MenuEntry::Submenu(item, menu) => {
                    let id =
                        ElementId::NamedChild(Box::new(parent.clone()), format!("sub-{ix}").into());
                    let (state, i) = use_interaction(&id, window, cx);
                    if i.hovered && sub_open != Some(ix) && !item.disabled {
                        open_sub = Some(ix);
                    }
                    let is_open = sub_open == Some(ix);
                    let shown = Interaction {
                        hovered: i.hovered || is_open,
                        pressed: i.pressed,
                    };
                    let (mut bg, fg) =
                        row_plate(shown, false, item.disabled, c.foreground_primary, cx);
                    if i.hovered && !item.disabled {
                        hovered_row = Some(ix);
                    }
                    // Open sub triggers keep their own fill (web
                    // `:not([aria-expanded=true])`); hover is the plate's.
                    if !is_open && !i.pressed {
                        bg = gpui::transparent_black();
                    }
                    let bg = animate(child_id(&id, "bg"), bg, Motion::FAST_EASE, window, cx);
                    let fg = animate(child_id(&id, "fg"), fg, Motion::FAST_EASE, window, cx);
                    let row_probe = use_layout_probe(&id, window, cx);
                    let row_bounds = row_probe.read(cx).container;
                    let panel_id = child_id(&id, "panel");
                    let (pstate, pi) = use_interaction(&panel_id, window, cx);
                    if is_open {
                        panel_hovered |= pi.hovered || (self.native_frost && sub_native_hovered);
                        open_row_hovered |= i.hovered;
                    }
                    let viewport = window.viewport_size();
                    let width = f32::from(menu.width);
                    let sub_menu = match &dismiss {
                        Some(d) => menu.on_dismiss(d.clone()),
                        None => *menu,
                    };
                    let row = menu_row(
                        id,
                        &item,
                        RowParts {
                            bg,
                            fg,
                            sub_fg: c.foreground_tertiary,
                            scale: 1.0,
                            inset: theme.row_inset(),
                        },
                        true,
                    )
                    .child(probe(&row_probe, None))
                    .child(probe(&host, Some(ix)))
                    .when(!item.disabled, |d| {
                        d.cursor_action().track_interaction(&state)
                    });
                    if self.native_frost {
                        // Inside a native-frost pop-up: the submenu gets its
                        // own pop-up window beside the row (never clipped).
                        let bounds = row_bounds.map(|rb| {
                            let mut left = rb.right();
                            let screen_right = crate::vibrancy::to_screen(
                                window,
                                gpui::point(rb.right() + px(width), rb.top()),
                            )
                            .x;
                            if let Some(display) = window.display(cx)
                                && screen_right > display.bounds().right() - px(4.)
                            {
                                left = rb.left() - px(width);
                            }
                            gpui::Bounds::new(
                                gpui::point(
                                    left,
                                    rb.top() - px(spacing::XX_SMALL + stroke::HAIRLINE),
                                ),
                                gpui::size(px(width), px(200.)),
                            )
                        });
                        let sub_menu = sub_menu.clone();
                        let st = sub_state.clone();
                        crate::vibrancy::drive_native_popup(
                            &ElementId::NamedChild(
                                Box::new(parent.clone()),
                                format!("native-sub-{ix}").into(),
                            ),
                            is_open,
                            bounds,
                            move || native_submenu_spec(sub_menu, st, ix),
                            window,
                            cx,
                        );
                    } else if let Some(rb) = row_bounds.filter(|_| is_open) {
                        // Web `ContextMenuSubContent`: `left = rect.right`
                        // (flip to `rect.left - width`), `top = rect.top`.
                        let mut left = rb.right();
                        if f32::from(left) + width > f32::from(viewport.width) - 4. {
                            left = (rb.left() - px(width)).max(px(4.));
                        }
                        let panel = anchored()
                            .position(gpui::point(left, rb.top()))
                            .snap_to_window_with_margin(px(4.))
                            .child(
                                div()
                                    .id(panel_id)
                                    .occlude()
                                    .track_interaction(&pstate)
                                    .child(sub_menu),
                            );
                        // (Root cause of the submenu crash: this used to be a
                        // `deferred` inside the context menu's `deferred`,
                        // which GPUI rejects; `layer` draws in place when
                        // nested.)
                        sub_panel = Some(crate::layer(panel).priority(2).into_any_element());
                    }
                    row.into_any_element()
                }
            });
        }
        // Close after the pointer has been away from the row and its panel
        // for 150ms (a hovered plain row still closes it at once).
        if sub_open.is_some() && open_sub.is_none() && !close_sub {
            let now = std::time::Instant::now();
            if open_row_hovered || panel_hovered {
                if sub_state.read(cx).left_at.is_some() {
                    sub_state.update(cx, |s, _| s.left_at = None);
                }
            } else {
                let left = sub_state.read(cx).left_at;
                match left {
                    None => {
                        sub_state.update(cx, |s, _| s.left_at = Some(now));
                        window.request_animation_frame();
                    }
                    Some(t) if now.duration_since(t) >= SUBMENU_CLOSE_DELAY => close_sub = true,
                    Some(_) => window.request_animation_frame(),
                }
            }
        }
        if close_sub || open_sub.is_some() || sub_state.read(cx).hovered != panel_hovered {
            sub_state.update(cx, |s, _| {
                if close_sub {
                    s.open = None;
                    s.left_at = None;
                }
                if let Some(ix) = open_sub {
                    s.open = Some(ix);
                }
                s.hovered = panel_hovered && s.open.is_some();
            });
            // `refresh` is ignored while drawing; schedule a redraw so the
            // panel actually opens / closes on the next frame.
            window.request_animation_frame();
        }
        let plate = crate::components::sliding_highlight::sliding_plate(
            &child_id(&self.id, "highlight"),
            &host,
            hovered_row,
            c.background_primary_hover,
            radius::LARGE,
            window,
            cx,
        );
        div()
            .id(self.id)
            .relative()
            .w(self.width)
            .p(px(spacing::XX_SMALL))
            .flex()
            .flex_col()
            .gap(px(1.))
            .when(!self.native_frost, |d| d.popover_surface(cx))
            .font_family(theme.font_family.clone())
            .child(probe(&host, None))
            .child(plate)
            .children(rows)
            .children(sub_panel)
    }
}

/// Wrap a floating panel in the frosted enter / exit fade (`duration-normal`,
/// standard easing — `refineui-menu-popover-enter` / `-exit`).
pub(crate) fn fade(progress: f32, panel: impl IntoElement) -> gpui::Div {
    div().opacity(progress).child(panel)
}

pub(crate) fn popup_state(id: &ElementId, window: &mut Window, cx: &mut App) -> Entity<bool> {
    window.use_keyed_state(
        ElementId::NamedChild(Box::new(id.clone()), "open".into()),
        cx,
        |_, _| false,
    )
}

/// A trigger that opens a [`Menu`] below it.
#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    trigger: AnyElement,
    menu: Menu,
    anchor: Corner,
}

impl DropdownMenu {
    /// New dropdown. `trigger` is any element (usually a [`crate::Button`] with `icon_end(ChevronDown)`).
    pub fn new(id: impl Into<ElementId>, trigger: impl IntoElement, menu: Menu) -> Self {
        Self {
            id: id.into(),
            trigger: trigger.into_any_element(),
            menu,
            anchor: Corner::TopLeft,
        }
    }

    /// Align the menu's right edge with the trigger.
    pub fn align_end(mut self) -> Self {
        self.anchor = Corner::TopRight;
        self
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = popup_state(&self.id, window, cx);
        let is_open = *open.read(cx);
        let p = presence(
            child_id(&self.id, "presence"),
            is_open,
            Motion::FROSTED,
            Motion::FROSTED,
            window,
            cx,
        );
        let toggle = open.clone();
        let close = open.clone();
        let close2 = open.clone();
        let anchor = self.anchor;
        let sub = submenu_state(self.menu.id(), window, cx);
        if crate::vibrancy::native_frost_active(cx) {
            return self
                .render_native(open, is_open, toggle, window, cx)
                .into_any_element();
        }
        div()
            .id(self.id)
            .relative()
            .flex_none()
            .child(
                div()
                    .id("trigger")
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        toggle.update(cx, |o, cx| {
                            *o = !*o;
                            cx.notify();
                        });
                    })
                    .child(self.trigger),
            )
            .when(p.mounted, |d| {
                let dismiss: MenuHandler = Rc::new(move |_, cx| {
                    close.update(cx, |o, cx| {
                        *o = false;
                        cx.notify();
                    })
                });
                d.child(
                    crate::layer(
                        anchored()
                            .anchor(anchor)
                            .snap_to_window_with_margin(px(8.))
                            .child(
                                fade(p.progress, self.menu.on_dismiss(dismiss))
                                    .id("panel")
                                    .mt(px(spacing::XX_SMALL))
                                    .when(p.open, |d| {
                                        d.occlude().on_mouse_down_out(move |_, _, cx| {
                                            if sub.read(cx).hovered {
                                                return;
                                            }
                                            close2.update(cx, |o, cx| {
                                                *o = false;
                                                cx.notify();
                                            })
                                        })
                                    }),
                            ),
                    )
                    .priority(1),
                )
            })
            .into_any_element()
    }
}

impl DropdownMenu {
    /// [`crate::FrostMode::Native`]: the panel opens in its own OS-blurred
    /// pop-up window (macOS) placed under the trigger.
    fn render_native(
        self,
        open: Entity<bool>,
        is_open: bool,
        toggle: Entity<bool>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let trigger_probe = use_layout_probe(&child_id(&self.id, "native-trigger"), window, cx);
        let width = self.menu.width;
        let bounds = trigger_probe.read(cx).container.map(|t| {
            let x = match self.anchor {
                Corner::TopRight => t.right() - width,
                _ => t.left(),
            };
            // Rough first height; the pop-up resizes itself to the menu.
            gpui::Bounds::new(
                gpui::point(x, t.bottom() + px(spacing::XX_SMALL)),
                gpui::size(width, px(200.)),
            )
        });
        let menu = self.menu.clone();
        let spec_open = open.clone();
        crate::vibrancy::drive_native_popup(
            &self.id,
            is_open,
            bounds,
            move || native_menu_spec(menu, spec_open, true),
            window,
            cx,
        );
        let close_out = open.clone();
        div().id(self.id).relative().flex_none().child(
            div()
                .id("trigger")
                .relative()
                .child(probe(&trigger_probe, None))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    toggle.update(cx, |o, cx| {
                        *o = !*o;
                        cx.notify();
                    });
                })
                .when(is_open, |d| {
                    // Clicks in the pop-up window never reach this window,
                    // so any click here outside the trigger dismisses.
                    d.on_mouse_down_out(move |_, _, cx| {
                        close_out.update(cx, |o, cx| {
                            *o = false;
                            cx.notify();
                        })
                    })
                })
                .child(self.trigger),
        )
    }
}

/// A native-frost pop-up showing `menu`; activating an item (or a
/// deactivation / move of the owning window) sets `open` to false.
pub(crate) fn native_menu_spec(
    menu: Menu,
    open: Entity<bool>,
    close_on_deactivate: bool,
) -> crate::vibrancy::NativePopupSpec {
    let close_state = open.clone();
    let close: Rc<dyn Fn(&mut App)> = Rc::new(move |cx| {
        close_state.update(cx, |o, cx| {
            if *o {
                *o = false;
                cx.notify();
            }
        })
    });
    let dismiss = close.clone();
    crate::vibrancy::NativePopupSpec {
        radius: radius::XX_LARGE,
        close,
        still_open: Rc::new(move |cx| *open.read(cx)),
        content: Rc::new(move |_, _| {
            let dismiss = dismiss.clone();
            menu.clone()
                .native_frost(true)
                .on_dismiss(Rc::new(move |_, cx| dismiss(cx)))
                .into_any_element()
        }),
        close_on_deactivate,
        report_size: None,
        popover: false,
    }
}

/// A submenu's native-frost pop-up: closing it clears `state.open`; hovering
/// it keeps the parent row's submenu open.
fn native_submenu_spec(
    menu: Menu,
    state: Entity<SubmenuState>,
    ix: usize,
) -> crate::vibrancy::NativePopupSpec {
    let close_state = state.clone();
    let open_state = state.clone();
    crate::vibrancy::NativePopupSpec {
        radius: radius::XX_LARGE,
        close: Rc::new(move |cx| {
            close_state.update(cx, |s, cx| {
                if s.open == Some(ix) {
                    s.open = None;
                    s.native_hovered = false;
                    cx.notify();
                }
            })
        }),
        still_open: Rc::new(move |cx| open_state.read(cx).open == Some(ix)),
        content: Rc::new(move |_, _| {
            let hover = state.clone();
            div()
                .id("native-submenu")
                .on_hover(move |inside, _, cx| {
                    let inside = *inside;
                    hover.update(cx, |s, cx| {
                        s.native_hovered = inside;
                        cx.notify();
                    })
                })
                .child(menu.clone().native_frost(true))
                .into_any_element()
        }),
        close_on_deactivate: false,
        report_size: None,
        popover: false,
    }
}
