//! # RefineUI Desktop
//!
//! A desktop-native design system for [GPUI](https://gpui.rs) apps that keeps
//! the RefineUI visual identity (`@refineui/tokens`) while adding desktop
//! interaction, workspace composition and an application shell.
//!
//! ## Layers (brief §3)
//!
//! | Layer | Module | Contents |
//! | --- | --- | --- |
//! | Foundation | [`foundation`] (+ [`tokens`]) | theme, density, focus ring, layers, cursor, motion, blur |
//! | Interaction | [`interaction`] | keymap, selection, focus, drag and drop |
//! | Components | [`components`] | buttons, inputs, menus, dialogs, tables, trees |
//! | Composition | [`workspace`] | split / tab group / dock / floating layout model |
//! | Application | [`shell`] | window, titlebar, menu bar, toolbar, activity bar, status bar |
//! | Platform | [`platform`] | macOS / Windows / Linux adapters |
//!
//! Shared contracts (tokens, density, z-order, window metrics, component
//! contracts) live in the GPUI-free `refineui-tokens` crate. Web-oriented
//! components (Carousel, Footer, NavigationMenu, Link, `container()`) are
//! behind the optional `web-extras` feature.
//!
//! ```no_run
//! use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, px, size, prelude::*};
//! use refineui::prelude::*;
//!
//! struct Hello;
//!
//! impl Render for Hello {
//!     fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
//!         refineui::root(cx).p_4().child(
//!             Button::new("save", "Save").variant(ButtonVariant::Primary),
//!         )
//!     }
//! }
//!
//! fn main() {
//!     Application::new().with_assets(refineui::Assets).run(|cx: &mut App| {
//!         refineui::init(cx);
//!         let bounds = Bounds::centered(None, size(px(480.), px(320.)), cx);
//!         cx.open_window(
//!             WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
//!             |_, cx| cx.new(|_| Hello),
//!         )
//!         .unwrap();
//!     });
//! }
//! ```

#![warn(missing_docs)]

mod assets;
pub mod components;
pub mod foundation;
mod icon;
mod icon_names;
pub mod interaction;
mod layer;
pub mod motion;
pub mod platform;
mod root;
pub mod shell;
mod styles;
mod theme;
mod theme_colors;
pub mod vibrancy;
pub mod workspace;

pub use assets::Assets;
pub use components::*;
pub use icon::{Icon, IconSize, IconStyle};
pub use icon_names::IconName;
pub use layer::{Layer, layer};
pub use refineui_tokens as tokens;
pub use refineui_tokens::{
    shadow::Elevation,
    size::{ControlMetrics, ControlSize, Density},
};
pub use root::{FocusNext, FocusPrevious, root};
pub use styles::StyledExt;
pub use theme::{ActiveTheme, Appearance, Theme};
pub use theme_colors::ThemeColors;
pub use vibrancy::{FrostMode, frost_mode, set_frost_mode};

/// Everything an app usually needs: `use refineui::prelude::*;`.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::foundation::{CursorRole, LayerOrder};
    pub use crate::interaction::keymap::KeybindingManager;
    pub use crate::interaction::selection::{SelectGesture, SelectionMode, SelectionModel};
    pub use crate::motion::{Motion, TrackInteraction};
    pub use crate::platform::{Os, PlatformAdapter, ScrollbarStyle};
    pub use crate::shell::{
        ActivityBar, ActivityBarPreset, ActivityBarStyle, ActivityItem, AppMenu, ApplicationShell,
        BarSide, IndicatorStyle, ItemBadge, ItemShape, LabelMode, MenuBar, RailEdge, ShellDock,
        ShellSpec, StatusBar, StatusItem, StatusTone, Titlebar, ToolbarPlacement, WindowControls,
        WindowFrame, WindowPreset, WindowSpec, WindowState,
    };
    pub use crate::{
        ActiveTheme, Appearance, ControlSize, Density, Elevation, Icon, IconName, IconSize,
        IconStyle, StyledExt, Theme,
    };
}

/// Initialise RefineUI: registers bundled fonts, the theme global and the
/// default key bindings (Tab / Shift-Tab focus traversal, text input editing,
/// Escape to dismiss overlays).
pub fn init(cx: &mut gpui::App) {
    #[cfg(feature = "bundled-fonts")]
    assets::register_fonts(cx);
    Theme::init(cx);
    root::bind_keys(cx);
    components::init(cx);
}
