---
id: WO-RUST-V1-INTEGRATE-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: d2b182e2ac7b9a8f328d7270a65d1644408b03a3
spec_sha256: 9989655c07197a9a1c55e7a7fa3d0e43a228ead49eed42b84b11045435a4a8fb
parallel_safe: false
network: false
---
# One document session: open, edit, render, save and preview through a single V1Document

## Goal
Today `vexel-app` has three disconnected document states: `FileUi` owns a
`project_session::ProjectSession` over a `V1Document` (Open/Save/Save As),
`compose::DocumentView` keeps its own bytes/decode state (CLI-arg SVG open ->
decode -> scene -> `vexel-render` -> present, pointer mapping, hit test), and
`preview`/`preview_app` (MCP basic-shape live preview) mutates a separate
preview document. Unify them: the `V1Document` source inside one session is the
single source of truth; compose renders from `source()`; File>Open (`.vexel`
and `.svg`), Save, Save As, undo/redo and MCP preview mutations all go through
it, mark dirty correctly and re-render the same view. This is the final
app-level integration before WALKTHROUGH-01; thin glue plus hooks only.

## Dependencies (blocking)
Integrates after, and only after, `WO-RUST-V1-DRAW-TOOLS-01` and
`WO-RUST-V1-ANIM-01` are ACCEPTED and merged (both are developed concurrently
and touch `vexel-app/src`). Already on main: SAVE-01 (`project_session`),
NATIVE-PANELS-01 (`file_flow`, `file_ui`), APP-COMPOSE-01 (`compose`). If
either blocker is not on main at start, stop and report; do not stub them.
`WO-RUST-V1-WALKTHROUGH-01` depends on this order (coordinator updates its
Dependencies section by note when INTEGRATE-01 is accepted; this order does not
edit it). Read first: `vexel-app/src/{main,file_ui,preview,preview_app}.rs`,
`compose/{mod,present,open,worker,pointer}.rs`, `project_session/mod.rs`,
`file_flow/mod.rs`, the DRAW-TOOLS `tools/**` and ANIM playback modules as
merged, and each predecessor's `artifacts/*/result.md` (from the merge
history).

## Assumptions
- `V1Document::source()` bytes are valid input to `compose::compose_document`
  (ADR-0029 subset); `.svg` opened via File>Open becomes a `V1Document` through
  `V1Document::from_source` (if it rejects a decodable SVG, report the gap).
- Opening a `.svg` yields an untitled document (no path) so Save routes to Save
  As and never overwrites the SVG; document dirty state starts clean.
- Compose stays on its worker/generation scheme; the session only supplies
  bytes and a revision counter. Stale worker results are dropped by generation.
- Animation: compose already discloses "static first frame". Playback from the
  ANIM order is hooked, not re-implemented here.
- DRAW-TOOLS `ToolController` and `CanvasMap` exist as merged; names are taken
  from main. If they differ from `work/WO-RUST-V1-DRAW-TOOLS-01.md`, follow main
  and list the delta.

## Scope
1. New `vexel-app/src/session/` module (window-free): `AppDocument` wrapping
   `ProjectSession`, with `revision() -> u64` incremented on every source change
   (edit, undo, redo, open, preview mutation), `source_bytes()`, `title()`,
   `is_dirty()`, `open_path(path)` choosing by extension/magic (`.vexel` via
   `ProjectSession::open`, `.svg` via `from_source`), failed open leaves the
   current document, revision and path untouched.
2. Hook boundary (trait/module, no hard dependency of `session` on tools or
   playback): `trait DocumentEditor { fn apply(&mut self, &mut V1Document) ->
   Result<EditOutcome, _> }`-style entry or equivalent narrow API through which
   `tools::ToolController` and ANIM mutations reach the document via
   `AppDocument::edit(|doc| ..)`, which bumps `revision` only if `source()`
   changed; plus `impl CanvasMap for` the compose `PointMap` (completing the
   DRAW-TOOLS integration step) and a `PlaybackHost` hook that lets the ANIM
   clock request a frame render from the same source. Hooks are documented in
   module docs and exercised by tests with fake editors.
3. Compose wiring: `compose::DocumentView` gains `show(bytes, revision, stage)`
   (render from session bytes); `DocumentView::open` (path based) is retained only
   for the CLI launch path and routes through `AppDocument::open_path`. No second
   copy of the document source in `DocumentView`.
4. `FileUi` is re-pointed at `AppDocument`; File>Open/Save/Save As/close prompt
   behaviour of NATIVE-PANELS-01 is unchanged (its tests stay green). Window
   title shows document name plus a dirty marker (testable `title()` pure fn).
