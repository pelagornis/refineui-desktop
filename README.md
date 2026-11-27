# RefineUI GPUI

The [RefineUI](https://github.com/pelagornis/refineui) design system for **desktop apps**,
built on [GPUI](https://gpui.rs). Same tokens as the web (light and dark),
with the same component anatomy, states and motion as the web kit
(`packages/react` / [ui.pelagornis.com](https://ui.pelagornis.com)), adapted for macOS:
keyboard focus, hover states, menus, popovers and dialogs. An optional compact density
is available; the default matches RefineUI web sizing.

The repository lives at [pelagornis/refineui-desktop](https://github.com/pelagornis/refineui-desktop).

![Button](docs/src/assets/components/button-light.png)

## Crates

| Crate | Description |
| --- | --- |
| [`refineui-foundation`](crates/refineui-foundation) | Colors, typography, spacing, radius, stroke, opacity, shadow, motion, size and material tokens, component contracts. No GPUI dependency. |
| [`refineui-tokens`](crates/refineui-tokens) | Compatibility re-export of `refineui-foundation`. |
| [`refineui`](crates/refineui) | GPUI theme, icons, components, interaction, workspace and shell. |
| [`refineui-material`](crates/refineui-material) | Material Engine: real backdrop blur, tint, lighting, grain and elevation. |
| [`refineui-gallery`](examples/gallery) | Gallery app and screenshot generator (not published). |

`vendor/gpui` is a patched gpui 0.2.2 used when the `material` feature is on (real backdrop blur).

## Quick start

```toml
[dependencies]
gpui = "0.2.2"
refineui = { git = "https://github.com/pelagornis/refineui-desktop" }
refineui-material = { git = "https://github.com/pelagornis/refineui-desktop" } # optional

# Real backdrop blur needs RefineUI's small GPUI patch (still version 0.2.2).
# Without it refineui-material does not compile; refineui alone works with stock gpui.
[patch.crates-io]
gpui = { git = "https://github.com/pelagornis/refineui-desktop" }
```

```rust
use refineui::prelude::*;

// once, at startup (and pass refineui::Assets to Application::with_assets)
refineui::init(cx);

// in a view's render()
refineui::root(cx).child(
    Button::new("save", "Save").icon(IconName::Checkmark).on_click(|_, _, _| {}),
)
```

## Components

**Actions**: Button, Toggle Button, Segmented Control, Link.
**Forms**: Input, Textarea, Search Field, Input OTP, Field, Checkbox, Radio and RadioGroup, Switch (web Toggle), Slider, Spin Button, Select.
**Display**: Text, Heading, Label, Badge, Chip/Tag, Avatar and Avatars, Card, Alert, Divider, Kbd, Spinner, Progress, ProgressStepper, Stepper, Skeleton, Tooltip, Toast and Toaster, ResultView.
**Navigation**: Tabs, Breadcrumb, Pagination, Sidebar, Tree, Accordion and Collapsible, Calendar.
**Overlays**: Menu, DropdownMenu, ContextMenu, Popover, Dialog, Drawer, Command and Command Palette.
**Data**: Table, Scroll Area.
**Layout**: `h_stack`, `v_stack`, `spacer`.
**Shell**: Window, Titlebar, Window Controls, Menu Bar, Toolbar, Status Bar, Activity Bar, Application Shell, Notification Center, Preferences, Shortcut Recorder, Shortcut Hint.

### Desktop behaviour

- **Select** matches the web (Radix item-aligned): the popup covers the trigger with the selected row exactly over the value. Near a window edge it shifts to stay inside the 10px collision padding, and when the list is too tall it shrinks and scrolls its `ScrollArea` so the selected row still lands on the trigger. Scrolling a popup that opened short grows it toward the window edge first, as on the web.
- **Tooltip** is collision-aware and anchored to its trigger (`place_anchored`), flipping when space is tight, with a beak aimed at the trigger.
- **Activity Bar** supports folders. Its rail uses a hidden scrollbar so the overlay track does not cover icons or badges.
- **Native blur** (`FrostMode::Native`, macOS) opens Dropdown, Context Menu, Menu submenus, Select and Popover in their own pop-up windows over a colorless `NSVisualEffectView`. The plate is the Regular material tint (`backgroundPrimary` at 0.62 light / 0.58 dark). The window's `NSAppearance` follows the RefineUI theme. Other platforms keep the opaque in-window fill.

## Development

Requires Rust 1.88+ (see `rust-toolchain.toml`). The main target is macOS on Apple Silicon.

```sh
cargo fmt
cargo build --all-features
cargo clippy --all-features -- -D warnings
cargo clippy --no-default-features -- -D warnings
cargo test --all-features --workspace

cargo run -p refineui-gallery            # browse every component
./scripts/screenshots.sh                 # regenerate docs screenshots
./scripts/visual-regression.sh           # compare gallery shots against baselines
cd docs && bun install && bun run dev    # documentation site
cd docs && bun run build                 # production docs build
```

See [AGENTS.md](AGENTS.md) for the architecture, conventions and workflows.

### Platform notes

- **macOS** (Apple Silicon) is the primary target. Native frost (`FrostMode::Native`) is macOS-only.
- **Linux** and **Windows** build and run the gallery; `Native` falls back to opaque in-window panels. The Material Engine's real backdrop blur is implemented on Metal (macOS) and Blade (Vulkan, Linux). DirectX (Windows) reports `backdrop_blur_supported() == false` and paints the documented fallback.
- The `material` feature requires the vendored gpui patch in `vendor/gpui`. Without `material`, `refineui` works with stock gpui 0.2.2.
- Visual regression on Linux is run through `scripts/visual-regression.sh` (Xvfb at 2×). A plain `cargo test` under an existing `DISPLAY` may see different window sizes.

## Documentation

- [Docs site source](docs/) — Astro + Starlight with [starlight-theme-page](https://github.com/pelagornis/starlight-theme-page)
- [Contributing](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)

## License

MIT; see [LICENSE](LICENSE). Bundled assets: RefineUI System Icons (MIT) and Inter (SIL OFL 1.1).
