---
id: WO-SVG-MEM-03
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 334e01ec47b5e54b6cb6aba64ce7df45bf2f822f85bece030d0194cea481b892
parallel_safe: false
network: false
---
# Goal
Remove the fixed 16-double dash array from every shape's stroke record. Keep
per-shape dash metadata compact and store values only for strokes that actually
specify a dash pattern, with the same parsing and rendering semantics.

# Context
- Follow `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018, and the
  active Vexel product goal.
- `svg_shape_stroke` currently reserves 16 doubles (128 bytes) for every shape,
  including undashed shapes. A maximum 65,536-shape document therefore reserves
  8 MiB of dash storage before any user dash values are considered.
- SVG memory work orders 01 and 02 made fill and stroke paint records sparse.
  Inkscape T2 uses a gradient stroke on many paths, so sparse stroke paints
  alone do not reduce records for that workload. Compact dash storage reduces
  per-shape metadata for documents that do not use dashes.
- Preserve the 16-value per-pattern ceiling, 65,536-shape ceiling, exact SVG
  source bytes, dash parsing, odd-pattern repetition, offsets, caps, joins,
  seam behavior, raster budgets, and all other resource limits.

# Files allowed to touch
- `work/WO-SVG-MEM-03.md`
- `src/svg/shape/shape.h`, `shape_internal.h`, `shape_geometry.c`,
  `shape_custom.c`, `shape_style.c`, `document.c`, `README.md`
- `src/raster/shape_render.c`, `shape_render_internal.h`,
  `shape_render_stroke.c`, `shape_render_dash_cache.c`
- `tests/svg/shape/stroke_dash_storage_test.c`,
  `tests/svg/shape/shape_render_reference.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-SVG-MEM-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all existing worktree changes. Do not change any shape/resource
  ceiling, dash parsing or rendering behavior, source preservation, paint-server
  behavior, corpus/oracle/golden fixtures, file formats, MCP, UI, or platform
  support.
- Do not add a dependency. Do not run broad UI/E2E, corpus, fuzz, or performance
  suites. Do not claim measured RSS or corpus-level speed/memory improvements.

# Required implementation
1. Replace inline dash values in each stroke with a bounded offset/count into a
   compact document-owned double array. Append parsed values with checked size
   and index arithmetic; default and undashed strokes store no values.
2. Add a checked accessor for a shape's dash slice. Use it in parsing,
   validation, seam detection, the raster dash-cache builder, direct stroke
   containment, the scalar reference, grouped document views, and animation
   render copies.
3. Split dash-cache construction/seam helpers from
   `shape_render_stroke.c` into `shape_render_dash_cache.c` so each source file
   remains below 800 lines; register the source in CMake.
4. Add focused tests for default empty storage, exact stored values and shape
   offsets, the 16-value maximum, invalid syntax diagnostics, and preservation
   of current render output through existing shape-render/dash fixtures.
5. Document the compact representation and remaining memory limitations. Do
   not change resource ceilings or claim Inkscape corpus completion.

# Verification
- Build `svgworkbench`, `shape_test`, and `stroke_dash_storage_test` in Debug,
  ASan/UBSan, and UBSan configurations.
- Run only `shape-render`, `svg-stroke-dash-storage`, and `svg-animation` CTests
  in each configuration; confirm those names are registered first.
- Validate this frozen order and ledger, parse `tests/test_map.json`, verify
  focused test selection, run `git diff --check`, and report changed source
  sizes. Keep all source files below 800 lines.

# Output
Leave changes uncommitted. Record exact commands/results in
`artifacts/WO-SVG-MEM-03/verification.txt`; update `STATE.md` and both ledgers.
Keep the full product goal active and all shape/resource ceilings unchanged.
