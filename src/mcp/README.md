# MCP control surface

`mcp_json` owns bounded JSON parsing and serialization. Input is capped at 2 MiB,
nesting at 32 levels, and the token table at 65,536 entries. The parser rejects
malformed UTF-8, invalid surrogate pairs, trailing values, and resource-limit
exhaustion. `mcp_protocol` implements newline-delimited JSON-RPC 2.0, the
2026-07-28 stateless discovery flow, and the 2025-11-25 initialize flow.

`mcp_tools` publishes the closed tool schemas and validates argument names and
types. `mcp_broker*` adapts those calls to the public `document_edit` API and
the existing SVG index, shape decoder, and rasterizer. The broker owns
MCP-opened document source and edit objects; the app-owned current document
remains owned by the viewer. Handles and returned source slices are valid only
for the current edit snapshot. Every mutation uses the edit engine; transaction
batches operate on a clone and replace the document only after all edits pass.

The broker supports at most eight open documents, each at most 64 MiB. Tree and
query pages contain at most 512 elements; transactions contain at most 64
operations. Render output is limited to 512 pixels on either axis. A clean
MCP-opened document can be closed while the app-owned active document stays
open. `save_as` accepts an existing regular file or a new destination inside
the granted directories; new files are published atomically without replacing
a path that appears concurrently. `element_add` supports `rect`, `circle`,
`ellipse`, `line`, `path`, `g`, and an empty `text` template; `text_set` can then
insert or remove escaped text content. `element_add_fragment` accepts one
well-formed top-level XML element up to 64 KiB and validates the resulting
parent subtree before committing.
New graphics elements without an `id` receive a stable unique
`vexel-object-N` value, returned as `elementId` and usable as a native animation
track target. Caller-provided unique IDs are retained; duplicate IDs return an
edit error.

`render_png` decodes with the app's CoreText and ImageIO media providers, so
bounded text and embedded raster images follow the same platform implementation
as viewer rendering. Its optional `time` argument samples the SMIL document
clock from 0 to 3600 seconds; omitted time keeps the static render. Network-loaded
media remains unsupported.

The animation tools expose `animation_list` with track IDs, targets, key values,
diagnostics, and source byte spans; `animation_add` inserts one validated SMIL
fragment under a target; `animation_set` edits one track attribute; and
`animation_remove` removes one identified track. Every mutation is an undoable
document edit. Track listing and mutation are limited to 16 MiB of source and
supported decoded tracks. `export_animation` writes either PNG frames into an
existing granted directory or a looping GIF at a new path in a granted
directory. FPS is capped at 30, frame count at 120, each frame at 512 pixels
per axis, and cumulative export at 16 million frame pixels. GIF encoding uses
the ImageIO platform helper. Existing output files are never replaced. For the
active app-owned `.vexel` project, export samples the current native timeline,
including unsaved property keys, drawing cels, and project assets, at integer
timeline ticks; requested duration cannot exceed the project timeline. For SVG
documents, export samples the SVG SMIL document clock. Onion-skin preview is
editor-only and is not included in either export path.

Native `.vexel` timeline control is separate from the SVG SMIL tools. On the
active app-owned native-v1 project, `native_timeline_get` returns timing,
drawing tracks, property tracks, and ordered cels/keys; `native_timeline_add_track`,
`native_timeline_add_drawing_asset`,
`native_timeline_set_cel`, `native_timeline_remove_cel`,
`native_timeline_move_item`,
`native_timeline_add_property_track`, `native_timeline_set_property_key`, and
`native_timeline_set_loop` perform validated, undoable session edits, with
`native_timeline_undo` and `native_timeline_redo` operating on that timeline
history. Scene materializing
and MCP listing are bounded to 16 MiB; structured results stay within the MCP
response cap. Property values are JSON numbers, booleans, or flat numeric arrays
of at most six components; native-v1 validation enforces the selected property's
exact type and range. Key value JSON is capped at 512 bytes. The loop setter
accepts `once`, `loop`, or `ping_pong`; the timeline mode button edits the same
project setting, and `native_timeline_get` returns its current value. These tools
reject standalone SVGs, inactive broker documents, and projects without a
recognized native-v1 session. Successful mutations
update the viewer status and dirty state immediately; project Save writes the
same edited session through the app's project-save route. MCP Save As also uses
the `.vexel` container writer for project destinations and preserves the
existing SVG path for SVG destinations. Drawing assets are complete SVG source
documents limited to 1 MiB per MCP call; Vexel validates them with its owned
SVG parser, stores the exact bytes under a unique safe asset name, and saves them
in the project container. Once added, the normal drawing-track and cel tools can
reference the new asset. Asset-only changes mark the project unsaved, and SVG
save is rejected until native assets are saved in `.vexel` format.

