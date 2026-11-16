//! Component dimensions — `semantic/componentSizes.ts`, `formControlSizes.ts`
//! and the size maps in `@refineui/react` recipes.
//!
//! [`Density::Comfortable`] (the default) is pixel-for-pixel RefineUI web.
//! [`Density::Compact`] and [`Density::Spacious`] are the desktop variants
//! (brief §4); [`Density::desktop`] gives the shell / workspace metrics.

use crate::typography::{TextStyle, role};

/// UI density.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Density {
    /// RefineUI web parity (default): buttons 28 / 36 / 48, fields 36 / 44 / 52.
    #[default]
    Comfortable,
    /// Optional denser desktop sizing: buttons 22 / 28 / 32, fields 24 / 28 / 32.
    Compact,
    /// Roomier desktop sizing (touch-friendly / presentation): buttons
    /// 32 / 40 / 52, fields 40 / 48 / 56.
    Spacious,
}

impl Density {
    /// All densities, densest first.
    pub const ALL: [Density; 3] = [Self::Compact, Self::Comfortable, Self::Spacious];

    /// `compact` / `comfortable` / `spacious`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Comfortable => "comfortable",
            Self::Spacious => "spacious",
        }
    }

    /// Shell and workspace metrics for this density.
    pub const fn desktop(self) -> DesktopMetrics {
        use crate::spacing as s;
        match self {
            Density::Compact => DesktopMetrics {
                row_height: 26.0,
                tree_row_height: 22.0,
                tab_height: 28.0,
                toolbar_height: 32.0,
                status_bar_height: 22.0,
                panel_header_height: 28.0,
                activity_bar_width: 40.0,
                activity_item: 32.0,
                panel_padding: s::SMALL,
                gap: s::XX_SMALL,
                icon: icon::X_SMALL,
                splitter_hit: 4.0,
            },
            Density::Comfortable => DesktopMetrics {
                row_height: 32.0,
                tree_row_height: 28.0,
                tab_height: 36.0,
                toolbar_height: 40.0,
                status_bar_height: 24.0,
                panel_header_height: 36.0,
                activity_bar_width: 48.0,
                activity_item: 36.0,
                panel_padding: s::MEDIUM,
                gap: s::X_SMALL,
                icon: icon::X_SMALL,
                splitter_hit: 6.0,
            },
            Density::Spacious => DesktopMetrics {
                row_height: 40.0,
                tree_row_height: 32.0,
                tab_height: 40.0,
                toolbar_height: 48.0,
                status_bar_height: 28.0,
                panel_header_height: 44.0,
                activity_bar_width: 56.0,
                activity_item: 44.0,
                panel_padding: s::LARGE,
                gap: s::SMALL,
                icon: icon::SMALL,
                splitter_hit: 8.0,
            },
        }
    }
}

/// Desktop shell / workspace metrics for one [`Density`] (px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesktopMetrics {
    /// List / menu / table row.
    pub row_height: f32,
    /// Tree row.
    pub tree_row_height: f32,
    /// Document / panel tab.
    pub tab_height: f32,
    /// Toolbar (outside a unified titlebar).
    pub toolbar_height: f32,
    /// Status bar.
    pub status_bar_height: f32,
    /// Panel header (title + actions).
    pub panel_header_height: f32,
    /// Activity bar / navigation rail width.
    pub activity_bar_width: f32,
    /// Activity bar item (square).
    pub activity_item: f32,
    /// Panel content padding (spacing token).
    pub panel_padding: f32,
    /// Default gap between siblings (spacing token).
    pub gap: f32,
    /// Default chrome icon size (icon token).
    pub icon: f32,
    /// Split-view handle hit thickness (the visible line stays a hairline).
    pub splitter_hit: f32,
}

/// Small / medium / large control size (`sm | md | lg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControlSize {
    /// `sm`
    Small,
    /// `md` (default)
    #[default]
    Medium,
    /// `lg`
    Large,
}

