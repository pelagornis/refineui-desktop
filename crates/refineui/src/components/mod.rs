//! All RefineUI components. Each module mirrors a component of `@refineui/react`.

mod accordion;
mod alert;
mod avatar;
mod badge;
mod breadcrumb;
mod bubble;
mod button;
mod button_group;
mod calendar;
mod card;
#[cfg(feature = "web-extras")]
mod carousel;
mod chart;
mod checkbox;
mod chip;
mod code_block;
pub mod code_highlight;
#[cfg(feature = "syntax-highlight")]
#[rustfmt::skip]
#[allow(clippy::unreadable_literal)]
mod code_theme_data;
mod command;
mod command_palette;
mod context_menu;
mod date_input;
mod dialog;
pub(crate) mod disclosure;
mod divider;
mod drag;
mod field;
pub(crate) mod floating;
#[cfg(feature = "web-extras")]
mod footer;
mod input;
mod input_otp;
mod kbd;
mod layout;
#[cfg(feature = "web-extras")]
mod link;
mod marker;
pub(crate) mod menu;
mod message;
#[cfg(feature = "web-extras")]
mod navigation_menu;
mod pagination;
pub(crate) mod picker;
pub(crate) mod plate;
mod popover;
pub(crate) mod progress;
pub(crate) mod progress_stepper;
mod radio;
mod resizable;
mod result;
mod scroll_area;
mod segmented_control;
mod select;
mod selection_box;
mod shortcut_hint;
mod sidebar;
mod skeleton;
mod slider;
pub mod sliding_highlight;
mod spin_button;
mod spinner;
mod stepper;
mod switch;
mod table;
mod tabs;
mod text;
mod time_input;
mod toast;
mod toggle;
mod toolbar;
pub(crate) mod tooltip;
mod tree;

pub use accordion::*;
pub use alert::*;
pub use avatar::*;
pub use badge::*;
pub use breadcrumb::*;
pub use bubble::*;
pub use button::*;
pub use button_group::*;
pub use calendar::*;
pub use card::*;
#[cfg(feature = "web-extras")]
pub use carousel::*;
pub use chart::*;
pub use checkbox::*;
pub use chip::*;
pub use code_block::*;
pub use command::*;
pub use command_palette::*;
pub use context_menu::*;
pub use date_input::*;
pub use dialog::*;
pub use divider::*;
pub use drag::*;
pub use field::*;
pub use floating::{Beak, BeakAlign, BeakSide, floating_surface, paint_floating_surface};
#[cfg(feature = "web-extras")]
pub use footer::*;
pub use input::*;
pub use input_otp::*;
pub use kbd::*;
pub use layout::*;
#[cfg(feature = "web-extras")]
pub use link::*;
pub use marker::*;
pub use menu::*;
pub use message::*;
#[cfg(feature = "web-extras")]
pub use navigation_menu::*;
pub use pagination::*;
pub use popover::*;
pub use progress::*;
pub use progress_stepper::*;
pub use radio::*;
pub use resizable::*;
pub use result::*;
pub use scroll_area::*;
pub use segmented_control::*;
pub use select::*;
pub use selection_box::*;
pub use shortcut_hint::*;
pub use sidebar::*;
pub use skeleton::*;
pub use slider::*;
pub use spin_button::*;
pub use spinner::*;
pub use stepper::*;
pub use switch::*;
pub use table::*;
pub use tabs::*;
pub use text::*;
pub use time_input::*;
pub use toast::*;
pub use toggle::*;
pub use toolbar::*;
pub use tooltip::*;
pub use tree::*;

pub(crate) fn init(cx: &mut gpui::App) {
    input::bind_keys(cx);
}
