//! Icon set — SVGs from RefineUI System Icons (MIT, github.com/pelagornis/refineui-system-icons),
//! 20px grid, `regular` and `filled` styles. Generated list; see AGENTS.md → "Adding an icon".

use crate::icon::IconStyle;

/// Available icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    #[allow(missing_docs)]
    Add,
    #[allow(missing_docs)]
    AddCircle,
    #[allow(missing_docs)]
    Alert,
    #[allow(missing_docs)]
    AppStore,
    #[allow(missing_docs)]
    Board,
    #[allow(missing_docs)]
    Bookmark,
    #[allow(missing_docs)]
    Bug,
    #[allow(missing_docs)]
    Calendar,
    #[allow(missing_docs)]
    Chat,
    #[allow(missing_docs)]
    Checkmark,
    #[allow(missing_docs)]
    ChevronDown,
    #[allow(missing_docs)]
    ChevronLeft,
    #[allow(missing_docs)]
    ChevronRight,
    #[allow(missing_docs)]
    ChevronUp,
    #[allow(missing_docs)]
    Circle,
    #[allow(missing_docs)]
    Clock,
    #[allow(missing_docs)]
    Cloud,
    #[allow(missing_docs)]
    Code,
    #[allow(missing_docs)]
    Copy,
    #[allow(missing_docs)]
    Cube,
    #[allow(missing_docs)]
    Database,
    #[allow(missing_docs)]
    Delete,
    #[allow(missing_docs)]
    Desktop,
    #[allow(missing_docs)]
    Dismiss,
    #[allow(missing_docs)]
    DismissCircle,
    #[allow(missing_docs)]
    Document,
    #[allow(missing_docs)]
    Drag,
    #[allow(missing_docs)]
    Edit,
    #[allow(missing_docs)]
    ErrorCircle,
    #[allow(missing_docs)]
    Eye,
    #[allow(missing_docs)]
    Filter,
    #[allow(missing_docs)]
    Folder,
    #[allow(missing_docs)]
    FolderOpen,
    #[allow(missing_docs)]
    FullScreenMaximize,
    #[allow(missing_docs)]
    Git,
    #[allow(missing_docs)]
    Grid,
    #[allow(missing_docs)]
    Heart,
    #[allow(missing_docs)]
    Home,
    #[allow(missing_docs)]
    Info,
    #[allow(missing_docs)]
    Keyboard,
    #[allow(missing_docs)]
    Layer,
    #[allow(missing_docs)]
    Lightbulb,
    #[allow(missing_docs)]
    Link,
    #[allow(missing_docs)]
    LockClosed,
    #[allow(missing_docs)]
    Mail,
    #[allow(missing_docs)]
    Moon,
    #[allow(missing_docs)]
    MoreHorizontal,
    #[allow(missing_docs)]
    MoreVertical,
    #[allow(missing_docs)]
    Navigation,
    #[allow(missing_docs)]
    Open,
    #[allow(missing_docs)]
    Options,
    #[allow(missing_docs)]
    PanelLeft,
    #[allow(missing_docs)]
    Person,
    #[allow(missing_docs)]
    Pin,
    #[allow(missing_docs)]
    Play,
    #[allow(missing_docs)]
    QuestionCircle,
    #[allow(missing_docs)]
    Save,
    #[allow(missing_docs)]
    Search,
    #[allow(missing_docs)]
    Settings,
    #[allow(missing_docs)]
    Share,
    #[allow(missing_docs)]
    Square,
    #[allow(missing_docs)]
    SquareMultiple,
    #[allow(missing_docs)]
    Star,
    #[allow(missing_docs)]
    Stop,
    #[allow(missing_docs)]
    Subtract,
    #[allow(missing_docs)]
    Tab,
    #[allow(missing_docs)]
    Textbox,
    #[allow(missing_docs)]
    Toolbox,
    #[allow(missing_docs)]
    Warning,
    #[allow(missing_docs)]
    WeatherSunny,
    #[allow(missing_docs)]
    Wrench,
}

impl IconName {
    /// Every icon, in alphabetical order.
    pub const ALL: &[IconName] = &[
        IconName::Add,
        IconName::AddCircle,
        IconName::Alert,
        IconName::AppStore,
        IconName::Board,
        IconName::Bookmark,
        IconName::Bug,
        IconName::Calendar,
        IconName::Chat,
        IconName::Checkmark,
        IconName::ChevronDown,
        IconName::ChevronLeft,
        IconName::ChevronRight,
        IconName::ChevronUp,
        IconName::Circle,
        IconName::Clock,
        IconName::Cloud,
        IconName::Code,
        IconName::Copy,
        IconName::Cube,
        IconName::Database,
        IconName::Delete,
        IconName::Desktop,
        IconName::Dismiss,
        IconName::DismissCircle,
        IconName::Document,
        IconName::Drag,
        IconName::Edit,
        IconName::ErrorCircle,
        IconName::Eye,
        IconName::Filter,
        IconName::Folder,
        IconName::FolderOpen,
        IconName::FullScreenMaximize,
        IconName::Git,
        IconName::Grid,
        IconName::Heart,
        IconName::Home,
        IconName::Info,
        IconName::Keyboard,
        IconName::Layer,
        IconName::Lightbulb,
        IconName::Link,
        IconName::LockClosed,
        IconName::Mail,
        IconName::Moon,
        IconName::MoreHorizontal,
        IconName::MoreVertical,
        IconName::Navigation,
        IconName::Open,
        IconName::Options,
        IconName::PanelLeft,
        IconName::Person,
        IconName::Pin,
        IconName::Play,
        IconName::QuestionCircle,
        IconName::Save,
        IconName::Search,
        IconName::Settings,
        IconName::Share,
        IconName::Square,
        IconName::SquareMultiple,
        IconName::Star,
        IconName::Stop,
        IconName::Subtract,
        IconName::Tab,
        IconName::Textbox,
        IconName::Toolbox,
        IconName::Warning,
        IconName::WeatherSunny,
        IconName::Wrench,
    ];

