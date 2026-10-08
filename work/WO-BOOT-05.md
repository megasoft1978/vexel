---
id: WO-BOOT-05
revision: 3
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: e0f3a5a3365e58979812d0c004f6c6523698556b
spec_sha256: eda0c890c282c25c1ad9522ff102c74c74472cdbc0b1cf63a89fd7ccf66ceec1
parallel_safe: false
---
# Goal
Add a repeatable machine-readable Python test runner, macOS ASan/UBSan build profiles, and a deterministic CLI fuzz smoke/scheduled workflow foundation.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, this file, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/03-wrapper-scripts-spec.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/CODING_STANDARD.md`, `scripts/README.md`, `CMakeLists.txt`, `CMakePresets.json`, `dev`.
- Relevant ADRs: `docs/adr/0001-product-and-platform-locks.md`, `docs/adr/0003-codex-cli-compatibility-amendment.md`.
- BOOT-05 recovery amendment: `docs/adr/0004-strict-json-schema-compatibility.md`.
- Frozen tests/oracles: do not edit `tests/oracle/**`, `tests/golden/manifest.lock`, or `bench/corpus/manifest.lock`.
- External docs: use local Apple Clang/CMake help and official CMake documentation only if a profile option is unclear. No external source code or downloads.

# Existing contracts and invariants
- C17, Apple Silicon macOS 13+ only, Xcode Apple Clang; no Linux/Windows code, CI, runners, or shims.
- Minimal dependencies; Python standard library only for the harness.
- Keep generated builds, temporary fault probes, and transient machine results under ignored `build/` or `.cache/`; retained work-order evidence goes under `artifacts/WO-BOOT-05/`.
- Deterministic seed reporting; no silent sanitizer/fuzzer skips; expected injected failures must be separated from real regressions.
- Fuzz only the existing CLI argument boundary; there is no SVG parser in this milestone.

# Files allowed to touch
- `CMakeLists.txt`, `CMakePresets.json`, `dev`, `.gitignore`
- `.github/workflows/fuzz.yml`
- `scripts/test_runner.py`, `scripts/fuzz_smoke.py`, `scripts/sanitizer_probes.py`, `scripts/check_fuzz_workflow.py`, `scripts/README.md`
- `tests/test_boot05.py`, `tests/README.md`, `tests/fuzz/README.md`, `tests/fuzz/**`
- `docs/schemas/test-results.schema.json`, `docs/ci.md`
- `docs/schemas/executor-report.schema.json`, `docs/adr/0004-strict-json-schema-compatibility.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `artifacts/WO-BOOT-05/**`
- `scripts/wo_driver.py` and `tests/test_wo_runner.py` only for the run-specific Codex preflight evidence path regression and strict per-work-order output schema generation described in Required implementation.

# Protected paths
- `OWNER-DIRECTIVES.md`
- `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`
- `app/**`, `src/**`, `platform/**`, `.github/workflows/ci.yml`
- `work/WO-BOOT-05.md` after it is frozen READY

# Required implementation
1. Add `scripts/test_runner.py`, using only Python stdlib, to discover/run the existing unittest suite and emit schema-valid JSON with UTC start/end, elapsed seconds, counts, and per-test IDs/statuses. Record skips explicitly; fail if a test is skipped or fails. `./dev test-fast` writes transient JSON beneath `.cache/test-results/` and keeps the fast loop under 30 seconds.
2. Add CMake `mac-asan` (AddressSanitizer + UndefinedBehaviorSanitizer) and `mac-ubsan` profiles/build/test presets. CMake must fail clearly if the requested compiler/profile is unsupported. Register a CTest smoke for `svgworkbench --version`; do not disable existing warning-as-error flags.
3. Add `./dev asan` and `./dev ubsan`, each configuring/building the named preset and running CTest with failure output. Add a macOS-only `./dev fuzz` command that builds the ASan profile and invokes the deterministic CLI-argument fuzzer.
4. Add `scripts/fuzz_smoke.py`: bounded printable/UTF-8 argv mutations, fixed default seed with explicit override, minimum one invocation, default 3-second smoke and maximum 60 seconds, per-process timeout, JSON result with seed/iterations/duration, and retained case plus stdout/stderr on crash, timeout, sanitizer report, or unexpected exit. Invalid CLI usage is expected; signals and sanitizer findings are failures. No network.
5. Add `scripts/sanitizer_probes.py`: compile temporary assertion and heap-overflow fixtures with the selected sanitizer flags under `.cache/`; confirm each is detected with a nonzero exit and an identifiable diagnostic; retain exact source, stderr, and probe JSON beneath `artifacts/WO-BOOT-05/probes/`. Missing sanitizer support is a failure, never a skip.
6. Add `.github/workflows/fuzz.yml`, macOS Apple Silicon only, with a short PR/manual smoke and a scheduled 60-second run. Use pinned actions, no dependency installation, no secrets, bounded job time, and upload no user data. Add a checker that rejects non-macOS runners and confirms the schedule uses `./dev fuzz --duration 60`.
7. Fix BOOT-04 runner preflight artifact placement: executor/verifier CLI version/help evidence must live under the current ignored `artifacts/runs/<WO-ID>/<run-id>/` directory, never hard-coded to WO-BOOT-04. Add regression coverage proving a new WO's root path guard does not see an out-of-scope BOOT-04 artifact mutation.
8. Document the module contracts, exact commands, result JSON, seed behavior, and the distinction between expected injected probe failures and real test failures. Update `STATE.md`, both ledgers, and retained artifacts only with observed results.
9. Remove the unsupported `minProperties` keyword from `docs/schemas/executor-report.schema.json`. The runner already enforces an exact, non-empty set of acceptance IDs and executed command evidence after parsing, so strict structured output compatibility must not weaken runtime validation. Add a regression test for empty/missing acceptance evidence.
10. Generate a strict executor report schema under the ignored per-run directory with an explicit property and required key for each frozen AC ID, because strict Codex output requires object properties to be enumerated. Continue validating the returned report against the shared schema and exact work-order AC set locally.

# Acceptance criteria (machine-checkable)
- AC-1 `./dev test-fast` exits 0 in under 30 seconds and writes schema-valid machine-readable results with zero failed or skipped tests; evidence `artifacts/WO-BOOT-05/fast-results.json`.
- AC-2 `./dev asan` and `./dev ubsan` both configure, build warning-clean, and pass CTest; evidence `artifacts/WO-BOOT-05/asan.txt` and `ubsan.txt`.
- AC-3 `./dev fuzz --duration 3 --seed 12648430` reports a fuzz window from 3 to 6 seconds, performs at least one case, records the exact seed and iteration count, and exits 0; evidence `artifacts/WO-BOOT-05/fuzz-smoke.json`.
- AC-4 `python3 scripts/sanitizer_probes.py --profile asan-ubsan` detects both injected faults, retains source and sanitizer trace, and returns 0 only when both are detected; evidence `artifacts/WO-BOOT-05/probes/probes.json`.
- AC-5 `.github/workflows/fuzz.yml` is macOS-only, has PR/manual 3-second smoke and scheduled 60-second run; `python3 scripts/check_fuzz_workflow.py` and `actionlint .github/workflows/fuzz.yml` pass; evidence `artifacts/WO-BOOT-05/workflow.txt`.
- AC-6 Fake-Codex runner test confirms preflight evidence is scoped under the active run directory and `assert_root_scope` accepts it; all new tests pass under `./dev verify`; evidence `artifacts/WO-BOOT-05/tests.txt`.
- AC-7 `python3 scripts/ledger.py validate ledger.jsonl`, `git diff --check`, and changed-path validation pass; evidence `artifacts/WO-BOOT-05/local-checks.txt`.
- AC-8 strict Codex schema preflight succeeds with the repository's executor report schema, and the runner continues rejecting empty or missing acceptance evidence.
- AC-9 the generated schema contains exactly the frozen acceptance IDs as required properties and disallows additional keys.

# Verification commands

```text
./dev test-fast
./dev asan
./dev ubsan
./dev fuzz --duration 3 --seed 12648430
python3 scripts/sanitizer_probes.py --profile asan-ubsan
python3 scripts/check_fuzz_workflow.py
python3 -m unittest tests.test_ledger tests.test_wo_runner tests.test_boot05
python3 scripts/ledger.py validate ledger.jsonl
actionlint .github/workflows/fuzz.yml
git diff --check 0bc1cfdc9f990878f087dfff936d12fb175f4678 HEAD
```

# Non-goals
No SVG/XML parser, image corpus, external dependencies, release packaging, other-OS support, UI changes, or production performance claims.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 120 min; output-token soft cap: 15000; dependency/network: none.

# Failure and escalation
Stop if this Xcode toolchain cannot run both sanitizers or the scheduled runner cannot be pinned to Apple Silicon macOS. Report exact compiler/runtime evidence; do not weaken sanitizer coverage or silently skip a profile. Retry only for implementation/test defects.

# Output
Commit convention: `test: add sanitizer and fuzz foundation`. Candidate changes must remain within the allowlist. Save all evidence under `artifacts/WO-BOOT-05/`; final response reports exact suite counts, seed/iterations, sanitizer probe results, and known limits.
