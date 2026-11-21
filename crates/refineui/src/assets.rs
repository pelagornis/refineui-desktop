use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};

/// GPUI [`AssetSource`] serving RefineUI System Icons (`icons/<slug>-<size>-<style>.svg`).
///
/// Pass it to `Application::new().with_assets(refineui::Assets)`. If your app
/// already has an asset source, fall back to this one for `icons/` paths.
#[derive(Clone, Copy, Debug, Default)]
pub struct Assets;

/// Built-in spinner arc used by [`crate::Spinner`].
pub(crate) const SPINNER_PATH: &str = "refineui/spinner.svg";
const SPINNER_SVG: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 2.5 A9.5 9.5 0 0 1 21.5 12" stroke="black" stroke-width="2.5" stroke-linecap="round"/></svg>"#;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == SPINNER_PATH {
            return Ok(Some(Cow::Borrowed(SPINNER_SVG)));
        }
        Ok(crate::icon::svg_for_path(path).map(|svg| Cow::Borrowed(svg.as_bytes())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if !"icons/".starts_with(path) && !path.starts_with("icons/") {
            return Ok(Vec::new());
        }
        // Every curated icon at 20px (the full set is 890 glyphs x 7 sizes x 2 styles;
        // listing all of it is never useful).
        Ok(crate::IconName::ALL
            .iter()
            .flat_map(|n| {
                [crate::IconStyle::Regular, crate::IconStyle::Filled]
                    .map(|st| crate::icon::asset_path(n.glyph(), 20, st))
            })
            .filter(|p| p.starts_with(path))
            .collect())
    }
}

#[cfg(feature = "bundled-fonts")]
pub(crate) fn register_fonts(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-ExtraBold.ttf")),
    ];
    if let Err(err) = cx.text_system().add_fonts(fonts) {
        eprintln!("refineui: failed to register bundled Inter fonts: {err}");
    }
}

#[cfg(not(feature = "bundled-fonts"))]
#[allow(dead_code)]
pub(crate) fn register_fonts(_cx: &mut App) {}
