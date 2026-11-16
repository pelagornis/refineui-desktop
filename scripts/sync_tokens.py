#!/usr/bin/env python3
"""Regenerate the RefineUI color tokens from the web monorepo.

  python3 scripts/sync_tokens.py [path/to/refineui]   # default: ../refineui
  cargo fmt && cargo test -p refineui-tokens

Writes:
  crates/refineui-tokens/src/palette.rs      <- packages/tokens/src/global/colors.ts
  crates/refineui-tokens/src/semantic.rs     <- packages/tokens/src/semantic/colors.ts (pairs block)
  crates/refineui/src/theme_colors.rs        <- the semantic alias list

Spacing, radius, typography, shadow and motion tokens are small and hand-ported;
compare them with packages/tokens/src/global/*.ts when the web tokens change.
"""
import pathlib, re, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
WEB = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ROOT.parent / "refineui").resolve()
TOKENS = WEB / "packages/tokens/src"
if not TOKENS.exists():
    sys.exit(f"RefineUI tokens not found at {TOKENS}")

def const(name):  # primaryLightGray -> PRIMARY_LIGHT_GRAY, neutral100 -> NEUTRAL_100
    s = re.sub(r"(?<=[a-z])(?=[A-Z])", "_", name)
    s = re.sub(r"(?<=[A-Za-z])(?=\d)", "_", s)
    return s.upper()

def snake(name):
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name).lower()

# ---- palette ---------------------------------------------------------------
palette = re.findall(r'^\s*([A-Za-z0-9]+):\s*"#([0-9A-Fa-f]{6})"', (TOKENS / "global/colors.ts").read_text(), re.M)
o = [
    "//! Global palette — ported 1:1 from `@refineui/tokens` `global/colors.ts`.",
    "//!",
    "//! Do not edit by hand; regenerate from the web tokens when they change",
    '//! (see AGENTS.md → "Syncing tokens").',
    "",
    "use crate::Color;",
    "",
]
for name, hx in palette:
    o += [f"/// `{name}` — `#{hx.upper()}`", f"pub const {const(name)}: Color = Color::rgb(0x{hx.upper()});"]
o += ["", "/// Every palette entry as `(web token name, color)`, in source order.", "pub const ALL: &[(&str, Color)] = &["]
o += [f'    ("{name}", {const(name)}),' for name, _ in palette]
o += ["];", ""]
(ROOT / "crates/refineui-tokens/src/palette.rs").write_text("\n".join(o))

# ---- semantic pairs ----------------------------------------------------------
sem_src = (TOKENS / "semantic/colors.ts").read_text()
block = sem_src.split("SEMANTIC_PALETTE_PAIRS = {", 1)[1].split("} as const", 1)[0]
pairs = re.findall(r'(\w+):\s*\{\s*light:\s*"(\w+)",\s*dark:\s*"(\w+)"\s*\}', block)
names = {n for n, _ in palette}
for alias, l, d in pairs:
    for key in (l, d):
        if key not in names:
            sys.exit(f"{alias}: unknown palette key {key}")
sem_path = ROOT / "crates/refineui-tokens/src/semantic.rs"
sem = sem_path.read_text()
head, rest = sem.split("semantic_colors! {\n", 1)
_, tail = rest.split("\n}\n", 1)
body = []
for alias, l, d in pairs:
    body += [f"    /// `{alias}` — light `{l}`, dark `{d}`.", f"    {snake(alias)}: ({const(l)}, {const(d)}),"]
sem_path.write_text(head + "semantic_colors! {\n" + "\n".join(body) + "\n}\n" + tail)

# ---- GPUI theme colors -------------------------------------------------------
fields = [snake(a) for a, _, _ in pairs] + ["surface_overlay", "surface_frosted"]
o = [
    "//! GPUI-resolved semantic colors. Generated from `refineui_tokens::SemanticColors`;",
    "//! field names match the web alias tokens in snake_case.",
    "",
    "use gpui::Hsla;",
    "use refineui_tokens::SemanticColors;",
    "",
    "use crate::theme::to_hsla;",
    "",
    "/// Semantic colors resolved to [`Hsla`] for the active appearance.",
    "#[derive(Clone, Copy, Debug, PartialEq)]",
    "pub struct ThemeColors {",
]
for f in fields:
    o += ["    #[allow(missing_docs)]", f"    pub {f}: Hsla,"]
o += ["}", "", "impl ThemeColors {", "    /// Convert token colors into GPUI colors.", "    pub fn from_tokens(c: &SemanticColors) -> Self {", "        Self {"]
o += [f"            {f}: to_hsla(c.{f})," for f in fields]
o += ["        }", "    }", "}", ""]
(ROOT / "crates/refineui/src/theme_colors.rs").write_text("\n".join(o))
print(f"synced {len(palette)} palette colors and {len(pairs)} semantic aliases from {WEB}")
