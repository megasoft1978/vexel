---
id: WO-SVG-PAINT-01
revision: 3
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 72b4fd5fa250b526c9c6cfba162ede40a37dcdaed0c5dc0880dfdafd64b4162c
parallel_safe: false
network: false
---
# Goal
Render supported local linear and radial gradient paint servers on SVG strokes, using the existing bounded gradient resolver and raster sampler. Preserve current solid and pattern stroke pixels and reject unresolved or unsupported stroke paint servers with diagnostics.

# Context
- Follow `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018, and the active Vexel product goal.
- The pinned synthetic corpus manifest identifies `generated-inkscape_t1` and `_t2` as synthetic fixtures with `inkscape-attributes`, `lpe-metadata`, gradients, filters, and markers. The current partial sweep reports both unsupported at byte 1214, inside `stroke="url(#g)"`.
- `svg_shape_parse_stroke` recognizes paint-server syntax. Shape geometry currently tries pattern resolution for stroke paint and then fails closed. `svg_paint_resolve_fill` already resolves bounded linear/radial gradient definitions and stop arrays for fills; raster sampling already evaluates them.
- Preserve the existing source bytes. Do not copy corpus bytes into tests or repository artifacts.

# Files allowed to touch
- `work/WO-SVG-PAINT-01.md`
- `src/svg/shape/shape_geometry.c`, `src/svg/shape/shape_custom.c`, `src/svg/shape/shape_internal.h`, `src/svg/shape/document.c`, `src/svg/shape/README.md`
- `src/raster/shape_render_sample.c`, `src/raster/shape_render_preflight.c`, `src/raster/shape_render.c`
- `tests/svg/shape/gradient_stroke_test.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-SVG-PAINT-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all existing worktree changes. Do not edit corpus files, oracle/golden/reference fixtures, other parser modules, UI, MCP, file formats, or renderer budget ceilings.
- No GPL editor source, no new dependency, no external corpus bytes copied into this repository, and no non-macOS platform code.

# Required implementation
1. Reuse the existing local gradient resolver for stroke `url(#id)` after pattern resolution declines it. Populate the existing per-shape stroke paint storage and shared bounded stop storage.
2. Sample linear and radial gradient strokes at the same document-space pixel center as fill gradients, with stroke opacity and source-over behavior correct. Preserve pattern sampling behavior.
3. Account for gradient stop work in raster preflight and retain correct paint storage across document destroy, grouped rendering, and animation render copies.
4. Add focused tests for linear and radial gradient stroke pixels, opacity/compositing, source-byte preservation, and unresolved/unsupported reference diagnostics. Keep solid/pattern stroke behavior covered by the existing shape-render suite.
5. Document supported semantics and limitations. Do not claim corpus-wide fidelity from focused fixtures.

# Verification
- Build `svgworkbench`, `gradient_stroke_test`, and `shape_test` in Debug, ASan/UBSan, and UBSan presets.
- Run only `svg-gradient-stroke` and `shape-render` in each configuration. Do not run broad UI/E2E, corpus sweep, fuzz, or performance suites in this slice.
- Validate this frozen order and ledger, parse `tests/test_map.json`, verify the focused quick-test selector, run `git diff --check`, and report changed source sizes.

# Output
Leave changes uncommitted. Record exact results in `artifacts/WO-SVG-PAINT-01/verification.txt`, update `STATE.md` and both ledgers, and keep the larger fidelity/performance/animation/MCP goal active.
