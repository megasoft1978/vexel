---
id: WO-ANIM-PROJECT-06
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 52b683f1ef99d13dfd0832743a1bc34f87901c0340dc88decc98dacc106ba84f
parallel_safe: false
network: false
---
# Goal
Add a bounded, source-preserving, undoable authoring model for native v1 drawing-track exposures. This is the project-model foundation that the timeline UI and MCP must call; it must not mutate SVG scene bytes or opaque future timelines.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0018/0019/0020, `src/project/project_timeline*`, project load/save integration, and existing document edit history contracts.
- WO-ANIM-PROJECT-05 validates drawing SVG assets but does not edit timeline JSON.
- Existing `viewer_timeline` edits SVG SMIL only. Native timeline JSON currently loads and saves as borrowed opaque bytes.
- No GPL editor source, no network, no broad UI/E2E suite, no commit.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, checked arithmetic, explicit ownership.
- Container v1 bytes, scene SVG, metadata, existing assets, and untouched timeline source spans remain byte-exact.
- Only recognized native `vexel.animation` version 1 can be edited. Opaque future payloads must be refused without mutation.
- Track/cel identifiers are stable serialized values; never persist process-local document handles.
- The existing timeline decoder remains the semantic validator and source-view implementation.

# Required implementation
1. Add a project-layer owned edit model initialized from a valid known-v1 timeline and its validation context. Reject malformed, unsupported, or invalid initial input without taking ownership of caller buffers.
2. Add bounded drawing-track creation for an existing valid SVG target ID and existing `image/svg+xml` asset. Enforce v1 track-ID, track-count, asset-name, timeline-size, and context limits.
3. Add sorted cel-exposure upsert and removal by track ID and integer tick. Enforce tick 0 for a track's first exposure, unique ticks, duration bounds, and valid existing SVG asset references.
4. Serialize edits by replacing only the timeline JSON spans needed for the operation. Preserve all unrelated root fields and existing track JSON bytes exactly; generated JSON must be valid UTF-8, escaped, bounded, and validated before commit.
5. Provide bounded undo/redo with explicit command-count and owned-byte caps. Failed or over-limit edits leave current bytes, history cursor, and redo state unchanged. Returned source views borrow the edit model and are invalidated only by a successful mutation, undo, redo, or destroy.
6. Keep scene SVG, metadata, assets, and opaque future payloads untouched. The model borrows the validation context only during each call.
7. Add focused tests for create/track/cel edit/remove, tick/order/asset/target limits, JSON escaping, byte preservation, undo/redo, redo truncation, failed-edit atomicity, memory limits, and malformed/opaque initialization.
8. Document model ownership, limits, serialization behavior, and the fact that UI/MCP wiring is a following integration step.

# Files allowed to touch
- new `src/project/project_timeline_edit.h/.c`
- `src/project/project_timeline.h`, `src/project/project_timeline.c`, `src/project/project_timeline_json.h`, `src/project/project_timeline_json.c` only if a required additive helper is unavoidable
- `src/project/README.md`
- `tests/project/project_timeline_edit_test.c`
- `CMakeLists.txt` project-format lane only, `tests/test_map.json`
- this work order, `artifacts/WO-ANIM-PROJECT-06/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
Application/UI/MCP integration, document internals, container layout, SVG/XML/parser/renderer implementations, release files, scripts, presets, and every path outside the allowlist. No UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug app and new project edit target build.
- AC-2: focused project timeline edit CTests pass.
- AC-3: same focused suite passes under ASan/UBSan with ownership and failed-edit cleanup coverage.
- AC-4: unchanged source spans are byte-exact; generated payload validates; opaque payloads remain uneditable and byte-exact.
- AC-5: independent read-only review has no blocking/high findings.
- AC-6: frozen spec, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build strict Debug app and timeline edit targets.
- Run focused project bundle/file/timeline/integration/edit CTests.
- Run the same focused project suite under ASan/UBSan.
- Run independent read-only review restricted to allowed paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-06/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not canonicalize untouched JSON or mutate opaque timelines. If source-preserving span edits cannot keep v1 validation, history bounded, and failed operations atomic under this scope, stop and record the specific ownership or serialization seam needed in a new order.

# Output
Leave changes uncommitted. Report edit/undo behavior, exact focused evidence, review result, and that UI/MCP wiring, drawing-asset creation, playback, and export remain subsequent work.
