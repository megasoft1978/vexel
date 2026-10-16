---
id: WO-ANIM-PROJECT-09
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: b9e4467bc1e06a95f564e4d6f6ad063426c7c6980de9f2d77a2ebbd2cfddfb1a
parallel_safe: false
network: false
---
# Goal
Add a bounded deterministic native-v1 frame renderer that selects drawing cel exposures at an integer tick, renders the project scene plus active drawing assets, and composes their RGBA output without changing source/project bytes.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0018/0019/0020, and project timeline, drawing-asset, session, shape/raster APIs.
- Project load/save and live MCP timeline editing are implemented. The existing renderer handles SVG source/SMIL but does not use native project timeline cels.
- Native drawing assets are complete SVG documents with independent ID scopes. V1 currently has no target-relative placement/transform metadata for a cel asset; this order renders assets in their SVG root coordinate space, alpha-composited in timeline track order over the project scene.
- No broad UI/E2E tests, no network, no commit.

# Existing contracts
- macOS Apple Silicon only, C17, no new dependencies, explicit byte ownership and bounded allocations.
- Integer tick addressing; tick must satisfy `tick < duration_ticks`.
- A drawing exposure remains active from its start tick through (but not including) the next exposure, with tick zero required by v1.
- `project_timeline_decode` validates against a caller-owned context; asset slices borrow from the project bundle.
- Rendering is deterministic; source/project bytes are immutable. Reject dimension mismatches rather than silently scale assets in this first compositor.

# Required implementation
1. Add a project animation render API that accepts a decoded scene, project bundle, validated timeline context/view, and integer tick and returns an owned raster tile or structured error.
2. Render the base scene and each active drawing cel in track order using the owned SVG decoder/rasterizer. Require asset viewport/output dimensions to match the base scene; reject unsupported mismatch with a typed error.
3. Alpha-composite premultiplied RGBA with checked dimensions/byte arithmetic and finite bounded work. Do not mutate decoded documents, source bytes, bundle assets, or timeline data.
4. Handle zero drawing tracks, missing exposures, invalid asset references, unsupported media, decode/render failures, allocation failure, and output pixel limits without partial output ownership leaks.
5. Add focused project animation renderer tests: known tick-zero and held-cel frame pixels, exposure transition, multiple-track order/compositing, no-active-drawing base output, rejected size mismatch, invalid tick, and scene/timeline/source immutability.
6. Document ownership, pixel format, track ordering, mismatch policy, and current placement limitation.

# Files allowed to touch
- `src/project/project_animation_render.h`, `src/project/project_animation_render.c`
- `tests/project/project_animation_render_test.c`
- `CMakeLists.txt` project-format lane only
- `tests/test_map.json`
- `src/project/README.md`
- this work order, `artifacts/WO-ANIM-PROJECT-09/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
Viewer/timeline UI and playback controls, export and onion-skin features, project schema/container, document edit internals, SVG/XML parser/rasterizer, app integration, release files, scripts, presets, and paths outside allowlist. No broad UI/E2E suite.

# Acceptance criteria
- AC-1: strict Debug project renderer and app targets build.
- AC-2: focused native project renderer and existing timeline/load/save tests pass.
- AC-3: focused renderer and project tests pass under ASan/UBSan.
- AC-4: exposure selection/hold and compositing are deterministic at integer ticks; invalid inputs fail without modifying source/timeline and without leaking partial outputs.
- AC-5: no blocking/high findings in final independent read-only review.
- AC-6: frozen order, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build `svgworkbench`, `project_animation_render_test`, and focused project timeline/load/save targets in Debug.
- Run `project-animation-render` and `project-(load|save|timeline|timeline-integration|timeline-edit|timeline-session)` CTests.
- Run those focused CTests under ASan/UBSan.
- Run independent read-only review of allowed implementation paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-09/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not silently stretch, crop, or alter an asset to fit the scene. Return a structured dimension or rendering error. Keep intermediate renders bounded by the existing raster and project limits.

# Output
Leave changes uncommitted. Report renderer API and verification, and state that app playback/UI integration, target-relative drawing placement, onion skin, and export remain subsequent work.
