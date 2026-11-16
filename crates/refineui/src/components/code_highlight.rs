//! Syntax colours for [`crate::CodeBlock`] — the web docs code frames use
//! Shiki with Night Owl Light / Night Owl (`remarkDocsCode.mjs`). Here the
//! same theme rules (generated into `code_theme_data.rs`) drive `syntect`'s
//! Sublime grammars (pure-Rust `fancy-regex`, no Oniguruma).
//!
//! Without the `syntax-highlight` feature every line is a single run in the
//! default foreground.

use std::ops::Range;

use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, Rgba};

#[cfg(feature = "syntax-highlight")]
use super::code_theme_data as data;

/// One coloured run of a line (byte range into the line).
pub type CodeRun = (Range<usize>, HighlightStyle);

fn hsla(rgba: u32) -> Hsla {
    Rgba {
        r: ((rgba >> 24) & 0xff) as f32 / 255.,
        g: ((rgba >> 16) & 0xff) as f32 / 255.,
        b: ((rgba >> 8) & 0xff) as f32 / 255.,
        a: (rgba & 0xff) as f32 / 255.,
    }
    .into()
}

/// The theme's default foreground (Night Owl `editor.foreground`).
pub fn code_foreground(dark: bool) -> Hsla {
    #[cfg(feature = "syntax-highlight")]
    {
        hsla(if dark { data::DARK_FG } else { data::LIGHT_FG })
    }
    #[cfg(not(feature = "syntax-highlight"))]
    {
        hsla(if dark { 0xd6deebff } else { 0x403f53ff })
    }
}

#[cfg(feature = "syntax-highlight")]
mod engine {
    use std::str::FromStr;
    use std::sync::OnceLock;

    use syntect::highlighting::{
        Color, FontStyle, HighlightIterator, HighlightState, Highlighter, ScopeSelectors,
        StyleModifier, Theme, ThemeItem, ThemeSettings,
    };
    use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

    use super::data;

    pub(super) fn syntaxes() -> &'static SyntaxSet {
        static SET: OnceLock<SyntaxSet> = OnceLock::new();
        SET.get_or_init(SyntaxSet::load_defaults_newlines)
    }

    fn color(rgba: u32) -> Color {
        Color {
            r: (rgba >> 24) as u8,
            g: (rgba >> 16) as u8,
            b: (rgba >> 8) as u8,
            a: rgba as u8,
        }
    }

    fn build(rules: &[data::Rule], fg: u32) -> Theme {
        // VS Code / Shiki: on equal specificity the *later* rule wins; syntect
        // keeps the first one, so feed the rules in reverse.
        let scopes: Vec<ThemeItem> = rules
            .iter()
            .rev()
            .filter_map(|(sel, fg, style)| {
                let scope = ScopeSelectors::from_str(sel).ok()?;
                Some(ThemeItem {
                    scope,
                    style: StyleModifier {
                        foreground: fg.map(color),
                        background: None,
                        font_style: (*style != 255).then(|| FontStyle::from_bits_truncate(*style)),
                    },
                })
            })
            .collect();
        // Sublime grammars name a few things differently from the TextMate
        // grammars Shiki uses; give them the colour of the TextMate scope.
        let mut scopes = scopes;
        for (sublime, textmate) in ALIASES {
            let rule = rules
                .iter()
                .rev()
                .find(|(sel, _, _)| sel.split(',').any(|s| s.trim() == *textmate));
            if let (Some((_, fg, style)), Ok(scope)) = (rule, ScopeSelectors::from_str(sublime)) {
                scopes.insert(
                    0,
                    ThemeItem {
                        scope,
                        style: StyleModifier {
                            foreground: fg.map(color),
                            background: None,
                            font_style: (*style != 255)
                                .then(|| FontStyle::from_bits_truncate(*style)),
                        },
                    },
                );
            }
        }
        Theme {
            name: None,
            author: None,
            settings: ThemeSettings {
                foreground: Some(color(fg)),
                ..Default::default()
            },
            scopes,
        }
    }

    /// `(Sublime scope, TextMate scope whose colour it takes)`.
    const ALIASES: &[(&str, &str)] = &[
        ("support.macro", "entity.name.function"),
        ("variable.function", "entity.name.function"),
        ("entity.name.type", "entity.name.type"),
    ];

    pub(super) fn theme(dark: bool) -> &'static Theme {
        static DARK: OnceLock<Theme> = OnceLock::new();
        static LIGHT: OnceLock<Theme> = OnceLock::new();
        if dark {
            DARK.get_or_init(|| build(data::DARK, data::DARK_FG))
        } else {
            LIGHT.get_or_init(|| build(data::LIGHT, data::LIGHT_FG))
        }
    }

    /// Per line: `(byte range, rgba, font style bits)`.
    pub(super) fn runs(
        code: &str,
        lang: &str,
        dark: bool,
    ) -> Option<Vec<Vec<(std::ops::Range<usize>, u32, u8)>>> {
        let set = syntaxes();
        let syntax = super::resolve(lang).and_then(|t| set.find_syntax_by_token(t))?;
        let highlighter = Highlighter::new(theme(dark));
        let mut parse = ParseState::new(syntax);
        let mut hl = HighlightState::new(&highlighter, ScopeStack::new());
        let mut out = Vec::new();
        for line in code.lines() {
            let with_nl = format!("{line}\n");
            let ops = parse.parse_line(&with_nl, set).ok()?;
            let mut runs = Vec::new();
            let mut at = 0;
            for (style, text) in HighlightIterator::new(&mut hl, &ops, &with_nl, &highlighter) {
                let start = at;
                at += text.len();
                let end = at.min(line.len());
                if start >= end {
                    continue;
                }
                let f = style.foreground;
                let rgba = (f.r as u32) << 24 | (f.g as u32) << 16 | (f.b as u32) << 8 | f.a as u32;
                runs.push((start..end, rgba, style.font_style.bits()));
            }
            out.push(runs);
        }
        Some(out)
    }
}

