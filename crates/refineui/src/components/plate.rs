//! Shared "list row plate" states (Sidebar link, Menu / Dropdown / Select item,
//! Tree item, Breadcrumb link) — the `refineui.css` rules:
//! hover → `backgroundPrimaryHover` + `foregroundPrimaryHover`,
//! pressed → `backgroundPrimaryActive` + `foregroundPrimary`,
//! selected / active → `backgroundPrimaryHover` + `foregroundPrimary`.

use gpui::{App, Hsla};

use crate::ActiveTheme;
use crate::motion::Interaction;

/// `(background, foreground)` for a row in the given state. `rest_fg` is the
/// row's resting text color (`foregroundPrimary` for menus, `foregroundTertiary`
/// for sidebar / tree rows).
pub(crate) fn row_plate(
    i: Interaction,
    selected: bool,
    disabled: bool,
    rest_fg: Hsla,
    cx: &App,
) -> (Hsla, Hsla) {
    let c = &cx.theme().colors;
    if disabled {
        return (gpui::transparent_black(), c.foreground_disabled);
    }
    if i.pressed {
        (c.background_primary_active, c.foreground_primary)
    } else if i.hovered {
        (c.background_primary_hover, c.foreground_primary_hover)
    } else if selected {
        (c.background_primary_hover, c.foreground_primary)
    } else {
        (gpui::transparent_black(), rest_fg)
    }
}
