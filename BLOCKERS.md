# Blockers

Large documents: files over 32 MiB stay refused by design (about 22 bytes of RSS per source byte; raising caps to 160 MiB would need about 3.5 GB). The fix is streaming/lazy decode, not higher limits; measured 2026-10-17 in `STATE.md`.

Active blockers: **none** for any merged work order. Everything below is either a
resolved item kept for history or an open *defect* with its own work order.

Last full gate: `./dev final` passed on 2026-10-12 at `bcdc3fa` (log
`.cache/final-gate-2026-10-12.log`; summary in `STATE.md`). Six work orders
failed independent review and are `blocked` in the ledger; see defect 4.

## Rust port status (2026-10-17)

No automated blockers. Open items: (1) live-window verification of pointer drawing, shortcuts,
panels, playback and a real Codex MCP session (automated click injection did not reach the
window; see `HANDOFF.md`); (2) corpus: 21/39 ok vs legacy 3/39, 0 timeouts; 2 XML failures (root size above 4096),
6 unsupported constructs (mostly stroked text), 9 limit-blocked (edges and text commands need memory proofs); (3) owner-gated: signing/notarization, icon and bundle id, VoiceOver/IME,
M4-08 scope, final UI/E2E and showcase recording. Defect 4 and WO-CORPUS-01/02 are `superseded`.

## Open defects

### 1. `generated/cad_t2` is refused by the element budget, with a further hidden limit

Corrected 2026-10-12 by the WO-CORPUS-02 probe (`artifacts/WO-CORPUS-02/probe.md`):
the 10.6 MB CAD file is refused with `limit at byte 2680259` by `SVG_SHAPE_MAX_SHAPES`
(65,536 elements; the file has about 106k paths, 53k rects, 53k lines and 42k text
elements). The earlier diagnosis (edge array exhausted at byte 9,365,917) was wrong:
the edge count at refusal is far below 4,194,304. A scratch run with shapes 1,048,576 and
edges 16M still failed at byte 9,448,701 (peak RSS about 375 MB) at a site not yet
identified. A per-element edge store alone will not render this file; WO-CORPUS-02
is blocked pending a re-plan that finds every limit site with a symbol-resolving probe.

The real-drawing budgets are documented and finite: edges 4M, per-path commands 4M,
elements 65,536, document text bytes 64 MiB (64 KiB per token), glyphs 1M document /
65,536 per run, transform operations 1M.

### 2. Corpus documents still refused for real content reasons

After the budget work, 3 of the 39 render-tested corpus files render. The
remaining verdicts are 13 `limit`, 13 `unsupported`, 8 `xml`, 2 `viewport`
(19 further files are larger than the sweep's `--max-bytes` and were not
render-tested, which is recorded per file rather than dropped). The
`unsupported` and `xml` groups are the known constructs: unknown non-namespaced
attributes failing closed, `<meta>`, `<!DOCTYPE>`/DTD, and `writing-mode`.
Per-file detail: `artifacts/WO-M4-10/render-diagnostics.{json,md}`.

Update (WO-CORPUS-01): unknown non-namespaced attributes, `<meta>`, `<!DOCTYPE>`
(with or without an internal subset, never expanded) and `writing-mode` are now
ignored for rendering and reported (`render-frame` prints one `note: ignored`
line per kind). The sweep still renders 3 of 39 (limit 17, path 2, unsupported 11,
viewport 2, xml 4), so the defect stays open: the formerly blocked files now fail
on later constructs. Next blocker per file: `artifacts/WO-CORPUS-01/residual-causes.md`;
histogram delta: `artifacts/WO-CORPUS-01/delta.json`. The order's stop condition
applies and the next step needs planner review.

Update (WO-CORPUS-04 candidate, not yet accepted): a bounded, canvas-size-aware
raster preflight work budget changes the pinned full-render sweep from 3/39 to
5/39 (`generated-science_t1` and `generated-embedded_t2` now render; no former
`ok` file regressed). The implementation and evidence are present in the
working tree under `artifacts/WO-CORPUS-04/`. Focused CTest passed 88/88 and the
two measured renders remain under 60 s and 2 GiB, but the remaining frozen
verification commands and fresh read-only review have not run. Treat this as a
candidate, not a cleared blocker, until that verification is complete.

### 3. Native goldens follow the host backing scale

