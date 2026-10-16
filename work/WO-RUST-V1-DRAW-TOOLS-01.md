---
id: WO-RUST-V1-DRAW-TOOLS-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: aefb9b70659a8d71165b70ff879964dbdf5328fc
spec_sha256: abffe8a3a394a75d93120cd1ffda71c0d49be05731bb3325a8802d92cbfd6e91
parallel_safe: false
network: false
---
# Drawing and revision tools with source-preserving undoable edits

## Goal
Provide the ADR-0029 "draw and revise" capability as a pure, window-free tool
layer: draw rectangle, ellipse and line; a pen tool producing a polyline or
cubic-Bezier `path`; select, move and delete; change fill and stroke colour.
Every action is a source-preserving edit through `vexel-v1-document` /
`vexel-document-edit` with undo/redo. Covers walkthrough steps 1-2 of
docs/product/vexel-v1-vertical-slice.md. No UI chrome redesign, zoom/pan, node
editing, resize/rotate handles, layers, text, animation or MCP change.

## Dependencies
Builds on a new module `rust/crates/vexel-app/src/tools/**`. It integrates after
WO-RUST-V1-APP-COMPOSE-01: pointer-to-document mapping is taken from that order
(`ViewMap`-based canvas<->document function and shape hit test). This order
must not depend on APP-COMPOSE code at build time: define a small trait
`CanvasMap { to_doc(x, y) -> Option<(f64, f64)> }` in `tools`, with an identity /
scale test implementation; the coordinator wires the compose mapping when both
are integrated (disclose as an unexecuted integration step). If APP-COMPOSE-01
is already integrated at start, implement that trait with its mapping instead.
Read first: `vexel-v1-document/src/lib.rs`, `vexel-document-edit/src/{insert,ops,
query,types}.rs` (`insert_element`, `TemplateKind::{Line,Path}`, `set_attribute`,
`add_attribute`, `delete_element`, `element_at_offset`, history), `vexel-app/src/
model.rs` (existing `Tool` enum: Select, Pen, Rectangle, Ellipse),
`vexel-ui/src/widget_input.rs` (`PointerEvent`/`PointerState` vocabulary).

## Assumptions
- Colours are `#rrggbb` (existing `InvalidColor` validation); stroke width is a
  finite number in 0..=256.
- Document is the only shape store; selection is an element stable id kept by
  the tool layer, never cached geometry. Elements created by tools receive
  unique stable ids (`rect-N`, `ellipse-N`, `line-N`, `path-N`).
- A tool gesture is one history entry: Down/Move/Up commits once on Up;
  Escape or Cancel commits nothing. Previews are not document edits.
- Move/recolour work on tool-created and imported `rect`, `ellipse`, `circle`,
  `line`, `path`; other elements are selectable but report `Unsupported` for
  move, never silently dropped or rewritten.
- Move of a `path` emits or updates a `transform="translate(dx dy)"` attribute
  (source of the `d` bytes is untouched); rect/ellipse/line update coordinates.
  Existing non-translate `transform` values are left as-is and the move is
  refused with a typed error.

## Scope
1. `vexel-v1-document` additive API (only with failing-test proof first; each
   listed in the audit): `add_line(x1,y1,x2,y2,stroke,width)`,
   `add_path(d: &str, fill, stroke, width)` (validated path-data grammar subset:
   `M m L l H h V v C c Z z`, finite numbers, <= 8 KiB, bounded segment count),
   `set_fill(id, colour)`, `set_stroke(id, colour, width)`, `delete(id)`,
   `translate(id, dx, dy)`, `element_kind(id)` and `hit_test(x, y) -> Option<id>`
   (topmost, bbox for path with a documented tolerance). All reuse the existing
   stable-id validation and undo history; none re-serializes untouched source.
2. `vexel-app/src/tools/` (each file < 800 lines): `ToolController` with
   `pointer(event, &mut V1Document, &dyn CanvasMap)` for events
   `Down{x,y} / Move{x,y} / Up{x,y} / Cancel`, `key(Escape|Delete|Enter)`,
   `set_tool`, `set_fill`, `set_stroke`, `selection()`; tools Select, Rectangle,
   Ellipse, Line, Pen. Pen: click adds a vertex, click-drag sets a cubic handle,
   Enter or double-click-equivalent (documented event) finishes; Escape cancels;
   `Z` close is a controller method. Fewer than two vertices commits nothing.
