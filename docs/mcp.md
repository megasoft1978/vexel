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

## Stdio relay contract

The sidecar uses bounded `poll` multiplexing of stdin, the broker socket, and
stdout. Complete client lines reach the broker in input order, including
notifications sent while earlier requests remain outstanding. Every complete
broker line reaches stdout unchanged, including progress notifications and
responses with unrelated or out-of-order IDs. Partial reads/writes and coalesced
line tails are retained. The relay tracks IDs only for inactivity accounting:
matching responses and forwarded cancellation notifications retire pending
entries, while clients retain protocol correlation. It never rewrites lines or
synthesizes cancellation replies.

Each direction holds at most one 2 MiB message (plus its newline) and one 8 KiB
read chunk. Backpressure pauses that direction while the other can continue.
Overlong stdin lines are discarded through the next newline (or EOF) and emit
one bounded parse error; an overlong or truncated broker line fails the
transport without publishing that partial JSON message. CRLF client delimiters
and a final unterminated client line retain their prior behavior.

After stdin EOF, queued client data is sent, broker input is half-closed, and
broker output is drained until closure or a 30-second drain deadline. Socket
writes, partial broker-line reads, stdout writes, and broker inactivity while a
client request is outstanding retain 30-second bounds. Only broker bytes renew
broker inactivity; new client traffic does not. A matching response clears its
pending entry when the broker line is framed, independently of stdout drain.
Idle open stdin with no pending requests has no deadline. Accounting is limited
to 256 simultaneous IDs and 2 MiB including table metadata; duplicate active
IDs or capacity exhaustion fail before forwarding the request. String IDs are
compared after JSON escape decoding; numeric IDs retain their JSON number text
and remain distinct from string IDs. This is transport readiness only:
cooperative task/export progress and cancellation still need broker integration.
The private socket checks and file-grant policy are unchanged.

The focused `mcp-stdio-relay` fixture exercises the public executable without
app UI automation. The r4 host run passed all 21 relay fixtures and both focused
CTest registrations (`mcp-stdio-relay` and `mcp-stdio-e2e`, 2/2), including
silent-broker deadlines, broker-only renewal, idle-after-completion/cancellation,
response retirement during slow stdout drain, invalid-ID forwarding, duplicate
and capacity rejection, and ID-specific late-response handling. This verifies
the local relay and headless broker path; it does not establish live
Codex/Claude interoperability or implement cooperative task/export cancellation.

Revision 4 is accepted after coordinator review and a fresh integrated run of all
21 relay fixtures plus the headless broker E2E registration (2/2 CTests). The
original 12 fixture bodies and broker E2E assertions are preserved. See
[revision 4 evidence](../artifacts/WO-MCP-TRANSPORT-01/verification-r4.txt).

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

## Draw a character

`rust/crates/vexel-app/tests/mcp_draw_character.rs` drives this sequence over
the stdio JSON-RPC path with no window. Each call passes `documentId`
`preview-document`, and the shape ids are `vexel-object-1`..`-3`:

1. `element_add`: `parentId` `root`, `template` `rect`, `attributes` `x` 200,
   `y` 260, `width` 112, `height` 140, `fill` `#3a7bd5` (body).
2. `element_add`: `template` `ellipse`, `cx` 256, `cy` 200, `rx` 80, `ry` 70,
   `fill` `#ffe0b0` (head).
3. `element_add`: `template` `path`, `d` `M 312 300 L 380 330 L 400 390`,
   `fill` `none`, `stroke` `#202020`, `stroke-width` 6 (arm).
4. `document_info`: `elementCount` is 3.
5. `attribute_set`: `elementId` `vexel-object-1`, `name` `fill`, `value` `#2a9d5c`.
6. `attribute_set`: `elementId` `vexel-object-2`, `name` `fill`, `value` `#ffd0a0`.
7. `attribute_set`: `elementId` `vexel-object-2`, `name` `transform`, `value`
   `translate(10 5)`. The head becomes `cx="2.66e2" cy="2.05e2"`.