5. MCP preview: `preview_app` mutations (basic-shape add and, if present on
   main, animation fragments) are applied through `AppDocument::edit` instead of
   its private document, then re-render the same view and mark dirty. If the
   preview mode and the shell mode remain separate launches on main, unify them
   in the shell launch so one process serves both; state exactly how.
6. Undo/redo (Cmd+Z / Shift+Cmd+Z, only if not already bound; report) call
   `V1Document::undo/redo` through the session and re-render.
7. Main loop re-render trigger is `revision` change, not ad-hoc dirty flags.

## Non-goals
New tools or drawing behaviour, new animation/GIF features, export, multi-doc,
autosave, recovery, MCP save/open tools (WALKTHROUGH-01), renderer changes, UI
redesign, live-window verification by the worker.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`, `README.md`
  (vexel-app section), `rust/crates/vexel-app/Cargo.toml` (existing path deps only)
- `rust/fixtures/v1/integrate/**` (new fixtures only)
- `artifacts/WO-RUST-V1-INTEGRATE-01/**`
- Other crates only for a minimal additive accessor proven by a failing test and
  listed in the audit (`vexel-v1-document` first choice).

## Protected paths
Workspace/lock manifests (coordinator only), `vexel-project`, `vexel-project-io`,
`vexel-render`, `vexel-svg-engine`, all other crates, C/ObjC, `tests/`, goldens,
oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders,
git HEAD/index. No unsafe, new external deps, network, installs, GUI launch,
commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (all headless: synthetic events, offscreen frames)
1. Single source: `AppDocument` holds the only `V1Document`; a test builds the
   shell state, adds a rectangle through `edit`, and asserts the composed frame
   hash equals `compose_svg(doc.source())` for the same size, with no second
   source copy in `DocumentView` (asserted by API shape).
2. Open `.vexel` fixture and `.svg` fixture through `open_path`: frame hashes
   equal the hashes of rendering the same bytes via the SVG path / `.vexel`
   scene bytes; `.svg` opens untitled and clean; Save on it requests Save As.
3. Failed open (corrupt, wrong version, oversize, missing) returns a typed
   error; source bytes, revision, path and dirty state are unchanged.
4. Dirty/title: edit -> dirty and title has the marker; Save -> clean; undo past
   the saved point -> dirty; redo back -> clean; reopen -> clean. Pure-function
   tests for `title()` with untitled/named/dirty.
5. Undo/redo: each bumps `revision` and produces a frame hash equal to the
   pre-edit/post-edit frames recorded earlier in the same test.
6. MCP preview: a synthetic shape-add request (and animation fragment if on
   main) applied through the same entry mutates the session, bumps `revision`,
   marks dirty, and the next frame equals the expected offscreen hash; a
   following Save/reopen round-trips the bytes (`source()` identical).
7. Hooks: a fake `ToolController`-style editor and a fake `PlaybackHost` drive
   `AppDocument::edit` and frame requests in tests; a no-op edit does not bump
   `revision`; `PointMap` as `CanvasMap` maps stage points to document points
   consistently with `DocumentView::hit` on the same fixture.
8. Stale compose results (older generation) never replace a newer frame (test
   with injected worker events).
9. File>Open, Save, Save As, close-prompt tests from NATIVE-PANELS-01 and SAVE-01
   still pass unmodified; counts may only increase. All new tests run without a
   display.
10. `cargo tree -p vexel-app` shows no new external package; no `unsafe`.
11. `result.md` (<=400 words): hook names and where DRAW-TOOLS/ANIM attach, how
    preview and shell modes were unified, deltas vs the predecessor orders, and
    the unexecuted live checks below.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Manual live-check list (coordinator, isolated session, not worker-verifiable)
1. Launch with an `.svg` argument: image appears, title untitled, not dirty.
2. File>Open a `.vexel`: replaces view; Cancel and a corrupt file leave the
   current view and title intact.
3. Draw a shape (DRAW-TOOLS), title shows dirty; Cmd+Z / Shift+Cmd+Z re-render.
4. Cmd+S on untitled opens Save As; saved name appears in title, dirty clears.
5. With MCP preview connected: add a shape; the window updates, title dirty;
   Save, quit, reopen shows it.
6. Play an animation (ANIM) from an opened document; edits while stopped
   re-render; close with unsaved changes shows the prompt.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-v1-document --locked --offline`
- `cargo +stable test -p vexel-project-io --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-v1-document -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-INTEGRATE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-INTEGRATE-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch after DRAW-TOOLS-01
and ANIM-01, reruns main-tree checks, does the live checks above, then updates
STATE.md, LEDGER.md and ledger.jsonl and notes WALKTHROUGH-01's dependency.
