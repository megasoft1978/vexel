#!/bin/bash
# Record a codex exec session driving the live Vexel window. usage: codex_take.sh out.mp4 log.txt
OUT=${1:-/private/tmp/vexel-video/takes/E_codex.mp4}; LOG=${2:-/private/tmp/vexel-video/codex_E.log}
G=/private/tmp/vexel-video/grant; W=${WINCAP:-/private/tmp/vexel-video/wincap}
APP=/Users/megasoft78/Desktop/Freelance/vexel/build/rust/aarch64-apple-darwin/release/vexel
mkdir -p $G; printf '<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512"></svg>' > $G/codex_base.svg
PROMPT="You have a Vexel MCP server (live vector editor). First document_open $G/codex_base.svg. Using only its tools, build a 512x512 animated night scene: dark blue vertical gradient sky, a glowing moon, three twinkling stars, a dark hill, and a small paper boat or bird gliding across. Create elements with the batch tool, animate with native timeline tracks (3 second loop, 24 ticks per second), then export_animation as gif to $G/demo.gif. Keep it to about 15 tool calls. Finish with one sentence."
codex exec --skip-git-repo-check -m gpt-6-luna -C $G \
  -c 'mcp_servers.vexel.default_tools_approval_mode="approve"' -c "mcp_servers.vexel.command=\"$APP\"" -c "mcp_servers.vexel.args=[\"--mcp-preview\",\"--grant-dir\",\"$G\"]" \
  "$PROMPT" < /dev/null > $LOG 2>&1 &
CP=$!
for i in $(seq 1 120); do P=$(pgrep -x vexel | head -1); [ -n "$P" ] && break; sleep 0.5; done
[ -z "$P" ] && { echo "no vexel"; kill $CP; exit 1; }
osascript -e "tell application \"System Events\" to tell (first process whose unix id is $P)
repeat 40 times
if (count of windows) > 0 then exit repeat
delay 0.25
end repeat
set position of window 1 to {60, 60}
set size of window 1 to {1440, 900}
end tell"
sleep 1; $W --pid $P --out $OUT --fps 60 < /dev/null > /private/tmp/vexel-video/cap_E.log 2>&1 &
WP=$!
wait $CP; sleep 3
kill -INT $WP 2>/dev/null; wait $WP
pkill -x vexel; sleep 1; pgrep -x vexel || echo "vexel gone"
