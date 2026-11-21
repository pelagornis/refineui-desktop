//! # refineui-tokens (compatibility)
//!
//! The token crate was renamed to [`refineui_foundation`] when RefineUI
//! Desktop split into Foundation / Desktop / Material layers. This crate
//! re-exports it unchanged so existing `refineui_tokens::…` paths keep
//! compiling. Depend on `refineui-foundation` in new code.

#![warn(missing_docs)]

pub use refineui_foundation::*;

#[cfg(test)]
mod tests {
    #[test]
    fn reexports_foundation_modules() {
        assert_eq!(
            crate::semantic::SemanticColors::LIGHT,
            refineui_foundation::semantic::SemanticColors::LIGHT
        );
        assert_eq!(crate::blur::BLUR_16, 16.0);
    }
}
