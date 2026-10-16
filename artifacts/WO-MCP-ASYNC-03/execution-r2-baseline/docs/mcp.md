# Vexel MCP server

Vexel exposes the open editor through a local Model Context Protocol server.
The `vexel-mcp` executable speaks newline-delimited JSON-RPC on stdin/stdout
and relays requests over a private Unix-domain socket to the running Vexel
viewer. The viewer remains the source of truth; edits run on its owner thread
through `document_edit` and appear in the editor status line.

The implementation follows the [MCP 2026-07-28 specification](https://modelcontextprotocol.io/specification/2026-07-28)
(`server/discover` and per-request protocol metadata) and also accepts the
2025-11-25 `initialize`/`notifications/initialized` lifecycle for existing
clients. It provides tools, resources, and one inspection prompt. There is no
HTTP endpoint.

## Start and grant access

Start Vexel and open a document in the viewer. The app creates a per-user
socket at `$TMPDIR/vexel-mcp-<uid>.sock`. The MCP client launches the sidecar
when needed; that sidecar connects to the running app. Both processes must
resolve the same socket path. If their `TMPDIR` values differ, set
`VEXEL_MCP_SOCKET` to the same absolute path in both processes (and restart
Vexel).

The current document is available without a path grant. To open additional
files, grant directories explicitly. The app checks `VEXEL_MCP_ALLOWED_DIRS`,
a colon-separated list of existing directories, and the optional
`~/.config/vexel/mcp-allowed-dirs.txt`, a newline-separated list. The config
file must be owned by your account and not writable by group or other users.
Paths are canonicalized before checking scope.

For example, configure the server with `/Users/me/Artwork` granted:

```sh
VEXEL_MCP_ALLOWED_DIRS=/Users/me/Artwork \
  /absolute/path/to/vexel/build/mac-release/bin/vexel-mcp
```

The environment applies to the sidecar. Set the same grant in the Vexel app
environment only if using the environment variable for both processes. A
config-file grant is shared by both.

## Register with MCP clients

Replace the example paths with the absolute path to the built `vexel-mcp`
binary and a directory you intend to grant. These examples launch the binary
directly and do not require `npx`.

### Claude Code

```sh
claude mcp add vexel --scope user \
  --env VEXEL_MCP_ALLOWED_DIRS=/Users/me/Artwork \
  -- /absolute/path/to/vexel/build/mac-release/bin/vexel-mcp
```

Or use a project `.mcp.json` entry:

```json
{
  "mcpServers": {
    "vexel": {
      "type": "stdio",
      "command": "/absolute/path/to/vexel/build/mac-release/bin/vexel-mcp",
      "args": [],
      "env": { "VEXEL_MCP_ALLOWED_DIRS": "/Users/me/Artwork" }
    }
  }
}
```

Claude Code's MCP server configuration is documented at
https://docs.anthropic.com/en/docs/claude-code/mcp .

### Claude Desktop

Add the server under `mcpServers` in Claude Desktop's
`claude_desktop_config.json`, then restart Claude Desktop:

```json
{
  "mcpServers": {
    "vexel": {
      "command": "/absolute/path/to/vexel/build/mac-release/bin/vexel-mcp",
      "args": [],
      "env": { "VEXEL_MCP_ALLOWED_DIRS": "/Users/me/Artwork" }
    }
  }
}
```

Claude Desktop's local stdio server setup is covered in
https://docs.anthropic.com/en/docs/mcp .

### Codex

Add this table to `~/.codex/config.toml`. The ChatGPT desktop app, Codex CLI,
and IDE extension share this configuration:

```toml
[mcp_servers.vexel]
command = "/absolute/path/to/vexel/build/mac-release/bin/vexel-mcp"
args = []
startup_timeout_sec = 10
tool_timeout_sec = 60

[mcp_servers.vexel.env]
VEXEL_MCP_ALLOWED_DIRS = "/Users/me/Artwork"
```

Codex MCP configuration is documented at https://developers.openai.com/codex/mcp .

## Tools

The server publishes document and SVG editing tools: `document_open`,
`document_close`, `document_list`, `document_info`, `tree_get`, `element_get`,
`element_query`, `attribute_set`, `attribute_remove`, `text_set`,
`element_add`, `element_add_fragment`, `element_delete`, `element_duplicate`,
`element_reorder`, `element_group`, `element_ungroup`, `transaction`, `undo`,
`redo`, `save`, `save_as`, `render_png`, `selection_get`, `selection_set`, and
`export_svg`. SVG animation tools are `animation_list`, `animation_add`,
`animation_set`, and `animation_remove`.

On the active app-owned native `.vexel` project, `native_timeline_get`,
`native_timeline_add_track`, `native_timeline_add_drawing_asset`,
`native_timeline_add_property_track`, `native_timeline_set_property_key`,
`native_timeline_set_cel`, `native_timeline_remove_cel`,
`native_timeline_move_item`, `native_timeline_set_loop`,
`native_timeline_undo`, `native_timeline_redo`, and `export_animation` expose
undoable timeline authoring and GIF/PNG-sequence export. Native timeline
authoring tools target only the active app-owned `.vexel` project. For
`export_animation`, `.vexel` uses its native timeline and a standalone SVG uses
its SMIL document clock.

Resources expose the current source, bounded tree data, and most recent PNG
render. The `inspect-document` prompt directs a client to inspect the open
document. Tool results include structured JSON and a text fallback; rendering
also returns an MCP image content block.

## Limits and known gaps

- JSON requests and responses are limited to 2 MiB, parser depth to 32, and
  parser tokens to 65,536; the rate limit is 120 calls per minute per client.
  Sidecar IPC send/receive waits time out after 30 seconds.
- The broker holds at most eight documents and 64 MiB per document. Tree/query
  pages are limited to 512 rows; an atomic transaction is limited to 64 edits.
- PNG output is limited to 512 pixels per axis. Animation export is capped at
  30 FPS, 120 frames, and 16 million cumulative frame pixels.
- Opening is limited to explicitly granted directories. No shell commands,
  network access, or external SVG resources are available.
- The active app document cannot be closed through MCP. MCP-opened dirty
  documents return `unsaved_changes` on close. `save_as` accepts an existing
  regular file or a new path inside a granted directory; new files are
  published atomically.
- `element_add` supports an empty `text` template and `text_set` inserts or
  removes escaped text. New graphics roots without an ID receive a unique
  `vexel-object-N` ID in the same undoable edit; the returned `elementId` can
  target later animation work. Existing unique IDs are preserved, and a
  duplicate supplied root ID is rejected.
- The app shows each completed MCP tool and its success/failure in the bounded
  operator log and status strip. Progress and cancellation notifications are
  accepted by the protocol but do not interrupt in-flight edits or renders.
  PNG rendering uses the current decoder and rasterizer subset.
- Verified live sessions with Codex, Claude Code, and an independent test
  client are still pending; the listed tool surface does not claim those
  interoperability gates are complete.
- Async native GIF export integration under WO-MCP-ASYNC-03 revision 1 is
  blocked before product edits: its required product-inventory update is
  outside the frozen allowlist. The existing task-store slice remains;
  asynchronous export, Tasks negotiation, and cooperative cancellation are
  not implemented by this run. See
  [`scope-audit.txt`](../artifacts/WO-MCP-ASYNC-03/scope-audit.txt).
- The raw stdio integration test requires local Unix-domain socket binding;
  the constrained execution environment may deny `bind(2)` with `EPERM`.

