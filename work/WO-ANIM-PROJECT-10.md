---
id: WO-ANIM-PROJECT-10
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: f84c058a4830e269495b1779a646c0ff9e8708e3c423ec6bd85cf3c6fc5a4cc1
parallel_safe: false
network: false
---
# Goal
Add a safe session-level read/render seam for the active native-v1 animation timeline. Callers must be able to render the current edited session at an integer tick without retaining a stale decoded view across edits.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0018/0019/0020, project timeline/session/render APIs, and their focused tests.
- WO-ANIM-PROJECT-09 added deterministic base-scene plus ordered drawing-cel composition. WO-ANIM-PROJECT-08 added live session edits through MCP. The render API still requires a caller-supplied decoded view and currently the session does not expose a safe way to obtain one.
- Existing viewer timeline UI is SMIL-specific in painting, hit-testing, editing, and accessibility. This order creates a robust model seam only; it must not mix native tracks into the SMIL timeline.
- No broad UI/E2E tests, no network, no commit.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, explicit ownership and checked bounds.
- Session owns editable timeline source and validated context; bundle and scene data are borrowed for a call.
- Decoded `project_timeline_view` borrows the current session source and becomes invalid after edit/undo/redo.
- Render output is an owned `raster_tile` on success; failures must not leak partial output.
- Native renderer uses integer ticks and currently places drawing assets in their SVG root coordinate space; no target-relative transform semantics are defined.

# Required implementation
1. Add a session decode API that validates/decodes the session's current bytes against its owned current context and returns an immutable view whose lifetime is explicitly bounded by the next mutation or session destruction.
2. Add a session render-frame API that decodes the current timeline internally and renders against a caller-supplied scene and current bundle, so callers need not hold or cache a decoded view across calls. Reject invalid or stale project context safely; do not mutate session/project bytes.
3. Ensure post-edit track/cel references and current bundle asset references are validated before render; if required, refresh context transactionally using current scene SVG while preserving the previous context when refresh fails.
4. Add focused project-session tests for decode metadata, edit then render, cel exposure transition, undo/redo render state, invalid tick/reference behavior, and unchanged timeline bytes on render.
5. Document API ownership, invalidation, validation, and placement limits.

# Files allowed to touch
- `src/project/project_timeline_session.h/.c`
- `tests/project/project_timeline_session_test.c`
- `src/project/README.md`
- `CMakeLists.txt` project-format lane only
- `tests/test_map.json`
- this work order, `artifacts/WO-ANIM-PROJECT-10/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
Viewer/timeline UI and playback controls, MCP protocol/tools, export and onion-skin features, project schema/container, document edit internals, SVG/XML parser/rasterizer, release files, scripts, presets, and paths outside allowlist. No broad UI/E2E suite.

# Acceptance criteria
- AC-1: strict Debug application and project session/render targets build.
- AC-2: focused project session, renderer, timeline/load/save tests pass.
- AC-3: the same focused project tests pass under ASan/UBSan.
- AC-4: session decode reflects edits and render reflects current held cels, undo, and redo; invalid requests fail without source mutation or partial output ownership.
- AC-5: independent read-only review has no blocking/high findings.
- AC-6: frozen order, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build `svgworkbench` and `project_timeline_session_test` in Debug.
- Run `project-timeline-session`, `project-animation-render`, and focused project timeline/load/save CTests.
- Run the same focused project tests under ASan/UBSan.
- Run independent read-only review limited to allowed implementation and test paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-10/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not return views that remain apparently valid after mutation. Do not silently use stale target/asset inventories; refresh transactionally or return a structured failure. Keep outputs empty on failure.

# Output
Leave changes uncommitted. Report the session render seam and verification, and state that visible native timeline UI/playback, target-relative placement, export, and onion skin remain subsequent work.
