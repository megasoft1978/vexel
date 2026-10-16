---
id: WO-ANIM-PROJECT-17
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 95191c157a571436aa83742f14b682b3260931576f4b59c53a339b99996fe5e0
parallel_safe: false
network: false
---
# Goal
Add a direct native timeline UI action that keys the selected SVG element's current opacity at the playhead, enabling visible authoring of Codex/MCP animation results.

# Context
- Follow ADR-0018, ADR-0020, and the source-preserving/undoable project timeline contracts.
- Native property-track/key creation already works through the live MCP session; native timeline rows remain read-only.
- Current UI selection uses document handles. Persisted tracks must target the selected element's unique SVG id, never a handle.
- Keep each C and `.inc` source under 800 lines. Do not disturb unrelated existing worktree changes.

# Contracts
- The action is available only for a single selected element in a native-v1 project and uses that element's current SVG `opacity` presentation attribute (default 1.0 when absent); reject unsupported inline-style-only opacity with a clear status rather than recording a misleading value.
- At tick zero, create a visible scalar `opacity` property track with `linear` interpolation and a tick-zero key. At later ticks, create the same track with the same initial value and add/upsert the current tick in one user-undoable operation, or add/upsert the key on the existing matching track. Reuse a matching target/property track; do not create duplicates.
- Resolve stable SVG id and current value from indexed document editing APIs; validate id/value bounds and escape/generate track id safely. Do not mutate SVG scene bytes.
- Mutations validate against the active project session, refresh timeline and preview, notify the live viewer, and report actionable success/failure status. Undo/redo must restore timeline content atomically.
- Add an accessible button/name and focused pointer/model coverage. Do not run broad UI/E2E or release tests; no commit.

# Files allowed to touch
- `src/app/viewer_editor.h`, `src/app/viewer_editor_document.c`
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_ui.c`, `src/app/viewer_timeline_accessibility.c`
- `src/project/project_timeline.h`, `src/project/project_timeline.c`
- `src/project/project_timeline_edit.h`, `src/project/project_timeline_edit.c`, `src/project/project_timeline_property_edit.inc`
- `src/project/project_timeline_session.h`, `src/project/project_timeline_session.c`
- `tests/app/viewer_timeline_test.c`, `tests/project/project_timeline_edit_test.c`, `tests/project/project_timeline_session_test.c`
- `src/project/README.md`, `work/WO-ANIM-PROJECT-17.md`
- `artifacts/WO-ANIM-PROJECT-17/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Expose selected element id and opacity information through bounded editor APIs that correctly handle a missing id, missing opacity, duplicate/invalid values, multi-selection, and document decode failures.
2. Expose decoded property name/type/interpolation in the borrowed track view without breaking existing positional initializers; audit all initializers/usages.
3. Add a transactional session/edit operation that can create a property track with keys at tick zero and the requested tick as one history step; existing matching scalar opacity track is reused and its requested tick is upserted.
4. Add a clearly labeled “Key Opacity” native timeline control, visible only when its basic native-project prerequisites hold, plus accessibility semantics and an explicit status on failures.
5. Verify create-at-zero, create-at-later-tick, reuse/upsert, existing tick-zero replacement, undo/redo, source preservation, selection errors, and viewer refresh/preview invalidation.
6. Document the first native UI property authoring workflow and its current opacity scope.

# Verification
- Freeze this order before implementation; front matter `spec_sha256` records the hash of the frozen content with the hash field set to `PENDING` (ecdce7edd8227623be2817abf3a814f5d778024bc85f4da0546fff69943a297e).
- Strict Debug app build and focused viewer timeline, project edit, and project session tests.
- Run the focused tests in Debug, ASan, and configured UBSan where supported by presets.
- Validate order and ledger, changed-path allowlist, all changed C/`.inc` line counts, and `git diff --check`.
- Keep broad UI/E2E acceptance deferred.

# Output
Leave changes uncommitted. Report the usable UI workflow and remaining broader property/cel authoring, export, showcase recording, and final broad acceptance work.