impl ControlSize {
    /// All sizes, smallest first.
    pub const ALL: [ControlSize; 3] = [Self::Small, Self::Medium, Self::Large];

    /// `sm` / `md` / `lg`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
}

/// Resolved box metrics for a sized control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlMetrics {
    /// Minimum / fixed height.
    pub height: f32,
    /// Inline padding.
    pub padding_x: f32,
    /// Block padding.
    pub padding_y: f32,
    /// Corner radius.
    pub radius: f32,
    /// Text style.
    pub text: TextStyle,
    /// Icon size.
    pub icon: f32,
}

impl Density {
    /// Button metrics (`buttonLabelSizeClass` in `button.recipe.ts`).
    pub const fn button(self, size: ControlSize) -> ControlMetrics {
        use crate::{radius as r, spacing as s};
        match (self, size) {
            (Density::Comfortable, ControlSize::Small) => ControlMetrics {
                height: 28.0,
                padding_x: s::MEDIUM,
                padding_y: s::XX_SMALL,
                radius: r::LARGE,
                text: role::BODY_SM,
                icon: icon::X_SMALL,
            },
            (Density::Comfortable, ControlSize::Medium) => ControlMetrics {
                height: 36.0,
                padding_x: s::MEDIUM,
                padding_y: s::X_SMALL,
                radius: r::X_LARGE,
                text: role::BODY_LG,
                icon: icon::SMALL,
            },
            (Density::Comfortable, ControlSize::Large) => ControlMetrics {
                height: 48.0,
                padding_x: s::LARGE,
                padding_y: s::MEDIUM,
                radius: r::XX_LARGE,
                text: role::SUBTITLE_LG,
                icon: icon::MEDIUM,
            },
            (Density::Spacious, ControlSize::Small) => ControlMetrics {
                height: 32.0,
                padding_x: s::MEDIUM,
                padding_y: s::X_SMALL,
                radius: r::LARGE,
                text: role::BODY_MD,
                icon: icon::X_SMALL,
            },
            (Density::Spacious, ControlSize::Medium) => ControlMetrics {
                height: 40.0,
                padding_x: s::LARGE,
                padding_y: s::SMALL,
                radius: r::X_LARGE,
                text: role::BODY_LG,
                icon: icon::SMALL,
            },
            (Density::Spacious, ControlSize::Large) => ControlMetrics {
                height: 52.0,
                padding_x: s::X_LARGE,
                padding_y: s::MEDIUM,
                radius: r::XX_LARGE,
                text: role::SUBTITLE_LG,
                icon: icon::MEDIUM,
            },
            (Density::Compact, ControlSize::Small) => ControlMetrics {
                height: 22.0,
                padding_x: s::SMALL,
                padding_y: s::XXX_SMALL,
                radius: r::MEDIUM,
                text: role::CAPTION_MD,
                icon: icon::XX_SMALL,
            },
            (Density::Compact, ControlSize::Medium) => ControlMetrics {
                height: 28.0,
                padding_x: s::MEDIUM,
                padding_y: s::XX_SMALL,
                radius: r::LARGE,
                text: role::BODY_SM,
                icon: icon::X_SMALL,
            },
            (Density::Compact, ControlSize::Large) => ControlMetrics {
                height: 32.0,
                padding_x: s::LARGE,
                padding_y: s::X_SMALL,
                radius: r::LARGE,
                text: role::BODY_MD,
                icon: icon::X_SMALL,
            },
        }
    }

