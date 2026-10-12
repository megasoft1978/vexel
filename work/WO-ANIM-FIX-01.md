---
id: WO-ANIM-FIX-01
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: claude-sonnet-5-5
base_sha: a1be0f46cc7836692e938ed6e1bbbb472bd06fba
spec_sha256: 4e529e3b1a8208132b6c9d5567da25dc674f3fcdc2cd1886235e2f3a93ac6512
parallel_safe: false
network: false
---
# Goal
Fix a heap-use-after-free in `test_spline_output_limit`, which AddressSanitizer reports on `main`.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `tests/anim/animation_authoring_edge_test.c`.
- The WO-M1-01 closure verifier's `./dev asan` run failed on CTest `animation-authoring-edge`: the decoded animation document borrows the source buffer, but the test freed that buffer before calling `svg_anim_authoring_add_key`. WO-MOTION-03's frozen commands did not run the sanitizers, so this reached `main`.

# Existing contracts and invariants
- Decoded animation documents borrow the source bytes; the caller keeps them alive until the document is destroyed.
- Test expectations stay the same: `add_key` on the padded spline track returns the limit error and no replacement.

# Files allowed to touch
- `tests/anim/animation_authoring_edge_test.c`
- `work/WO-ANIM-FIX-01.md`
- `artifacts/WO-ANIM-FIX-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other `work/**`, `tests/oracle/**`, `tests/golden/**`, `src/**`, `scripts/**`, `CMakeLists.txt`, all prior artifacts, and all paths outside the allowlist.

# Required implementation
1. Keep the source buffer alive until after the `add_key` call and the document destroy, then free it. Do not weaken any assertion.

# Acceptance criteria
- AC-1: `./dev asan` and `./dev ubsan` pass, including `animation-authoring-edge`.
- AC-2: Debug CTest and `./dev t --all` pass; ledger, work-order validation and `git diff --check` pass.

# Verification commands

```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev asan
./dev ubsan
./dev t --all
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-ANIM-FIX-01 --ready
git diff --check
```

# Non-goals
Any production-code change, new tests, or other test edits.

# Budget
One run; no network.

# Failure and escalation
If the sanitizer reports another error, record it in `BLOCKERS.md` through a new work order; do not weaken tests.

# Output
Commit `test: fix source lifetime in spline output limit test`.
