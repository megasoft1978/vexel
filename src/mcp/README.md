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
operations. Render output is limited to 512 pixels on either axis. MCP-opened
documents can only be closed when clean. The app's active document cannot be
closed by MCP. `save_as` currently requires an existing regular file because
that is the contract of the public save API. The public template API supports
`rect`, `circle`, `ellipse`, `line`, `path`, and `g`; the advertised `text`
template returns `invalid_operation` until the edit API can create text nodes.

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
