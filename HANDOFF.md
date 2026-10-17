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

## State (2026-10-17)

- **Everything below is merged on `main` and pushed; the Rust workspace passes 1727+ tests**
  (`cargo test --workspace --locked`; ledger validates). Each order below was implemented
  by a worker and accepted only after a fresh read-only review (ledger events, `work/WO-*.md`).
- **V1 slice (ADR-0029), automated level:** SVG decode subset (`vexel-svg-engine`), renderer
  (`vexel-render`, byte-exact vs the legacy C goldens), `.vexel` save/reopen, native
  Open/Save/Save As panels (Open offers `.svg` and `.vexel`), SVG compose pipeline, drawing tools,
  one `AppDocument` session, playback + timeline strip, animation + GIF (`vexel-gif`),
  `render_png` (`vexel-png`), MCP tools (save/open/attribute/delete/export_svg/render_png,
  launch-time `--grant-dir`), scripted MCP walkthrough of the cutline.
- **Beyond V1, also done:** masks and opacity groups, patterns, filters, text (CoreText),
  stroke dashes, markers, PNG images, 32 MiB compose cap with `limit_decode`/`limit_render`
  verdicts, renderer speed-ups, Rust release tooling (`vexel-dev bundle|dmg|sbom|install-smoke`,
  ad-hoc signed), fmt gate, seeded fuzz tests, perf gate, `vexel-corpus` measurement tool.
- **Corpus (external, read-only at `~/Desktop/Freelance/vexel-corpus-real`):** 18/39 ok (legacy C: 3/39), 1 timeout
  (`STATE.md` 2026-10-17). Remaining: 3 xml failures (root size above 4096, DOCTYPE internal entities, one more), 10 unsupported
  constructs, 7 limit-blocked (edges above 4.19M and text-command budgets need their own memory proofs).
- **Verified on a real window (2026-10-16/17):** app launches, opens SVGs, renders them, and the new UI
  (toolbar, rail, sectioned inspector, artboard canvas at 400% fit, status bar; screenshots in
  `docs/archive/live/`). **Not yet verified
  live:** pointer drawing, Undo/Redo shortcuts, Save/Open panels, playback strip, MCP session from
  a real Codex client, the Cmd+S double-fire guard — automated click injection did not reach
  the window (process not frontmost when launched from a harness terminal). Run
  `artifacts/WO-RUST-V1-WALKTHROUGH-02/manual-checklist.md` (M1-M10) on a Mac session.
- **Owner-gated, not done:** signing with a Developer ID and notarization, app icon and final
  bundle id (`app.vexel.Vexel` is a placeholder), VoiceOver/IME review, M4-08 scope, the full
  UI/E2E final acceptance and the showcase recording.
- **Known limits:** decoder rejects nested layered groups and some pattern/mask orderings;
  dashes deferred in animation frames; blend modes, gradients, JPEG and interlaced PNG are
  typed disclosures; C quirks reproduced for text parity (dx[0], x[i], word spacing) pending a
  post-parity decision; `cargo fmt` drift in 27 crates is allowlisted in `rust/ci/fmt-debt.txt`.
- **Legacy C removed** (tag `legacy-c-final`); four C raster files remain under
  `rust/fixtures/legacy/c-raster/` for the render parity map.
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
