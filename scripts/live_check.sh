#!/bin/bash
# Live check for vexel-app: launch it on a fixture, find its window with
# CGWindowList, capture that window, and report whether the app is frontmost
# and its window is key. macOS only; needs a logged-in GUI session.
#
# usage: scripts/live_check.sh [fixture.svg] [out_dir]
#   VEXEL_BIN    binary to launch (default: first existing of
#                $CARGO_TARGET_DIR/aarch64-apple-darwin/debug/vexel,
#                build/rust/aarch64-apple-darwin/debug/vexel)
#   VEXEL_OWNER  window owner name to look for (default: vexel)
#   VEXEL_WAIT   seconds to wait for the window (default: 20)
#   VEXEL_KEEP   set to 1 to leave the app running afterwards
#
# Output: key=value lines (window_id, window_title, frontmost, key_window,
# capture). Exit status is non-zero when no window is found or the capture
# fails; frontmost/key are reported as "yes", "no" or "unknown" (System Events
# needs Accessibility permission) and do not change the exit status.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE="${1:-$ROOT/rust/fixtures/v1/compose/compose-v1.svg}"
OUT_DIR="${2:-$ROOT/artifacts/live-check}"
OWNER="${VEXEL_OWNER:-vexel}"
WAIT="${VEXEL_WAIT:-20}"

BIN="${VEXEL_BIN:-}"
if [ -z "$BIN" ]; then
  for candidate in \
    "${CARGO_TARGET_DIR:-$ROOT/build/rust}/aarch64-apple-darwin/debug/vexel" \
    "$ROOT/build/rust/aarch64-apple-darwin/debug/vexel"; do
    if [ -x "$candidate" ]; then BIN="$candidate"; break; fi
  done
fi
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "error: vexel binary not found; build it or set VEXEL_BIN" >&2
  exit 2
fi
if [ ! -f "$FIXTURE" ]; then
  echo "error: fixture not found: $FIXTURE" >&2
  exit 2
fi
mkdir -p "$OUT_DIR" || exit 2

# Prints "<window id><TAB><title>" of the largest on-screen layer-0 window
# owned by $1, or nothing.
find_window() {
  osascript -l JavaScript - "$1" <<'JXA' 2>/dev/null
ObjC.import('CoreGraphics');
function run(argv) {
  var owner = argv[0];
  var opts = $.kCGWindowListOptionOnScreenOnly | $.kCGWindowListExcludeDesktopElements;
  var list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo(opts, 0)));
  var best = null, bestArea = -1;
  list.forEach(function (w) {
    if (w.kCGWindowOwnerName !== owner || w.kCGWindowLayer !== 0) return;
    var b = w.kCGWindowBounds || {};
    var area = (b.Width || 0) * (b.Height || 0);
    if (area > bestArea) { best = w; bestArea = area; }
  });
  return best ? best.kCGWindowNumber + '\t' + (best.kCGWindowName || '') : '';
}
JXA
}

"$BIN" "$FIXTURE" >"$OUT_DIR/vexel.log" 2>&1 &
APP_PID=$!
cleanup() {
  if [ "${VEXEL_KEEP:-0}" != "1" ]; then kill "$APP_PID" 2>/dev/null; fi
}
trap cleanup EXIT

WINDOW=""
for _ in $(seq 1 "$((WAIT * 2))"); do
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "error: vexel exited before showing a window (see $OUT_DIR/vexel.log)" >&2
    exit 1
  fi
  WINDOW="$(find_window "$OWNER")"
  [ -n "$WINDOW" ] && break
  sleep 0.5
done
if [ -z "$WINDOW" ]; then
  echo "error: no window owned by '$OWNER' within ${WAIT}s" >&2
  exit 1
fi
WINDOW_ID="${WINDOW%%$'\t'*}"
WINDOW_TITLE="${WINDOW#*$'\t'}"

# Let the first compose and redraw finish before capturing.
sleep 2
WINDOW="$(find_window "$OWNER")"
if [ -n "$WINDOW" ]; then
  WINDOW_ID="${WINDOW%%$'\t'*}"
  WINDOW_TITLE="${WINDOW#*$'\t'}"
fi

CAPTURE="$OUT_DIR/window-$WINDOW_ID.png"
if ! screencapture -x -l "$WINDOW_ID" "$CAPTURE" || [ ! -s "$CAPTURE" ]; then
  echo "error: screencapture failed for window $WINDOW_ID" >&2
  exit 1
fi

FRONT_NAME="$(osascript -e 'tell application "System Events" to get name of first process whose frontmost is true' 2>/dev/null)"
if [ -z "$FRONT_NAME" ]; then
  FRONTMOST="unknown"
elif [ "$FRONT_NAME" = "$OWNER" ]; then
  FRONTMOST="yes"
else
  FRONTMOST="no (frontmost is '$FRONT_NAME')"
fi
KEY_VALUE="$(osascript -e "tell application \"System Events\" to tell process \"$OWNER\" to get value of attribute \"AXMain\" of window 1" 2>/dev/null)"
case "$KEY_VALUE" in
  true) KEY_WINDOW="yes" ;;
  false) KEY_WINDOW="no" ;;
  *) KEY_WINDOW="unknown" ;;
esac

echo "fixture=$FIXTURE"
echo "window_id=$WINDOW_ID"
echo "window_title=$WINDOW_TITLE"
echo "frontmost=$FRONTMOST"
echo "key_window=$KEY_WINDOW"
echo "capture=$CAPTURE"
exit 0