`native_ui_render` follows the host backing scale, so on a Retina host every
native render is exactly 2× the 1× golden size (for example 1200×1300 versus
600×650). The fix is to force a 1× offscreen backing scale in the native
renderer. Do not regenerate those goldens. Carried over from WO-M4-06; the work
order that would fix it is not yet written.

### 4. Six shipped work orders fail their acceptance criteria

WO-UI-01 r2, UI-02, UI-03, BUG-01, M4-10 and HYGIENE-01 were implemented and
self-reported as passing, but a fresh read-only review failed each one (verdicts in
`artifacts/retro-verification-2026-10-12/`). Examples: the workspace names differ
from the spec, there is no settings round-trip test, `editor-smoke` was recorded
7/10 not 10/10, the archive tags point at the root `d97da2f`, and stream-token
exhaustion maps to `SVG_SHAPE_ERROR_XML` rather than `LIMIT`. All six are `blocked`
pending gap-closure orders or an owner decision to supersede.

## Resolved

- **WO-M4-06 closure** — revision 5 passed a fresh verifier and merged
  (`b04981c`) on 2026-10-12, after WO-UI-GOLDEN-01 made the native goldens
  host-independent for the tested path.
- **WO-M1-04 shutdown lifetime contract** — the contract was amended to require
  owner quiescence; concurrent entry during destroy is defined as undefined
  behaviour. Revision 2 exposed a pre-existing flaky native pointer-down test in
  both the accepted M1-03 binary and its own candidate; the comparison is
  retained in `artifacts/WO-M1-04/baseline-ui-smoke-comparison.json`. Revision 3
  added a test-only direct dispatch to the target view while preserving the
  event type, coordinate, bounds and button assertions (ADR-0007 records that
  change). Revision 3 passed all nine frozen commands and two fresh read-only
  reviews and merged at
  `91ff611782c843428fb5bdc4ba874e3ccdd37f94`; final verifier evidence is under
  `artifacts/runs/WO-M1-04/verify-e71955ec-d0a3-4888-8fa2-4355f82ce469/`.
- **Editor save data loss** — a canvas gesture that missed its target left the
  selection on the document root, and composing a transform onto the root
  rewrote the whole document, so the saved file lost its shapes. The transform
  now refuses a root target, with a regression test that fails on the unguarded
  code.
- **`Control-Tab` swallowed by macOS** — the workspace cycle uses Blender's own
  `Control-PageUp`/`Control-PageDown`; `Control-Tab` is an alias only.
- **Duplicate accessible label** — the tool-options strip rendered the same label
  as the tool rail button, so any label lookup could resolve to the wrong node.
- **Escape ordering** — the Adjust Last Operation dismissal ran before the canvas
  gesture cancel, so a gesture could survive an Escape and mutate later.
- **Interrupted round** — a ledger append path outside the repository is
  rejected; entries are written from `.cache/` instead.

## Owner decisions (2026-10-17)

- Root size above 4096 (aegean, norway): keep refusal. Raster caps and the frozen oracle (height=5000 as error) unchanged. Both files stay `xml`.
- Limit-blocked files (9): keep edge (4.19M) and text-command budgets. Files stay `limit_decode`.
- Symbol clip (use_t2): conditional clip tried. Rejected: the document-wide `<style>` check blocks the gate, so use_t2 stays limit_decode. Decision: no symbol clipping on main (use_t2 ok). Candidate kept at `.cache/symbol-clip-candidate.patch` (local, not committed) for a later selector-aware gate (option A).

## Corpus known-unsupported (2026-10-17, decided by Sonnet 5.5)

- **corsica:** two independent blockers. (1) Prefixed SVG elements: `<svg:switch>` at byte 5116 is rejected by `vexel-svg-decode/src/traversal.rs` (`t.prefix.length != 0`). Fixing needs namespace-bound prefix support across css, media, animation, index and scene crates; planned as a work order, not a quick fix. (2) `<tspan id="tspan3993">` in `text14619` near byte 14832283: unresolved; native CoreText and stub providers both accept the tspan in isolation. Left unsupported.
- **ideogram:** `<path id="path277302">` near byte 20705253 is rejected. Not caused by the gradient fill (a solid-fill copy still fails on a path), not by the clip-path ancestors, and not by `inkscape:connector-curvature` alone (removing it moves the failure to an earlier path). Multiple rejected constructs; left unsupported.