    /// Text-field metrics (`formControlInputSizeClass` — Input, SpinButton, SearchField).
    pub const fn field(self, size: ControlSize) -> ControlMetrics {
        use crate::{radius as r, spacing as s};
        match (self, size) {
            (Density::Comfortable, ControlSize::Small) => ControlMetrics {
                height: 36.0,
                padding_x: s::MEDIUM,
                padding_y: s::X_SMALL,
                radius: r::LARGE,
                text: role::BODY_SM,
                icon: icon::SMALL,
            },
            (Density::Comfortable, ControlSize::Medium) => ControlMetrics {
                height: 44.0,
                padding_x: s::LARGE,
                padding_y: s::MEDIUM,
                radius: r::X_LARGE,
                text: role::BODY_MD,
                icon: icon::MEDIUM,
            },
            (Density::Comfortable, ControlSize::Large) => ControlMetrics {
                height: 52.0,
                padding_x: s::X_LARGE,
                padding_y: s::LARGE,
                radius: r::XX_LARGE,
                text: role::BODY_LG,
                icon: icon::LARGE,
            },
            (Density::Spacious, ControlSize::Small) => ControlMetrics {
                height: 40.0,
                padding_x: s::MEDIUM,
                padding_y: s::SMALL,
                radius: r::LARGE,
                text: role::BODY_MD,
                icon: icon::SMALL,
            },
            (Density::Spacious, ControlSize::Medium) => ControlMetrics {
                height: 48.0,
                padding_x: s::LARGE,
                padding_y: s::MEDIUM,
                radius: r::X_LARGE,
                text: role::BODY_LG,
                icon: icon::MEDIUM,
            },
            (Density::Spacious, ControlSize::Large) => ControlMetrics {
                height: 56.0,
                padding_x: s::X_LARGE,
                padding_y: s::LARGE,
                radius: r::XX_LARGE,
                text: role::BODY_LG,
                icon: icon::LARGE,
            },
            (Density::Compact, ControlSize::Small) => ControlMetrics {
                height: 24.0,
                padding_x: s::SMALL,
                padding_y: s::XX_SMALL,
                radius: r::MEDIUM,
                text: role::CAPTION_MD,
                icon: icon::XX_SMALL,
            },
            (Density::Compact, ControlSize::Medium) => ControlMetrics {
                height: 28.0,
                padding_x: s::MEDIUM,
                padding_y: s::XX_SMALL,
                radius: r::LARGE,
                text: role::BODY_SM,
                icon: icon::X_SMALL,
            },
            (Density::Compact, ControlSize::Large) => ControlMetrics {
                height: 32.0,
                padding_x: s::MEDIUM,
                padding_y: s::X_SMALL,
                radius: r::LARGE,
                text: role::BODY_MD,
                icon: icon::X_SMALL,
            },
        }
    }

    /// Select trigger (`selectTriggerSizeClass`: button min-heights, select typography).
    pub const fn select(self, size: ControlSize) -> ControlMetrics {
        let b = self.button(size);
        let text = match (self, size) {
            (_, ControlSize::Small) => role::CAPTION_MD,
            (_, ControlSize::Medium) => role::CAPTION_LG,
            (_, ControlSize::Large) => role::BODY_MD,
        };
        ControlMetrics {
            height: b.height,
            padding_x: crate::spacing::MEDIUM,
            padding_y: crate::spacing::X_SMALL,
            radius: crate::radius::LARGE,
            text,
            icon: icon::X_SMALL,
        }
    }

    /// Chip metrics (`chipSizeClass`).
    pub const fn chip(self, size: ControlSize) -> ControlMetrics {
        use crate::spacing as s;
        let (height, padding_x, text, ic) = match size {
            ControlSize::Small => (20.0, s::X_SMALL, role::LABEL_SM, icon::XX_SMALL),
            ControlSize::Medium => (24.0, s::X_SMALL, role::BODY_SM, icon::X_SMALL),
            ControlSize::Large => (32.0, s::SMALL, role::BODY_LG, icon::SMALL),
        };
        ControlMetrics {
            height,
            padding_x,
            padding_y: 0.0,
            radius: crate::radius::MEDIUM,
            text,
            icon: ic,
        }
    }

