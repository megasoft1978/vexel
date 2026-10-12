---
id: WO-UI-02
revision: 1
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: 66c58276fdf703d459bbf3a3ab561fc325fd59d3
spec_sha256: 9a28cb50ce7fa8f33f2cb7575066144bf150c5805e0f0290a05b09fb13bbb37c
parallel_safe: false
network: false
---
# Goal
Add Blender-style properties tabs to the inspector (ADR-0017 item 5).

# Context pointers
- `docs/adr/0017-ui-direction-blender-aligned.md`, `docs/ui/blender-alignment-review.md` (defect B3), `docs/ui/README.md`.
- `src/app/viewer_chrome_inspector.c`, `src/app/viewer_editor_view.c` (dispatch), `src/app/viewer_editor_internal.h`.

# Existing contracts and invariants
- Every attribute of the selection must stay reachable: a tab may group attributes but must not hide any of them permanently.
- No document byte changes, no new dependency, macOS-only, C17, custom-drawn UI, source files under 800 lines.
- Existing fixtures change only with a recorded baseline change; a new fixture is preferred.

# Files allowed to touch
- `src/app/**`, `tests/editor/**`, `tests/ui/**` (not `tests/oracle/**`, `tests/golden/**`), `docs/ui/**`, `artifacts/WO-UI-02/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `work/WO-UI-02.md`.

# Required implementation
1. A tab strip at the top of the inspector pane with `Object`, `Fill`, `Stroke`, `Document`, the active tab distinct, each tab a widget-tree node with a stable id, a command id, a label, a checked state and a tooltip.
2. `Object` shows the element name, geometry and every attribute no other tab owns. `Fill` shows `fill` and `fill-opacity`. `Stroke` shows `stroke`, `stroke-width`, `stroke-opacity`, `stroke-linecap`, `stroke-linejoin`. `Document` always describes the document and carries Fit Canvas and Actual Size.
3. A filtered tab is its own group selector and its group is always open; only `Object` honours the collapsible section state.
4. With no selection, `Fill` and `Stroke` show a clear empty state; `Object` and `Document` stay useful.
5. Tests assert, per tab, what is shown and what must not be shown, driven through real pointer clicks on the tab widgets, plus the semantic tree shape.
6. Docs updated; goldens regenerated with a recorded baseline change covering every changed fixture.

# Acceptance criteria
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-2 `./dev verify` and `./dev t --all` exit 0.
- AC-3 `./dev editor-smoke` exits 0.
- AC-4 The per-tab test fails if any tab shows another tab's attribute (record the deliberate mutation you used to prove it).
- AC-5 No attribute of the test element becomes unreachable across the four tabs; the test enumerates the element's attributes and finds each on exactly one tab.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-UI-02 --ready` and `git diff --check` exit 0.
- AC-7 A fresh read-only verifier reports pass.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
./dev editor-smoke
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-UI-02 --ready
git diff --check
```

# Non-goals
No attribute editing behaviour change, no dockable panes, no tool-options footer, no log pane.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 180 min; dependency/network: none.

# Failure and escalation
If grouping makes an attribute unreachable, stop and report the attribute and the tab layout rather than shipping a hidden attribute.

# Output
Implementation commits, evidence under `artifacts/WO-UI-02/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
