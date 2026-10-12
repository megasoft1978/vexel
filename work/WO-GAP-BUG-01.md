---
id: WO-GAP-BUG-01
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: aa7cefb93dccffaf0b339d7a80cec6489a72da87b695bcca611ffd530405099d
parallel_safe: false
network: false
---
# Goal
Close the failed and unverifiable acceptance criteria of `work/WO-BUG-01.md`: ten consecutive `./dev editor-smoke` passes recorded (parent AC-5), the missing `artifacts/WO-BUG-01/` evidence packet including the exact gesture sequence (parent item 1, item 5), and the missing command evidence (parent AC-4, AC-7). This is an evidence order; the fix itself shipped in `f18d771`.

# Context pointers
- Parent spec `work/WO-BUG-01.md` and verdict `artifacts/retro-verification-2026-10-12/WO-BUG-01.verdict.txt`.
- `artifacts/WO-UI-02/editor-smoke-flake.md` (pre-fix bad bytes), `artifacts/WO-UI-02/check-06.json`, commit `f18d771` (root-transform guard, regression test, pre-fix failure `root transform guard failed: 4`), `tests/editor/native_smoke_actions.sh` (`assert_core_save`), `src/app/viewer_editor_transform.c`.

# Existing contracts and invariants
- A save never loses an element; the transform refuses a root target. No frozen test, oracle or smoke assertion is weakened, retried-until-green or skipped.

# Files allowed to touch
- `artifacts/WO-BUG-01/**`, `artifacts/WO-GAP-BUG-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`. Scratch logs go to `.cache/WO-GAP-BUG-01/`.

# Protected paths
- Every source, test, script and doc file; `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, all other prior artifacts.

# Required implementation
1. Write `artifacts/WO-BUG-01/reproduction.md`: the exact gesture sequence that produced the bad save (each key/pointer action of `tests/editor/native_smoke_actions.sh` up to the failing `assert_core_save`, in order, with the script line numbers at `f18d771^`), the bad bytes quoted from `artifacts/WO-UI-02/editor-smoke-flake.md`, and the code path that resolved the edit target to the root.
2. Write `artifacts/WO-BUG-01/regression.md`: the regression test name and file from `f18d771`, the pre-fix failure text, and the post-fix pass from this order's CTest run.
3. Run the ten-run batch below exactly once per executor run and keep every batch you run, passing or not, as `artifacts/WO-BUG-01/editor-smoke-10-runN.txt`. Do not discard or re-run single failing iterations.
4. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-GAP-BUG-01/check-NN.txt`.

# Acceptance criteria
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-2 `./dev verify` and `./dev t --all` exit 0, with output recorded.
- AC-3 One recorded batch shows ten lines `run i exit 0` for i = 1..10, and every batch run is recorded.
- AC-4 `reproduction.md` and `regression.md` exist with the content of items 1 and 2.
- AC-5 `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-BUG-01 --ready`, `python3 scripts/wo_validate.py WO-GAP-BUG-01 --ready` and `git diff --check` exit 0.
- AC-7 A fresh read-only verifier re-judges AC-1..AC-8 of `work/WO-BUG-01.md` and AC-1..AC-6 here against the candidate commit and reports pass in `artifacts/WO-GAP-BUG-01/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
mkdir -p .cache/WO-GAP-BUG-01
for i in 1 2 3 4 5 6 7 8 9 10; do ./dev editor-smoke > .cache/WO-GAP-BUG-01/run-$i.log 2>&1; echo "run $i exit $?"; done
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-BUG-01 --ready
python3 scripts/wo_validate.py WO-GAP-BUG-01 --ready
git diff --check
```

# Non-goals
No code change, no smoke-script change, no new test.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependency/network: none.

# Failure and escalation
If any iteration of a batch fails, keep the batch file, copy that run's log and `VEXEL_SMOKE_CAPTURE_DIR` captures to `artifacts/WO-GAP-BUG-01/failure-runN/`, and stop for planner review. Do not edit code under this order.

# Output
One commit `docs(evidence): record WO-BUG-01 reproduction and ten-run smoke`, evidence under `artifacts/WO-BUG-01/` and `artifacts/WO-GAP-BUG-01/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
