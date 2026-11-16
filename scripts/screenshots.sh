#!/usr/bin/env bash
# Capture every gallery story (light + dark) into docs/src/assets/components/.
#
#   ./scripts/screenshots.sh                 # all stories
#   ./scripts/screenshots.sh button,input    # only these story ids
#
# macOS: grant "Screen Recording" to your terminal app the first time
#        (System Settings → Privacy & Security → Screen Recording), then rerun.
# Linux: runs under the current X11 $DISPLAY, or starts Xvfb if there is none.
set -euo pipefail

cd "$(dirname "$0")/.."
OUT="${OUT:-docs/src/assets/components}"
ONLY="${1:-}"
PROFILE="${PROFILE:-release}"

args=(--out "$OUT")
[[ -n "$ONLY" ]] && args+=(--only "$ONLY")

cargo build -p refineui-gallery --bin refineui-screenshots --profile "$PROFILE"
dir="$PROFILE"; [[ "$PROFILE" == "dev" ]] && dir="debug"
BIN="target/$dir/refineui-screenshots"
mkdir -p "$OUT"

case "$(uname -s)" in
  Darwin)
    "$BIN" "${args[@]}"
    ;;
  Linux)
    # Render at 2x so docs images are as crisp as macOS Retina captures.
    export GPUI_X11_SCALE_FACTOR="${GPUI_X11_SCALE_FACTOR:-2}"
    unset WAYLAND_DISPLAY
    if [[ -z "${DISPLAY:-}" ]]; then
      command -v Xvfb >/dev/null || { echo "Xvfb not found (apt install xvfb)" >&2; exit 1; }
      export DISPLAY=:97
      Xvfb "$DISPLAY" -screen 0 2400x1800x24 >/dev/null 2>&1 &
      XVFB_PID=$!
      trap 'kill $XVFB_PID 2>/dev/null || true' EXIT
      sleep 1
    fi
    "$BIN" "${args[@]}"
    ;;
  *)
    echo "screenshots are supported on macOS and Linux" >&2
    exit 1
    ;;
esac

echo "Screenshots written to $OUT"
