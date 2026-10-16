# WO-RUST-V1-WALKTHROUGH-02 result

## Passed
- `tests/walkthrough.rs`, 4 tests, all green: steps a-h (`walkthrough_steps_a_to_h`), 3-run/in-process byte identity (`walkthrough_outputs_are_byte_identical_across_runs`, hashes in hash-table.txt), step i SVG open/edit (`step_i_...`), negative paths (`negative_paths_...`).
- Full `vexel-app` suite (173 lib, 1 bin, 16+9 MCP-tools, 4 walkthrough), `vexel-mcp-stdio`/`vexel-protocol` suites, fmt, wo_validate, ledger validate.
- GIF: 10 frames = fps*duration, constant 10 cs delay, NETSCAPE loop 0, 512x512, eye pixel differs at t=0.5 while body pixel is equal.
- SVG: export equals source, decodes via compose (vexel-svg-decode) and renders within max channel delta 1 of the reopened saved project (actual: identical).

## Deviations from the order (main's actual behavior)
- AC6: `element_add` ellipse, `undo`, `redo`, `animation_add` replies do not carry `operation`/`dirty` (only rect-era shape: `shapeCount`, `elementId`, `message`). Those steps assert what exists plus `dirty`/revision from the document state. No glue added.
- Pause `timeSeconds` is wall-clock dependent (MonotonicClock); asserted `playing:false`, then seek to 250 ms for an exact value.
- "unsupported disclosed": `document_open` of an unsupported `.svg` is not rejected; disclosure is `compose_svg` error text (`unsupported in V1: text`), asserted on the existing compose fixture.
- `history_count` includes redo entries, so after undo then a new edit the count is not +1; asserted >= 1 there.
- Step a: `tools/list` asserts routed names, absence of `render_png` and of any grant tool; the grant is the launch policy (`McpGrants`).
- fmt command must run from the worktree root.

## Glue added
None. No src, manifest, or lock changes.

## Unexecuted
All native-window rows (manual-checklist.md M1-M10), including real-client `--grant-dir` run and activity display.

## Deferred gap
`render_png` stays unrouted (no PNG encoder; MCP-TOOLS-01); not a cutline item.

## Git
Sandbox hook refused git commands; work is left uncommitted on the branch.
