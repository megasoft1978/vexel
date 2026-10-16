# WO-RUST-V1-MCP-TOOLS-01 result (candidate, uncommitted)

Routed into the live session (`McpHost` + `LiveBackend`, all through
`AppDocument::edit`): `save`, `save_as`, `document_open`, `attribute_set`
(fill, stroke, stroke-width, transform translate), `element_delete`,
`element_add` line and path, `export_svg`. Preview `tools/list` now has 15
tools (9 existing + 6 new names; line/path reuse `element_add`).

`vexel_app` library target added (`src/lib.rs`, all modules `pub` because the
`vexel` bin imports them); both roots keep `#![forbid(unsafe_code)]`.
`McpGrants` (`session/grants.rs`) + repeatable `--grant-dir` in
`compose::parse_launch` build the one `AccessPolicy` used by `McpHost` and
`ExportJob::spawn` (main now passes `mcp_host.policy()`).

## render_png deferral
Not routed, not listed. No PNG encoder exists in the workspace and none may be
added. A call returns the protocol's `unknown tool`. Gap for a later order.

## Deviations (coordinator must accept)
1. `vexel-protocol/src/protocol/tool_methods.rs` also carries a preview
   allow-list and an `element_add` rect/ellipse-only gate. Both had to change
   for `tools/call` to reach the backend; the order named only `tools.rs`.
2. `vexel-protocol/tests/mcp_preview_profile.rs`: the existing test asserted
   exactly 9 tools in a fixed order; updated to 15 in declaration order (plus
   `render_png` absent). No assertion removed. One test added.
3. AC2 says "8 newly routed names"; the order's scope yields 6 new names
   (element_add was already listed).
4. Preview `element_add` schema still advertises `enum ["rect","ellipse"]`
   (order: schemas unchanged); the backend accepts line and path.
5. `document_open` checks the grant via `resolve_save_path` first, then
   `resolve_existing`, so ungranted paths never reveal existence.
6. `save` re-checks the grant on the current path (agent cannot write a
   user-chosen ungranted path).
7. Legacy `preview_app.rs`: `start_server` made `pub`; its dead
   `from_environment` export path left as is.
8. `transform` is a relative move (`V1Document::translate` delta).

## Not run
Native run loop and window (never launched). `git diff --check` and git
commit were refused by the sandbox hook; a manual trailing-whitespace scan was
clean. `fmt --check` for `vexel-protocol` fails on lines this order did not
touch (async_protocol.rs, lib.rs, parity.rs and old hunks); `vexel-app` is clean.
