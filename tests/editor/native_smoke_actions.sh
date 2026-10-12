#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
binary=${1:-"$repo_root/build/mac-debug/bin/svgworkbench"}
temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/vexel-editor-smoke.XXXXXX")
document="$temp_dir/edited.svg"
layout_dump="$temp_dir/layout.json"
log_file="$temp_dir/viewer.log"
viewer_pid=

cleanup() {
    if [ -n "$viewer_pid" ] && kill -0 "$viewer_pid" 2>/dev/null; then
        kill -TERM "$viewer_pid" 2>/dev/null || true
        wait "$viewer_pid" 2>/dev/null || true
    fi
    if [ -n "${VEXEL_SMOKE_CAPTURE_DIR:-}" ]; then
        mkdir -p "$VEXEL_SMOKE_CAPTURE_DIR"
        [ ! -f "$layout_dump" ] || cp "$layout_dump" "$VEXEL_SMOKE_CAPTURE_DIR/last-layout.json"
        [ ! -f "$document" ] || cp "$document" "$VEXEL_SMOKE_CAPTURE_DIR/last-document.svg"
        [ ! -f "$log_file" ] || cp "$log_file" "$VEXEL_SMOKE_CAPTURE_DIR/last-viewer.log"
    fi
    rm -rf "$temp_dir"
}
trap cleanup EXIT
trap 'cleanup; exit 130' HUP INT TERM

cat >"$document" <<'SVG'
<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'><rect id='box' x='8' y='8' width='16' height='12' fill='#4f7cff'/></svg>
SVG

VEXEL_TEST_LAYOUT_DUMP="$layout_dump" VEXEL_TEST_WORKSPACE=draw "$binary" "$document" >"$log_file" 2>&1 &
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
    end tell
end tell
APPLESCRIPT
mouse="$repo_root/tests/editor/mouse.py"
python3 "$mouse" wait-layout "$layout_dump"

capture() {
    screenshot="$temp_dir/$1.png"
    python3 "$mouse" capture "$screenshot"
    [ -z "${VEXEL_SMOKE_CAPTURE_DIR:-}" ] || {
        mkdir -p "$VEXEL_SMOKE_CAPTURE_DIR"
        cp "$screenshot" "$VEXEL_SMOKE_CAPTURE_DIR/$1.png"
        cp "$layout_dump" "$VEXEL_SMOKE_CAPTURE_DIR/$1-layout.json"
    }
}

assert_core_save() {
    python3 - "$document" <<'PY'
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET

path = Path(sys.argv[1])
saved = path.read_bytes()
expected = (b"<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
            b"viewBox='0 0 64 64'><rect id='box' x='8' y='8' width='16' "
            b"height='12' fill='#4f7cff'/></svg>\n")
without_transform = re.sub(rb" transform=\"[^\"]*\"", b"", saved)
if without_transform != expected:
    raise SystemExit(f"saved SVG changed bytes outside the selected transform: {saved!r}")
root = ET.fromstring(saved)
rectangles = root.findall("{http://www.w3.org/2000/svg}rect")
if len(rectangles) != 1 or root.findall("{http://www.w3.org/2000/svg}polygon"):
    raise SystemExit("the draw/undo smoke did not leave exactly one rectangle")
transform = rectangles[0].get("transform", "")
if not transform.startswith("matrix(") or transform.count("matrix(") != 1:
    raise SystemExit("move and resize were not composed into one transform matrix")
print("editor canvas save: selected move/resize and rectangle/polygon draw/undo passed")
PY
}

assert_untouched_save() {
    cmp -s "$document" "$temp_dir/core-saved.svg" || {
        cat "$log_file" >&2
        exit 1
    }
}

assert_final_save() {
    python3 - "$document" <<'PY'
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET

saved = Path(sys.argv[1]).read_bytes()
expected = (b"<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
            b"viewBox='0 0 64 64'><rect id='box' x='13' y='8' width='16' "
            b"height='12' fill='#35a74b'/></svg>\n")
without_transform = re.sub(rb" transform=\"[^\"]*\"", b"", saved)
if without_transform != expected:
    raise SystemExit("inspector save changed unexpected SVG bytes")
root = ET.fromstring(saved)
rectangle = root.find("{http://www.w3.org/2000/svg}rect")
if rectangle is None or rectangle.get("x") != "13" or \
        rectangle.get("fill") != "#35a74b" or rectangle.get("display") is not None:
    raise SystemExit("inspector values were not saved or UI visibility leaked to SVG")
transform = rectangle.get("transform", "")
if not transform.startswith("matrix(") or transform.count("matrix(") != 1:
    raise SystemExit("saved inspector edits lost the selected transform")
print("editor chrome save: numeric X and fill edits saved; visibility and lock remained UI-only")
PY
}

# Select, move, and resize through document-to-screen coordinates from the live
# editor dump. These points are in document pixels, independent of chrome size.
# System Events key codes 9, 15, and 16 are the physical V, R, and Y keys.
keys 'key code 9'
python3 "$mouse" wait-node "$layout_dump" "Selection tool · V" state=checked
python3 "$mouse" click-doc "$layout_dump" 16 14
python3 "$mouse" drag-doc "$layout_dump" 16 14 18 16
sleep 0.4
python3 "$mouse" drag-doc "$layout_dump" 26 22 28 24
sleep 0.5

