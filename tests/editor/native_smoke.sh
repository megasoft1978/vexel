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
<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'><rect id='box' x='8' y='8' width='16' height='12' fill='#4f7cff'/></svg>
SVG

"$binary" "$document" >"$log_file" 2>&1 &
viewer_pid=$!
sleep 1

osascript <<'APPLESCRIPT'
tell application "System Events"
    tell process "svgworkbench"
        set frontmost to true
        delay 0.5
        set position of window 1 to {20, 40}
        delay 0.5
        set {windowX, windowY} to position of window 1
        set contentTop to windowY + 28
        click at {windowX + 447, contentTop + 232}
        drag from {windowX + 447, contentTop + 232} to {windowX + 468, contentTop + 253}
        delay 1
        drag from {windowX + 554, contentTop + 317} to {windowX + 565, contentTop + 328}
        delay 1
        keystroke "r"
        drag from {windowX + 590, contentTop + 400} to {windowX + 640, contentTop + 450}
        delay 0.4
        keystroke "z" using {command down}
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
import re
import sys
import xml.etree.ElementTree as ET

path = Path(sys.argv[1])
after = path.read_bytes()
before = (b"<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
          b"viewBox='0 0 64 64'><rect id='box' x='8' y='8' width='16' "
          b"height='12' fill='#4f7cff'/></svg>\n")
without_transform = re.sub(rb" transform=\"[^\"]*\"", b"", after)
if without_transform != before:
    raise SystemExit("saved SVG changed bytes outside the selected transform")
root = ET.fromstring(after)
rectangles = root.findall("{http://www.w3.org/2000/svg}rect")
if len(rectangles) != 1:
    raise SystemExit("the draw/undo smoke did not leave exactly one rectangle")
transform = rectangles[0].get("transform", "")
if "scale(" not in transform or transform.count("translate(") < 2:
    raise SystemExit("move and resize transforms were not both saved")
print("native editor smoke: select, move, resize, draw, undo, and exact save passed")
PY

trap - EXIT HUP INT TERM
rm -rf "$temp_dir"
