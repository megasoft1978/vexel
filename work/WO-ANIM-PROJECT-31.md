---
id: WO-ANIM-PROJECT-31
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: e5b4a74ace2850260b20df308a904642bd7eb5a37cfb5428737f5ae26a2278dd
---
# Goal
Let users key selected-object visibility at the native Animate playhead, enabling blink and hide/show animation in the user-authored character workflow.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`, ADR-0018 through ADR-0021, and the active product plan.
- Native v1 already supports boolean `visibility` property tracks and the renderer applies them as SVG `visibility="visible|hidden"` at integer ticks.
- The timeline currently exposes opacity, transform, fill, stroke, and drawing-cel actions; there is no user-facing visibility action.
- Preserve the heavily modified shared worktree and anchor comparisons to this current HEAD. Do not stage, reset, clean, or rewrite unrelated changes.
- Keep full UI/E2E acceptance deferred; run only the focused native timeline test.

# Contracts
- Read only the selected element's explicit `visibility` presentation attribute, or use SVG's default visible value if absent. Accept exact `visible`, `hidden`, or `collapse`; reject inline style declarations for visibility, duplicate/ambiguous IDs, unsupported values, and selected elements whose inherited visibility cannot be safely established from an ancestor chain. Do not mutate SVG bytes.
- At tick zero create a deterministic visibility property track with a baseline or update the existing target track. At later ticks require a tick-zero baseline. Use `boolean` value type and `step` interpolation; repeated same value is a no-op.
- Keep mutations in native timeline session history, update selection/preview/dirty notifications consistently, and preserve existing opacity/transform/color/cel actions.
- Apple Silicon macOS only, C17, no dependencies, source files under 800 lines.

# Files allowed to touch
- `work/WO-ANIM-PROJECT-31.md`
- `src/app/viewer_editor.h`, `src/app/viewer_editor_visibility.c` (new)
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline.c`, `src/app/viewer_timeline_native_move.c`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_ui.c`, `src/app/viewer_timeline_accessibility.c`, `src/app/viewer_timeline_transform.c`
- `tests/app/viewer_timeline_native_test.c`, `tests/app/viewer_editor_visibility_test.c` (new)
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-ANIM-PROJECT-31/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve existing unrelated working changes. Do not change project schema/renderer, MCP semantics, SVG parsing/rasterization, inspector or workspace behavior, project recovery, ADRs, website, previous work orders/evidence, or paths outside the allowlist.
- Do not run broad UI/E2E, all CTest/Python tests, corpus, fuzz, performance, sanitizer matrix, or release gates.

# Required implementation
1. Add a bounded selected-visibility capture API with stable unique ID validation and explicit handling of local/ancestor visibility. Do not infer through unsupported CSS/style or unsafe inheritance.
2. Add a selected-visibility key action that creates/reuses the native boolean visibility track, enforces baseline-at-zero before later keys, selects its track, and triggers preview/dirty state only on a real mutation.
3. Add a visible and accessible Key Visibility timeline control with pointer dispatch and width accounted in compact/expanded layout without overlap.
4. Add focused regressions for absent/default visible, explicit visible/hidden/collapse, style override rejection, inherited-hidden ancestor rejection, missing/duplicate IDs, baseline and later-tick rules, same-value no-op, undo/redo, and unchanged SVG bytes.
5. Keep modified C/Objective-C/test sources below 800 lines and document limitations in `artifacts/WO-ANIM-PROJECT-31/verification.txt`.

# Verification
- Confirm `viewer-timeline-native` is registered in CMake and mapped in `tests/test_map.json`.
- Build `svgworkbench` and `viewer_timeline_native_test` in Debug.
- Run only `ctest --test-dir build/mac-debug --output-on-failure -R '^viewer-timeline-native$'`.
- Validate frozen order, `tests/test_map.json` JSON, ledger, changed-path scope against the frozen base, source line counts, and `git diff --check`.
- Do not run broad UI/E2E, corpus, performance, fuzz, sanitizer matrix, or release checks.

# Output
Leave changes uncommitted. Record exact evidence in `artifacts/WO-ANIM-PROJECT-31/verification.txt`, `STATE.md`, `LEDGER.md`, and `ledger.jsonl`. Keep the full Vexel product goal active.

# Verification commands
```text
cmake --build build/mac-debug --target svgworkbench viewer_timeline_native_test
ctest --test-dir build/mac-debug --output-on-failure -R '^viewer-timeline-native$'
python3 scripts/wo_validate.py WO-ANIM-PROJECT-31 --ready
python3 -c "import json; json.load(open('tests/test_map.json'))"
git diff --check
```
