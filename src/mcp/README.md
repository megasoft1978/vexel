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
the ImageIO platform helper. Existing output files are never replaced.

`mcp_access` resolves real paths and permits opens only inside canonical
directories listed in `VEXEL_MCP_ALLOWED_DIRS` or
`~/.config/vexel/mcp-allowed-dirs.txt`. The latter is a newline-separated list
of absolute directories; the file must be owned by the current user and not
group/world writable. No shell, network, or external SVG loading is exposed.

The viewer hook owns a same-user Unix-domain socket, polls it from the AppKit
event loop, and applies broker work on that owner thread. Its socket is mode
0600 and accepts at most eight clients. `vexel-mcp` is a stdio relay; it keeps
protocol traffic on stdout and diagnostics on stderr. Both processes must use
the same socket path and Unix user.

Tests live in `tests/mcp/`: protocol and schema checks, a deterministic 200,000
case parser/validator fuzz executable, and a raw-stdio end-to-end test against
the headless broker.
