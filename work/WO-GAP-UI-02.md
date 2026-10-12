---
id: WO-GAP-UI-02
revision: 1
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: f157ddfc6a0e9d9b11914bdce1dd2d6d374faca7d73e1ec90d89db987172f876
parallel_safe: false
network: false
---
# Goal
Close the failed and unverifiable acceptance criteria of `work/WO-UI-02.md`: add the per-attribute single-tab reachability test (parent AC-5) and record the missing command evidence (parent AC-2).

# Context pointers
- Parent spec `work/WO-UI-02.md` and verdict `artifacts/retro-verification-2026-10-12/WO-UI-02.verdict.txt`.
- `src/app/viewer_chrome_inspector.c`, `src/app/viewer_editor_internal.h` (inspector tab ids), `tests/editor/viewer_chrome_test.c` (existing Fill/Stroke visibility checks), `artifacts/WO-UI-02/mutation.md`.

# Existing contracts and invariants
- Every attribute of the selection stays reachable; a tab groups attributes but never hides one permanently.
- No document byte change, no fixture change, macOS-only, C17, no new dependency, files under 800 lines.

# Files allowed to touch
- `tests/editor/**`, `src/app/viewer_chrome_inspector.c` (only if the new test exposes an unreachable or doubly shown attribute), `CMakeLists.txt`, `artifacts/WO-GAP-UI-02/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `tests/oracle/**`, `tests/golden/**`, every golden fixture, `src/svg/**`, `src/raster/**`, `src/document/**`, all prior artifacts, every path outside the allowlist.

# Required implementation
1. A focused CTest test selects one element carrying at least: `id`, geometry (`x`, `y`, `width`, `height`), `transform`, `fill`, `fill-opacity`, `stroke`, `stroke-width`, `stroke-opacity`, `stroke-linecap`, `stroke-linejoin`, one unknown non-namespaced attribute and one namespaced attribute.
2. The test enumerates that element's attributes from the document (not from a hard-coded list), opens each of `Object`, `Fill`, `Stroke`, `Document` by a real pointer click on the tab widget, collects the attribute rows each tab shows, and asserts every enumerated attribute appears on exactly one tab.
3. Prove the test bites: apply one deliberate mutation (e.g. `Object` skips one attribute it should own, or `Fill` also lists `stroke`), record the mutation and the resulting failure in `artifacts/WO-GAP-UI-02/mutation.md`, then revert it.
4. If the test finds a real gap, fix it in `src/app/viewer_chrome_inspector.c` and record it; never relax the assertion.
5. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-GAP-UI-02/check-NN.txt`.

# Acceptance criteria
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0 and runs the new test.
- AC-2 `./dev verify` and `./dev t --all` exit 0, with output recorded.
- AC-3 `./dev editor-smoke` exits 0.
- AC-4 The new test enumerates the element's attributes from the document and asserts each appears on exactly one of the four tabs, reached by pointer clicks.
- AC-5 `artifacts/WO-GAP-UI-02/mutation.md` records the mutation and the failing assertion it produced.
- AC-6 `git show --name-only --format= HEAD` (this order's single commit) lists no golden fixture.
- AC-7 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-UI-02 --ready`, `python3 scripts/wo_validate.py WO-GAP-UI-02 --ready` and `git diff --check` exit 0.
- AC-8 A fresh read-only verifier re-judges AC-1..AC-7 of `work/WO-UI-02.md` and AC-1..AC-7 here against the candidate commit and reports pass in `artifacts/WO-GAP-UI-02/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
./dev editor-smoke
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-UI-02 --ready
python3 scripts/wo_validate.py WO-GAP-UI-02 --ready
git diff --check
```

# Non-goals
No inspector redesign, no attribute editing change, no new tab, no fixture regeneration.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependency/network: none.

# Failure and escalation
If fixing a discovered gap would change a golden fixture, stop and report the attribute and tab layout for planner review instead of regenerating fixtures.

# Output
One commit `test(ui): prove every attribute is on exactly one inspector tab`, evidence under `artifacts/WO-GAP-UI-02/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
