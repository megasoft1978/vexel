---
id: WO-CORPUS-05
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 1f153a43d0553938c945cd6f1d13ba1b133755e99df9239df23996399211f162
---
# Goal
Measure how the accepted gradient-stroke implementation changes the next
full-document failure for the two pinned generated Inkscape-style SVG files.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`,
  ADR-0018, and the active Vexel product plan.
- Accepted `WO-SVG-PAINT-01` adds supported local linear and radial gradient
  strokes, but did not run a corpus probe. The prior full-render diagnostics
  report `generated-inkscape_t1` and `_t2` unsupported at byte 1214 inside
  `stroke="url(#g)"`.
- The unaccepted `WO-CORPUS-04` candidate changes the work budget and records
  5 of 39 probed files rendering, with no regressions against 3 of 39. Nineteen
  of 58 manifest files exceed the 32 MiB sweep cap and remain untested.
- Corpus inputs are external and read-only. They must not be copied into this
  repository.

# Existing contracts and invariants
- Test the current worktree, which contains approved but uncommitted product
  work. Make no source-code changes in this order.
- Build with the repository's current source. Render only the two exact
  manifested files named above at 128x128.
- This is a targeted diagnostic, not a corpus-wide acceptance or fidelity claim.

# Files allowed to touch
- `work/WO-CORPUS-05.md`
- `artifacts/WO-CORPUS-05/**`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all pre-existing worktree changes. Do not modify source, test,
  corpus, oracle, golden, prior work-order, or prior evidence files.
- Do not run a complete corpus sweep, UI/E2E suite, fuzz, broad benchmark, or
  release acceptance. No source changes, dependencies, platform changes, or
  network access.

# Required implementation
1. Verify the two input paths and pinned manifest identities read-only.
2. Build `svgworkbench` in Release from the current tree.
3. Run exactly two full-document `--render-frame` commands for generated
   Inkscape-style T1 and T2 at 128x128. Record exact exit status, diagnostic,
   wall time, max RSS, and resulting next blocker (if any).
4. Compare those outcomes to the prior `stroke="url(#g)"` failure and record
   only per-file observations. Do not report a corpus pass rate from two files.
5. Update the product plan with the evidenced next SVG slice, explicitly
   retaining the 3/39 accepted baseline, the 5/39 unaccepted candidate, and
   the 19-file skipped set as separate evidence.

# Verification
- Confirm the paths and hashes against `bench/corpus/manifest.json` and the
  external corpus manifest without modifying either.
- Build only `svgworkbench` with `cmake --preset mac-release` and
  `cmake --build --preset mac-release --target svgworkbench`.
- Run only the two target `--render-frame` probes under `/usr/bin/time -l`.
- Validate this frozen order, ledger, file scope, and `git diff --check`.
- Do not run any UI/E2E or full-corpus suite.

# Output
Leave changes uncommitted. Record exact evidence in
`artifacts/WO-CORPUS-05/verification.txt`, update `STATE.md` and `LEDGER.md`,
and keep the full product objective active.
