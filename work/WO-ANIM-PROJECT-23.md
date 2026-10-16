---
id: WO-ANIM-PROJECT-23
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ec91c18344098201911428adf454bc5f67738db44274c44c59e01ea4effe4a7a
parallel_safe: false
network: false
---
# Goal
Export the active native `.vexel` animation through MCP as a bounded GIF or PNG frame sequence.

# Context
- Follow OWNER-DIRECTIVES.md, ADR-0018, ADR-0019, and ADR-0020.
- `export_animation` currently renders SVG SMIL through `raster_shape_render_at`; it does not use the active native timeline session, property keys, drawing cels, or project assets.
- The project renderer and `project_timeline_session_render_frame` already compose evaluated native timeline frames. The broker host exposes current session and bundle callbacks for the active app-owned document.
- Preserve legacy SVG SMIL export behavior for non-project documents. Native export must reflect current unsaved scene/timeline/assets, not saved bundle bytes or onion-skin preview state.
- Preserve unrelated worktree changes. No broad UI/E2E or release suite; no commit.

# Contracts
- Auto-select native rendering only when the requested document is the active app-owned `.vexel` document and a native-v1 session/bundle are available. Keep the existing SVG SMIL route for ordinary SVG and opened non-active documents.
- Native frame sampling uses the native timeline's integer tick rate and duration. `fps` and `duration` remain output sampling requests; reject a requested duration beyond native timeline duration and reject any sample outside its tick range. No fractional tick or floating-time evaluation is passed to the native renderer.
- Decode the current editable scene and render current session data against the current bundle/asset overlay. Do not serialize preview onion ghosts or mutate document/session state.
- Keep GIF and PNG-sequence path grants, exclusive file creation, cleanup on failed sequence export, pixel/frame/source limits, and existing result structure.
- Keep C and `.inc` sources below 800 lines. Focused MCP/project tests only.

# Files allowed to touch
- `CMakeLists.txt`
- `src/mcp/mcp_broker_render.c`, `src/mcp/mcp_broker_animation.c`,
  `src/mcp/mcp_broker_internal.h`, `src/mcp/mcp_tools.c`, `src/mcp/README.md`
- `tests/mcp/mcp_native_timeline_test.c`, `tests/mcp/mcp_features_test.c`
- `work/WO-ANIM-PROJECT-23.md`, `artifacts/WO-ANIM-PROJECT-23/**`,
  `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Route native export through `project_timeline_session_render_frame` for the active app-owned `.vexel` document, materializing the current editable SVG and using current bundle assets and edited timeline bytes.
2. Sample deterministic native integer ticks from requested `fps` and `duration`; constrain duration to the project timeline and enforce existing max frame/pixel/output limits.
3. Preserve the existing PNG-sequence and GIF encoder/writer, granted-path policy, exclusive output semantics, partial-output cleanup, and response JSON.
4. Preserve SVG SMIL animation export for ordinary SVG documents.
5. Add native focused coverage proving exported frame pixels reflect a drawing cel and an edited property key, reject out-of-range duration/invalid native state, and retain SVG fallback coverage. Do not run broad UI/E2E.
6. Update MCP documentation with native `.vexel` versus SVG export behavior and limits.

# Verification
- Freeze this order before implementation and validate the recorded SHA-256.
- Strict Debug, ASan/UBSan, and configured UBSan builds of `mcp_native_timeline_test`, `mcp_features_test`, and `svgworkbench`; run only those two focused MCP binaries.
- Validate frozen order, ledger, allowed paths, C/`.inc` line caps, and `git diff --check`.

# Output
Leave changes uncommitted. Report native export behavior, SVG fallback, focused verification, and remaining export breadth, final UI/MCP acceptance, release gates, and recorded Codex/Vexel website/README showcase.