    /// List / menu row height (item padding x-small + body-md line height).
    pub const fn row_height(self) -> f32 {
        match self {
            Density::Comfortable => 32.0,
            Density::Compact => 26.0,
            Density::Spacious => 40.0,
        }
    }
}

/// `iconSizes` (`global/sizes.ts`).
pub mod icon {
    /// `xxsmall`
    pub const XX_SMALL: f32 = 12.0;
    /// `xsmall`
    pub const X_SMALL: f32 = 16.0;
    /// `small`
    pub const SMALL: f32 = 20.0;
    /// `medium`
    pub const MEDIUM: f32 = 24.0;
    /// `large`
    pub const LARGE: f32 = 28.0;
    /// `xlarge`
    pub const X_LARGE: f32 = 32.0;
    /// `xxlarge`
    pub const XX_LARGE: f32 = 48.0;
}

/// Fixed component dimensions (`componentSizes`, px).
pub mod component {
    /// `controlCheckbox` / `controlCheckboxRadio`
    pub const CHECKBOX: f32 = 20.0;
    /// Radio inner dot (`spacing.medium`).
    pub const RADIO_DOT: f32 = 10.0;
    /// `controlTouchMin` — accordion trigger min height.
    pub const TOUCH_MIN: f32 = 44.0;
    /// `controlTextareaMin`
    pub const TEXTAREA_MIN: f32 = 80.0;
    /// `switchWidth`
    pub const SWITCH_WIDTH: f32 = 44.0;
    /// `switchHeight`
    pub const SWITCH_HEIGHT: f32 = 24.0;
    /// `switchThumb`
    pub const SWITCH_THUMB: f32 = 20.0;
    /// `switchPadding`
    pub const SWITCH_PADDING: f32 = 2.0;
    /// `progressTrackHeightSm`
    pub const PROGRESS_TRACK_SM: f32 = 2.0;
    /// `progressTrackHeightLg`
    pub const PROGRESS_TRACK_LG: f32 = 4.0;
    /// `sliderTrackHeightSm`
    pub const SLIDER_TRACK_SM: f32 = 2.0;
    /// `sliderTrackHeightMd`
    pub const SLIDER_TRACK: f32 = 4.0;
    /// `sliderInteractionHeight`
    pub const SLIDER_INTERACTION: f32 = 28.0;
    /// Slider thumb diameter (`foundationSize200`).
    pub const SLIDER_THUMB: f32 = 20.0;
    /// `menuPanelWidth`
    pub const MENU_PANEL_WIDTH: f32 = 200.0;
    /// `popoverPanelWidth`
    pub const POPOVER_PANEL_WIDTH: f32 = 325.0;
    /// `popoverBeakWidth`
    pub const POPOVER_BEAK_WIDTH: f32 = 24.0;
    /// `popoverBeakHeight`
    pub const POPOVER_BEAK_HEIGHT: f32 = 12.0;
    /// `popoverBeakTipRadius`
    pub const POPOVER_BEAK_TIP_RADIUS: f32 = 4.0;
    /// `popoverBeakInsetStart`
    pub const POPOVER_BEAK_INSET_START: f32 = 34.0;
    /// `popoverBeakInsetEnd`
    pub const POPOVER_BEAK_INSET_END: f32 = 30.0;
    /// `sidebarWidth`
    pub const SIDEBAR_WIDTH: f32 = 256.0;
    /// `toastMaxWidth`
    pub const TOAST_MAX_WIDTH: f32 = 325.0;
    /// `dialogMaxWidth`
    pub const DIALOG_MAX_WIDTH: f32 = 600.0;
    /// `dialogWidthSm`
    pub const DIALOG_WIDTH_SM: f32 = 300.0;
    /// `drawerWidthSm`
    pub const DRAWER_WIDTH_SM: f32 = 320.0;
    /// `drawerWidthMd`
    pub const DRAWER_WIDTH_MD: f32 = 575.0;
    /// `drawerWidthLg`
    pub const DRAWER_WIDTH_LG: f32 = 850.0;
    /// `calendarDaySize`
    pub const CALENDAR_DAY: f32 = 32.0;
    /// `calendarMinWidth`
    pub const CALENDAR_MIN_WIDTH: f32 = 256.0;
    /// `tooltipMaxWidth`
    pub const TOOLTIP_MAX_WIDTH: f32 = 200.0;
    /// `tooltipBeakWidth`
    pub const TOOLTIP_BEAK_WIDTH: f32 = 16.0;
    /// `tooltipBeakHeight`
    pub const TOOLTIP_BEAK_HEIGHT: f32 = 8.0;
    /// `tooltipBeakTipRadius`
    pub const TOOLTIP_BEAK_TIP_RADIUS: f32 = 2.0;
    /// `spinStepperWidth`
    pub const SPIN_STEPPER_WIDTH: f32 = 32.0;
    /// `spinFieldPaddingInlineStart`
    pub const SPIN_FIELD_PADDING_START: f32 = 12.0;
    /// `paginationButtonMinWidth`
    pub const PAGINATION_BUTTON: f32 = 40.0;
    /// `spinnerSize*` — xs, sm, md, lg, xl, xxl
    pub const SPINNER_SIZES: [f32; 6] = [16.0, 20.0, 24.0, 28.0, 32.0, 48.0];
    /// `spinnerRingWidth*` — xs, sm, md, lg, xl, xxl
    pub const SPINNER_RING_WIDTHS: [f32; 6] = [2.0, 2.0, 2.0, 3.0, 3.0, 4.0];
    /// `skeletonDefaultHeight`
    pub const SKELETON_HEIGHT: f32 = 20.0;
    /// `dividerShortEnd`
    pub const DIVIDER_SHORT_END: f32 = 8.0;
    /// Avatar sizes — 3xs, 2xs, xs, sm, md, lg, xl, 2xl, 3xl (`avatarSizeVar`).
    pub const AVATAR_SIZES: [f32; 9] = [16.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0, 54.0, 60.0];
    /// Table header row height (`controlHeightSm`).
    pub const TABLE_HEAD_HEIGHT: f32 = 36.0;
    /// Table body row height (`controlHeightMd`).
    pub const TABLE_ROW_HEIGHT: f32 = 44.0;
    /// `avatarStatusXxxlarge`
    pub const AVATAR_STATUS_XXXLARGE: f32 = 14.0;
    /// `timeInputWheelHeight`
    pub const TIME_INPUT_WHEEL_HEIGHT: f32 = 140.0;
    /// `navigationMenuItemMinWidth`
    pub const NAVIGATION_MENU_ITEM_MIN_WIDTH: f32 = 40.0;
    /// `navigationMenuPanelWidth`
    pub const NAVIGATION_MENU_PANEL_WIDTH: f32 = 325.0;
    /// `navigationMenuMegaWidth`
    pub const NAVIGATION_MENU_MEGA_WIDTH: f32 = 1000.0;
    /// `dropdownMenuWidth`
    pub const DROPDOWN_MENU_WIDTH: f32 = 200.0;
    /// `dialogMaxHeightViewport` (fraction of the window height; web `90vh`).
    pub const DIALOG_MAX_HEIGHT_FRACTION: f32 = 0.9;
    /// `dividerIconSlot`
    pub const DIVIDER_ICON_SLOT: f32 = 20.0;
    /// `dividerIconCircleDiameter`
    pub const DIVIDER_ICON_CIRCLE_DIAMETER: f32 = 13.33;
    /// `dividerIconCircleInset`
    pub const DIVIDER_ICON_CIRCLE_INSET: f32 = 3.0;
    /// `accessibleClip` (visually hidden text clip size).
    pub const ACCESSIBLE_CLIP: f32 = 1.0;
}
