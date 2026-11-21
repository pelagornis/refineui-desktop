#!/usr/bin/env bash
# Type-check `crates/refineui/src/platform/windows.rs` for a Windows target
# from any OS. The full crate can't be checked cross-OS (gpui's Windows deps
# need MSVC tools), so this builds a scratch crate around the module with a
# stub `gpui::Window` (raw-window-handle only).
#   rustup target add x86_64-pc-windows-msvc
#   ./scripts/typecheck_windows.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$root/target/typecheck-windows"
mkdir -p "$work/stub/src" "$work/check/src"
cat > "$work/stub/Cargo.toml" <<'TOML'
[package]
name = "gpui"
version = "0.0.0"
edition = "2024"
[dependencies]
raw-window-handle = "0.6"
TOML
cat > "$work/stub/src/lib.rs" <<'RS'
use raw_window_handle::{HandleError, HasWindowHandle, WindowHandle};
pub struct Window;
impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}
RS
cat > "$work/check/Cargo.toml" <<'TOML'
[package]
name = "refineui-windows-typecheck"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
gpui = { path = "../stub" }
raw-window-handle = "0.6"
[lints.rust]
unsafe_code = "deny"
missing_docs = "warn"
TOML
cat > "$work/check/src/lib.rs" <<RS
//! Scratch crate: type-checks platform::windows for Windows targets.
#[path = "$root/crates/refineui/src/platform/windows.rs"]
pub mod windows;
RS
# Keep the stub out of the outer workspace.
printf '[workspace]\n' >> "$work/stub/Cargo.toml"
target="${1:-x86_64-pc-windows-msvc}"
cd "$work/check"
RUSTFLAGS="-D warnings" cargo check --target "$target" --quiet
if cargo clippy --version >/dev/null 2>&1; then
  cargo clippy --target "$target" --quiet -- -D warnings
fi
echo "platform::windows type-checks for $target"
