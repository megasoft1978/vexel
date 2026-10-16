---
id: WO-ANIM-PROJECT-07
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d4a0f0293841a139f0258260656d1c08fe0ec9ccbd45590d91ce0dde5fbaa1b6
parallel_safe: false
network: false
---
# Goal
Keep native-v1 timeline edits attached to the loaded `.vexel` project lifetime and serialize the edited timeline on project save. This is the persistence seam required before UI and MCP callers can safely edit a live native animation.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0019/0020, `project_timeline_integration`, `project_timeline_edit`, viewer load/save ownership, and project save tests.
- WO-ANIM-PROJECT-06 adds a standalone timeline editor but it is not owned by the app document and project saves still use borrowed original timeline bytes.
- Current MCP animation tools edit SVG SMIL; this order only wires native project model lifetime and save persistence. Timeline UI/MCP commands follow in a separate order.
- No GPL editor source, no network, no broad UI/E2E suite, no commit.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, checked sizes, explicit ownership.
- The encoded project source owns its scene, manifest, timeline, and asset slices. The loaded document must destroy owned timeline state before releasing those slices.
- Known native-v1 timelines may be edited; opaque future timelines and empty legacy payloads remain byte-exact passthrough data.
- Context IDs are decoded from the current exact scene source, sorted, unique, and bounded. Assets borrow the current decoded project bundle.
- Save remains atomic and must validate the current scene/timeline/assets before publication.

# Required implementation
1. Extend timeline integration to return an owned, bounded validation-context inventory on successful native-v1 validation without duplicating the existing scene-ID traversal. Preserve the existing entry point behavior for current callers.
2. Add an owned `project_timeline_session` that owns the W06 editor and validation context, supports context refresh after scene/bundle changes, and exposes track/cel edits, undo/redo, and borrowed current timeline bytes. Failed context refresh must leave the previous context usable.
3. Initialize and destroy the session with `viewer_loaded_document` only for valid native-v1 projects. Do not create an editable session for opaque or empty timelines. Clean up on load failure/cancel and document replacement.
4. Make `.vexel` save use the session's current timeline bytes when present, while retaining exact passthrough behavior otherwise. Keep scene SVG, manifest, assets, and edit history behavior unchanged on failed save.
5. Add focused project/session and app save/load tests proving edit -> save -> decode round-trip, refresh/rejected refresh behavior, unknown/empty timeline passthrough, exact preservation of unrelated bundle fields, and lifecycle cleanup.
6. Document ownership, context refresh, save routing, and that UI/MCP operations are a following integration step.

# Files allowed to touch
- `src/project/project_timeline_integration.h/.c`
- new `src/project/project_timeline_session.h/.c`
- `src/project/project_timeline_edit.h/.c` only for a necessary additive helper
- `src/project/README.md`
- `src/app/viewer_load.h/.c`, `src/app/viewer_internal.h`, `src/app/viewer.c`, `src/app/viewer_save.c`
- new `tests/project/project_timeline_session_test.c`, `tests/app/project_save_test.c`, `tests/project/project_load_test.c`
- `CMakeLists.txt` project-format lane only, `tests/test_map.json`
- this work order, `artifacts/WO-ANIM-PROJECT-07/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
MCP implementation, timeline UI/rendering, document edit internals, container layout, SVG/XML/parser/renderer implementations, release files, scripts, presets, and paths outside the allowlist. Do not run broad UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug application and new session target build.
- AC-2: focused project session and bundle/file/load/save/timeline CTests pass.
- AC-3: the same focused project suite passes under ASan/UBSan with load/session failure cleanup.
- AC-4: known-v1 timeline edits survive `.vexel` save/decode; unknown and empty payloads and every unrelated project field remain byte-exact.
- AC-5: independent read-only review has no blocking/high findings.
- AC-6: frozen order, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build strict Debug app and project session, load, and save targets.
- Run focused project bundle/file/load/save/timeline/integration/session CTests.
- Run the same focused suite under ASan/UBSan.
- Run independent read-only review limited to allowed paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-07/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not create editable sessions for opaque or invalid payloads, do not mutate encoded bundle slices, and do not publish a save if timeline validation or atomic replacement fails. If the context cannot be retained within existing scene/asset bounds without a duplicate full scan, record the exact ownership seam and stop.

# Output
Leave changes uncommitted. Report project session ownership, edit-to-save round-trip, exact focused evidence, review result, and that live UI/MCP callers remain subsequent work.
