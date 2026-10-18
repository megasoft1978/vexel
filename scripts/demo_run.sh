#!/bin/sh
# Prints the MCP config and the commands to run Vexel for the Codex demo.
# Does NOT launch the app. Copy and run the printed commands yourself.
set -eu

VEXEL_ROOT=${VEXEL_ROOT:-"$(cd "$(dirname "$0")/.." && pwd)"}
GRANT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/vexel-demo.XXXXXX")
BIN="$VEXEL_ROOT/build/mac-release/bin/vexel-mcp"

cat <<EOF
# 1. Grant directory (temp, created by this script): $GRANT_DIR

# 2. Codex MCP config (~/.codex/config.toml), documented in docs/mcp.md:
[mcp_servers.vexel]
command = "$BIN"
args = []
startup_timeout_sec = 10
tool_timeout_sec = 60

[mcp_servers.vexel.env]
VEXEL_MCP_ALLOWED_DIRS = "$GRANT_DIR"

# 3. Preview profile, documented in docs/product/mcp-live-preview.md
#    (5 tools only; full-profile demo steps need the full profile):
[mcp_servers.vexel_preview]
command = "cargo"
args = ["run", "-p", "vexel-app", "--", "--mcp-preview"]
cwd = "$VEXEL_ROOT"

# 4. Run commands (run these yourself; this script does not launch them):
#    Build the sidecar:
#      cargo build --release -p vexel-app
#    Start the app (live-check path, docs/ui/live-check.md):
#      VEXEL_MCP_ALLOWED_DIRS="$GRANT_DIR" \\
#        "$VEXEL_ROOT/build/rust/aarch64-apple-darwin/release/vexel" \\
#        "$VEXEL_ROOT/rust/fixtures/v1/anim/loop.svg"

# Notes:
#  - --grant-dir is NOT documented in docs/; the grant is VEXEL_MCP_ALLOWED_DIRS.
#  - Remove the grant directory when done: rm -rf "$GRANT_DIR"
EOF