    /// Kebab-case name (matches RefineUI System Icons slug).
    pub const fn slug(self) -> &'static str {
        match self {
            IconName::Add => "add",
            IconName::AddCircle => "add-circle",
            IconName::Alert => "alert",
            IconName::AppStore => "app-store",
            IconName::Board => "board",
            IconName::Bookmark => "bookmark",
            IconName::Bug => "bug",
            IconName::Calendar => "calendar",
            IconName::Chat => "chat",
            IconName::Checkmark => "checkmark",
            IconName::ChevronDown => "chevron-down",
            IconName::ChevronLeft => "chevron-left",
            IconName::ChevronRight => "chevron-right",
            IconName::ChevronUp => "chevron-up",
            IconName::Circle => "circle",
            IconName::Clock => "clock",
            IconName::Cloud => "cloud",
            IconName::Code => "code",
            IconName::Copy => "copy",
            IconName::Cube => "cube",
            IconName::Database => "database",
            IconName::Delete => "delete",
            IconName::Desktop => "desktop",
            IconName::Dismiss => "dismiss",
            IconName::DismissCircle => "dismiss-circle",
            IconName::Document => "document",
            IconName::Drag => "drag",
            IconName::Edit => "edit",
            IconName::ErrorCircle => "error-circle",
            IconName::Eye => "eye",
            IconName::Filter => "filter",
            IconName::Folder => "folder",
            IconName::FolderOpen => "folder-open",
            IconName::FullScreenMaximize => "full-screen-maximize",
            IconName::Git => "git",
            IconName::Grid => "grid",
            IconName::Heart => "heart",
            IconName::Home => "home",
            IconName::Info => "info",
            IconName::Keyboard => "keyboard",
            IconName::Layer => "layer",
            IconName::Lightbulb => "lightbulb",
            IconName::Link => "link",
            IconName::LockClosed => "lock-closed",
            IconName::Mail => "mail",
            IconName::Moon => "moon",
            IconName::MoreHorizontal => "more-horizontal",
            IconName::MoreVertical => "more-vertical",
            IconName::Navigation => "navigation",
            IconName::Open => "open",
            IconName::Options => "options",
            IconName::PanelLeft => "panel-left",
            IconName::Person => "person",
            IconName::Pin => "pin",
            IconName::Play => "play",
            IconName::QuestionCircle => "question-circle",
            IconName::Save => "save",
            IconName::Search => "search",
            IconName::Settings => "settings",
            IconName::Share => "share",
            IconName::Square => "square",
            IconName::SquareMultiple => "square-multiple",
            IconName::Star => "star",
            IconName::Stop => "stop",
            IconName::Subtract => "subtract",
            IconName::Tab => "tab",
            IconName::Textbox => "textbox",
            IconName::Toolbox => "toolbox",
            IconName::Warning => "warning",
            IconName::WeatherSunny => "weather-sunny",
            IconName::Wrench => "wrench",
        }
    }

    /// Asset path for a style.
    pub const fn path(self, style: IconStyle) -> &'static str {
        match (self, style) {
            (IconName::Add, IconStyle::Regular) => "icons/add-regular.svg",
            (IconName::Add, IconStyle::Filled) => "icons/add-filled.svg",
            (IconName::AddCircle, IconStyle::Regular) => "icons/add-circle-regular.svg",
            (IconName::AddCircle, IconStyle::Filled) => "icons/add-circle-filled.svg",
            (IconName::Alert, IconStyle::Regular) => "icons/alert-regular.svg",
            (IconName::Alert, IconStyle::Filled) => "icons/alert-filled.svg",
            (IconName::AppStore, IconStyle::Regular) => "icons/app-store-regular.svg",
            (IconName::AppStore, IconStyle::Filled) => "icons/app-store-filled.svg",
            (IconName::Board, IconStyle::Regular) => "icons/board-regular.svg",
            (IconName::Board, IconStyle::Filled) => "icons/board-filled.svg",
            (IconName::Bookmark, IconStyle::Regular) => "icons/bookmark-regular.svg",
            (IconName::Bookmark, IconStyle::Filled) => "icons/bookmark-filled.svg",
            (IconName::Bug, IconStyle::Regular) => "icons/bug-regular.svg",
            (IconName::Bug, IconStyle::Filled) => "icons/bug-filled.svg",
            (IconName::Calendar, IconStyle::Regular) => "icons/calendar-regular.svg",
            (IconName::Calendar, IconStyle::Filled) => "icons/calendar-filled.svg",
            (IconName::Chat, IconStyle::Regular) => "icons/chat-regular.svg",
            (IconName::Chat, IconStyle::Filled) => "icons/chat-filled.svg",
            (IconName::Checkmark, IconStyle::Regular) => "icons/checkmark-regular.svg",
            (IconName::Checkmark, IconStyle::Filled) => "icons/checkmark-filled.svg",
            (IconName::ChevronDown, IconStyle::Regular) => "icons/chevron-down-regular.svg",
            (IconName::ChevronDown, IconStyle::Filled) => "icons/chevron-down-filled.svg",
            (IconName::ChevronLeft, IconStyle::Regular) => "icons/chevron-left-regular.svg",
            (IconName::ChevronLeft, IconStyle::Filled) => "icons/chevron-left-filled.svg",
            (IconName::ChevronRight, IconStyle::Regular) => "icons/chevron-right-regular.svg",
            (IconName::ChevronRight, IconStyle::Filled) => "icons/chevron-right-filled.svg",
            (IconName::ChevronUp, IconStyle::Regular) => "icons/chevron-up-regular.svg",
            (IconName::ChevronUp, IconStyle::Filled) => "icons/chevron-up-filled.svg",
            (IconName::Circle, IconStyle::Regular) => "icons/circle-regular.svg",
            (IconName::Circle, IconStyle::Filled) => "icons/circle-filled.svg",
            (IconName::Clock, IconStyle::Regular) => "icons/clock-regular.svg",
            (IconName::Clock, IconStyle::Filled) => "icons/clock-filled.svg",
            (IconName::Cloud, IconStyle::Regular) => "icons/cloud-regular.svg",
            (IconName::Cloud, IconStyle::Filled) => "icons/cloud-filled.svg",
            (IconName::Code, IconStyle::Regular) => "icons/code-regular.svg",
            (IconName::Code, IconStyle::Filled) => "icons/code-filled.svg",
            (IconName::Copy, IconStyle::Regular) => "icons/copy-regular.svg",
            (IconName::Copy, IconStyle::Filled) => "icons/copy-filled.svg",
            (IconName::Cube, IconStyle::Regular) => "icons/cube-regular.svg",
            (IconName::Cube, IconStyle::Filled) => "icons/cube-filled.svg",
            (IconName::Database, IconStyle::Regular) => "icons/database-regular.svg",
            (IconName::Database, IconStyle::Filled) => "icons/database-filled.svg",
            (IconName::Delete, IconStyle::Regular) => "icons/delete-regular.svg",
            (IconName::Delete, IconStyle::Filled) => "icons/delete-filled.svg",
            (IconName::Desktop, IconStyle::Regular) => "icons/desktop-regular.svg",
            (IconName::Desktop, IconStyle::Filled) => "icons/desktop-filled.svg",
            (IconName::Dismiss, IconStyle::Regular) => "icons/dismiss-regular.svg",
            (IconName::Dismiss, IconStyle::Filled) => "icons/dismiss-filled.svg",
            (IconName::DismissCircle, IconStyle::Regular) => "icons/dismiss-circle-regular.svg",
            (IconName::DismissCircle, IconStyle::Filled) => "icons/dismiss-circle-filled.svg",
            (IconName::Document, IconStyle::Regular) => "icons/document-regular.svg",
            (IconName::Document, IconStyle::Filled) => "icons/document-filled.svg",
            (IconName::Drag, IconStyle::Regular) => "icons/drag-regular.svg",
            (IconName::Drag, IconStyle::Filled) => "icons/drag-filled.svg",
            (IconName::Edit, IconStyle::Regular) => "icons/edit-regular.svg",
            (IconName::Edit, IconStyle::Filled) => "icons/edit-filled.svg",
            (IconName::ErrorCircle, IconStyle::Regular) => "icons/error-circle-regular.svg",
            (IconName::ErrorCircle, IconStyle::Filled) => "icons/error-circle-filled.svg",
            (IconName::Eye, IconStyle::Regular) => "icons/eye-regular.svg",
            (IconName::Eye, IconStyle::Filled) => "icons/eye-filled.svg",
            (IconName::Filter, IconStyle::Regular) => "icons/filter-regular.svg",
            (IconName::Filter, IconStyle::Filled) => "icons/filter-filled.svg",
            (IconName::Folder, IconStyle::Regular) => "icons/folder-regular.svg",
            (IconName::Folder, IconStyle::Filled) => "icons/folder-filled.svg",
            (IconName::FolderOpen, IconStyle::Regular) => "icons/folder-open-regular.svg",
            (IconName::FolderOpen, IconStyle::Filled) => "icons/folder-open-filled.svg",
            (IconName::FullScreenMaximize, IconStyle::Regular) => {
                "icons/full-screen-maximize-regular.svg"
            }
            (IconName::FullScreenMaximize, IconStyle::Filled) => {
                "icons/full-screen-maximize-filled.svg"
            }
            (IconName::Git, IconStyle::Regular) => "icons/git-regular.svg",
            (IconName::Git, IconStyle::Filled) => "icons/git-filled.svg",
            (IconName::Grid, IconStyle::Regular) => "icons/grid-regular.svg",
            (IconName::Grid, IconStyle::Filled) => "icons/grid-filled.svg",
            (IconName::Heart, IconStyle::Regular) => "icons/heart-regular.svg",
            (IconName::Heart, IconStyle::Filled) => "icons/heart-filled.svg",
            (IconName::Home, IconStyle::Regular) => "icons/home-regular.svg",
            (IconName::Home, IconStyle::Filled) => "icons/home-filled.svg",
            (IconName::Info, IconStyle::Regular) => "icons/info-regular.svg",
            (IconName::Info, IconStyle::Filled) => "icons/info-filled.svg",
            (IconName::Keyboard, IconStyle::Regular) => "icons/keyboard-regular.svg",
            (IconName::Keyboard, IconStyle::Filled) => "icons/keyboard-filled.svg",
            (IconName::Layer, IconStyle::Regular) => "icons/layer-regular.svg",
            (IconName::Layer, IconStyle::Filled) => "icons/layer-filled.svg",
            (IconName::Lightbulb, IconStyle::Regular) => "icons/lightbulb-regular.svg",
            (IconName::Lightbulb, IconStyle::Filled) => "icons/lightbulb-filled.svg",
            (IconName::Link, IconStyle::Regular) => "icons/link-regular.svg",
            (IconName::Link, IconStyle::Filled) => "icons/link-filled.svg",
            (IconName::LockClosed, IconStyle::Regular) => "icons/lock-closed-regular.svg",
            (IconName::LockClosed, IconStyle::Filled) => "icons/lock-closed-filled.svg",
            (IconName::Mail, IconStyle::Regular) => "icons/mail-regular.svg",
            (IconName::Mail, IconStyle::Filled) => "icons/mail-filled.svg",
            (IconName::Moon, IconStyle::Regular) => "icons/moon-regular.svg",
            (IconName::Moon, IconStyle::Filled) => "icons/moon-filled.svg",
            (IconName::MoreHorizontal, IconStyle::Regular) => "icons/more-horizontal-regular.svg",
            (IconName::MoreHorizontal, IconStyle::Filled) => "icons/more-horizontal-filled.svg",
            (IconName::MoreVertical, IconStyle::Regular) => "icons/more-vertical-regular.svg",
            (IconName::MoreVertical, IconStyle::Filled) => "icons/more-vertical-filled.svg",
            (IconName::Navigation, IconStyle::Regular) => "icons/navigation-regular.svg",
            (IconName::Navigation, IconStyle::Filled) => "icons/navigation-filled.svg",
            (IconName::Open, IconStyle::Regular) => "icons/open-regular.svg",
            (IconName::Open, IconStyle::Filled) => "icons/open-filled.svg",
            (IconName::Options, IconStyle::Regular) => "icons/options-regular.svg",
            (IconName::Options, IconStyle::Filled) => "icons/options-filled.svg",
            (IconName::PanelLeft, IconStyle::Regular) => "icons/panel-left-regular.svg",
            (IconName::PanelLeft, IconStyle::Filled) => "icons/panel-left-filled.svg",
            (IconName::Person, IconStyle::Regular) => "icons/person-regular.svg",
            (IconName::Person, IconStyle::Filled) => "icons/person-filled.svg",
            (IconName::Pin, IconStyle::Regular) => "icons/pin-regular.svg",
            (IconName::Pin, IconStyle::Filled) => "icons/pin-filled.svg",
            (IconName::Play, IconStyle::Regular) => "icons/play-regular.svg",
            (IconName::Play, IconStyle::Filled) => "icons/play-filled.svg",
            (IconName::QuestionCircle, IconStyle::Regular) => "icons/question-circle-regular.svg",
            (IconName::QuestionCircle, IconStyle::Filled) => "icons/question-circle-filled.svg",
            (IconName::Save, IconStyle::Regular) => "icons/save-regular.svg",
            (IconName::Save, IconStyle::Filled) => "icons/save-filled.svg",
            (IconName::Search, IconStyle::Regular) => "icons/search-regular.svg",
            (IconName::Search, IconStyle::Filled) => "icons/search-filled.svg",
            (IconName::Settings, IconStyle::Regular) => "icons/settings-regular.svg",
            (IconName::Settings, IconStyle::Filled) => "icons/settings-filled.svg",
            (IconName::Share, IconStyle::Regular) => "icons/share-regular.svg",
            (IconName::Share, IconStyle::Filled) => "icons/share-filled.svg",
            (IconName::Square, IconStyle::Regular) => "icons/square-regular.svg",
            (IconName::Square, IconStyle::Filled) => "icons/square-filled.svg",
            (IconName::SquareMultiple, IconStyle::Regular) => "icons/square-multiple-regular.svg",
            (IconName::SquareMultiple, IconStyle::Filled) => "icons/square-multiple-filled.svg",
            (IconName::Star, IconStyle::Regular) => "icons/star-regular.svg",
            (IconName::Star, IconStyle::Filled) => "icons/star-filled.svg",
            (IconName::Stop, IconStyle::Regular) => "icons/stop-regular.svg",
            (IconName::Stop, IconStyle::Filled) => "icons/stop-filled.svg",
            (IconName::Subtract, IconStyle::Regular) => "icons/subtract-regular.svg",
            (IconName::Subtract, IconStyle::Filled) => "icons/subtract-filled.svg",
            (IconName::Tab, IconStyle::Regular) => "icons/tab-regular.svg",
            (IconName::Tab, IconStyle::Filled) => "icons/tab-filled.svg",
            (IconName::Textbox, IconStyle::Regular) => "icons/textbox-regular.svg",
            (IconName::Textbox, IconStyle::Filled) => "icons/textbox-filled.svg",
            (IconName::Toolbox, IconStyle::Regular) => "icons/toolbox-regular.svg",
            (IconName::Toolbox, IconStyle::Filled) => "icons/toolbox-filled.svg",
            (IconName::Warning, IconStyle::Regular) => "icons/warning-regular.svg",
            (IconName::Warning, IconStyle::Filled) => "icons/warning-filled.svg",
            (IconName::WeatherSunny, IconStyle::Regular) => "icons/weather-sunny-regular.svg",
            (IconName::WeatherSunny, IconStyle::Filled) => "icons/weather-sunny-filled.svg",
            (IconName::Wrench, IconStyle::Regular) => "icons/wrench-regular.svg",
            (IconName::Wrench, IconStyle::Filled) => "icons/wrench-filled.svg",
        }
    }
}

