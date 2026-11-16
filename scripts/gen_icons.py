#!/usr/bin/env python3
"""Regenerate crates/refineui/src/icon_names.rs from the SVGs in crates/refineui/assets/icons.

Adding an icon:
  1. Copy `<slug>-regular.svg` and `<slug>-filled.svg` (20px) from RefineUI System Icons
     (https://github.com/pelagornis/refineui-system-icons) into crates/refineui/assets/icons/.
  2. python3 scripts/gen_icons.py && cargo fmt
"""
import pathlib, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ICONS = ROOT / "crates/refineui/assets/icons"
OUT = ROOT / "crates/refineui/src/icon_names.rs"

files = sorted(p.name for p in ICONS.glob("*.svg"))
slugs = sorted({f.rsplit("-", 1)[0] for f in files if f.endswith(("-regular.svg", "-filled.svg"))})
missing = [f"{s}-{st}.svg" for s in slugs for st in ("regular", "filled") if f"{s}-{st}.svg" not in files]
if missing:
    sys.exit("missing icon files: " + ", ".join(missing))

def pascal(slug):
    return "".join(part.capitalize() for part in slug.split("-"))

o = []
w = o.append
w("//! Icon set — SVGs from RefineUI System Icons (MIT, github.com/pelagornis/refineui-system-icons),")
w("//! 20px grid, `regular` and `filled` styles. Generated list; see AGENTS.md → \"Adding an icon\".")
w("")
w("use crate::icon::IconStyle;")
w("")
w("/// Available icons.")
w("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
w("pub enum IconName {")
for s in slugs:
    w("    #[allow(missing_docs)]")
    w(f"    {pascal(s)},")
w("}")
w("")
w("impl IconName {")
w("    /// Every icon, in alphabetical order.")
w("    pub const ALL: &[IconName] = &[")
for s in slugs:
    w(f"        IconName::{pascal(s)},")
w("    ];")
w("")
w("    /// Kebab-case name (matches RefineUI System Icons slug).")
w("    pub const fn slug(self) -> &'static str {")
w("        match self {")
for s in slugs:
    w(f'            IconName::{pascal(s)} => "{s}",')
w("        }")
w("    }")
w("")
w("    /// Asset path for a style.")
w("    pub const fn path(self, style: IconStyle) -> &'static str {")
w("        match (self, style) {")
for s in slugs:
    w(f'            (IconName::{pascal(s)}, IconStyle::Regular) => "icons/{s}-regular.svg",')
    w(f'            (IconName::{pascal(s)}, IconStyle::Filled) => "icons/{s}-filled.svg",')
w("        }")
w("    }")
w("}")
w("")
w("pub(crate) fn embedded(path: &str) -> Option<&'static [u8]> {")
w("    Some(match path {")
svgs = [f for f in files if f.rsplit("-", 1)[0] in slugs]
for f in svgs:
    w(f'        "icons/{f}" => include_bytes!("../assets/icons/{f}"),')
w("        _ => return None,")
w("    })")
w("}")
w("")
w("pub(crate) const EMBEDDED_PATHS: &[&str] = &[")
for f in svgs:
    w(f'    "icons/{f}",')
w("];")
OUT.write_text("\n".join(o) + "\n")
print(f"wrote {OUT.relative_to(ROOT)} ({len(slugs)} icons)")
