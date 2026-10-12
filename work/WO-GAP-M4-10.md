---
id: WO-GAP-M4-10
revision: 1
status: READY
milestone: M4
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: 59d72ead910c6608d99fe96d8abb5118ade883484a37f7cc7a0b635229089157
parallel_safe: false
network: false
---
# Goal
Close the failed and unverifiable acceptance criteria of `work/WO-M4-10.md`: map token-storage exhaustion to `SVG_SHAPE_ERROR_LIMIT`, add the four focused tests, write `corpus-delta.json`, state the token count, record the missing command evidence, and state the scope truthfully.

# Context pointers
- Parent spec `work/WO-M4-10.md`, verdict `artifacts/retro-verification-2026-10-12/WO-M4-10.verdict.txt`.
- `src/svg/shape/decode.c` (stream-failure branch maps any XML diagnostic to `SVG_SHAPE_ERROR_XML`, including `SVG_XML_ERROR_OUTPUT_EXHAUSTED`), `src/svg/shape/traverse_internal.h` (`SHAPE_MAX_SOURCE_BYTES`, `SHAPE_MAX_TOKEN_BYTES`), `src/svg/xml/xml_tokenizer.h`, `src/svg/shape/shape_alloc.h` (allocation stats hook), `tests/svg/shape/test_shape.c`.
- The implementation landed in `071e0ec` together with ungoverned changes described by `work/WO-RETRO-01.md`. Parent base: `5894cb40acfaf0aac10127d82c30fb8be9d38cb4`.

# Existing contracts and invariants
- Parent invariants hold: `svg_shape_decode` keeps its signature and diagnostic contract; every other limit is unchanged; stream tokens are freed with `free()`, shape tokens with `svg_shape_free()`.
- No new dependency, C17, files under 800 lines; no frozen test, golden or oracle weakened. Corpus is read-only; no corpus byte enters the repository.

# Files allowed to touch
- `src/svg/shape/decode.c`, `src/svg/shape/traverse_internal.h`, `tests/svg/shape/**` (not frozen fixtures), `CMakeLists.txt`, `reports/M4/render-diagnostics.json`, `reports/M4/render-diagnostics.md`, `artifacts/WO-M4-10/**`, `artifacts/WO-GAP-M4-10/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`. Scratch builds go to `.cache/WO-GAP-M4-10/`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, all other `src/**` and `app/**`, `src/svg/xml/**`, `scripts/**`, `tests/oracle/**`, `tests/golden/**`, `bench/**`, other prior artifacts, every path outside the allowlist.

# Required implementation
1. In the stream-failure branch, map `SVG_XML_ERROR_OUTPUT_EXHAUSTED` (and cancellation) to `SVG_SHAPE_ERROR_LIMIT` with the offset read; keep grammar failures on the XML mapping.
2. State in the `SHAPE_MAX_TOKEN_BYTES` comment the resulting token count (`SHAPE_MAX_TOKEN_BYTES / sizeof(svg_xml_token)`, with the number).
3. Add an internal test-only decode entry taking an explicit token budget, declared in `traverse_internal.h`, which `svg_shape_decode` calls with `SHAPE_MAX_TOKEN_BYTES`.
4. Focused tests in a new CTest-registered file under `tests/svg/shape/`: (a) a generated well-formed document over 16 MiB decodes and keeps its shapes; (b) a tiny token budget fails with `SVG_SHAPE_ERROR_LIMIT` and a nonzero offset not beyond the source length; (c) a source of 1 GiB + 1 bytes (anonymous `mmap`, never written) is refused with `SVG_SHAPE_ERROR_LIMIT`; (d) with the allocation stats hook active, shape-tracked live bytes return to zero after destroy on the success path and after the failure of (b); ASan covers the stream tokens.
5. Pre-change evidence in `artifacts/WO-M4-10/pre-change.md`: build the parent base from `git archive 5894cb4` under `.cache/WO-GAP-M4-10/base` with the new test file added; record (a) failing there, and (b) failing at `27e7781` with `SVG_SHAPE_ERROR_XML` instead of `LIMIT`. Tests (c) and (d) are guards that also pass before the change; record that truthfully rather than claiming a failure.
6. Run the parent AC-5 corpus command; write `artifacts/WO-M4-10/corpus-delta.json` with the before histogram (`git show 5894cb4:reports/M4/render-diagnostics.json`), the after histogram, rendered count before/after and the per-category delta.
7. Write `artifacts/WO-M4-10/scope.md`: list the paths `071e0ec` changed inside and outside the parent allowlist, and state that out-of-allowlist paths are governed by WO-RETRO-01, not WO-M4-10.
8. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-GAP-M4-10/check-NN.txt`.

# Acceptance criteria
- AC-1 `./dev verify`, `./dev asan`, `./dev ubsan` and `./dev t --all` exit 0, with output recorded.
- AC-2 Tests (a)-(d) run under CTest and pass; `pre-change.md` records item 5's results.
- AC-3 The corpus command exits 0, its JSON carries manifest digest `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`, and `corpus-delta.json` has the fields of item 6.
- AC-4 `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths; `scope.md` exists per item 7.
- AC-5 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-M4-10 --ready`, `python3 scripts/wo_validate.py WO-GAP-M4-10 --ready` and `git diff --check` exit 0.
- AC-6 A fresh read-only verifier re-judges AC-1..AC-8 of `work/WO-M4-10.md` (AC-4 in the form of item 5, AC-6 in the form of item 7 plus AC-4 here) and AC-1..AC-5 here, and reports pass in `artifacts/WO-GAP-M4-10/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev asan
./dev ubsan
./dev t --all
python3 scripts/corpus_render_report.py --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real --output-json reports/M4/render-diagnostics.json --output-markdown reports/M4/render-diagnostics.md
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-M4-10 --ready
python3 scripts/wo_validate.py WO-GAP-M4-10 --ready
git diff --check
```

# Non-goals
No budget change, no change to DOCTYPE or attribute policy, no edge-storage change, no index/viewer path change.

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 240 min; dependency/network: none.

# Failure and escalation
If the base tree cannot build the new test, record the compile diagnostic as the pre-change result (WO-UI-01 AC-4 precedent). If mapping exhaustion to `LIMIT` changes any corpus verdict other than `xml` to `limit`, record the files and stop for planner review.

# Output
One commit `fix(shape): map token exhaustion to LIMIT and add decode budget tests`, evidence under `artifacts/WO-M4-10/` and `artifacts/WO-GAP-M4-10/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
