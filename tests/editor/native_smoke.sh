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

keys() {
    osascript -e 'tell application "System Events" to tell process "svgworkbench" to '"$1"
}
osascript <<'APPLESCRIPT'
tell application "System Events"
    tell process "svgworkbench"
        set frontmost to true
        delay 0.5
        set position of window 1 to {20, 40}
        delay 0.5
    end tell
end tell
APPLESCRIPT
mouse="$repo_root/tests/editor/mouse.py"
window_x=20
content_top=68
python3 "$mouse" click $((window_x + 447)) $((content_top + 232))
python3 "$mouse" drag $((window_x + 447)) $((content_top + 232)) $((window_x + 468)) $((content_top + 253))
sleep 1
python3 "$mouse" drag $((window_x + 554)) $((content_top + 317)) $((window_x + 565)) $((content_top + 328))
sleep 1
keys 'keystroke "r"'
python3 "$mouse" drag $((window_x + 590)) $((content_top + 400)) $((window_x + 640)) $((content_top + 450))
sleep 0.4
keys 'keystroke "z" using {command down}'
keys 'keystroke "s" using {command down}'
sleep 0.5
keys 'keystroke "q" using {command down}'

if ! wait "$viewer_pid"; then
    cat "$log_file" >&2
    exit 1
fi
viewer_pid=

[ -z "${VEXEL_SMOKE_KEEP:-}" ] || cp "$document" "$VEXEL_SMOKE_KEEP"
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
