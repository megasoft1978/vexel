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

- **Rust V1 slice is built and accepted at the automated level** (main `49ef038f`,
  1473 workspace tests pass). Accepted orders: decode subset (`vexel-svg-engine`),
  render (`vexel-render`, byte-exact vs C goldens), save/reopen, native Open/Save panels,
  SVG compose, animation + GIF (`vexel-gif`), drawing tools, integration (single
  `AppDocument` session), playback + timeline strip, MCP tools (save/open/attribute/
  delete/export_svg, launch-time `--grant-dir`), automated MCP walkthrough of the
  ADR-0029 cutline.
- **Not yet verified on screen:** every native/live behaviour. Run
  `artifacts/WO-RUST-V1-WALKTHROUGH-02/manual-checklist.md` (M1-M10) plus the live-check
  lists in each order's `result.md` (via `git log` / tag history): native Save/Open panels and a
  possible double-fire on Cmd+S, drawing in the window, playback strip and held-Space
  repeat, edit-while-playing blank frame, MCP session from a real Codex client.
- **Known gaps:** filters, mask-on-group, patterns, dashes, text; `render_png` (no PNG
  encoder crate); File > Open panel offers `.vexel` only (`.svg` via path/CLI/MCP);
  `element_add` schema enum stale (line/path work but unadvertised); legacy
  `McpHost::handle` and `preview_app.rs` dead code; synchronous playback render; no
  corpus measurement for Rust.
- **Legacy C** (`src/`, `app/`, `CMakeLists.txt`, C-bound scripts/tests, fuzz/nightly/release
  workflows) is still in the tree, frozen; `./dev verify` does not link (undefined
  `viewer_editor_*`). Tag `legacy-c-final` marks the last tree before the Rust work landed.
  Deletion plan: archive/port per the cleanup map, delete after owner sign-off.
- Large artifacts and third-party PDFs are not committed: `docs/archive/EXCLUDED-LARGE.txt`.

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
