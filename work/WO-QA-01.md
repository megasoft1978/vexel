---
id: WO-QA-01
revision: 2
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: 5894cb40acfaf0aac10127d82c30fb8be9d38cb4
spec_sha256: 2d778c85910dbf1720f76b7937da0839e177d3f87dcce7f2ddf7dae28ba666cf
parallel_safe: false
network: false
---
# Goal
Record the first complete full-gate baseline for the project and repair the one failing gate: `./dev editor-smoke` times out because two of its assertions predate the `Selected on canvas` layer-row state that the W15 accessibility work introduced.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/adr/0016-product-target-and-gates.md`, `docs/testing.md`, `tests/editor/native_smoke_actions.sh`, `tests/editor/mouse.py`, `src/app/viewer_chrome_structure.c`.
- Baseline evidence for every other gate is already recorded under `.cache/final-baseline/` for this session: `verify`, `asan`, `ubsan`, `shape-fuzz`, `mcp-fuzz`, `e2e`, `soak`, `viewer-smoke`, `fuzz-xml` and `animation-fuzz` all exit 0; `editor-smoke` exits 1.

# Existing contracts and invariants
- Selection is intentionally independent of layer visibility and lock state: the scenario's own later step expects the message `Unlock the selected layer to transform it` while the locked row is still selected, so hiding or locking must **not** clear the canvas selection.
- `src/app/viewer_chrome_structure.c` appends `Selected on canvas`, `Hidden` and `Locked` to a layer row's value independently; a canvas-selected visible row therefore has value `Selected on canvas` and an empty value is no longer a valid proxy for "not hidden".
- The smoke driver `tests/editor/mouse.py` supports only `present`, `state=checked`, `contains=` and `value=`. No assertion may be deleted or made unconditional.
- Do not change product behaviour, localization strings, or any C source.

# Files allowed to touch
- `tests/editor/mouse.py`
- `tests/editor/native_smoke_actions.sh`
- `artifacts/WO-QA-01/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `src/**`, `app/**`, `CMakeLists.txt`, `tests/oracle/**`, `tests/golden/**`, `bench/**`, `docs/testing.md`, all prior artifacts, and every path outside the allowlist.

# Required implementation
1. Add a `not-contains=SUBSTRING` condition to `assert_node` in `tests/editor/mouse.py`, implemented symmetrically to `contains=`: it passes when at least one node with the label exists and none of those nodes' `value` contains the substring. Keep the existing conditions unchanged.
2. In `tests/editor/native_smoke_actions.sh`, replace the two stale `"Rectangle #box" value=` expectations (after re-showing and after unlocking) with `not-contains=Hidden` and `not-contains=Locked` respectively, so each step asserts the specific state it toggled. Do not remove or weaken any other assertion, and do not change the surrounding scenario.
3. Run the full gate set and record every command's exit code and key output under `artifacts/WO-QA-01/`, including the two native smokes, `./dev fuzz-xml` and `build/mac-asan/bin/animation_fuzz 1000000 <seed>`.
4. Record in `artifacts/WO-QA-01/baseline.md` which commands the standard `./dev final` gate runs and which pass/fail, and state explicitly which tests are not part of `./dev final` and how they are invoked.

# Acceptance criteria (machine-checkable)
- AC-1 `./dev editor-smoke` exits 0 and reaches the end of the scenario.
- AC-2 `python3 -m unittest tests.e2e.test_dev_final` and `python3 -m unittest tests.test_quick_tools` exit 0.
- AC-3 `./dev verify`, `./dev asan`, `./dev ubsan`, `./dev e2e`, `./dev soak --minutes 1`, `./dev shape-fuzz --seed 20261010 --output .cache/qa/shape-fuzz-final.json`, `./dev mcp-fuzz`, `./dev viewer-smoke`, `./dev fuzz-xml --duration 30 --seed 20261012` and `build/mac-asan/bin/animation_fuzz 1000000 20261012` all exit 0 on the candidate.
- AC-4 No file under `src/`, `app/`, `tests/oracle/` or `tests/golden/` changed; `git diff --name-status` stays inside the allowlist.
- AC-5 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-QA-01 --ready` and `git diff --check` exit 0.
- AC-6 A fresh read-only verifier reports pass against the exact candidate commit.

# Verification commands
Run sequentially from the repository root unless the command changes directory itself.

```text
./dev editor-smoke
python3 -m unittest tests.e2e.test_dev_final
python3 -m unittest tests.test_quick_tools
./dev verify
./dev asan
./dev ubsan
./dev e2e
./dev soak --minutes 1
./dev shape-fuzz --seed 20261010 --output .cache/qa/shape-fuzz-final.json
./dev mcp-fuzz
./dev viewer-smoke
./dev fuzz-xml --duration 30 --seed 20261012
build/mac-asan/bin/animation_fuzz 1000000 20261012
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-QA-01 --ready
git diff --check
```

# Non-goals
No product, renderer, parser, UI, localization or platform change. No new test framework. No change to `./dev final` itself in this order; bringing the ungated fuzzers into the standard final gate is a separate work order.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 150 min; dependency/network: none.

# Failure and escalation
If `./dev editor-smoke` still fails after the assertion repair, capture the retained smoke log path and the exact failing node condition in `artifacts/WO-QA-01/`, and stop for planner review rather than relaxing any further assertion.

# Output
One implementation commit using `fix(smoke): assert the toggled layer state precisely`. Include the baseline and per-command evidence under `artifacts/WO-QA-01/`, update `STATE.md` and both ledgers, and provide the schema-valid executor report.
