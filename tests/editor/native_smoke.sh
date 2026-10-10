#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
binary=${1:-"$repo_root/build/mac-debug/bin/svgworkbench"}
temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/vexel-editor-smoke.XXXXXX")
document="$temp_dir/edited.svg"
log_file="$temp_dir/viewer.log"
viewer_pid=

cleanup() {
    if [ -n "$viewer_pid" ] && kill -0 "$viewer_pid" 2>/dev/null; then
        kill -TERM "$viewer_pid" 2>/dev/null || true
        wait "$viewer_pid" 2>/dev/null || true
    fi
    rm -rf "$temp_dir"
}
trap cleanup EXIT HUP INT TERM

cat >"$document" <<'SVG'
<svg width='32' height='32' x='1'><rect id='box' x='2' y='3' width='8' height='9'/></svg>
SVG

"$binary" "$document" >"$log_file" 2>&1 &
viewer_pid=$!
sleep 1

osascript <<'APPLESCRIPT'
tell application "System Events"
    tell process "svgworkbench"
        set frontmost to true
        keystroke "e"
        keystroke "a" using {command down}
        keystroke "96"
        key code 36
        keystroke "s" using {command down}
        delay 0.5
        keystroke "q" using {command down}
    end tell
end tell
APPLESCRIPT

if ! wait "$viewer_pid"; then
    cat "$log_file" >&2
    exit 1
fi
viewer_pid=

python3 - "$document" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
after = path.read_bytes()
before = b"<svg width='32' height='32' x='1'><rect id='box' x='2' y='3' width='8' height='9'/></svg>\n"
old = b"width='32'"
new = b"width='96'"
start = before.index(old)
expected = before[:start] + new + before[start + len(old):]
if after != expected:
    raise SystemExit("saved SVG differs outside the edited width value")
print("native editor smoke: edited width and saved only its value range")
PY

trap - EXIT HUP INT TERM
rm -rf "$temp_dir"
