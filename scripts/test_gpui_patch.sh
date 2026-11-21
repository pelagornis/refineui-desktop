#!/usr/bin/env bash
# Run the RefineUI patch unit tests inside vendor/gpui (blur planning, params
# layout, layer ordering, cache keys). vendor/gpui is not a workspace member,
# so it is tested standalone with the workspace lockfile pinned.
set -euo pipefail
cd "$(dirname "$0")/.."
cp Cargo.lock vendor/gpui/Cargo.lock
trap 'rm -f vendor/gpui/Cargo.lock' EXIT
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/gpui-patch}" \
  cargo test --manifest-path vendor/gpui/Cargo.toml --lib backdrop "$@"
