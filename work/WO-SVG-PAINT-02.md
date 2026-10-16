---
id: WO-SVG-PAINT-02
revision: 2
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 66bb819723088a873f2939f39830df5a0dd829a46652253f2793f48e09bd20e6
parallel_safe: false
network: false
---
# Goal
Composite antialiased coverage per SVG shape before source-over onto the accumulated raster. Preserve SVG paint order inside each shape and match reference behavior for overlapping translucent/gradient strokes.

# Context
- WO-CORPUS-12/13 show that the generated Inkscape T1 mismatch is dominated by a group of 34,714 small gradient-stroked paths.
- The isolated single-path and non-overlapping Inkscape-style probes match within channel tolerance 2. Repeated identical paths expose the mismatch: Vexel preserves a roughly half-covered pixel, while Inkscape source-over accumulates the shape coverage to opaque.
- `shape_render.c` currently composites samples from successive shapes into the same subpixel buffer and resolves the pixel only after the full shape stack. This loses per-shape antialias coverage accumulation at overlapping edges.
- Follow OWNER-DIRECTIVES.md, AGENTS.md, `docs/CODING_STANDARD.md`, ADR-0018, and the current product goal. Preserve all unrelated working-tree changes.

# Files allowed to touch
- `work/WO-SVG-PAINT-02.md`
- `src/raster/shape_render.c`
- `tests/svg/shape/gradient_stroke_test.c`
- `docs/research-r4/05-revised-roadmap.md`
- `artifacts/WO-SVG-PAINT-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Do not edit unrelated SVG parsing, paint-coordinate, path geometry, render budgets, UI, MCP, project format, corpus sources, goldens, or prior frozen orders/evidence.
- Do not read/fetch/copy GPL editor source; installed Inkscape may only be used as a binary rendering oracle.
- Do not modify files outside the allowlist. Leave changes uncommitted.
- No broad UI/E2E, corpus sweep, fuzz, performance suite, or release gate.

# Required implementation
1. Add a focused regression with two coincident antialiased gradient-stroked paths and assert that two copies produce the Inkscape-observed roughly 75% alpha and that a repeated stack approaches opaque, matching separate per-shape source-over coverage. Retain the existing one-path, gradient, opacity, and diagnostic assertions.
2. Resolve each shape's subpixel samples to a pixel source, then source-over that pixel onto the accumulated destination. Preserve fill/stroke paint order within a shape, clips, center-only sampling, and transparent-pixel behavior.
3. Use bounded existing row buffers; do not add an unbounded allocation. Keep modified source and test files under 800 lines.
4. Compare the synthetic regression with installed Inkscape at native output dimensions. Re-render the manifested generated T1 input and compare at tolerance 2; record exact deltas without changing thresholds or claiming corpus-wide parity.
5. Update renderer documentation only if the current contract needs clarification. Record limitations and work evidence in the allowed paths.

# Verification
- Validate this frozen work order and `ledger.jsonl`; parse `tests/test_map.json` and confirm the focused selector.
- Build `shape_render_cli`, `gradient_stroke_test`, and `shape_test` in Debug, ASan/UBSan, and configured UBSan directories; run only `svg-gradient-stroke` and `shape-render` in those configurations.
- Compare the repeated-path fixture and exact T1 candidate against installed Inkscape using transparent output composited over white and `scripts/pixdiff.py --tolerance 2`.
- Record builds, focused checks, reference metrics, changed-path scope, and line counts. Run `git diff --check`.
- Do not run comprehensive UI/E2E or other broad suites.

# Output
Write `artifacts/WO-SVG-PAINT-02/verification.txt`, update `STATE.md` and both ledgers, leave changes uncommitted, and keep the full product goal active.
