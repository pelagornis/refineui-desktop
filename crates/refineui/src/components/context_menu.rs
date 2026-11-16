//! ContextMenu — web `ContextMenu`: a region that opens a frosted [`Menu`]
//! at the pointer on right click (`contextmenu` → `openAt(clientX, clientY)`),
//! kept inside the window with a 4px pad (`placeInView`). Submenus
//! ([`Menu::submenu`]) open to the side on hover. Fades in / out over
//! `duration-normal`; Escape or an outside press closes it.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, Point, RenderOnce, Window, anchored, div, prelude::*, px,
};

use crate::Menu;
use crate::components::menu::{MenuHandler, fade, native_menu_spec, submenu_state};
use crate::motion::{Motion, child_id, presence};

/// Right-click menu: wraps `content` and opens a frosted [`Menu`] at the
/// pointer, fading in / out over `duration-normal`.
#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    content: AnyElement,
    menu: Menu,
}

impl ContextMenu {
    /// New context-menu region.
    pub fn new(id: impl Into<ElementId>, content: impl IntoElement, menu: Menu) -> Self {
        Self {
            id: id.into(),
            content: content.into_any_element(),
            menu,
        }
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pos = window.use_keyed_state(
            ElementId::NamedChild(Box::new(self.id.clone()), "pos".into()),
            cx,
            |_, _| (false, None::<Point<Pixels>>),
        );
        let (is_open, at) = *pos.read(cx);
        let p = presence(
            child_id(&self.id, "presence"),
            is_open,
            Motion::FROSTED,
            Motion::FROSTED,
            window,
            cx,
        );
        let set = pos.clone();
        let close = pos.clone();
        let close2 = pos.clone();
        let close_key = pos.clone();
        let sub = submenu_state(self.menu.id(), window, cx);
        if crate::vibrancy::native_frost_active(cx) {
            // FrostMode::Native: the menu opens in its own OS-blurred pop-up
            // window at the pointer; a new right click reopens it there.
            let open = window.use_keyed_state(child_id(&self.id, "native-open"), cx, |_, _| false);
            let is_open = *open.read(cx);
            let width = self.menu.panel_width();
            let bounds = at
                .filter(|_| is_open)
                .map(|at| gpui::Bounds::new(at, gpui::size(width, px(200.))));
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
            let shut = |open: &gpui::Entity<bool>| {
                let open = open.clone();
                move |cx: &mut App| {
                    open.update(cx, |o, cx| {
                        *o = false;
                        cx.notify();
                    })
                }
            };
            let (close_left, close_out, close_esc) = (shut(&open), shut(&open), shut(&open));
            let opener = open.clone();
            let esc_open = open.clone();
            return div()
                .id(self.id)
                .on_mouse_down(MouseButton::Right, move |e, _, cx| {
                    set.update(cx, |p, cx| {
                        *p = (false, Some(e.position));
                        cx.notify();
                    });
                    opener.update(cx, |o, cx| {
                        *o = true;
                        cx.notify();
                    });
                })
                .when(is_open, |d| {
                    d.on_mouse_down(MouseButton::Left, move |_, _, cx| close_left(cx))
                        .on_mouse_down_out(move |_, _, cx| close_out(cx))
                })
                .on_key_down(move |e: &gpui::KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" && *esc_open.read(cx) {
                        cx.stop_propagation();
                        close_esc(cx);
                    }
                })
                .child(self.content)
                .into_any_element();
        }
        div()
            .id(self.id)
            .on_mouse_down(MouseButton::Right, move |e, _, cx| {
                set.update(cx, |p, cx| {
                    *p = (true, Some(e.position));
                    cx.notify();
                })
            })
            .on_key_down(move |e: &gpui::KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && close_key.read(cx).0 {
                    cx.stop_propagation();
                    close_key.update(cx, |p, cx| {
                        p.0 = false;
                        cx.notify();
                    })
                }
            })
            .child(self.content)
            .when_some(at.filter(|_| p.mounted), |d, at| {
                let dismiss: MenuHandler = Rc::new(move |_, cx| {
                    close.update(cx, |p, cx| {
                        p.0 = false;
                        cx.notify();
                    })
                });
                d.child(
                    crate::layer(
                        anchored()
                            .position(at)
                            .snap_to_window_with_margin(px(8.))
                            .child(
                                fade(p.progress, self.menu.on_dismiss(dismiss))
                                    .id("panel")
                                    .when(p.open, |d| {
                                        d.occlude().on_mouse_down_out(move |_, _, cx| {
                                            if sub.read(cx).hovered {
                                                return;
                                            }
                                            close2.update(cx, |p, cx| {
                                                p.0 = false;
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
