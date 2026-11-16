//! Typography — `global/fonts.ts` + `global/typographys.ts`.

/// Font family used by RefineUI (`'Inter', sans-serif` on the web). The
/// `refineui` crate bundles Inter so desktop rendering matches.
pub const FONT_FAMILY: &str = "Inter";

/// Monospace family for code / keyboard shortcuts (not in the web tokens; desktop addition).
pub const MONO_FONT_FAMILY: &str = "Menlo";

/// CSS-style numeric font weight.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct FontWeight(pub f32);

impl FontWeight {
    /// `fontWeightRegular` (400)
    pub const REGULAR: Self = Self(400.0);
    /// `fontWeightMedium` (500)
    pub const MEDIUM: Self = Self(500.0);
    /// `fontWeightSemibold` (600)
    pub const SEMIBOLD: Self = Self(600.0);
    /// `fontWeightBold` (700)
    pub const BOLD: Self = Self(700.0);
    /// `fontWeightHeavy` (800)
    pub const HEAVY: Self = Self(800.0);
}

/// One named text style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Web token name (`heading1`, `body3`, …).
    pub name: &'static str,
    /// Font size in px.
    pub size: f32,
    /// Line height in px.
    pub line_height: f32,
    /// Font weight.
    pub weight: FontWeight,
}

macro_rules! styles {
    ($($konst:ident = $name:literal, $size:literal, $lh:literal, $w:ident;)*) => {
        $(
            #[doc = concat!("`", $name, "` — ", stringify!($size), "px / ", stringify!($lh), "px, ", stringify!($w), ".")]
            pub const $konst: TextStyle = TextStyle {
                name: $name,
                size: $size,
                line_height: $lh,
                weight: FontWeight::$w,
            };
        )*
        /// Every text style in scale order.
        pub const ALL: &[TextStyle] = &[$($konst),*];
    };
}

styles! {
    HEADING_1 = "heading1", 64.0, 86.0, HEAVY;
    HEADING_2 = "heading2", 40.0, 60.0, BOLD;
    HEADING_3 = "heading3", 32.0, 40.0, BOLD;
    HEADING_4 = "heading4", 28.0, 36.0, SEMIBOLD;
    TITLE_1 = "title1", 32.0, 40.0, SEMIBOLD;
    TITLE_2 = "title2", 28.0, 36.0, SEMIBOLD;
    TITLE_3 = "title3", 24.0, 32.0, SEMIBOLD;
    SUB_TITLE_1 = "subTitle1", 20.0, 28.0, SEMIBOLD;
    SUB_TITLE_2 = "subTitle2", 16.0, 24.0, SEMIBOLD;
    BODY_1 = "body1", 16.0, 24.0, MEDIUM;
    BODY_2 = "body2", 14.0, 20.0, MEDIUM;
    BODY_3 = "body3", 14.0, 20.0, REGULAR;
    BODY_4 = "body4", 12.0, 16.0, REGULAR;
    CAPTION_1 = "caption1", 12.0, 16.0, MEDIUM;
    CAPTION_2 = "caption2", 12.0, 16.0, REGULAR;
    CAPTION_3 = "caption3", 10.0, 14.0, REGULAR;
}

/// Semantic text roles (`semantic/text.ts`) — what components reference.
pub mod role {
    use super::*;
    /// `displayLg` → heading1
    pub const DISPLAY_LG: TextStyle = HEADING_1;
    /// `displayMd` → heading2
    pub const DISPLAY_MD: TextStyle = HEADING_2;
    /// `displaySm` → heading3
    pub const DISPLAY_SM: TextStyle = HEADING_3;
    /// `headingLg` → heading2
    pub const HEADING_LG: TextStyle = HEADING_2;
    /// `headingMd` → heading3
    pub const HEADING_MD: TextStyle = HEADING_3;
    /// `headingSm` → heading4
    pub const HEADING_SM: TextStyle = HEADING_4;
    /// `titleLg` → title1
    pub const TITLE_LG: TextStyle = TITLE_1;
    /// `titleMd` → title2
    pub const TITLE_MD: TextStyle = TITLE_2;
    /// `titleSm` → title3
    pub const TITLE_SM: TextStyle = TITLE_3;
    /// `subtitleLg` → subTitle1
    pub const SUBTITLE_LG: TextStyle = SUB_TITLE_1;
    /// `subtitleMd` → subTitle2
    pub const SUBTITLE_MD: TextStyle = SUB_TITLE_2;
    /// `bodyLg` → body1
    pub const BODY_LG: TextStyle = BODY_1;
    /// `bodyMd` → body2
    pub const BODY_MD: TextStyle = BODY_2;
    /// `bodySm` → body3
    pub const BODY_SM: TextStyle = BODY_3;
    /// `bodyXs` → body4
    pub const BODY_XS: TextStyle = BODY_4;
    /// `labelLg` → body1
    pub const LABEL_LG: TextStyle = BODY_1;
    /// `labelMd` → body2
    pub const LABEL_MD: TextStyle = BODY_2;
    /// `labelSm` → caption1
    pub const LABEL_SM: TextStyle = CAPTION_1;
    /// `captionLg` → caption1
    pub const CAPTION_LG: TextStyle = CAPTION_1;
    /// `captionMd` → caption2
    pub const CAPTION_MD: TextStyle = CAPTION_2;
    /// `captionSm` → caption3
    pub const CAPTION_SM: TextStyle = CAPTION_3;
}
