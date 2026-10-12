# ADR-0016: Product target, fidelity bar, and owner gates

- Status: accepted
- Date: 2026-10-12
- Decision owner: project owner
- Supersedes the scope framing of ADR-0012 where it conflicts

## Context

Vexel's original framing (round 3 product scope, carried by the round-4
roadmap) was a narrow "large-SVG workbench": open, inspect and lightly edit huge
SVGs, explicitly *not* a parity replacement, with animation deferred behind a
demand gate. The roadmap's kill criteria also assumed user interviews and real
user files.

The owner-requested deep review of 2026-10-12
(`reports/PROJECT-REVIEW-2026-10-12.md`) established that:

- the shipped tree has already moved past that framing, implementing an editor
  with layers, path-node editing, style controls, undo/redo, animation
  authoring, tabs, rulers/guides, a command palette, and a local MCP server;
- the headline corpus evidence does not measure what it appears to measure:
  `svg_index_make_preview_source` (`src/svg/index/index.c:419-440`) builds a
  synthetic one-element document, so "2 of 58 previewable" and the 41.6x/30.4x
  ratios describe a preview of an extracted element, not the file. A direct
  full-render sweep of the 39 pinned corpus files under 30 MB rendered 1;
- the dominant corpus blockers are fail-closed policy (unknown non-namespaced
  attributes are fatal; document-wide caps refuse whole files) rather than
  missing renderer features;
- the M0, M3, M4-scope, M6, M8, M10 and M13 owner gates are all still open, and
  M13 animation shipped without its required demand ADR.

## Decision

1. **Target.** Vexel is a macOS-first *alternative to Inkscape* with animation
   authoring and MCP integration, not a narrow viewer. Rendering fidelity is a
   product requirement: a document that Inkscape renders is expected to render
   in Vexel, or to report a specific, bounded, per-feature diagnostic. The
   "not a parity replacement" and "animation after the static editor" framing of
   the round-3 scope is retired.
2. **Fidelity bar.** The declared bar is the pinned 58-file real corpus
   (`manifest.json`, SHA-256
   `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`) and the
   librsvg differential fixtures. Coverage and fidelity claims must be derived
   from full-document renders, with per-file `status_code` and
   `diagnostic_category` recorded. Synthetic single-element previews may not be
   presented as file coverage.
3. **Owner gates.** M0 and M3 proceed on synthetic and public-corpus evidence
   without recorded user interviews or user-supplied files. This is a deliberate
   acceptance of that risk, recorded here so it is not later mistaken for
   evidence that does not exist. M11 and M12 remain void per
   `OWNER-DIRECTIVES.md`.
4. **Animation.** Animation is in scope as a first-class feature. The M13
   demand/semantics ADR that the roadmap required is replaced by this decision;
   animation scope is bounded by the implementation's existing documented limits
   and by UI/fidelity work orders rather than by a demand gate.
5. **Corpus priority.** The immediate priority is making the real corpus render
   and recording why each file does or does not, before further feature breadth.
   Fail-closed attribute and cap behaviour is to be replaced by
   ignore-and-diagnose where the SVG specification makes the construct
   non-fatal.
6. **Governance.** Work that reached `main` without a frozen work order is to be
   retro-frozen as descriptive work orders so the ledger reflects what shipped;
   no code change is implied by that documentation pass. All new work uses
   frozen work orders as before.

## Consequences

- `README.md`, `STATE.md` and the roadmap's milestone table no longer describe
  the product accurately; they are to be reconciled with this decision.
- Passing a bounded set of "supported fixtures" is not sufficient evidence;
  corpus render counts from full renders become the primary fidelity metric.
- Retiring the demand gate means animation fidelity and authoring quality must
  be covered by ordinary work orders and owner review.
- Accepting M0/M3 without user evidence means the product-value risk stated in
  the round-3 kill criteria remains live and unmitigated by evidence; it is
  carried knowingly.
