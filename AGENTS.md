# AGENTS.md — RefineUI Desktop (GPUI)

Guidance for coding agents (and humans) working in this repository.

## What this is

RefineUI Desktop is a **desktop-native design system** on [GPUI](https://gpui.rs). It keeps the
[RefineUI](https://github.com/pelagornis/refineui) visual identity (tokens, color roles,
type scale, restrained radius, elevation, motion, frost) and adds desktop interaction,
workspace composition, an application shell and platform adapters. It is **not** a port
of the React components (`DESKTOP_BRIEF.md` / docs → Architecture).

Targets: **macOS and Windows** through platform adapters; Linux builds and runs (CI,
headless screenshots) and has an adapter too.

The web monorepo (`refineui`: `packages/tokens`, `packages/react`, DESIGN.md) is the
reference for the **visual language** (tokens, color usage, states, motion). It is not the
reference for APIs or layout: design for desktop workflows (keyboard-first, resizable
panels, docks, multi-document, multi-window).

## Architecture (read docs/src/content/docs/architecture/*.mdx)

| Layer | Where | Rule |
| --- | --- | --- |
| Foundation | `refineui-foundation` (GPUI-free data) + `refineui::foundation` | tokens, material tokens, theme, density, layers, cursor, motion, blur |
| Material | `refineui-material` (gpui + foundation only) | Material, real backdrop blur, lighting, grain, shadows, `MaterialSurface` |
| Interaction | `refineui::interaction` + interaction components | keymap (→ GPUI bindings), selection, focus regions, dnd — logic unit-tested; visuals in `components::{drag, selection_box, shortcut_hint, command_palette}` |
| Components | `refineui::components` | buttons, inputs, menus, dialogs, tables, trees |
| Composition | `refineui::workspace` | `layout` = persisted app state (serde feature); views in Phase 3 |
| Application | `refineui::shell` | `ShellSpec` + views: Window, Titlebar, WindowControls, MenuBar, StatusBar, ActivityBar, ApplicationShell |
| Platform | `refineui::platform` | `PlatformAdapter` (macOS / Windows / Linux); all `cfg(target_os)` design decisions go here |

- A layer uses only layers below it (platform is queried through the trait).
- Shared contracts never depend on GPUI: anything a non-GPUI tool could need
  (tokens, density numbers, window metrics, z-order, component contracts) goes in
  `refineui-foundation` (`refineui-tokens` is only a compatibility re-export).
- Crates follow the three brief layers: `refineui-foundation` (A), `refineui` (B, Desktop),
  `refineui-material` (C). Material never depends on components. Don't add crates per sub-layer.
- `vendor/gpui` is gpui 0.2.2 + the BackdropBlur primitive, applied with `[patch.crates-io]`
  (one gpui in the graph). Keep the patch minimal and update `vendor/gpui/refineui.patch` +
  `REFINEUI_PATCH.md` whenever it changes.
- Link and Carousel are core; Footer and NavigationMenu were removed. Only the page
  `container()` stays behind `web-extras`.

## Phase plan (brief §9)

1. **Done** — inspect, inventory, contracts, layer architecture, shell design.
2. **Done** — Foundation + interaction primitives in GPUI (density everywhere incl. Spacious, F6
   regions, keymap → GPUI bindings + Shortcut Hint, drag handle / preview / drop zone, selection
   box, cursor roles), Command Palette, shell chrome (Window, Titlebar, Window Controls, Menu Bar,
   Toolbar, Status Bar, Activity Bar / Navigation Rail replacing Side Dock, Application Shell),
   inline application-shell story.
3. **Next** — Workspace composition (Split View, Resizable Handle, Panel, Panel Group, Dock / Dock Area,
   Inspector, Tab Bar / Tab Group / Document Tab, Floating Panel, persistence).
4. Adapt reusable components to their contracts (all 8 states, density, keyboard;
   virtualized Table / Tree with multi-select + DnD; Empty / Error State).
5. System components + macOS / Windows validation.
6. Public API docs and examples.

Material Engine track (docs → Architecture → Material Engine; summary in docs → Final report):
phases 0–6 done — engine, recipes,
components on Material (§9) and desktop components on Material (§10: chrome materials, Notification
Center, Preferences Window, Shortcut Recorder, Empty / Error State, springs, `desktop-showcase`).
Phase 6: generated API reference, `scripts/bench.sh`, visual regression, cross-platform notes,
the 13 final deliverables (docs → Final report).

Do not implement everything in one pass; build and validate each layer before the next.
The inventory (docs → Architecture → Component inventory) lists every component's layer,
status and phase.

## Rules (brief §7–§8)

- GPUI-native primitives and lifecycle; don't copy React APIs.
- Keep **application state** (layout, selection, documents) as plain data owned by the app;
  **transient** state (hover, drag-in-flight, animation) in components.
- Composable primitives over monolithic components.
- Every interactive component defines default / hovered / pressed / focused / disabled /
  loading / selected / dragging (where applicable), keyboard navigation, focus order and an
  a11y role — in its contract. Nothing may be reachable by hover alone.
- Keyboard and mouse for everything; shortcuts through `interaction::keymap` (check
  `conflicts()`), labels through `PlatformAdapter::shortcut_label`.
- Virtualize large lists, tables and trees.
- OS-specific behaviour only behind `refineui::platform`.
- No hard-coded visual values when a semantic token exists.
- **Contracts are machine-readable and enforced**: every public component needs
  `crates/refineui-foundation/contracts/components/<id>.json` (see "Contracts" below).

## Layout

```
Cargo.toml                 workspace (resolver 3, edition 2024, MSRV 1.98), [patch.crates-io] gpui
vendor/gpui/               gpui 0.2.2 + BackdropBlur (REFINEUI_PATCH.md, refineui.patch)
crates/refineui-tokens/    compatibility re-export of refineui-foundation
crates/refineui-material/  Material Engine: surface.rs, backdrop.rs, lighting.rs, texture.rs,
                           shadow.rs (+ InnerShadow), composite.rs (BlendMode, Mask, rules),
                           recipes.rs (sidebar/titlebar/popover/menu/tooltip/HUD/sheet/toast/
                           scrim/glass), element.rs (MaterialSurface); examples/blur_bench.rs
crates/refineui/src/material.rs  SurfaceKind, materials_active, backing, chrome (feature `material`)
crates/refineui/src/components/surface.rs  Surface (material plate / opaque fallback)
crates/refineui-foundation/ pure-Rust shared design language + contracts, no GPUI dependency
  src/material.rs          material roles (thin/regular/thick/chrome/glass/scrim)
  src/palette.rs           GENERATED by scripts/sync_tokens.py
  src/semantic.rs          semantic aliases (pairs block GENERATED)
  src/{spacing,radius,stroke,opacity,motion,typography,shadow,size,blur,z_index}.rs  hand-ported
  src/size.rs              Density (compact/comfortable/spacious) + DesktopMetrics
  src/window.rs            per-platform titlebar / window-controls metrics
  src/contracts.rs         contract vocabulary + embedded JSON
  contracts/               schema.json, density.json (GENERATED by test), components/*.json,
                           index.rs (GENERATED by scripts/contracts.py index)
crates/refineui/           GPUI design system
  src/foundation.rs        foundation facade: LayerOrder, CursorRole, density, theme re-exports
  src/interaction/         keymap.rs, selection.rs, focus.rs, dnd.rs (logic, unit-tested)
  src/workspace/layout.rs  persisted workspace layout model (serde feature)
  src/shell/               mod.rs (ShellSpec regions / metrics), window.rs (presets, restore),
                           titlebar.rs (Titlebar, WindowControls, WindowFrame), menu_bar.rs,
                           status_bar.rs, activity_bar.rs, app_shell.rs (ApplicationShell, F6)
  src/platform/            mod.rs (PlatformAdapter, adapters, chrome preview), macos.rs (AppKit glue)
  src/vibrancy.rs          native frost pop-ups (public API over platform::macos)
  tests/contracts.rs       contract validator
  src/theme.rs             Theme global (appearance, density, colors), ActiveTheme
  src/theme_colors.rs      GENERATED: Hsla version of the semantic colors
  src/icon_names.rs        GENERATED by scripts/gen_icons.py
  src/icon.rs, assets.rs   Icon element, AssetSource (SVGs from the refineui-system-icons crate), Inter
  src/root.rs              refineui::root(cx) and Tab / Shift-Tab focus actions
  src/styles.rs            StyledExt helpers (elevation, text styles, popover surface)
  src/components/*.rs      one file per component family
  assets/fonts/            Inter (OFL)
examples/gallery/          gallery app + screenshot generator
  src/stories/*.rs         one Story per component (shared by both binaries)
  src/app.rs               gallery window (sidebar, light/dark, density, toasts)
  src/bin/screenshots.rs   refineui-screenshots (one PNG per story and appearance)
docs/                      Astro + Starlight + @pelagornis/page documentation site
  src/content/docs/        pages (components/, shell/, foundations/ and reference/ are GENERATED;
                           architecture/, guides/ and report/ are hand-written)
  src/assets/components/   screenshots <story-id>-light.png / -dark.png
  src/components/Screenshot.astro
scripts/                   screenshots.sh, gen_docs.py, gen_icons.py, sync_tokens.py, contracts.py,
                           bench.sh (blur benchmark → bench/results/), visual-regression.sh
bench/results/             recorded benchmark runs (<os>-<tag>-<date>.md), committed
tests/visual/baselines/<os>/  visual regression baselines (<story>-<light|dark>.png)
examples/gallery/tests/    stories_render.rs (every story renders), visual_regression.rs
```

## Commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings  # web-extras, serde, vibrancy typecheck
cargo test --workspace
cargo test -p refineui --all-features
cargo build --workspace
cargo build -p refineui --no-default-features          # Desktop Core without extras
cargo clippy -p refineui --all-targets --features typecheck-macos-vibrancy -- -D warnings
cargo clippy -p refineui --all-targets -- -D warnings  # refineui alone: no `material` (the gallery turns it on)
./scripts/test_gpui_patch.sh                     # vendor/gpui patch unit tests (blur plan, ordering, cache keys)
./scripts/macos-check/check.sh                   # type-check the Metal side from Linux (not a substitute for a Mac)

cargo run -p refineui-gallery                    # gallery
cargo run -p refineui-gallery -- --story select  # open on one story (add --dark)
cargo run --release -p refineui-material --example blur_bench -- 16  # blur benchmark (moving / static / fallback)
./scripts/bench.sh                     # reproducible benchmark: 1/8/16 panels, cache on/off → bench/results/
./scripts/visual-regression.sh         # compare stories with tests/visual/baselines/<os> (--update, --strict)
GPUI_BLUR_CACHE=0 cargo run --release -p refineui-material --example blur_bench -- 16  # same, blur cache off
./scripts/screenshots.sh [id,id]                 # regenerate docs screenshots
python3 scripts/gen_docs.py                      # regenerate component doc pages
cd docs && bun install && bun run dev            # docs at http://localhost:4321
cd docs && bun run build                         # static site in docs/dist
```

All cargo commands above must pass before you hand work back. CI
(`.github/workflows/ci.yml`) runs them on `macos-latest`, plus the docs build.

## Conventions

- **Components are `RenderOnce` builders**: `#[derive(IntoElement)]`, `Thing::new(id, …)`
  plus chained setters (`.variant()`, `.size()`, `.disabled()`, `.on_change()`).
  Long-lived editable state (text input) is an `Entity` (`TextInput`).
- **Controlled by default**: value-bearing components take the current value and report
  changes through `on_change(|value, window, cx| …)`. Popup open state and other
  transient UI state lives in `window.use_keyed_state` keyed by the element id.
- **Colors only through the theme**: `cx.theme().colors.<semantic_name>`. Never hard-code
  hex values in components; add a token instead. Sizes come from
  `refineui_tokens::size` / `Theme::control_*` so density switching works.
- **Desktop behavior**: every interactive element is focusable (`tab_index`), shows the
  focus ring (`theme.focus_ring()`) on keyboard focus, activates with Enter/Space,
  and has hover and active states. Popups close on Escape and outside click.
  Use the `secondary-` key modifier (⌘ on macOS) for shortcuts.
- **Cursors**: controls keep the arrow (`.cursor_action()`); only links use the hand
  (`.cursor_pointer()`); other roles via `.cursor_role(CursorRole::…)`.
- **Density**: rows add `cx.theme().row_inset()` to their vertical padding (tabs:
  `tab_inset()`); control plates read `theme.density.button(size)`.
- **Shortcuts in UI**: never hard-code labels; use `ShortcutHint::action`,
  `Button::tooltip_action`, `MenuItem::action` — they read the installed keymap.
- **`on_hover` once per element**: `track_interaction` already installs it; use
  `on_mouse_move` for extra hover logic (GPUI `debug_assert`s; `tests/stories_render.rs`
  catches it).
- **Shared box displays**: screenshots on Linux must run on your own Xvfb on a free display
  number (`DISPLAY=:NNN ./scripts/screenshots.sh`); never use another agent's display.
- **Naming** follows the web package (`ButtonVariant::Primary`, `ControlSize::Small`,
  `AlertVariant::Danger`). Document the web counterpart in the module doc comment.
- Every public item has a `///` doc comment (`missing_docs` is on). The first line of a
  method's docs becomes its description in the generated API table.
- `unsafe` is denied workspace-wide; the exceptions are the AppKit glue in
  `crates/refineui/src/platform/macos.rs` and the macOS window-id lookup in
  `examples/gallery/src/bin/screenshots.rs`.
- Keep `Cargo.lock` committed. `libc` is pinned to `0.2.189` because `xattr 0.2.3`
  (a GPUI dependency) does not build with newer `libc` on Linux; do not `cargo update`
  it blindly.

## Web parity and motion

- The reference is the web kit: `~/Desktop/refineui/packages/react` (`refineui.css`,
  `src/components/*/style.ts`, `src/recipes/*.recipe.ts`,
  `src/tokens/component{Typography,Color}Tokens.ts`) and
  https://ui.pelagornis.com/llms.txt. Match tokens, sizes, anatomy and every state
  (rest / hover / pressed / focus-visible / disabled / selected) before adding
  desktop-only extras.
- Default density (`Density::Comfortable`) is web sizing; `Compact` and `Spacious` are the
  desktop variants. Shell / workspace sizes come from `Density::desktop()` (`DesktopMetrics`).
- Animate through `crate::motion`, never ad-hoc timers: `Motion::{FAST, MEDIUM, PANEL,
  …}` presets mirror the web durations / easings; `animate` / `animate_from` tween a
  value keyed by element id, `use_interaction` + `track_interaction` give hover /
  pressed state, `use_layout_probe` + `probe` measure for sliding indicators and
  height animations (`components/disclosure.rs::collapse_panel`). Honour
  `reduced_motion(cx)` for looping animations.
- GPUI has no CSS transforms, backdrop blur or inset shadows: emulate scale by
  resizing, translate with margins / offsets, and inset rings with an overlay div.
  Note any remaining difference in the module doc comment.

## Adding a component

1. `crates/refineui/src/components/<name>.rs`, exported from `components/mod.rs`
   (`mod` + `pub use`). Mirror the web API in `packages/react` where it makes sense.
2. Add a story in the matching `examples/gallery/src/stories/<group>.rs`
   (`Story { id, name, group, summary, shot, render }`). `id` is kebab-case and is
   the docs slug and screenshot file name. Show every variant and state.
3. Add a contract: add a row to `scripts/contracts.py` and run
   `python3 scripts/contracts.py bootstrap` (or write the JSON by hand), then review states,
   keyboard, focus and role. `cargo test -p refineui --test contracts` fails until every
   public component is covered.
4. Add a `page(...)` entry in `scripts/gen_docs.py` (summary, usage notes, Rust example,
   types for the API table), then `python3 scripts/gen_docs.py`.
5. `./scripts/screenshots.sh <id>` to capture `docs/src/assets/components/<id>-{light,dark}.png`.
6. Update the inventory page if the component is in the brief.
7. Run fmt, clippy, tests and `cd docs && bun run build`.

## Contracts

`crates/refineui-foundation/contracts/components/<id>.json`, schema `contracts/schema.json`,
docs → Architecture → Component contracts. JSON is canonical; `scripts/contracts.py
refresh` re-derives `tokens`, `sizes` and state status from the source (heuristic — confirm
states by hand when you adapt a component). After changing `Density`, run
`UPDATE_CONTRACTS=1 cargo test -p refineui-foundation` then `python3 scripts/contracts.py refresh`.

## Adding an icon

Icons come from the [refineui-system-icons](https://github.com/pelagornis/refineui-system-icons)
crate (git dependency pinned by `rev`, `default-features = false` so its optional git gpui is
not pulled in). Every glyph is reachable with `Icon::named(refineui::system_icons::icons::AIRPLANE)`;
to add a typed `IconName` variant, append `(Variant, "slug")` to `CURATED` in
`scripts/gen_icons.py`, then `python3 scripts/gen_icons.py && cargo fmt`. Bump the pinned `rev`
to pick up new glyphs; never copy SVGs into this repo.

## Syncing tokens

With the web monorepo checked out next to this one (`../refineui`):

```sh
python3 scripts/sync_tokens.py ../refineui && cargo fmt && cargo test -p refineui-foundation
```

This regenerates the palette, semantic pairs and `ThemeColors`. Spacing, radius,
typography, shadow and motion are hand-ported; diff them against
`packages/tokens/src/global/*.ts` when those change.

## Screenshots

`refineui-screenshots` opens each story in its own window at the story's `shot` size,
captures only that window, trims the empty background and writes light and dark PNGs.

- macOS: `screencapture -l <CGWindowID>`; the terminal needs the **Screen Recording**
  permission (System Settings → Privacy & Security). The first run may fail until it is granted.
- Linux: X11 only (uses `$DISPLAY`, or `scripts/screenshots.sh` starts `Xvfb`), at
  `GPUI_X11_SCALE_FACTOR=2`. Headless X servers only present a frame after input, so the
  tool warps the pointer into the window before capturing.
- A missing PNG renders as a labelled placeholder in the docs, so the docs always build.

## Docs

The site uses `@pelagornis/page` (starlight-theme-page) **2.0.1** with Astro 7, Starlight 0.42
and React 19 (`@astrojs/react`). The plugin injects its own stylesheets, so don't import
`@pelagornis/page/styles` (removed in 2.0). Sidebar groups use `{ label, items: [{ autogenerate }] }`.
`docs/patches/` holds a `bun patch` for 2.0.1: its LanguageSelect/Footer overrides crash on a
single-language site because Starlight leaves `config.locales` undefined. Drop the patch once
upstream guards it.
`docs/public/fonts/` holds the RefineUI icon font that the theme CSS expects.
Update `docs/src/content/docs/report/` (hand-written) when test counts, benchmarks or
platform status change; `bench/results/*.md` is the source for benchmark tables.

## Don'ts

- Don't add web-only components or mobile patterns to Desktop Core; if one is genuinely
  needed, put it behind `web-extras`.
- Don't put GPUI types in `refineui-foundation` or in contract data.
- Don't fake blur with translucent fills: use `refineui-material` (real blur or the documented fallback).
- Components reach materials only through `refineui::material` (`backing()`, `frosted_plate()`,
  `Surface`), never `refineui_material` directly outside `#[cfg(feature = "material")]` — the
  crate must build on stock gpui without the feature. A material layer is the **first** child;
  give the box a transparent border / no shadow and let the material draw both (`backing()` does).
- Window chrome (bars, rails, docks) uses `material::chrome(kind, fill, cx)`: square, no border or
  shadow, outset 0 so the existing one-sided hairline stays visible. Keep the workspace opaque.
- Springs: `motion::animate_spring` / `spring_presence` (interruptible, reduced-motion aware) for
  things that move into place (panels sliding in); tweens (`animate`) for color / opacity.
- Don't branch on `cfg(target_os)` in components; use `refineui::platform`.
- Don't commit `target/`, `docs/node_modules/`, `docs/dist/` or `._*` files.
- Don't edit generated files by hand (`palette.rs`, `theme_colors.rs`, `icon_names.rs`,
  `docs/src/content/docs/{components,foundations}/*.mdx`, `contracts/density.json`,
  `contracts/index.rs`).

## The GPUI patch (vendor/gpui)

`vendor/gpui` is the crates.io gpui 0.2.2 source plus the `BackdropBlur` primitive (scene,
window API, Blade/Metal renderers, DirectX no-op). Everything is described in
`vendor/gpui/REFINEUI_PATCH.md`. After changing it, regenerate the diff:

```sh
mkdir -p /tmp/gpui-orig && tar -xzf ~/.cargo/registry/cache/*/gpui-0.2.2.crate -C /tmp/gpui-orig --strip-components=1
diff -ruN /tmp/gpui-orig/src vendor/gpui/src > vendor/gpui/refineui.patch  # exit 1 = differences
```

To upgrade gpui: unpack the new version into `vendor/gpui`, run `patch -p0 < vendor/gpui/refineui.patch`
from the repo root
(fix rejects), bump the version in `[patch.crates-io]` users, and check `cargo tree -i gpui`
shows exactly one gpui.

Material rules for the patch: every effect lives in the one composite pass (keep
`backdrop_blur.wgsl` and the `refine_blur_composite_fragment` in `shaders.metal` in sync —
same `BackdropBlurParams` layout, 368 bytes); drop shadows are drawn outside the shape so a
material never blurs its own shadow; new primitive kinds must be hashed in
`scene.rs::backdrop_keys` or the blur cache will reuse stale backdrops. The Metal side cannot be
run on Linux — after changing it, run `./scripts/test_gpui_patch.sh` (patch unit tests) and `./scripts/macos-check/check.sh` (Rust type-check for
`aarch64-apple-darwin` + MSL syntax check through a C++ shim) and list it for Mac verification.
