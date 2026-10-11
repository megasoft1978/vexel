# m1-scene lane

This lane adds source-preserving MCP scene tools on top of `document_edit` and
defines the broker/UI-host seam consumed by the live-app integration lane.

## Tools

- Read: `element_tree`, richer `element_get`, and `element_query` with id,
  tag, exact attribute, CSS-lite selector, and transformed-bounds hit testing.
- Structure: create the supported SVG element subset with parent/index,
  attribute/style edits, delete, duplicate, reorder, group, and ungroup.
- Geometry/style: set/apply transforms, fill/stroke/opacity helpers, local
  linear/radial gradients and editable stops, path command create/append/node
  edit, align, and distribute.
- `batch` stages up to 32 supported scene operations, rolls back on error, and
  commits one source replacement and one undo entry on success.

Scene operations first edit a bounded scratch model. At commit, the engine
computes the common unchanged prefix and suffix and sends the single changed
span through `document_edit_replace_source_span`. A 64 MiB source limit and
64 KiB committed-span limit bound work and preserve untouched source bytes.
Bounds are geometric AABBs after supported SVG transforms; text bounds and
curve/path bounds can be approximate. Boolean path operations are excluded.

## Live host seam

`mcp_broker_commit_mutation` is the single post-mutation hook. It is a no-op
when no app host callback is configured; the app integration supplies its
owner-thread hook. Broker results include `mode: "live"` for that bound app
document and `mode: "headless"` for broker-owned documents or a standalone
headless host. `mcp_ui_host` declares view, tool, selection, UI state, window
capture, command, and timeline callbacks. The in-tree default host returns
`unavailable`; tests inject a fake host. This lane does not implement the app
bridge or touch UI/chrome code.

## Verification

The lane adds `mcp-scene` C broker coverage for the fake-host/live seam,
mutation hook, selection, history, rendered-frame updates, scene edits,
atomic rollback, and headless UI-host behavior. `tests/mcp/test_e2e.py` covers
the published tool list and headless stdio/UDS workflow.

## Codex continuation verification (2026-10-11)

The first resumed MCP e2e run exposed that `element_duplicate` returned
`changed: true` but omitted the creation flag expected by the published
response contract. The scene dispatcher now sets `created: true` on successful
duplication, and the C scene test asserts this response field. This also gives
batch duplicate operations the same result shape.

- Debug build and Release build pass.
- `./dev t -k mcp`: 5/5 CTests and the MCP Python e2e pass.
- `./dev mcp-fuzz`: 200,000 parser cases pass.
- `./dev test-fast`: 293/293 Python tests pass; ledger validates with 406 entries.
- Headless CTest sweep excluding `native-viewer-smoke` and the known 2x-display
  `native-shell-goldens` mismatch: 61/61 pass.

The two excluded tests are separately documented in the W16 integration
verification. No real window was driven, and no worktree commit was made.

## Sanitizer follow-up (2026-10-11)

The first ASan/UBSan MCP scene run exposed an unterminated decoded JSON string
in the local style helper. `gradient_stop_set` passed its buffer to `strlen`,
reading beyond `blue` and making the source edit malformed under sanitizer
allocation layout. The helper now validates capacity and embedded NULs, then
writes the terminator. The gradient regression also asserts the changed result.

After the fix, all five MCP CTests pass under Debug, ASan+UBSan, and UBSan. The
full headless Debug CTest sweep passes 61/61 with only the visible-window smoke
and the known 2x-display native-shell golden excluded. `./dev test-fast` passes
293/293 Python tests and ledger validation (406 entries); Release build and
200,000-case MCP fuzz pass. No commit was made.
