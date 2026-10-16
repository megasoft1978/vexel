---
id: WO-SVG-CLIP-02
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: b12ec66275fafc3d3a9d4a6d5ed4d4b351fba649e0778597fc118f0c39a1e454
parallel_safe: false
network: false
---
# Goal
Honor the inherited SVG `clip-rule` property for geometry inside a referenced local `<clipPath>`, including presentation attributes and the supported CSS cascade, without altering authored SVG bytes.

# Context
- Follow OWNER-DIRECTIVES.md, docs/CODING_STANDARD.md, ADR-0018, and the active product goal.
- `src/svg/shape/ignored.c` currently treats `clip-rule` as ignored. `src/svg/clip/clip.c` rejects every computed CSS property on clip children, and clip point tests use `shape_evenodd` produced from `fill-rule` only.
- The pinned diagnostics include real clipPath documents, but this order must use only synthetic SVG input and must not claim any corpus coverage.
- W3C SVG 1.1 section 14.5 defines `clip-rule` as inherited, initial `nonzero`, applicable to graphics elements inside a clipPath, and gives an `evenodd` example: https://www.w3.org/TR/SVG11/masking.html . This is the normative behavior/provenance for the implementation.

# Files allowed to touch
- work/WO-SVG-CLIP-02.md
- src/svg/css/css.h, src/svg/css/css.c, src/svg/css/README.md
- src/svg/clip/clip.c
- src/svg/shape/ignored.c, src/svg/shape/README.md
- tests/svg/shape/clip_definition_scope_test.c, tests/svg/css/test_css.c
- CMakeLists.txt, tests/test_map.json
- docs/product/vexel-goal-and-delivery-plan.md
- artifacts/WO-SVG-CLIP-02/**, STATE.md, LEDGER.md, ledger.jsonl

# Protected paths
- Preserve all existing uncommitted work. Do not edit SVG inputs, corpus reports, goldens/oracles, native project/MCP/UI implementations, frozen prior work orders or evidence, platform support, or paths outside the allowlist.
- Keep unrelated clip styling unsupported and diagnosed. Do not broaden to fill/stroke/filter/visibility effects or nested clip-path composition.
- No comprehensive UI/E2E, corpus, fuzz, or performance suite in this slice.

# Required implementation
1. Add `clip-rule` to the bounded CSS cascade and presentation-attribute mapping; accept only `nonzero` and `evenodd` where clipping geometry is resolved. Preserve inherited values through nested clipPath containers, with initial `nonzero`.
2. In the clip resolver, allow the computed clip-rule property while retaining fail-closed handling of every other unsupported computed property. Resolve each decoded clip shape to its source element and apply the computed clip-rule to clip containment, without rewriting or changing the original source bytes. Keep geometry-depth and temporary-storage use explicitly bounded.
3. Support clip-rule from presentation attributes, inline `style`, and supported stylesheet selectors. A clip-rule on the referring drawable outside the clipPath must not affect the referenced geometry.
4. Add synthetic render tests for nonzero default, evenodd on a clip geometry element, inherited evenodd from a nested clip container, stylesheet/inline-style resolution, referring-element non-inheritance, invalid-value diagnostics, and byte-exact source retention. Add CSS property coverage. Do not touch external corpus inputs.
5. Document the narrow supported rule and remaining clip limitations; update the product plan status without claiming corpus improvement.
6. Keep changed C sources under 800 lines and source behavior/resource limits bounded.

# Verification
- Build `svgworkbench`, `shape_test`, `clip_definition_scope_test`, and `css_test` in Debug, ASan/UBSan, and UBSan configurations.
- Run only `shape-render`, `svg-nested-clip-definition`, `svg-animation`, and the focused CSS CTest in each configuration; confirm exact registered test names first.
- Validate this frozen order and ledger, parse tests/test_map.json, verify focused selectors, run git diff --check, and report changed source sizes.
- Do not run broad UI/E2E, corpus, fuzz, or performance suites.

# Output
Leave changes uncommitted. Record exact focused results and limitations in artifacts/WO-SVG-CLIP-02/verification.txt; update STATE.md and both ledgers. Keep the full product goal active.
