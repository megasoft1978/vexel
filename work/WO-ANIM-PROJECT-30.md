---
id: WO-ANIM-PROJECT-30
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 62db3978f06d7efbb4daed450b4bde318e3b0e849767feb3d8a816b3fdb75675
---
# Goal
Let users key selected-object fill and stroke colors directly in the native Animate timeline, extending the character workflow beyond opacity and transform animation without changing the v1 project format.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`, ADR-0018 through ADR-0021, and `docs/product/vexel-goal-and-delivery-plan.md`.
- Native v1 already supports `fill` and `stroke` as four-component RGBA property tracks; the renderer, MCP authoring path, and project validation implement this pair.
- The UI currently exposes only Key Opacity and Key Transform. User-authored fill/stroke keying is missing from the promised vector-character workflow.
- The repository has a heavily modified shared worktree. Preserve its existing changes; this order is anchored to the current HEAD and must not inspect, stage, reset, or rewrite unrelated changes.
- Keep broad UI/E2E deferred. Use only this order's focused native timeline test/build checks.

# Existing contracts and invariants
- Continue to target a unique selected SVG element ID and edit only the native timeline session; the SVG scene bytes remain unchanged.
- Read explicit fill/stroke presentation attributes only. Reject inline style declarations for that property and unsupported/inherited/currentColor/URL paint values rather than guessing an effective color.
- Support SVG hex colors `#RGB`, `#RRGGBB`, `#RGBA`, and `#RRGGBBAA`, plus the small documented named-color subset (`black`, `white`, `red`, `green`, `blue`, `transparent`). Normalize to four finite RGBA components in 0..1. `none` and context-dependent values are unsupported.
- At tick zero, create or update a baseline key. At later ticks, require an existing track and tick-zero key before creating/updating that key, matching transform-key behavior.
- Reuse existing target/property tracks and session validation/history; repeated identical keys are successful no-ops. Mutations update preview/dirty state once.
- Preserve Apple Silicon macOS-only support, C17, custom UI, no dependencies, and sub-800-line source/test files.

# Files allowed to touch
- `work/WO-ANIM-PROJECT-30.md`
- `src/app/viewer_editor.h`, `src/app/viewer_editor_document.c`
- `src/app/viewer_editor_paint.c` (new)
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline.c`, `src/app/viewer_timeline_transform.c`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_ui.c`
- `tests/app/viewer_timeline_native_test.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-ANIM-PROJECT-30/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve unrelated dirty changes. Do not modify project/schema/render/MCP implementations, SVG parsing/rasterization, generic inspector, workspace routing, project recovery, UI golden files, product-goal ADRs, earlier work orders/evidence, or anything outside the allowlist.
- Do not run broad UI/E2E, complete CTest/Python suites, corpus, fuzz, performance, sanitizer matrix, or release gates.

# Required implementation
1. Add a bounded editor API that returns the selected element's unique ID and one explicit supported fill/stroke paint encoded as RGBA; distinguish unsupported/ambiguous values without mutating the scene.
2. Add an undoable native timeline action for selected fill/stroke at the playhead. Reuse matching property tracks; create deterministic collision-free track IDs and baseline keys at tick zero; require a baseline for later ticks. Update selected timeline track, preview, dirty notification, and status accurately.
3. Provide user-visible controls for Key Fill and Key Stroke in the native timeline, with accessible semantic labels and working pointer dispatch. Keep controls within existing timeline layout bounds and retain opacity/transform controls.
4. Add focused tests for hex/named parsing including alpha, missing/duplicate IDs, style precedence rejection, unsupported values, tick-zero baseline, later-tick creation, same-tick no-op, distinct fill/stroke tracks, undo/redo, and unchanged SVG scene bytes. Existing test target remains `viewer-timeline-native`.
5. Keep modified C files below 800 lines and document supported color forms/limitations in test comments or existing module guide only if an allowed path permits; do not change project format or ADR.

# Verification
- Confirm focused selector `viewer-timeline-native` in `tests/test_map.json` and the CMake registration before running.
- Build `svgworkbench` and `viewer_timeline_native_test` in Debug.
- Run only `ctest --test-dir build/mac-debug --output-on-failure -R '^viewer-timeline-native$'`.
- Run `python3 scripts/wo_validate.py WO-ANIM-PROJECT-30 --ready`, validate `tests/test_map.json` JSON, report changed C line counts, confirm order-specific changed paths, and run `git diff --check`.
- Do not run broad UI/E2E or any full test suite.

# Output
Leave changes uncommitted. Record exact commands/results in `artifacts/WO-ANIM-PROJECT-30/verification.txt`, update `STATE.md`, `LEDGER.md`, and `ledger.jsonl`, and keep the complete product objective active.

# Verification commands
```text
cmake --build build/mac-debug --target svgworkbench viewer_timeline_native_test
ctest --test-dir build/mac-debug --output-on-failure -R '^viewer-timeline-native$'
python3 scripts/wo_validate.py WO-ANIM-PROJECT-30 --ready
python3 -c "import json; json.load(open('tests/test_map.json'))"
git diff --check
```
