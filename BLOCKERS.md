# Blockers

Active blockers: **none** for any merged work order. Everything below is either a
resolved item kept for history or an open *defect* with its own work order.

Last full gate: `./dev final` passed on 2026-10-12 at `bcdc3fa` (log
`.cache/final-gate-2026-10-12.log`; summary in `STATE.md`). Six work orders
failed independent review and are `blocked` in the ledger; see defect 4.

## Open defects

### 1. Multi-million-edge drawings exceed one document-wide edge array

`SVG_SHAPE_MAX_EDGES` is 4,194,304 flattened edges. The CAD corpus file
`generated/cad_t2` (10.6 MB) now reaches that bound at byte 9,365,917 and is
refused. Raising the constant again buys a larger constant, not a fix: a viewer
for large documents should not need every edge of every element resident in one
array. The fix is per-element edge storage or a streaming traversal, and it is a
design change rather than a budget change. No work order exists for this yet; `work/WO-M4-10.md` covers only the 16 MiB source cap.

The real-drawing budgets that were previously far too small are now documented
and finite: edges 4M, per-path commands 4M, elements 65,536, document text bytes
64 MiB (64 KiB per token), glyphs 1M document / 65,536 per run, transform
operations 1M. Each was raised only after a symbol-resolving probe identified the
exact site that was refusing ordinary drawings.

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
