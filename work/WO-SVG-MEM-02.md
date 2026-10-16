---
id: WO-SVG-MEM-02
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: e52465c6f71c930eb77d18653ca5e5fd8cb3de8a8c794ee177d112a2ffee8804
parallel_safe: false
network: false
---
# Goal
Replace the dense per-shape SVG stroke-paint allocation with a bounded sparse
index map and compact records, reducing memory for gradient and pattern strokes
without changing rendered behavior or any resource ceiling.

# Context
- Follow `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018, and the
  active Vexel product goal.
- `WO-SVG-MEM-01` already stores non-default fill paints sparsely. A first
  non-solid stroke currently reserves one full `svg_shape_paint` for every
  shape-capacity slot; its record is 272 bytes, compared with a 4-byte index.
  At 65,536 shapes this reserves about 17 MiB for even a single gradient or
  pattern stroke.
- The synthetic Inkscape T2 input contains 356,821 paths and exercises inherited
  shared gradient strokes. Sparse stroke storage reduces allocation but does
  not itself raise the shape ceiling or prove that T2 decodes.
- Keep the shape ceiling and all other parser/render limits unchanged. Preserve
  exact SVG source bytes and existing solid, pattern, linear-gradient, and
  radial-gradient stroke behavior.

# Files allowed to touch
- `work/WO-SVG-MEM-02.md`
- `src/svg/shape/shape.h`, `shape_internal.h`, `shape_geometry.c`,
  `shape_custom.c`, `pattern.c`, `document.c`, `README.md`
- `src/raster/shape_render_sample.c`, `shape_render_preflight.c`,
  `shape_render.c`, `shape_render_group.c`, `shape_render_animation.c`
- `tests/svg/shape/shape_render_reference.c`,
  `tests/svg/shape/gradient_stroke_test.c`
- `artifacts/WO-SVG-MEM-02/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all existing worktree changes. Do not alter `SVG_SHAPE_MAX_SHAPES`,
  any other resource ceiling, source preservation, paint-server semantics,
  corpus/oracle/golden fixtures, file formats, MCP, UI, or platform support.
- Do not add a dependency. Do not run the broad UI/E2E, corpus, fuzz, or
  performance suites in this slice. Do not claim the Inkscape T2 sample now
  decodes or that RSS/performance targets have been measured.

# Required implementation
1. Give each shape a bounded 32-bit stroke-paint index, where zero means the
   ordinary solid stroke; append paint records only for non-default stroke
   semantics. Add centralized const/mutable accessors and checked allocation.
2. Update linear/radial gradient and pattern stroke resolution to populate the
   sparse record table. Preserve stroke-opacity behavior and shared stop storage.
3. Update raster sampling, preflight, scalar-reference rendering, grouped views,
   and animation render copies to resolve the sparse indices correctly.
4. Add focused assertions showing solid strokes allocate no stroke-paint
   records and gradient strokes allocate only the required record. Preserve
   pattern coverage in the existing shape-render suite.
5. Document the memory representation and limitations. Keep shape-limit
   expansion and large-document measurement as later separately scoped work.

# Verification
- Build `svgworkbench`, `shape_test`, and `gradient_stroke_test` in Debug,
  ASan/UBSan, and UBSan configurations.
- Run only `shape-render`, `svg-gradient-stroke`, and `svg-animation` CTests in
  each configuration, confirming registered names before execution.
- Validate this frozen order and ledger, parse `tests/test_map.json`, verify
  focused test selection, run `git diff --check`, and report changed source
  sizes.

# Output
Leave changes uncommitted. Record exact commands/results in
`artifacts/WO-SVG-MEM-02/verification.txt`; update `STATE.md` and both ledgers.
Keep the full product goal active and the 65,536-shape limit unchanged.
