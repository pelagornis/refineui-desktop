#!/usr/bin/env bash
# Type-check the macOS (Metal) side of vendor/gpui + refineui from Linux.
#
#   ./scripts/macos-check/check.sh
#
# 1. `cargo check --target aarch64-apple-darwin` with fake C toolchains and stub
#    bindgen outputs (dispatch, media, stitched shaders), so rustc checks
#    metal_renderer.rs and the rest of the mac platform code.
# 2. `clang++ -fsyntax-only` over the blur composite in shaders.metal via a C++ shim.
#
# Nothing here proves the Metal shaders compile or render: that still needs a Mac.
# Needs: rustup target add aarch64-apple-darwin, clang.
set -euo pipefail
cd "$(dirname "$0")/../.."
HERE=scripts/macos-check
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/macos-check}"
export CC_aarch64_apple_darwin=$PWD/$HERE/fakecc CXX_aarch64_apple_darwin=$PWD/$HERE/fakecc
export AR_aarch64_apple_darwin=$PWD/$HERE/fakear
run() { cargo check -p refineui-material -p refineui --all-features --target aarch64-apple-darwin; }
for attempt in 1 2 3 4; do
  if out=$(run 2>&1); then echo "$out" | tail -n 2; break; fi
  missing=$(echo "$out" | grep -oE "/[^ ]*/out/(bindings\.rs|dispatch_sys\.rs|stitched_shaders\.metal)" | head -n1 || true)
  [[ -z "$missing" ]] && { echo "$out" | grep -E "^error" -A12; exit 1; }
  case "$missing" in
    */bindings.rs) cp $HERE/gpui_media_bindings.rs "$missing" ;;
    */dispatch_sys.rs) cp $HERE/dispatch_sys.rs "$missing" ;;
    */stitched_shaders.metal) cp vendor/gpui/src/platform/mac/shaders.metal "$missing" ;;
  esac
  echo "stubbed $missing"
done
python3 $HERE/msl_extract.py vendor/gpui/src/platform/mac/shaders.metal > "$CARGO_TARGET_DIR/msl_check.cpp"
clang++ -std=c++17 -fsyntax-only -Wno-unused-variable -I $HERE "$CARGO_TARGET_DIR/msl_check.cpp"
echo "macOS type-check OK (Rust + MSL shim)"
