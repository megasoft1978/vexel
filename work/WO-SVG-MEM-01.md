---
id: WO-SVG-MEM-01
revision: 2
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 4dd8ec0c20a0c101fb9b68b192ef75e057fb5d3e882d0bc4ae49908959b0bfe0
parallel_safe: false
network: false
---
# Goal
Stop allocating a full `svg_shape_paint` record for every shape whose fill uses implicit solid paint. Store only non-default fill paint records, indexed by a bounded per-shape map, to reduce memory use in large SVG documents without changing the shape ceiling.

# Context
- Follow `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018, and the active Vexel product goal.
- The synthetic Inkscape T2 document contains 356,821 paths and currently reaches `SVG_SHAPE_MAX_SHAPES` after the stroke-gradient fix.
- A local `sizeof` probe measured `svg_shape_paint` at 272 bytes. Fill and stroke paint arrays plus other shape metadata dominate memory at high shape counts; raising the 65,536-shape ceiling without storage changes would allocate hundreds of MiB before geometry and XML storage.
- This order addresses the unconditional fill paint array only. Stroke paint, stroke dash, image, and other metadata storage remain follow-up optimization targets. Do not raise the shape ceiling or claim the Inkscape sample now decodes.

# Files allowed to touch
- `work/WO-SVG-MEM-01.md`
- `src/svg/shape/shape.h`, `shape_internal.h`, `shape_geometry.c`, `shape_custom.c`, `pattern.c`, `image.c`, `document.c`, `selection.c`, `README.md`
- `src/raster/shape_render_fill.c`, `shape_render_sample.c`, `shape_render_preflight.c`, `shape_render.c`, `shape_render_group.c`, `shape_render_animation.c`
- `tests/svg/shape/test_shape.c`, `tests/svg/shape/shape_render_reference.c`, `tests/svg/shape/gradient_stroke_test.c`
- `artifacts/WO-SVG-MEM-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all existing worktree changes. Do not change `SVG_SHAPE_MAX_SHAPES`, other resource ceilings, parser semantics, source preservation, paint server behavior, file formats, MCP, UI, platform support, or corpus/golden/oracle fixtures.
- No new dependency and no broad UI/E2E, corpus, fuzz, or performance suite in this slice.

# Required implementation
1. Represent absence of a non-solid fill paint with a zero per-shape paint index; append bounded paint records only for gradient, pattern, image, or other non-default fill semantics.
2. Add centralized const/mutable accessors and update decode, selection, sampling, preflight, rendering, grouped views, and animation render copies so sparse fill paint indices resolve correctly.
3. Preserve pattern/image behavior and object-bounding-box gradient transforms. Keep stroke paint storage unchanged.
4. Add or adapt focused assertions for default-solid fills and non-solid fill paint behavior. Existing shape-render coverage remains authoritative for gradients, patterns, images, groups, and animation.
5. Document the representation and current shape-limit status without claiming broader corpus fidelity.

# Verification
- Build `svgworkbench`, `shape_test`, and `gradient_stroke_test` in Debug, ASan/UBSan, and UBSan.
- Run only `shape-render`, `svg-gradient-stroke`, and `svg-animation` CTests in each configuration (omit names not registered only if confirmed absent).
- Validate this frozen order and ledger, parse `tests/test_map.json`, verify focused test selection, run `git diff --check`, and report changed source sizes.

# Output
Leave changes uncommitted. Record exact verification in `artifacts/WO-SVG-MEM-01/verification.txt`; update `STATE.md` and both ledgers. Keep the full product goal active and leave shape-limit expansion to a separately measured work order.
