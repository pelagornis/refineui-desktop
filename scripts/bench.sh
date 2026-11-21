#!/usr/bin/env bash
# Reproducible Material Engine benchmark.
#
#   ./scripts/bench.sh                 # panels 1 8 16, cache on + off
#   ./scripts/bench.sh 4 32            # custom panel counts
#
# Runs crates/refineui-material/examples/blur_bench (release) for every
# panel count with the blur cache on and off (GPUI_BLUR_CACHE=0) and writes
# bench/results/<os>-<host>-<date>.md (environment + raw output + a table).
#
# Linux: uses $DISPLAY or starts Xvfb. Headless X only presents frames after
# input, so the script moves the pointer continuously with xdotool while it
# measures (install xdotool). Numbers are then those of the software
# renderer (e.g. lavapipe) and paced by the 60 Hz timer (~16.7 ms steps).
# macOS: run from a normal session; the window must stay visible.
set -euo pipefail
cd "$(dirname "$0")/.."

PANELS=("$@"); [[ ${#PANELS[@]} -eq 0 ]] && PANELS=(1 8 16)
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
HOST="$(hostname -s 2>/dev/null || hostname)"
STAMP="$(date +%Y-%m-%d)"
OUTDIR=bench/results; mkdir -p "$OUTDIR"
OUT="$OUTDIR/$OS-${BENCH_TAG:-$HOST}-$STAMP.md"   # BENCH_TAG=m2-pro etc.

cargo build --release -p refineui-material --example blur_bench
BIN=target/release/examples/blur_bench

JIGGLE_PID=""
cleanup() { [[ -n "$JIGGLE_PID" ]] && kill "$JIGGLE_PID" 2>/dev/null || true; [[ -n "${XVFB_PID:-}" ]] && kill "$XVFB_PID" 2>/dev/null || true; }
trap cleanup EXIT
if [[ "$OS" == "linux" ]]; then
  unset WAYLAND_DISPLAY
  if [[ -z "${DISPLAY:-}" ]]; then
    command -v Xvfb >/dev/null || { echo "Xvfb not found" >&2; exit 1; }
    export DISPLAY=:96
    Xvfb "$DISPLAY" -screen 0 1920x1200x24 >/dev/null 2>&1 & XVFB_PID=$!
    sleep 1
  fi
  if command -v xdotool >/dev/null; then
    ( while :; do xdotool mousemove 480 320 mousemove 500 330 2>/dev/null || true; sleep 0.002; done ) &
    JIGGLE_PID=$!
  else
    echo "warning: xdotool missing; headless X may not present frames" >&2
  fi
fi

{
  echo "# Material Engine benchmark — $STAMP"
  echo
  echo "- OS: $(uname -srm)"
  if [[ "$OS" == "darwin" ]]; then
    echo "- CPU / GPU: $(sysctl -n machdep.cpu.brand_string 2>/dev/null) / $(system_profiler SPDisplaysDataType 2>/dev/null | awk -F': ' '/Chipset Model/{print $2; exit}')"
  else
    echo "- CPU: $(awk -F': ' '/model name/{print $2; exit}' /proc/cpuinfo 2>/dev/null) ($(nproc) threads)"
    echo "- Vulkan: $(vulkaninfo --summary 2>/dev/null | awk -F'= ' '/deviceName/{print $2; exit}' || echo unknown)"
    echo "- Display: ${DISPLAY:-none}"
  fi
  echo "- rustc: $(rustc --version)"
  echo "- Command: \`$BIN <panels>\` (cache on) and \`GPUI_BLUR_CACHE=0 $BIN <panels>\` (cache off)"
  echo
  echo "| Panels | Cache | blur/moving | blur/static | fallback |"
  echo "| --- | --- | --- | --- | --- |"
} > "$OUT"

RAW=""
for n in "${PANELS[@]}"; do
  for cache in on off; do
    if [[ $cache == off ]]; then log="$(GPUI_BLUR_CACHE=0 "$BIN" "$n" 2>&1)"; else log="$("$BIN" "$n" 2>&1)"; fi
    RAW+=$'\n'"## $n panels, cache $cache"$'\n'"$log"$'\n'
    cell() { grep -m1 "^$1" <<<"$log" | sed -E 's/.*avg= *([0-9.]+) ms +fps= *([0-9.]+).*/\1 ms (\2 fps)/' || true; }
    echo "| $n | $cache | $(cell 'blur/moving') | $(cell 'blur/static') | $(cell 'fallback') |" >> "$OUT"
    echo "$n panels, cache $cache:"; echo "$log" | grep -E "^(blur|fallback)" || echo "$log" | tail -3
  done
done
{ echo; echo "<details><summary>Raw output</summary>"; echo; echo '```'; echo "$RAW"; echo '```'; echo; echo "</details>"; } >> "$OUT"
echo "wrote $OUT"
