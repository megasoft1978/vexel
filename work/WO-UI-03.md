---
id: WO-UI-03
revision: 1
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: 66c58276fdf703d459bbf3a3ab561fc325fd59d3
spec_sha256: a6f94c92c7f117a60b62112f648adcde06083f683694701ee334398fd5ba2f73
parallel_safe: false
network: false
---
# Goal
Add Blender's Info-editor pattern: a collapsible log pane that records executed operations and every warning and error (ADR-0017 item 7, review defect D3).

# Context pointers
- `docs/ui/blender-alignment-review.md` (defect D3: no operator/command log), `docs/adr/0017-ui-direction-blender-aligned.md`.
- `src/app/viewer_chrome_editor.c`, `viewer_chrome_inspector.c`, `viewer_editor.c`, `viewer_editor_view.c`, `viewer_editor_internal.h`, `src/ui/ui_core.h`.

# Existing contracts and invariants
- Bounded resources: the log holds a fixed maximum number of entries with fixed-size text; overflow drops the oldest entry. No unbounded allocation, no dynamic growth per keystroke.
- The log is UI state only: it must never change document bytes and must survive undo/redo without altering history.
- Existing error reporting keeps working; the log is an additional sink, not a replacement.
- macOS-only, C17, custom-drawn UI, no new dependency, source files under 800 lines.
- Existing fixtures change only with a recorded baseline change.

# Files allowed to touch
- `src/app/**`, `src/ui/**`, `tests/editor/**`, `tests/ui/**` (not `tests/oracle/**`, `tests/golden/**`), `docs/ui/**`, `artifacts/WO-UI-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `work/WO-UI-03.md`.

# Required implementation
1. A bounded log in the editor: a fixed number of entries, each with a severity (operation, information, warning, error), a fixed-size message and a monotonic sequence number. Newest last; oldest dropped on overflow.
2. Every command the editor dispatches records one line using the command's own user-visible name; every status message and every error records a line with the matching severity. Do not scatter string literals: derive operation names from the command registry or the widget label already in the tree.
3. A docked pane above the status bar, hidden by default, toggled by a Log button in the options band and by a documented keyboard shortcut. It lists the newest entries with a severity colour that is never the only channel: each row also carries a severity label or icon.
4. The pane is exposed in the semantic tree with a role, a label, and one row per entry; it never steals canvas focus and Escape closes it.
5. The pane is bounded (a maximum visible row count) and shows the newest entries.
6. Tests: an operation adds a row; a warning and an error add rows with the right severity; overflow keeps the newest entries and drops the oldest; toggling shows and hides the pane in the semantic tree; opening the log never marks the document dirty and never changes materialised bytes.
7. Docs updated; goldens regenerated with a recorded baseline change covering every changed fixture.

# Acceptance criteria
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-2 `./dev verify` and `./dev t --all` exit 0.
- AC-3 `./dev editor-smoke` exits 0, opens and closes the log in a real window, and still saves byte-identically.
- AC-4 The overflow test fails against a build with the bound removed or raised, proving the bound is real (record the mutation).
- AC-5 A recorded check shows the document bytes are unchanged after logging a warning and an error and toggling the pane.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-UI-03 --ready` and `git diff --check` exit 0.
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
python3 scripts/wo_validate.py WO-UI-03 --ready
git diff --check
```

# Non-goals
No log persistence across launches, no copy-as-script, no filtering or search in the log, no click-to-repeat, no tool-options footer, no density work.

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 240 min; dependency/network: none.

# Failure and escalation
If command names cannot be derived without scattering literals, record the exact command ids that lack a name and log those by id, reporting the gap rather than inventing labels.

# Output
Implementation commits, evidence under `artifacts/WO-UI-03/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
