---
id: WO-RUST-V1-WALKTHROUGH-02
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: b10a78b3231c70901e55edafd8057064a4219716
spec_sha256: e1cf57d077d457b19537d3862f47784780f9dc56eac4f6a93263219e8bf1952a
parallel_safe: false
network: false
---
# Scripted Codex-over-MCP V1 walkthrough (revision using routed tools)

## Goal
Same goal as WO-RUST-V1-WALKTHROUGH-01: prove the ADR-0029 cutline end to end
with a headless test that speaks the real `vexel-mcp-stdio` JSON-RPC protocol to
the app's live backend, plus a manual checklist for native rows. This order
supersedes WALKTHROUGH-01 (frozen, unedited) once `WO-RUST-V1-MCP-TOOLS-01` is
integrated; the coordinator marks -01 superseded in the ledger. Not a UI/E2E
campaign, not the Codex video, no full-product claim.

## Dependencies (blocking)
`WO-RUST-V1-MCP-TOOLS-01` ACCEPTED (routed save, save_as, document_open,
attribute_set, element_delete, element_add line/path, export_svg, grants, and
the `vexel_app` lib target), plus ANIM-01, PLAYBACK-01 and SAVE-01 integrated.
If a needed tool is absent, stop and report the exact gap; add no features.
`render_png` is deferred by MCP-TOOLS-01 and is not used here.

## Assumptions
- The test links `vexel_app` as a library: `AppDocument`, `McpHost`, the shared
  `Driver`, `LiveBackend`, grant policy constructor, `ExportJob`. It uses
  `vexel_mcp_stdio::serve` over in-memory buffers (or OS pipes) with a pump
  thread applying requests to the one `AppDocument`; no NSWindow, no display.
- The grant is the explicit launch grant (`McpGrants`/`--grant-dir`
  equivalent) for a temp directory; there is no MCP grant tool by design.
- The client is a deterministic scripted JSON-RPC sequence; no network, LLM or
  API spend. Native frame appearance is covered by the manual checklist only.

## Scope
1. `rust/crates/vexel-app/tests/walkthrough.rs` (+ `tests/common/`), steps in
   order, each checking the MCP result (operation name, element ids, `dirty`):
   a. `initialize`, `tools/list` (assert routed names), grant the temp dir;
   b. draw a text-free character, >=5 shapes with distinct ids (body path or
      ellipse, head, two eyes, mouth via `path`/`line`), solid fill/stroke;
   c. revise one shape with `attribute_set` (fill, then `transform`
      translate); assert only that element's bytes changed;
   d. delete one scratch element with `element_delete`;
   e. keyframed loop: `animation_add` 2-4 keys (blink opacity or position) on
      one element; `animation_control` play, seek, pause with expected
      `timeSeconds`/`playing`;
   f. undo then redo an edit; full source bytes equal before/after each;
   g. `save_as` to `<grant>/walk.vexel`; new `AppDocument` + host;
      `document_open` it; source bytes equal; animation intact; one more edit
      (`attribute_set`) succeeds and undo reverts it; `save` succeeds;
   h. `export_animation` GIF (fixed fps/duration) and `export_svg`; write the
      SVG text into the grant via test code;
   i. also `document_open` of a fixture `.svg` and edit it.
2. GIF check: parse with the ANIM-01 test decoder: frame count = fps*duration,
   constant delay, loop block, canvas size, frame at blink time differs from
   frame 0 at the animated element's pixels.
3. SVG check: exported SVG decodes through `vexel-svg-decode` and re-renders
   within RENDER-01 tolerance of the saved document render.
4. Negative paths: ungranted path, `..` escape, symlink out of grant,
   malformed JSON-RPC, unknown tool, wrong documentId, out-of-order `redo`,
   oversize argument: typed errors, document bytes and revision unchanged.
5. Manual checklist `artifacts/WO-RUST-V1-WALKTHROUGH-02/manual-checklist.md`:
   native rows (Draw/Animate switch, canvas draw, inspector recolor,
   scrub/play, Save/Open panels, GIF in a second viewer, undo/redo shortcuts,
   keyboard focus, MCP-launched `--grant-dir` run with a real client, activity
   display), each with expected observable result, pass/fail box and "not run
   by worker".

## ADR-0029 cutline mapping (each: named test or checklist row)
one document; one text-free character; few edit actions; short keyframed loop;
editable save/reopen; basic SVG subset (import/export, unsupported disclosed);
GIF export; local authorized MCP; visible activity/results; undo/redo of
demonstrated edits. `cutline-map.json` maps all ten; gaps are named, never
omitted (PNG render is listed as a deferred gap, not a cutline item).

## Non-goals
Pixel goldens, accessibility, performance, remote MCP, multi-client, recovery,
packaging, real Codex, video, new product features.

## Files allowed to touch
- `rust/crates/vexel-app/tests/**` (new), `rust/fixtures/v1/walkthrough/**` (new)
- `rust/crates/vexel-app/src/**` only for thin glue proven necessary by a
  failing step (each change listed in the audit with the failing step)
- `rust/crates/vexel-app/Cargo.toml` (dev-dependency path crates only)
- `artifacts/WO-RUST-V1-WALKTHROUGH-02/**`

## Protected paths
Workspace/lock manifests, all other crates, C/ObjC, `tests/`, goldens, oracles,
`docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders
(including WALKTHROUGH-01), git HEAD/index. No unsafe, external deps, network,
installs, GUI launch, real Codex/API calls, commit, stage, reset, deletion.
Files under 800 lines. Never weaken another order's tests.

## Acceptance criteria
1. Steps a-i pass in one `cargo test` run; 3 consecutive runs give
   byte-identical saved project, GIF and SVG (hash table).
2. Each undo/redo assertion compares full source bytes.
3. Reopened project equals the saved one, animation fragments intact, and
   accepts and undoes a further edit.
4. GIF and SVG checks (scope 2, 3) pass as stated.
5. Negative paths (scope 4) pass with no document mutation.
6. Each step asserts its reply names operation and element id(s).
7. `cutline-map.json` covers all ten items, each pointing at an existing test
   name or checklist row.
8. Manual checklist exists and every row states the expected result.
9. Existing tests of touched crates pass; no new external package.
10. `result.md` (<=400 words) lists passed, failed, glue added, unexecuted
    (all native-window rows) and the PNG deferral.

If an upstream gap blocks an AC, stop with the exact blocker; do not fake.

## Verification (frozen; repo-local cargo dirs, locked, offline)
- `cargo +stable test -p vexel-app --test walkthrough --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-mcp-stdio -p vexel-protocol --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-WALKTHROUGH-02 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-WALKTHROUGH-02/`: `result.md`, `verification.json`,
`cutline-map.json`, `manual-checklist.md`, `changed-path-audit.json`,
three-run hash table.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate until a fresh read-only reviewer (separate session,
Luna or Sol Medium; diff, order, checklist only) reruns frozen commands and maps
every cutline row. The coordinator integrates, reruns main checks, executes the
manual checklist on an isolated desktop session, then updates STATE.md,
LEDGER.md and ledger.jsonl. V1 is declared only after the checklist passes.
