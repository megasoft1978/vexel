# MCP control surface

`mcp_json` owns bounded JSON parsing and serialization. Input is capped at 2 MiB,
nesting at 32 levels, and the token table at 65,536 entries. The parser rejects
malformed UTF-8, invalid surrogate pairs, trailing values, and resource-limit
exhaustion. `mcp_protocol` implements newline-delimited JSON-RPC 2.0, the
2026-07-28 stateless discovery flow, and the 2025-11-25 initialize flow.

`mcp_tools` publishes the closed tool schemas and validates argument names and
types. `mcp_broker*` adapts those calls to the public `document_edit` API and
the existing SVG index, shape decoder, and rasterizer. The broker owns
MCP-opened document source and edit objects. Live window records borrow each
viewer editor's existing `document_edit` object; the active window is
addressable as `active` and each window retains a `window:N` identity. The GUI
and MCP share the editor selection, dirty/save state, render refresh, and undo
history. Handles and returned source slices are valid only for the current edit
snapshot. Headless transaction batches clone and swap after validation; live
transactions validate on a clone and apply one bounded source-span edit to the
existing editor object.

The broker supports up to eight live windows and 16 headless documents (24
total), each at most 64 MiB. Tree and query pages contain at most 512 elements;
transactions contain at most 64 operations. `render_png` output is limited to 512 pixels on
either axis. Live-window PNG capture uses the viewer's last presented frame and
is limited to 4096 pixels per axis and 16 million pixels total. A clean
headless document can be closed while live app documents stay open. `save_as`
accepts an existing regular file or a new destination inside
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

The UI host API is attached to live document records. It reads or changes view
zoom/pan, tool, panel and timeline state, runs named viewer commands, and
encodes the composed window frame. Headless records return `unavailable` for
these controls. Set `VEXEL_MCP_DISABLE_LIVE=1` in the app environment to turn
off live document and UI control.

The AppKit event loop serializes MCP requests with GUI input. While a canvas
gesture, text/number field edit, or timeline scrub is active, the socket hook
queues at most 16 requests and 4 MiB for up to two seconds, then returns a
`busy_retry` error if the interaction has not ended.

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

Tests live in `tests/mcp/`: protocol and schema checks, the deterministic
200,000-case parser/validator fuzz executable, a raw-stdio end-to-end test
against the headless broker, and an in-process viewer/editor test with a fake
frame presenter. `./dev mcp-fuzz` also runs the live UI-host operation fuzzer.
