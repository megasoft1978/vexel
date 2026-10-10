#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
binary=${1:-"$repo_root/build/mac-debug/bin/svgworkbench"}
first=${2:-"$repo_root/tests/viewer/fixtures/simple-shape.svg"}
second=${3:-"$repo_root/tests/viewer/fixtures/unsupported.svg"}
log_file=$(mktemp "${TMPDIR:-/tmp}/vexel-viewer-smoke.XXXXXX")
viewer_pid=

cleanup() {
    if [ -n "$viewer_pid" ] && kill -0 "$viewer_pid" 2>/dev/null; then
        kill -TERM "$viewer_pid" 2>/dev/null || true
        wait "$viewer_pid" 2>/dev/null || true
    fi
    rm -f "$log_file"
}
trap cleanup EXIT HUP INT TERM

"$binary" "$first" >"$log_file" 2>&1 &
viewer_pid=$!
sleep 1

osascript - "$second" <<'APPLESCRIPT'
on run argv
    set secondDocument to item 1 of argv
    tell application "System Events"
        tell process "svgworkbench"
            set frontmost to true
            keystroke "r" using {command down}
            delay 0.4
            keystroke "+"
            delay 0.3
            keystroke "t"
            delay 0.3
            keystroke "o" using {command down}
            delay 0.6
            keystroke "g" using {command down, shift down}
            delay 0.3
            keystroke secondDocument
            key code 36
            delay 0.4
            key code 36
            delay 1
            keystroke "1"
            delay 0.3
            keystroke "q" using {command down}
        end tell
    end tell
end run
APPLESCRIPT

if ! wait "$viewer_pid"; then
    cat "$log_file" >&2
    exit 1
fi
viewer_pid=
trap - EXIT HUP INT TERM
rm -f "$log_file"
printf '%s\n' 'native viewer smoke: open panel, reload, zoom, actual size, quit passed'
