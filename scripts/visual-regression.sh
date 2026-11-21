#!/usr/bin/env bash
# Visual regression against tests/visual/baselines/<os>/.
#
#   ./scripts/visual-regression.sh               # compare (skips if it cannot render)
#   ./scripts/visual-regression.sh --strict      # compare; fail instead of skipping
#   ./scripts/visual-regression.sh --update      # (re)write this OS's baselines
#   ./scripts/visual-regression.sh --only button,select
#
# Env: REFINEUI_VISUAL_TOLERANCE (fraction of pixels, default 0.005),
#      REFINEUI_VISUAL_THRESHOLD (per-channel, default 24).
# Linux: uses $DISPLAY, or starts Xvfb if there is none. Renders at 2x.
# macOS: the terminal needs Screen Recording permission (like screenshots.sh).
set -euo pipefail
cd "$(dirname "$0")/.."

while [[ $# -gt 0 ]]; do
  case "$1" in
    --update) export REFINEUI_VISUAL_UPDATE=1 ;;
    --strict) export REFINEUI_VISUAL_STRICT=1 ;;
    --only) shift; export REFINEUI_VISUAL_ONLY="$1" ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

if [[ "$(uname -s)" == "Linux" ]]; then
  export GPUI_X11_SCALE_FACTOR="${GPUI_X11_SCALE_FACTOR:-2}"
  unset WAYLAND_DISPLAY
  if [[ -z "${DISPLAY:-}" ]] && command -v Xvfb >/dev/null; then
    export DISPLAY=:98
    Xvfb "$DISPLAY" -screen 0 2400x1800x24 >/dev/null 2>&1 &
    XVFB_PID=$!
    trap 'kill $XVFB_PID 2>/dev/null || true' EXIT
    sleep 1
  fi
fi

cargo test -p refineui-gallery --test visual_regression -- --nocapture --test-threads 1