3. `Tool` enum in `model.rs` gains `Line` (rail layout updated: `tools` array
   and its layout tests only), otherwise the shell is unchanged. Live pointer
   routing into `ToolController` is limited to the stage rectangle and is
   covered by a pure function test; no window is launched.
4. Undo/redo of each action works through existing `V1Document::undo/redo` and
   dirty tracking (`is_dirty` / `mark_saved`) stays correct.

## Non-goals
Zoom, pan, snapping, grid, resize/rotate, multi-select, groups, text, arcs and
quadratic/smooth commands, gradients, node editing, rendering changes, MCP tool
additions, save/open UI (PANELS-01), animation, performance targets.

## Files allowed to touch
- `rust/crates/vexel-app/src/tools/**` (new), `rust/crates/vexel-app/src/
  {model,main}.rs` (Tool::Line and routing only), `rust/crates/vexel-app/tests/**`
- `rust/crates/vexel-v1-document/src/**`, `tests/**` (additive only, see Scope 1)
- `rust/crates/vexel-document-edit/src/**` only if a failing test proves a
  minimal additive accessor is needed (listed in the audit; no behaviour change)
- `rust/fixtures/v1/draw/**` (new fixtures only)
- `artifacts/WO-RUST-V1-DRAW-TOOLS-01/**`
Path-dependency changes to `Cargo.toml` are coordinator-only; if one is needed
(for example `vexel-document-edit` in `vexel-app`), stop and report.

## Protected paths
Workspace/lock manifests and all `Cargo.toml`, `vexel-project*`, `vexel-ui`,
renderer/svg crates, other crates, C/ObjC, `tests/`, goldens, oracles, `docs/**`,
`STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD/index. No
unsafe, new external deps, network, installs, GUI launch, commit, stage, reset,
deletion. Existing public APIs and existing test expectations are unchanged.

## Acceptance criteria
1. Headless gesture tests with synthetic pointer events: Rectangle, Ellipse and
   Line drags each add exactly one element with expected attributes in
   `source()`; click without drag, Cancel and Escape add nothing; coordinates
   are mapped through a `CanvasMap` with non-identity scale/offset.
2. Pen: three clicks plus Enter yields a `path` whose `d` is the expected
   `M..L..L..` string; a click-drag vertex yields a `C` segment with expected
   control points; one-vertex finish adds nothing; the produced `d` passes the
   same parser used by `add_path` and, where `vexel-svg-path` is already a
   dev-dependency or available, decodes without error (otherwise stated).
3. Select/move/delete: Down on a shape selects topmost id (overlap fixture);
   drag moves it by the mapped delta (rect coordinates change; path gets
   `translate`); Delete removes it; click on empty canvas clears selection;
   unsupported element or non-translate transform returns the typed error.
4. Fill/stroke: `set_fill`/`set_stroke` change only the targeted attributes;
   invalid colour, negative or non-finite width, and unknown id return typed
   errors and leave source bytes identical.
5. Undo/redo: for every action in AC 1-4, undo restores the exact prior
   `source()` bytes and redo the exact post-edit bytes; a gesture is one
   history entry (`history_count` increases by exactly 1); `is_dirty` follows.
6. Source preservation: on a fixture with unknown elements, a foreign-namespace
   attribute and comments, edits elsewhere leave all untouched byte ranges
   identical (byte-range compare), including after undo/redo.
7. Reopen: a document built only through tools round-trips through
   `V1Document::from_source(source())` with equal bytes and ids.
8. Bounds: path data and id limits enforced; 10,000 synthetic events do not
   grow memory without bound (preview state is O(vertices), capped).
9. All tests run without a display; existing `vexel-app`, `vexel-v1-document`
   and `vexel-document-edit` tests still pass (counts may only increase); no
   `unsafe`; `cargo tree -p vexel-app` shows no new package.
10. For each new `vexel-v1-document` API, the audit lists the failing test that
    preceded it. `result.md` (<=400 words) states APP-COMPOSE-01 mapping status
    and lists unexecuted live canvas drawing as manual verification.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-v1-document --locked --offline`
- `cargo +stable test -p vexel-document-edit --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-v1-document -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-DRAW-TOOLS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-DRAW-TOOLS-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch after APP-COMPOSE-01,
wires the compose mapping, performs the live drawing check in an isolated
session, then updates STATE.md, LEDGER.md and ledger.jsonl.
