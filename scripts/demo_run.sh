#!/bin/sh
# Prints the MCP config and the commands to run Vexel for the Codex demo.
# Does NOT launch the app. Copy and run the printed commands yourself.
set -eu

VEXEL_ROOT=${VEXEL_ROOT:-"$(cd "$(dirname "$0")/.." && pwd)"}
GRANT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/vexel-demo.XXXXXX")
BIN="$VEXEL_ROOT/build/rust/aarch64-apple-darwin/release/vexel"

cat <<EOF
# 1. Grant directory (temp, created by this script): $GRANT_DIR

# 2. Codex MCP config (~/.codex/config.toml), documented in docs/mcp.md:
[mcp_servers.vexel]
command = "$BIN"
args = ["--mcp-preview", "--grant-dir", "$GRANT_DIR"]
startup_timeout_sec = 10
tool_timeout_sec = 60

# 3. Run commands (run these yourself; this script does not launch them):
#    Build the app and start it as the MCP stdio server:
#      cargo build --release --locked -p vexel-app
#      "$VEXEL_ROOT/build/rust/aarch64-apple-darwin/release/vexel" \\
#        --mcp-preview --grant-dir "$GRANT_DIR" \\
#        "$VEXEL_ROOT/rust/fixtures/v1/anim/loop.svg"

# Notes:
#  - --grant-dir <directory> is the grant (rust/crates/vexel-app/src/compose/launch.rs). It is repeatable.
#  - Remove the temporary grant directory manually when done: rmdir "$GRANT_DIR"
EOF