The app records each completed MCP tool call in its bounded operator log and
status strip as `MCP <tool> completed` or `MCP <tool> failed`. This provides a
visible action/result trail; it does not report in-progress state or add
cancellation for synchronous long-running operations.

`mcp_access` resolves real paths and permits opens only inside canonical
directories listed in `VEXEL_MCP_ALLOWED_DIRS` or
`~/.config/vexel/mcp-allowed-dirs.txt`. The latter is a newline-separated list
of absolute directories; the file must be owned by the current user and not
group/world writable. No shell, network, or external SVG loading is exposed.

The viewer hook owns a same-user Unix-domain socket, polls it from the AppKit
event loop, and applies broker work on that owner thread. Its socket is mode
0600 and accepts at most eight clients. `vexel-mcp` is a stdio relay; it keeps
protocol traffic on stdout and diagnostics on stderr. Both processes must use
the same socket path and Unix user. The sidecar uses nonblocking `poll` to
multiplex client input, broker output, and pending writes. It forwards complete
client lines in order without waiting for replies, and retains every partial or
coalesced broker line, including progress and out-of-order responses. It never
synthesizes a cancellation reply. Each direction owns one line bounded to
`MCP_JSON_MAX_BYTES` plus its newline and one 8 KiB read chunk. Stdio overflow
emits one parse error after discarding through newline/EOF; broker overflow or
truncation fails without emitting the partial line. Queued client data at EOF
is flushed before `SHUT_WR`, then output drains to closure or a 30-second bound.
Pending writes and partial broker reads have 30-second stall bounds, renewed
by successful I/O. While at least one client request awaits a JSON-RPC response,
broker inactivity is bounded to 30 seconds and any broker bytes (including
progress notifications) renew that deadline; new client traffic cannot renew
it. A matching response clears its pending entry as soon as the broker line is
framed, before stdout drain. Forwarded `notifications/cancelled` retires only
the matching active request ID; unknown and duplicate cancellations do nothing.
String IDs compare after escape decoding, while numeric IDs retain their JSON
number text and remain distinct from string IDs. Accounting is capped at 256
active IDs and 2 MiB including table metadata; duplicate active IDs or capacity
exhaustion fail before forwarding. Idle stdin with no pending request has no
timeout. stdout contains only protocol lines, stderr diagnostics.

Transport multiplexing does not implement asynchronous tasks, export workers,
cooperative cancellation, or app progress. The existing task-store slice is
preserved for the following async integration order. No live-client acceptance
is established by this transport change.

Tests live in `tests/mcp/`: protocol and schema checks, a deterministic 200,000
case parser/validator fuzz executable, and a raw-stdio end-to-end test against
the headless broker. `McpStdioRelayTests` in `test_e2e.py` adds a private-socket
fixture for the public executable's progress/cancellation, response IDs,
partial/coalesced lines, EOF, size bounds, and backpressure. It is registered as
`mcp-stdio-relay`, separately from `mcp-stdio-e2e`. The r4 host run passed all 21
relay fixtures and both focused CTest registrations (2/2), covering broker-only
inactivity renewal, idle completion/cancellation, slow stdout drain, typed ID
matching, malformed-ID forwarding, and pending accounting limits. This verifies
local relay and headless broker transport behavior; live Codex/Claude sessions
and async task, export, app-progress, and cooperative cancellation integration
remain open.

Revision 4 is accepted with 21 relay fixtures and 2/2 focused CTests in the main
checkout after isolated implementation, exact-delta integration and coordinator
review. Transport acceptance supplies the prerequisite for async integration;
async GIF tasks, app progress/cancel and real-client evidence remain open.