/// Map a fence language to a syntect token (Sublime default grammars have
/// no TypeScript / TOML: TS / TSX use JavaScript, TOML uses INI-like YAML-less
/// fallback = none).
pub fn resolve(lang: &str) -> Option<&'static str> {
    Some(match lang.to_ascii_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "js" | "javascript" | "jsx" | "mjs" | "ts" | "typescript" | "tsx" => "js",
        "sh" | "bash" | "shell" | "zsh" | "console" => "sh",
        "json" => "json",
        "css" => "css",
        "html" | "xml" => "html",
        "py" | "python" => "py",
        "yaml" | "yml" => "yaml",
        "md" | "markdown" | "mdx" => "md",
        "c" | "h" => "c",
        "cpp" | "c++" | "hpp" => "cpp",
        "go" => "go",
        "java" => "java",
        "sql" => "sql",
        "diff" | "patch" => "diff",
        _ => return None,
    })
}

/// Coloured runs for every line of `code`, or `None` when the language is
/// unknown (or highlighting is compiled out) — render plain text then.
pub fn highlight(code: &str, lang: &str, dark: bool) -> Option<Vec<Vec<CodeRun>>> {
    #[cfg(feature = "syntax-highlight")]
    {
        let lines = engine::runs(code, lang, dark)?;
        Some(
            lines
                .into_iter()
                .map(|runs| {
                    runs.into_iter()
                        .map(|(r, rgba, bits)| {
                            let style = HighlightStyle {
                                color: Some(hsla(rgba)),
                                font_weight: (bits & 1 != 0).then_some(FontWeight::BOLD),
                                font_style: (bits & 2 != 0).then_some(FontStyle::Italic),
                                ..Default::default()
                            };
                            (r, style)
                        })
                        .collect()
                })
                .collect(),
        )
    }
    #[cfg(not(feature = "syntax-highlight"))]
    {
        let _ = (code, lang, dark, FontStyle::Italic, FontWeight::BOLD);
        None
    }
}

#[cfg(all(test, feature = "syntax-highlight"))]
mod tests {
    use super::*;

    /// Expected colours are Shiki's output for the same line with the same
    /// themes (`codeToTokens`, night-owl / night-owl-light).
    #[test]
    fn rust_matches_shiki_night_owl() {
        let code = "fn main() { let s = \"hi\"; let n = 42; println!(\"{}\", s); }";
        for (dark, expect) in [
            (
                true,
                [
                    0xc792eaff, 0x82aaffff, 0xecc48dff, 0xf78c6cff, 0x82aaffff, 0xd9f5ddff,
                ],
            ),
            (
                false,
                [
                    0x994cc3ff, 0x4876d6ff, 0xc96765ff, 0xaa0982ff, 0x4876d6ff, 0x111111ff,
                ],
            ),
        ] {
            let lines = highlight(code, "rust", dark).unwrap();
            let colour_at = |at: usize| {
                lines[0]
                    .iter()
                    .find(|(r, _)| r.contains(&at))
                    .unwrap()
                    .1
                    .color
                    .unwrap()
            };
            let probes = [
                code.find("fn").unwrap(),
                code.find("main").unwrap(),
                code.find("hi").unwrap(),
                code.find("42").unwrap(),
                code.find("println").unwrap(),
                code.find('"').unwrap(),
            ];
            for (at, rgba) in probes.into_iter().zip(expect) {
                assert_eq!(colour_at(at), hsla(rgba), "dark={dark} at {at}");
            }
        }
        assert!(highlight("x", "klingon", true).is_none());
    }
}
