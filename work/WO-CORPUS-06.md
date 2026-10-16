---
id: WO-CORPUS-06
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 78688d06a807efb2d824c59191d62966991ff6e7f280f9ab23a856cf64441a76
---
# Goal
Identify the exact raster work-limit call site and work totals for the two
manifested generated Inkscape-style SVGs in the current worktree snapshot.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`,
  ADR-0018, and the active Vexel product plan.
- `WO-CORPUS-05` rebuilt current Release and observed `limit at byte 0` for
  `generated-inkscape_t1` and `limit at byte 1848989` for `_t2`. Earlier reports
  showed `unsupported at byte 1214` inside a gradient stroke. The current tree
  includes the accepted gradient-stroke implementation and an unaccepted
  raster-work candidate capped at 16,000,000,000 units.
- Do not infer that increasing the cap is safe. Find the exact call site,
  requested work amount and measured required total first.

# Existing contracts and invariants
- Preserve the shared user worktree byte-for-byte. Perform all copying,
  instrumentation, building and probing in ignored `.cache/WO-CORPUS-06/`.
- Snapshot the current worktree sources, including uncommitted and untracked
  source files, excluding `.git/`, existing `build/`, `.cache/`, and external
  corpus files. Verify hashes of the relevant copied files before instrumentation.
- Instrument only the scratch snapshot. The probe emits the preflight call
  source line, accumulated work, next requested amount and active budget when
  `work_add` refuses. Emit a distinct diagnostic if preflight returns false
  without a work-budget refusal.
- Inputs remain the exact manifest-verified T1/T2 files from WO-CORPUS-05.

# Files allowed to touch
- `work/WO-CORPUS-06.md`
- `artifacts/WO-CORPUS-06/**`
- `.cache/WO-CORPUS-06/**` (ignored scratch snapshot and build outputs only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all shared worktree changes. No instrumentation, build output, test
  artifact, or corpus copy may remain outside the ignored scratch directory.
- Do not modify shared source or prior work orders/evidence. No UI/E2E, full
  corpus sweep, fuzz, broad benchmark, dependency, platform, or network work.

# Required implementation
1. Create the scratch source snapshot and prove selected source hashes match
   the current worktree before instrumentation.
2. Instrument work-budget refusal with caller line, accumulated count, rejected
   amount, and active cap. If the refusal occurs outside that counter, identify
   the exact error branch instead.
3. Build only Release `svgworkbench` in the scratch snapshot.
4. Probe exactly the T1 and T2 inputs at 128x128. Record exit, diagnostic,
   source offset, wall time, RSS, instrumented refusal, and any measured full
   work total obtainable without exceeding a finite bound.
5. Make no product-code changes. Recommend a bounded follow-up that optimizes
   the measured hot operation or preserves the cap if required work is unsafe.

# Verification
- Confirm the external manifest hash and both input hashes against
  `artifacts/WO-CORPUS-05/verification.txt`.
- Verify the source snapshot hashes for `CMakeLists.txt`,
  `src/raster/shape_render_preflight.c`, `src/raster/shape_render.c`, and
  `src/svg/shape/shape_geometry.c` before instrumentation.
- Configure/build only the scratch Release `svgworkbench`; run exactly two
  instrumented `--render-frame` probes under `/usr/bin/time -l`.
- Verify the shared source files remain byte-identical to the recorded hashes,
  validate this order and ledger, and run `git diff --check`.
- No UI/E2E, corpus sweep, fuzz or broad performance suite.

# Output
Leave shared source changes untouched and uncommitted. Record exact evidence
in `artifacts/WO-CORPUS-06/verification.txt`; update `STATE.md` and `LEDGER.md`.
Keep the full product objective active.
