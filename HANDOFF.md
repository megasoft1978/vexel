# Handoff

Current as of 2026-10-16 (main pushed at `2898e15` plus the ledger/doc follow-up).
Replaces the 2026-10-12 C17 handoff.

## What this repository is

A macOS-first (Apple Silicon, macOS 13+) vector drawing and animation editor with
an owned XML/SVG parser, rasteriser, custom-drawn UI, native animation timeline
and a local MCP server so agents (Codex, Claude Code) edit the same open document.
Product goal: `docs/adr/0018-product-goal-and-completion-gates.md`. Active
direction: **full Rust port** (`OWNER-DIRECTIVES.md` item 4, ADR-0022). The legacy
C17 code (`src/`, `app/`, `CMakeLists.txt`) is a frozen migration reference; the
owner intends to delete it once Rust reaches parity.

## State

- **Rust workspace** (`rust/`, 39 crates, ~88k LOC): accepted libraries for XML,
  JSON, scene model, SVG values/paint/path/CSS/animation, tile cache, timeline,
  document edit, project IO, MCP stdio/protocol, UI toolkit and Apple platform
  bindings. `vexel-app` runs a basic-shape live MCP preview only.
- **Not connected yet (V1 critical path):**
  1. `vexel-svg-decode` is a stub. WO-RUST-SVG-DECODE-01 is still `running` in the
     ledger, but no process is live; the best candidate is
     `.cache/WO-RUST-SVG-DECODE-01/current-continuation-11/` (not in git).
  2. No renderer crate (C reference: `src/raster/shape_render*.c`).
  3. App composition, `.vexel` save/reopen wiring, animation playback, GIF export.
- **C implementation**: passed `./dev final` at `bcdc3fa`; work after that (anim
  project, recovery, SVG paint/clip, MCP async) is committed but has no fresh full
  gate. Corpus: 3/39 render (5/39 candidate WO-CORPUS-04, unaccepted).
- **Superseded** under directive 4: the six C orders blocked on 2026-10-12 and
  WO-CORPUS-01/02. The `work/WO-GAP-*` files stay READY but are not to be run.
- Large artifacts (>1 MiB) and third-party PDFs are not committed; see
  `artifacts/EXCLUDED-LARGE.txt` (sha256 manifest; originals only on the author disk).

## V1 critical path (ADR-0029)

decode subset → renderer crate → app composition → save/reopen → animation + GIF →
Codex MCP walkthrough. Orders: `work/WO-RUST-V1-DECODE-SUBSET-01.md`,
`work/WO-RUST-V1-RENDER-01.md`; later orders are listed in
`docs/product/vexel-v1-vertical-slice.md`. Freeze new library/contract orders that
are not on this path.

## How work is done here

Read `OWNER-DIRECTIVES.md`, then `AGENTS.md`. Frozen work order in `work/` →
implementation worker → fresh read-only verifier → `ledger.jsonl` / `LEDGER.md`
(`python3 scripts/ledger.py validate ledger.jsonl`). Lessons: never freeze an
acceptance criterion the candidate cannot literally satisfy; run a symbol-resolving
probe before raising a limit; when behaviour changes deliberately, fix the test that
pinned the old behaviour, never the guard.

## Open owner decisions

- Timing/scope of deleting the legacy C code and cleaning the repository.
- Whether to port the 1x native golden fix (WO-UI-GOLDEN-02) from
  `archive/second-lineage-final` into the renderer order.
- M4-08 scope, signing/notarization, VoiceOver/IME review.
