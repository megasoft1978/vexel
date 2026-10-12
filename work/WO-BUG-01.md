---
id: WO-BUG-01
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: deepseek-flash
base_sha: 6c27757a34f960a623971d2e140554d8c4c38728
spec_sha256: 6d5fe0e3aa2f43748b7b8c82f18f5a5c1d436e2c5edacfb4d53f8d7734944e94
parallel_safe: false
network: false
---
# Goal
Fix the intermittent data-loss defect in the editor save path where the selected shape is written out as a transform on the root `<svg>` element and the shape itself disappears.

# Context pointers
- Evidence: `artifacts/WO-UI-02/editor-smoke-flake.md` (exact bad bytes).
- `src/app/viewer_editor_*.c` (transform composition and serialisation), `src/document/document_edit*.c`, `tests/editor/native_smoke_actions.sh` (`assert_core_save`).

# Existing contracts and invariants
- The document is never corrupted: a save writes a document that still contains every element it contained before the edit.
- Move/resize/rotate compose into exactly one `matrix(...)` on the selected element and preserve authored geometry and unknown attributes.
- Undo restores exact prior bytes. No frozen oracle or acceptance test may be weakened.
- macOS-only, C17, custom-drawn UI, no new dependency, source files under 800 lines.

# Files allowed to touch
- `src/app/**`, `src/document/**` only where the defect is proven to live, `tests/editor/**`, `tests/document/**`, `docs/**`, `artifacts/WO-BUG-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `work/WO-BUG-01.md`.

# Required implementation
1. Reproduce the defect deterministically before changing any code: state the exact gesture sequence and the resulting bytes, and identify the code path that resolves the edit target handle.
2. Fix the cause, not the symptom. If the cause is a stale or zero handle being treated as the document root, make the edit reject an invalid target instead of silently rewriting the root.
3. Add a regression test that fails on the pre-fix build: at minimum a document-level test that applies the same edit to the same stale-handle condition and asserts the element still exists with the matrix on the element.
4. Run `./dev editor-smoke` at least ten consecutive times and record the pass count, so the intermittent failure is shown to be gone rather than merely unobserved.
5. Record the exact command, the observed pre-fix failure, and the post-fix result under `artifacts/WO-BUG-01/`.

# Acceptance criteria
- AC-1 The defect is reproduced before the fix and the reproduction is recorded with exact bytes.
- AC-2 A regression test fails against the pre-fix build for the right reason (record the failure).
- AC-3 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-4 `./dev verify` and `./dev t --all` exit 0.
- AC-5 `./dev editor-smoke` passes ten consecutive runs and the count is recorded.
- AC-6 A save after the reproducing gesture contains every element the document had before the edit, with exactly one matrix on the intended element.
- AC-7 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-BUG-01 --ready` and `git diff --check` exit 0.
- AC-8 A fresh read-only verifier reports pass.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
./dev editor-smoke
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-BUG-01 --ready
git diff --check
```

# Non-goals
No change to the transform composition maths, no new editing feature, no UI change, no rewrite of the undo model.

# Budget
Executor max runs: 4; verifier max rounds: 2; elapsed hard limit: 300 min; dependency/network: none.

# Failure and escalation
If the defect cannot be reproduced deterministically, record the ten-run statistics and the exact conditions, and implement the invalid-target rejection plus its regression test rather than guessing at the root cause.

# Output
One fix commit, evidence under `artifacts/WO-BUG-01/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
