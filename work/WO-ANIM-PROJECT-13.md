---
id: WO-ANIM-PROJECT-13
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: a2bec642757e746edcc1c9d8476b1f78732428e1d5940cf88f8ef1a39e9daf9a
parallel_safe: false
network: false
---
# Goal
Evaluate native-v1 property tracks in deterministic integer-tick project frame rendering.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, ADR-0020, timeline decode/view APIs, document edit APIs, project animation renderer and focused renderer tests.
- Property metadata and key values are already schema-validated; renderer currently ignores them.

# Contracts
- macOS Apple Silicon only, C17, no dependencies, bounded allocation, borrowed input bytes stay immutable.
- Implement scalar, RGBA, transform2d and boolean values; step, linear and smoothstep cubic per ADR-0020. Clamp before/after key ranges to endpoint values.
- Apply properties only to temporary scene bytes and render those bytes. Preserve existing cel ordering/composition and renderer semantics when no property tracks exist.
- Keep source files under 800 lines. No broad UI/E2E tests and no commit.

# Files allowed to touch
- `src/project/project_animation_render.h`, `src/project/project_animation_render.c`
- `src/project/README.md`
- `tests/project/project_animation_render_test.c`
- `work/WO-ANIM-PROJECT-13.md`, `artifacts/WO-ANIM-PROJECT-13/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Timeline schema/codec/editing, MCP protocol/tools, SVG/XML/raster internals, UI/E2E suites, fixtures/oracles, release files, scripts and all paths outside the allowlist.

# Required behavior
1. Read property/property type/interpolation metadata and ordered key values from validated timeline spans.
2. Evaluate step, component-wise linear, and smoothstep cubic interpolation at requested integer tick, holding endpoint values outside key range.
3. Map supported properties to SVG attributes: scalar property name, fill/stroke RGBA as valid deterministic color plus opacity, transform2d as matrix, and visibility as visible/hidden.
4. Apply grouped values to target IDs through a bounded temporary document edit; decode and render the modified snapshot, then continue drawing-track composition.
5. Return deterministic renderer errors for malformed view/application failure and leave caller input/source bytes unchanged.
6. Add focused renderer coverage for interpolation, endpoint holds, type mappings and unchanged source bytes.

# Acceptance and verification
- Build `project_animation_render_test` and required app target with strict warnings.
- Run focused project animation render tests in Debug, ASan, and configured UBSan where available; do not run broad UI/E2E suites.
- Validate frozen work order, ledger, changed-path allowlist, source line counts, and `git diff --check`.
- Record exact outcomes in `artifacts/WO-ANIM-PROJECT-13/verification.txt`.

# Output
Leave changes uncommitted. Report supported deterministic property evaluation and remaining UI property authoring, export, and broad UI/MCP acceptance work.
