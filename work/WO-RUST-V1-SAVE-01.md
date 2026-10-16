---
id: WO-RUST-V1-SAVE-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 8f10553a1a7874c1843ae8161e82c8c2de255351
spec_sha256: 680a5f30a54d1d3800c4b8083aeddee9309b77644f6ba9b04152163b786a2d4d
parallel_safe: false
network: false
---
# Save and reopen the editable .vexel project in vexel-app

## Goal
Wire `vexel-project-io` and `vexel-v1-document` into `vexel-app` so one open
document can be saved to a `.vexel` file and reopened with drawing, animation
fragments and undo/redo behaving as specified below. Covers walkthrough step 4
of docs/product/vexel-v1-vertical-slice.md. No recovery/migration matrix, no
cloud, no export, no new project format features.

## Dependencies
Builds on the integrated `vexel-app` (preview/model) and does not require
APP-COMPOSE-01 for its core; if APP-COMPOSE-01 is integrated first, open-SVG
documents are saved through the same path (disclose how). Read first:
`vexel-project` (bundle format), `vexel-project-io` (`project_file_save_as`,
`project_file_load`), `vexel-v1-document` (`V1Document::source`, history,
`is_dirty`, `mark_saved`).

## Assumptions
- The `.vexel` bundle stores the document source bytes as produced by
  `V1Document::source()`; the project crates need not change. If they must,
  stop and report the gap instead of editing them.
- Undo/redo history is session state. "Preserved" means: history works across
  save (saving does not reset or corrupt the stack) and after reopen the
  document equals the saved state; persisting history across reopen is NOT
  required unless the bundle already carries it (report which).
- Source preservation: unknown SVG source untouched by edits survives a
  save/reopen byte-for-byte.

## Scope
1. Pure, window-free `project_session` module in `vexel-app`: `save`,
   `save_as`, `open`, dirty tracking via `is_dirty`/`mark_saved`, atomic write
   through `vexel-project-io` (no direct fs writes of project bytes).
2. Menu/keyboard actions Save (Cmd+S), Save As..., Open... through existing
   native panels; unsaved-changes prompt text produced by a testable function.
3. Typed, user-visible errors for unreadable, corrupt, oversize or
   wrong-version files; failed open leaves the current document untouched.
4. MCP preview path: document tools keep working after save/open; MCP save/open
   tools are out of scope here (WALKTHROUGH-01 owns the MCP flow) unless already
   present in `vexel-protocol`.

## Non-goals
Recovery files, autosave, migration, multi-document, export, GIF, new tools,
UI redesign, performance targets, live-window verification by the worker.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`, `README.md`,
  `Cargo.toml` (path deps `vexel-project`, `vexel-project-io`,
  `vexel-v1-document` only)
- `rust/crates/vexel-v1-document/src/**`, `tests/**` only if a failing test
  proves a minimal additive accessor is needed (listed in the audit)
- `rust/fixtures/v1/save/**` (new fixtures only)
- `artifacts/WO-RUST-V1-SAVE-01/**`

## Protected paths
Workspace/lock manifests (coordinator only), `vexel-project`, `vexel-project-io`,
all other crates, C/ObjC, `tests/`, goldens, oracles, `docs/**`, `STATE.md`,
`LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD/index. No unsafe, new
external deps, network, installs, GUI launch, commit, stage, reset, deletion.
Files under 800 lines.

## Acceptance criteria
1. Round trip: build a document (rects, ellipses, one animation fragment),
   save, reopen: `source()` bytes identical; element ids, order and animation
   fragment intact; `is_dirty()` false after save and after reopen.
2. Source preservation: a fixture with unknown elements, a foreign-namespace
   attribute and an editor-metadata element, edited only elsewhere, saves and
   reopens with the untouched bytes identical (asserted by byte-range compare).
3. Undo/redo: undo, save, redo works; edit-undo-edit-save-reopen yields the
   post-edit state; `history_cursor` semantics tested; reopen state documented
   (history persisted or reset) and asserted accordingly.
4. Dirty tracking: edit marks dirty; save clears; undo past the saved point
   marks dirty; covered by tests.
5. Failure paths each return a typed error with no panic and leave the live
   document and the existing file unchanged: truncated file, bad magic/
   version, oversize, missing path, unwritable directory, mid-write failure
   injected via the existing `vexel-project-io` test hook if public, otherwise
   a read-only directory.
6. Atomicity: save over an existing file never leaves a partial file when
   interrupted at the injected failure point (existing file bytes unchanged).
7. All new tests run without a display; existing `vexel-app`,
   `vexel-v1-document` and `vexel-project-io` tests still pass (counts may only
   increase).
8. `cargo tree -p vexel-app` shows no new external package; no `unsafe`.
9. `result.md` (<=400 words) states history persistence behaviour and what is
   unexecuted (live Save/Open panels).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-v1-document --locked --offline`
- `cargo +stable test -p vexel-project-io --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-v1-document -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-SAVE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-SAVE-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, reruns main-tree
checks and does the live Save/Open check in an isolated session, then updates
STATE.md, LEDGER.md and ledger.jsonl.