8. `animation_add`: `targetId` `vexel-object-2`, `xml`
   `<animate attributeName="opacity" values="1;0.2;1" dur="1s"/>`.
9. `export_animation`: `path` inside a granted directory, `format` `gif`,
   `fps` 10, `duration` 1. The reply's `bytes` equals the file size.
10. `export_svg`: returns the saved source, including the `<animate>`.

The `native_timeline_*` tools are not in the live-preview tool list, so this
character uses the SMIL `animation_add` path instead.

## Limits and known gaps

- JSON requests and responses are limited to 2 MiB, parser depth to 32, and
  parser tokens to 65,536; the rate limit is 120 calls per minute per client.
  Sidecar socket/stdout stalls and EOF draining are bounded to 30 seconds
  as described in the relay contract above.
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
- The live async owner bridge remains pending. Sidecar transport r4, the strict
  owned snapshot, snapshot-only GIF worker, optional async protocol adapter r2,
  and atomic synchronous GIF publication are accepted in the working tree.
  The adapter validates per-request Tasks support, operation-specific results,
  legacy deferred/cancellation routing, and bounded monotonic progress. The app
  still installs the synchronous backend, so production does not advertise Tasks
  or run the new worker yet. The legacy C `WO-MCP-CAPTURE-01` candidate and its
  focused review evidence are preserved under `.worktrees/mcp-owned-capture`
  and `artifacts/WO-MCP-CAPTURE-01/` as migration reference. They are not a
  current production capability; the owner-capture path must be implemented in
  Rust. The live owner bridge, stale-generation protection and visible Cancel
  remain pending.
  Historical WO-MCP-ASYNC-03 r2 scope audit is preserved at
  [execution audit](../artifacts/WO-MCP-ASYNC-03/scope-block-execution-r2.txt).
  Current evidence: [transport r4](../artifacts/WO-MCP-TRANSPORT-01/verification-r4.txt),
  [GIF worker](../artifacts/WO-MCP-GIF-WORKER-01/verification-integrated.txt),
  [protocol r2](../artifacts/WO-MCP-TASK-PROTOCOL-01/verification-integrated-r2.txt).
- Both synchronous GIF routes now use atomic no-replace publication after a
  final canonical grant check. Responses include `published` and `durable`;
  `published:true,durable:false` means the output exists but cleanup or directory
  durability was not confirmed. Required response metadata is encoded before
  publication; failure to encode it leaves no output. PNG sequences retain their
  existing per-frame route. [Focused evidence](../artifacts/WO-MCP-GIF-PUBLISH-01/verification-integrated.txt).
- The blocked design is limited to native `.vexel` GIF `export_animation`.
  For `2026-07-28`, it requires the request's `io.modelcontextprotocol/tasks`
  extension before returning `resultType:"task"`, with `tasks/get` and
  `tasks/cancel`; task cancellation never uses `notifications/cancelled`.
  For `2025-11-25`, progress requires a valid request `progressToken`, and
  `notifications/cancelled` targets the in-flight original request, suppressing
  its eventual response. Planned bounds are two active tasks, eight retained
  terminal tasks, 15-minute lifetime, 16 MiB scene and 64 MiB encoded bundle
  snapshots, alongside the existing frame/pixel limits. These are frozen
  requirements, not implemented async task/export capability or acceptance.
- The focused relay fixture and raw stdio integration test require local
  Unix-domain socket binding;
  the constrained execution environment may deny `bind(2)` with `EPERM`.


### App socket scheduling

The hook uses a nonblocking owner-thread IO pool with stable connection IDs.
Each transport poll transfers at most64KiB across reads+writes and8KiB per
client, dispatches at most8lines and consumes at most128socket syscalls.
Progress queues coalesce unsent messages only; terminal replies are retained
until drained or an explicit connection close. Queue allocation limits include
partially sent frames. Expensive synchronous tool callbacks are outside this
transport bound and remain part of the pending async bridge work.
[Focused socket evidence](../artifacts/WO-MCP-SOCKET-IO-01/verification-integrated.txt).