# Tool selection uses the editor's documented keyboard shortcuts.
keys 'key code 15'
python3 "$mouse" drag-doc "$layout_dump" 30 30 40 40
sleep 0.3
keys 'keystroke "z" using {command down}'
keys 'key code 16'
python3 "$mouse" click-doc "$layout_dump" 30 30
python3 "$mouse" click-doc "$layout_dump" 36 30
python3 "$mouse" click-doc "$layout_dump" 33 36
keys 'key code 36'
sleep 0.3
keys 'keystroke "z" using {command down}'
keys 'keystroke "s" using {command down}'
sleep 0.5
assert_core_save
cp "$document" "$temp_dir/core-saved.svg"
capture core-selection

# The layer row selects the rectangle; eye visibility only changes the preview.
keys 'key code 9'
python3 "$mouse" wait-node "$layout_dump" "Selection tool · V" state=checked
python3 "$mouse" click-node "$layout_dump" "Rectangle #box" select
python3 "$mouse" wait-node "$layout_dump" "Rectangle #box" state=checked
capture layer-selected
python3 "$mouse" click-node "$layout_dump" "Rectangle #box" visibility
python3 "$mouse" wait-node "$layout_dump" "Rectangle #box" contains=Hidden
python3 "$mouse" wait-node "$layout_dump" "Preview updated" present
capture layer-hidden
keys 'keystroke "s" using {command down}'
sleep 0.3
assert_untouched_save
python3 "$mouse" click-node "$layout_dump" "Rectangle #box" visibility
# Selection deliberately survives hiding and locking (see the locked-move step
# below), so a selected row keeps the "Selected on canvas" state value. Assert
# the specific state this step toggled rather than an empty value.
python3 "$mouse" wait-node "$layout_dump" "Rectangle #box" not-contains=Hidden

# Locking blocks canvas translation without changing the editable source.
python3 "$mouse" click-node "$layout_dump" "Rectangle #box" lock
python3 "$mouse" wait-node "$layout_dump" "Rectangle #box" contains=Locked
capture layer-locked
python3 "$mouse" drag-doc "$layout_dump" 19 17 21 19
sleep 0.4
python3 "$mouse" wait-node "$layout_dump" "Unlock the selected layer to transform it" present
capture locked-move-blocked
keys 'keystroke "s" using {command down}'
sleep 0.3
assert_untouched_save
python3 "$mouse" click-node "$layout_dump" "Rectangle #box" lock
python3 "$mouse" wait-node "$layout_dump" "Rectangle #box" not-contains=Locked

# Edit X and Fill in the inspector, and verify both the live fields and saved SVG.
python3 "$mouse" click-node "$layout_dump" X
keys 'keystroke "a" using {command down}'
python3 "$mouse" type-text 13
keys 'key code 36'
sleep 0.5
python3 "$mouse" wait-node "$layout_dump" X value=13
capture inspector-x-edited
keys 'keystroke "s" using {command down}'
sleep 0.3
python3 - "$document" <<'PY'
from pathlib import Path
import re
import sys

saved = Path(sys.argv[1]).read_bytes()
expected = (b"<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
            b"viewBox='0 0 64 64'><rect id='box' x='13' y='8' width='16' "
            b"height='12' fill='#4f7cff'/></svg>\n")
if re.sub(rb" transform=\"[^\"]*\"", b"", saved) != expected:
    raise SystemExit("inspector X value was not saved without unrelated SVG changes")
PY
# The fill lives on the Fill property tab, so open that tab first the way a user
# would, then click the valued row (the tab carries the same label).
python3 "$mouse" click-node "$layout_dump" Fill
python3 "$mouse" click-value "$layout_dump" Fill
keys 'keystroke "a" using {command down}'
python3 "$mouse" type-text '#35a74b'
keys 'key code 36'
sleep 0.5
python3 "$mouse" wait-node "$layout_dump" Fill value=#35a74b
capture inspector-fill-edited
# Task workspaces: Control-PageDown cycles the tab strip and Control-PageUp cycles
# back, with wrap. The save and byte comparison below run after these switches,
# so assert_final_save also proves a workspace switch never edits the document.
keys 'key code 121 using {control down}'
python3 "$mouse" wait-node "$layout_dump" "Nodes" state=checked
keys 'key code 121 using {control down}'
python3 "$mouse" wait-node "$layout_dump" "Animate" state=checked
keys 'key code 116 using {control down}'
python3 "$mouse" wait-node "$layout_dump" "Nodes" state=checked
keys 'key code 116 using {control down}'
python3 "$mouse" wait-node "$layout_dump" "Draw" state=checked
capture workspace-cycled
keys 'keystroke "s" using {command down}'
sleep 0.5
assert_final_save
capture inspector-saved

keys 'keystroke "q" using {command down}'
if ! wait "$viewer_pid"; then
    cat "$log_file" >&2
    exit 1
fi
viewer_pid=

[ -z "${VEXEL_SMOKE_KEEP:-}" ] || cp "$document" "$VEXEL_SMOKE_KEEP"
printf '%s\n' "native editor smoke: live canvas mapping, layers, visibility, lock, inspector, draw/undo, and exact save passed"

trap - EXIT HUP INT TERM
rm -rf "$temp_dir"