pub(crate) fn embedded(path: &str) -> Option<&'static [u8]> {
    Some(match path {
        "icons/add-circle-filled.svg" => include_bytes!("../assets/icons/add-circle-filled.svg"),
        "icons/add-circle-regular.svg" => include_bytes!("../assets/icons/add-circle-regular.svg"),
        "icons/add-filled.svg" => include_bytes!("../assets/icons/add-filled.svg"),
        "icons/add-regular.svg" => include_bytes!("../assets/icons/add-regular.svg"),
        "icons/alert-filled.svg" => include_bytes!("../assets/icons/alert-filled.svg"),
        "icons/alert-regular.svg" => include_bytes!("../assets/icons/alert-regular.svg"),
        "icons/app-store-filled.svg" => include_bytes!("../assets/icons/app-store-filled.svg"),
        "icons/app-store-regular.svg" => include_bytes!("../assets/icons/app-store-regular.svg"),
        "icons/board-filled.svg" => include_bytes!("../assets/icons/board-filled.svg"),
        "icons/board-regular.svg" => include_bytes!("../assets/icons/board-regular.svg"),
        "icons/bookmark-filled.svg" => include_bytes!("../assets/icons/bookmark-filled.svg"),
        "icons/bookmark-regular.svg" => include_bytes!("../assets/icons/bookmark-regular.svg"),
        "icons/bug-filled.svg" => include_bytes!("../assets/icons/bug-filled.svg"),
        "icons/bug-regular.svg" => include_bytes!("../assets/icons/bug-regular.svg"),
        "icons/calendar-filled.svg" => include_bytes!("../assets/icons/calendar-filled.svg"),
        "icons/calendar-regular.svg" => include_bytes!("../assets/icons/calendar-regular.svg"),
        "icons/chat-filled.svg" => include_bytes!("../assets/icons/chat-filled.svg"),
        "icons/chat-regular.svg" => include_bytes!("../assets/icons/chat-regular.svg"),
        "icons/checkmark-filled.svg" => include_bytes!("../assets/icons/checkmark-filled.svg"),
        "icons/checkmark-regular.svg" => include_bytes!("../assets/icons/checkmark-regular.svg"),
        "icons/chevron-down-filled.svg" => {
            include_bytes!("../assets/icons/chevron-down-filled.svg")
        }
        "icons/chevron-down-regular.svg" => {
            include_bytes!("../assets/icons/chevron-down-regular.svg")
        }
        "icons/chevron-left-filled.svg" => {
            include_bytes!("../assets/icons/chevron-left-filled.svg")
        }
        "icons/chevron-left-regular.svg" => {
            include_bytes!("../assets/icons/chevron-left-regular.svg")
        }
        "icons/chevron-right-filled.svg" => {
            include_bytes!("../assets/icons/chevron-right-filled.svg")
        }
        "icons/chevron-right-regular.svg" => {
            include_bytes!("../assets/icons/chevron-right-regular.svg")
        }
        "icons/chevron-up-filled.svg" => include_bytes!("../assets/icons/chevron-up-filled.svg"),
        "icons/chevron-up-regular.svg" => include_bytes!("../assets/icons/chevron-up-regular.svg"),
        "icons/circle-filled.svg" => include_bytes!("../assets/icons/circle-filled.svg"),
        "icons/circle-regular.svg" => include_bytes!("../assets/icons/circle-regular.svg"),
        "icons/clock-filled.svg" => include_bytes!("../assets/icons/clock-filled.svg"),
        "icons/clock-regular.svg" => include_bytes!("../assets/icons/clock-regular.svg"),
        "icons/cloud-filled.svg" => include_bytes!("../assets/icons/cloud-filled.svg"),
        "icons/cloud-regular.svg" => include_bytes!("../assets/icons/cloud-regular.svg"),
        "icons/code-filled.svg" => include_bytes!("../assets/icons/code-filled.svg"),
        "icons/code-regular.svg" => include_bytes!("../assets/icons/code-regular.svg"),
        "icons/copy-filled.svg" => include_bytes!("../assets/icons/copy-filled.svg"),
        "icons/copy-regular.svg" => include_bytes!("../assets/icons/copy-regular.svg"),
        "icons/cube-filled.svg" => include_bytes!("../assets/icons/cube-filled.svg"),
        "icons/cube-regular.svg" => include_bytes!("../assets/icons/cube-regular.svg"),
        "icons/database-filled.svg" => include_bytes!("../assets/icons/database-filled.svg"),
        "icons/database-regular.svg" => include_bytes!("../assets/icons/database-regular.svg"),
        "icons/delete-filled.svg" => include_bytes!("../assets/icons/delete-filled.svg"),
        "icons/delete-regular.svg" => include_bytes!("../assets/icons/delete-regular.svg"),
        "icons/desktop-filled.svg" => include_bytes!("../assets/icons/desktop-filled.svg"),
        "icons/desktop-regular.svg" => include_bytes!("../assets/icons/desktop-regular.svg"),
        "icons/dismiss-circle-filled.svg" => {
            include_bytes!("../assets/icons/dismiss-circle-filled.svg")
        }
        "icons/dismiss-circle-regular.svg" => {
            include_bytes!("../assets/icons/dismiss-circle-regular.svg")
        }
        "icons/dismiss-filled.svg" => include_bytes!("../assets/icons/dismiss-filled.svg"),
        "icons/dismiss-regular.svg" => include_bytes!("../assets/icons/dismiss-regular.svg"),
        "icons/document-filled.svg" => include_bytes!("../assets/icons/document-filled.svg"),
        "icons/document-regular.svg" => include_bytes!("../assets/icons/document-regular.svg"),
        "icons/drag-filled.svg" => include_bytes!("../assets/icons/drag-filled.svg"),
        "icons/drag-regular.svg" => include_bytes!("../assets/icons/drag-regular.svg"),
        "icons/edit-filled.svg" => include_bytes!("../assets/icons/edit-filled.svg"),
        "icons/edit-regular.svg" => include_bytes!("../assets/icons/edit-regular.svg"),
        "icons/error-circle-filled.svg" => {
            include_bytes!("../assets/icons/error-circle-filled.svg")
        }
        "icons/error-circle-regular.svg" => {
            include_bytes!("../assets/icons/error-circle-regular.svg")
        }
        "icons/eye-filled.svg" => include_bytes!("../assets/icons/eye-filled.svg"),
        "icons/eye-regular.svg" => include_bytes!("../assets/icons/eye-regular.svg"),
        "icons/filter-filled.svg" => include_bytes!("../assets/icons/filter-filled.svg"),
        "icons/filter-regular.svg" => include_bytes!("../assets/icons/filter-regular.svg"),
        "icons/folder-filled.svg" => include_bytes!("../assets/icons/folder-filled.svg"),
        "icons/folder-open-filled.svg" => include_bytes!("../assets/icons/folder-open-filled.svg"),
        "icons/folder-open-regular.svg" => {
            include_bytes!("../assets/icons/folder-open-regular.svg")
        }
        "icons/folder-regular.svg" => include_bytes!("../assets/icons/folder-regular.svg"),
        "icons/full-screen-maximize-filled.svg" => {
            include_bytes!("../assets/icons/full-screen-maximize-filled.svg")
        }
        "icons/full-screen-maximize-regular.svg" => {
            include_bytes!("../assets/icons/full-screen-maximize-regular.svg")
        }
        "icons/git-filled.svg" => include_bytes!("../assets/icons/git-filled.svg"),
        "icons/git-regular.svg" => include_bytes!("../assets/icons/git-regular.svg"),
        "icons/grid-filled.svg" => include_bytes!("../assets/icons/grid-filled.svg"),
        "icons/grid-regular.svg" => include_bytes!("../assets/icons/grid-regular.svg"),
        "icons/heart-filled.svg" => include_bytes!("../assets/icons/heart-filled.svg"),
        "icons/heart-regular.svg" => include_bytes!("../assets/icons/heart-regular.svg"),
        "icons/home-filled.svg" => include_bytes!("../assets/icons/home-filled.svg"),
        "icons/home-regular.svg" => include_bytes!("../assets/icons/home-regular.svg"),
        "icons/info-filled.svg" => include_bytes!("../assets/icons/info-filled.svg"),
        "icons/info-regular.svg" => include_bytes!("../assets/icons/info-regular.svg"),
        "icons/keyboard-filled.svg" => include_bytes!("../assets/icons/keyboard-filled.svg"),
        "icons/keyboard-regular.svg" => include_bytes!("../assets/icons/keyboard-regular.svg"),
        "icons/layer-filled.svg" => include_bytes!("../assets/icons/layer-filled.svg"),
        "icons/layer-regular.svg" => include_bytes!("../assets/icons/layer-regular.svg"),
        "icons/lightbulb-filled.svg" => include_bytes!("../assets/icons/lightbulb-filled.svg"),
        "icons/lightbulb-regular.svg" => include_bytes!("../assets/icons/lightbulb-regular.svg"),
        "icons/link-filled.svg" => include_bytes!("../assets/icons/link-filled.svg"),
        "icons/link-regular.svg" => include_bytes!("../assets/icons/link-regular.svg"),
        "icons/lock-closed-filled.svg" => include_bytes!("../assets/icons/lock-closed-filled.svg"),
        "icons/lock-closed-regular.svg" => {
            include_bytes!("../assets/icons/lock-closed-regular.svg")
        }
        "icons/mail-filled.svg" => include_bytes!("../assets/icons/mail-filled.svg"),
        "icons/mail-regular.svg" => include_bytes!("../assets/icons/mail-regular.svg"),
        "icons/moon-filled.svg" => include_bytes!("../assets/icons/moon-filled.svg"),
        "icons/moon-regular.svg" => include_bytes!("../assets/icons/moon-regular.svg"),
        "icons/more-horizontal-filled.svg" => {
            include_bytes!("../assets/icons/more-horizontal-filled.svg")
        }
        "icons/more-horizontal-regular.svg" => {
            include_bytes!("../assets/icons/more-horizontal-regular.svg")
        }
        "icons/more-vertical-filled.svg" => {
            include_bytes!("../assets/icons/more-vertical-filled.svg")
        }
        "icons/more-vertical-regular.svg" => {
            include_bytes!("../assets/icons/more-vertical-regular.svg")
        }
        "icons/navigation-filled.svg" => include_bytes!("../assets/icons/navigation-filled.svg"),
        "icons/navigation-regular.svg" => include_bytes!("../assets/icons/navigation-regular.svg"),
        "icons/open-filled.svg" => include_bytes!("../assets/icons/open-filled.svg"),
        "icons/open-regular.svg" => include_bytes!("../assets/icons/open-regular.svg"),
        "icons/options-filled.svg" => include_bytes!("../assets/icons/options-filled.svg"),
        "icons/options-regular.svg" => include_bytes!("../assets/icons/options-regular.svg"),
        "icons/panel-left-filled.svg" => include_bytes!("../assets/icons/panel-left-filled.svg"),
        "icons/panel-left-regular.svg" => include_bytes!("../assets/icons/panel-left-regular.svg"),
        "icons/person-filled.svg" => include_bytes!("../assets/icons/person-filled.svg"),
        "icons/person-regular.svg" => include_bytes!("../assets/icons/person-regular.svg"),
        "icons/pin-filled.svg" => include_bytes!("../assets/icons/pin-filled.svg"),
        "icons/pin-regular.svg" => include_bytes!("../assets/icons/pin-regular.svg"),
        "icons/play-filled.svg" => include_bytes!("../assets/icons/play-filled.svg"),
        "icons/play-regular.svg" => include_bytes!("../assets/icons/play-regular.svg"),
        "icons/question-circle-filled.svg" => {
            include_bytes!("../assets/icons/question-circle-filled.svg")
        }
        "icons/question-circle-regular.svg" => {
            include_bytes!("../assets/icons/question-circle-regular.svg")
        }
        "icons/save-filled.svg" => include_bytes!("../assets/icons/save-filled.svg"),
        "icons/save-regular.svg" => include_bytes!("../assets/icons/save-regular.svg"),
        "icons/search-filled.svg" => include_bytes!("../assets/icons/search-filled.svg"),
        "icons/search-regular.svg" => include_bytes!("../assets/icons/search-regular.svg"),
        "icons/settings-filled.svg" => include_bytes!("../assets/icons/settings-filled.svg"),
        "icons/settings-regular.svg" => include_bytes!("../assets/icons/settings-regular.svg"),
        "icons/share-filled.svg" => include_bytes!("../assets/icons/share-filled.svg"),
        "icons/share-regular.svg" => include_bytes!("../assets/icons/share-regular.svg"),
        "icons/square-filled.svg" => include_bytes!("../assets/icons/square-filled.svg"),
        "icons/square-multiple-filled.svg" => {
            include_bytes!("../assets/icons/square-multiple-filled.svg")
        }
        "icons/square-multiple-regular.svg" => {
            include_bytes!("../assets/icons/square-multiple-regular.svg")
        }
        "icons/square-regular.svg" => include_bytes!("../assets/icons/square-regular.svg"),
        "icons/star-filled.svg" => include_bytes!("../assets/icons/star-filled.svg"),
        "icons/star-regular.svg" => include_bytes!("../assets/icons/star-regular.svg"),
        "icons/stop-filled.svg" => include_bytes!("../assets/icons/stop-filled.svg"),
        "icons/stop-regular.svg" => include_bytes!("../assets/icons/stop-regular.svg"),
        "icons/subtract-filled.svg" => include_bytes!("../assets/icons/subtract-filled.svg"),
        "icons/subtract-regular.svg" => include_bytes!("../assets/icons/subtract-regular.svg"),
        "icons/tab-filled.svg" => include_bytes!("../assets/icons/tab-filled.svg"),
        "icons/tab-regular.svg" => include_bytes!("../assets/icons/tab-regular.svg"),
        "icons/textbox-filled.svg" => include_bytes!("../assets/icons/textbox-filled.svg"),
        "icons/textbox-regular.svg" => include_bytes!("../assets/icons/textbox-regular.svg"),
        "icons/toolbox-filled.svg" => include_bytes!("../assets/icons/toolbox-filled.svg"),
        "icons/toolbox-regular.svg" => include_bytes!("../assets/icons/toolbox-regular.svg"),
        "icons/warning-filled.svg" => include_bytes!("../assets/icons/warning-filled.svg"),
        "icons/warning-regular.svg" => include_bytes!("../assets/icons/warning-regular.svg"),
        "icons/weather-sunny-filled.svg" => {
            include_bytes!("../assets/icons/weather-sunny-filled.svg")
        }
        "icons/weather-sunny-regular.svg" => {
            include_bytes!("../assets/icons/weather-sunny-regular.svg")
        }
        "icons/wrench-filled.svg" => include_bytes!("../assets/icons/wrench-filled.svg"),
        "icons/wrench-regular.svg" => include_bytes!("../assets/icons/wrench-regular.svg"),
        _ => return None,
    })
}

