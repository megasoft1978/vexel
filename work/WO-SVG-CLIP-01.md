---
id: WO-SVG-CLIP-01
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 51d98ff4a992c6285a6acbdfe1be5200e5812ca7aa7b899637d9e3254e71dc32
parallel_safe: false
network: false
---
# Goal
Treat a local `<clipPath>` as a non-rendering definition wherever it appears in the SVG tree, including inside a rendered group, while keeping it resolvable from `clip-path="url(#id)"` at its use site.

# Context
- Follow `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018, and the active Vexel product goal.
- `artifacts/WO-CORPUS-01/residual-causes.md` records two real Natural Earth documents refused as unsupported at the `<clipPath>` nested directly inside a group. Their clip reference is on the surrounding group.
- `svg_reference_definition_end` currently skips `<defs>`, `<symbol>`, and `<marker>` during rendered traversal, but not a standalone `<clipPath>`. The clip resolver already looks up local clip IDs independently, so the definition must remain indexed while traversal skips painting its contents.
- Use a small synthetic SVG regression in the repository. Do not read, copy, modify, or render the external corpus as part of this slice.

# Files allowed to touch
- `work/WO-SVG-CLIP-01.md`
- `src/svg/reference/reference.c`
- `src/svg/shape/README.md`
- `tests/svg/shape/clip_definition_scope_test.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-SVG-CLIP-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all pre-existing changes. Do not change clip geometry semantics, CSS inheritance, resource limits, source preservation, other SVG traversal behavior, raster budgets, UI, MCP, or project formats.
- Do not modify corpus files, fixtures, goldens, oracle outputs, or external corpus paths. Do not add dependencies, GPL editor source, or non-macOS code.
- Do not run broad UI/E2E, corpus, fuzz, or performance suites in this slice.

# Required implementation
1. Extend the existing reference-definition traversal skip to include unprefixed SVG `<clipPath>` elements. Keep ID/reference indexing over the original token stream unchanged so group and shape clip uses still resolve.
2. Add a synthetic test with a `<clipPath>` nested inside a rendered `<g>` and referenced by that group. Confirm decode and rendering succeed, the clip interior is painted, the exterior remains transparent, only the consuming drawable contributes to the main scene, and borrowed SVG source bytes remain exact.
3. Include a malformed or missing-reference case to confirm the existing diagnostic behavior is unchanged.
4. Document that local clip definitions are non-rendering even when nested outside `<defs>`; do not claim broad corpus coverage from this fixture.

# Verification
- Build `svgworkbench`, `shape_test`, and `clip_definition_scope_test` in Debug, ASan/UBSan, and UBSan configurations.
- Run only `shape-render`, `svg-nested-clip-definition`, and `svg-animation` CTests in each configuration; confirm those names are registered first.
- Validate this frozen order and ledger, parse `tests/test_map.json`, verify focused test selection, run `git diff --check`, and report changed source sizes. Keep changed source files below 800 lines.
- Do not run the comprehensive UI/E2E suite or any real-corpus sweep; the partial corpus report remains unchanged and unaccepted.

# Output
Leave changes uncommitted. Record exact commands/results in `artifacts/WO-SVG-CLIP-01/verification.txt`; update `STATE.md` and both ledgers. Keep the full Vexel product goal active.
