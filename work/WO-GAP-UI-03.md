---
id: WO-GAP-UI-03
revision: 1
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: c0fe122461f46d1e092c808b49ff19df99eeaec56f0d1e259eccb8d2d5ce5a6a
parallel_safe: false
network: false
---
# Goal
Close the failed and unverifiable acceptance criteria of `work/WO-UI-03.md`: open and close the log pane in the native smoke (parent AC-3), log every status message including `ok` with its matching severity (parent item 2), and record the missing command evidence (parent AC-2, AC-6).

# Context pointers
- Parent spec `work/WO-UI-03.md` and verdict `artifacts/retro-verification-2026-10-12/WO-UI-03.verdict.txt`.
- `src/app/viewer_editor_status.c` (`editor_status_ok` does not log; `editor_status_error` logs the previous status before building the new one), `src/app/viewer_editor_log.c`, `tests/editor/viewer_workspace_test.c`, `tests/editor/native_smoke_actions.sh`, `tests/editor/mouse.py`, `docs/ui/editor-interaction.md` (log shortcut).

# Existing contracts and invariants
- The log stays bounded, UI-only, never changes document bytes or undo history; existing error reporting keeps working.
- macOS-only, C17, no new dependency, files under 800 lines; fixtures change only with a recorded baseline change.

# Files allowed to touch
- `src/app/**`, `src/ui/**`, `tests/editor/**`, `tests/ui/**` (not `tests/oracle/**`, `tests/golden/**`), `docs/ui/**`, `artifacts/WO-GAP-UI-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `tests/oracle/**`, `tests/golden/**`, `src/svg/**`, `src/raster/**`, `src/document/**`, all prior artifacts, every path outside the allowlist.

# Required implementation
1. `editor_status_ok` records one information entry carrying the new status text. `editor_status_error` records one error entry carrying the new error text it builds, not the previous status. Any other status setter routes through one of these or logs with the matching severity. No duplicate entry per status.
2. Focused tests: an `ok` status adds exactly one information row with that text; an edit error adds exactly one error row with the new error text; a status set before the error does not appear as the error row.
3. Native smoke: open the log pane with its documented shortcut, assert the pane node is present in the live layout dump, close it with Escape (parent item 4), assert it is gone, then run the existing byte-identical save assertion after the toggle.
4. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-GAP-UI-03/check-NN.txt`; record any fixture change in `artifacts/WO-GAP-UI-03/golden-changes.md` with reason and side-by-side comparison.

# Acceptance criteria
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-2 `./dev verify` and `./dev t --all` exit 0, with output recorded.
- AC-3 `./dev editor-smoke` exits 0, opens and closes the log pane by keyboard with layout-dump assertions for both states, and the following save is byte-identical.
- AC-4 The tests of item 2 exist under CTest and pass; reverting item 1 makes at least one fail (record the reverted build's failure in `artifacts/WO-GAP-UI-03/pre-change.md`).
- AC-5 Every changed fixture, if any, has an entry in `artifacts/WO-GAP-UI-03/golden-changes.md`.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-UI-03 --ready`, `python3 scripts/wo_validate.py WO-GAP-UI-03 --ready` and `git diff --check` exit 0.
- AC-7 A fresh read-only verifier re-judges AC-1..AC-7 of `work/WO-UI-03.md` and AC-1..AC-6 here against the candidate commit and reports pass in `artifacts/WO-GAP-UI-03/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
./dev editor-smoke
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-UI-03 --ready
python3 scripts/wo_validate.py WO-GAP-UI-03 --ready
git diff --check
```

# Non-goals
No log persistence, filtering, search or click-to-repeat; no change to the log bound or pane layout beyond what item 3 needs.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 150 min; dependency/network: none.

# Failure and escalation
If a status path cannot be routed without touching a protected path, record the call site and log it at that site's severity, reporting the gap. If the smoke cannot drive the shortcut in a real window, record the exact failure and stop; do not replace the real key event with a test-only hook.

# Output
One commit `fix(ui): log every status and exercise the log pane in the native smoke`, evidence under `artifacts/WO-GAP-UI-03/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