pub(crate) const EMBEDDED_PATHS: &[&str] = &[
    "icons/add-circle-filled.svg",
    "icons/add-circle-regular.svg",
    "icons/add-filled.svg",
    "icons/add-regular.svg",
    "icons/alert-filled.svg",
    "icons/alert-regular.svg",
    "icons/app-store-filled.svg",
    "icons/app-store-regular.svg",
    "icons/board-filled.svg",
    "icons/board-regular.svg",
    "icons/bookmark-filled.svg",
    "icons/bookmark-regular.svg",
    "icons/bug-filled.svg",
    "icons/bug-regular.svg",
    "icons/calendar-filled.svg",
    "icons/calendar-regular.svg",
    "icons/chat-filled.svg",
    "icons/chat-regular.svg",
    "icons/checkmark-filled.svg",
    "icons/checkmark-regular.svg",
    "icons/chevron-down-filled.svg",
    "icons/chevron-down-regular.svg",
    "icons/chevron-left-filled.svg",
    "icons/chevron-left-regular.svg",
    "icons/chevron-right-filled.svg",
    "icons/chevron-right-regular.svg",
    "icons/chevron-up-filled.svg",
    "icons/chevron-up-regular.svg",
    "icons/circle-filled.svg",
    "icons/circle-regular.svg",
    "icons/clock-filled.svg",
    "icons/clock-regular.svg",
    "icons/cloud-filled.svg",
    "icons/cloud-regular.svg",
    "icons/code-filled.svg",
    "icons/code-regular.svg",
    "icons/copy-filled.svg",
    "icons/copy-regular.svg",
    "icons/cube-filled.svg",
    "icons/cube-regular.svg",
    "icons/database-filled.svg",
    "icons/database-regular.svg",
    "icons/delete-filled.svg",
    "icons/delete-regular.svg",
    "icons/desktop-filled.svg",
    "icons/desktop-regular.svg",
    "icons/dismiss-circle-filled.svg",
    "icons/dismiss-circle-regular.svg",
    "icons/dismiss-filled.svg",
    "icons/dismiss-regular.svg",
    "icons/document-filled.svg",
    "icons/document-regular.svg",
    "icons/drag-filled.svg",
    "icons/drag-regular.svg",
    "icons/edit-filled.svg",
    "icons/edit-regular.svg",
    "icons/error-circle-filled.svg",
    "icons/error-circle-regular.svg",
    "icons/eye-filled.svg",
    "icons/eye-regular.svg",
    "icons/filter-filled.svg",
    "icons/filter-regular.svg",
    "icons/folder-filled.svg",
    "icons/folder-open-filled.svg",
    "icons/folder-open-regular.svg",
    "icons/folder-regular.svg",
    "icons/full-screen-maximize-filled.svg",
    "icons/full-screen-maximize-regular.svg",
    "icons/git-filled.svg",
    "icons/git-regular.svg",
    "icons/grid-filled.svg",
    "icons/grid-regular.svg",
    "icons/heart-filled.svg",
    "icons/heart-regular.svg",
    "icons/home-filled.svg",
    "icons/home-regular.svg",
    "icons/info-filled.svg",
    "icons/info-regular.svg",
    "icons/keyboard-filled.svg",
    "icons/keyboard-regular.svg",
    "icons/layer-filled.svg",
    "icons/layer-regular.svg",
    "icons/lightbulb-filled.svg",
    "icons/lightbulb-regular.svg",
    "icons/link-filled.svg",
    "icons/link-regular.svg",
    "icons/lock-closed-filled.svg",
    "icons/lock-closed-regular.svg",
    "icons/mail-filled.svg",
    "icons/mail-regular.svg",
    "icons/moon-filled.svg",
    "icons/moon-regular.svg",
    "icons/more-horizontal-filled.svg",
    "icons/more-horizontal-regular.svg",
    "icons/more-vertical-filled.svg",
    "icons/more-vertical-regular.svg",
    "icons/navigation-filled.svg",
    "icons/navigation-regular.svg",
    "icons/open-filled.svg",
    "icons/open-regular.svg",
    "icons/options-filled.svg",
    "icons/options-regular.svg",
    "icons/panel-left-filled.svg",
    "icons/panel-left-regular.svg",
    "icons/person-filled.svg",
    "icons/person-regular.svg",
    "icons/pin-filled.svg",
    "icons/pin-regular.svg",
    "icons/play-filled.svg",
    "icons/play-regular.svg",
    "icons/question-circle-filled.svg",
    "icons/question-circle-regular.svg",
    "icons/save-filled.svg",
    "icons/save-regular.svg",
    "icons/search-filled.svg",
    "icons/search-regular.svg",
    "icons/settings-filled.svg",
    "icons/settings-regular.svg",
    "icons/share-filled.svg",
    "icons/share-regular.svg",
    "icons/square-filled.svg",
    "icons/square-multiple-filled.svg",
    "icons/square-multiple-regular.svg",
    "icons/square-regular.svg",
    "icons/star-filled.svg",
    "icons/star-regular.svg",
    "icons/stop-filled.svg",
    "icons/stop-regular.svg",
    "icons/subtract-filled.svg",
    "icons/subtract-regular.svg",
    "icons/tab-filled.svg",
    "icons/tab-regular.svg",
    "icons/textbox-filled.svg",
    "icons/textbox-regular.svg",
    "icons/toolbox-filled.svg",
    "icons/toolbox-regular.svg",
    "icons/warning-filled.svg",
    "icons/warning-regular.svg",
    "icons/weather-sunny-filled.svg",
    "icons/weather-sunny-regular.svg",
    "icons/wrench-filled.svg",
    "icons/wrench-regular.svg",
];
