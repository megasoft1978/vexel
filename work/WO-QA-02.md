---
id: WO-QA-02
revision: 6
status: READY
milestone: M2
owner: project-owner
executor: deepseek-flash
base_sha: ef09a95423604fc4b41a9802bb9a4f3c94511cc7
spec_sha256: fd4a95b5fc5ffcc4a55d9d4d28ae77eab0032774a5a48c25ed53e5b293bb25a4
parallel_safe: false
network: false
---
# Goal
Close two real gaps in the test process: the animation and XML tokenizer fuzzers are not run by the standard final gate, and running `./dev fuzz-xml` rewrites a pinned milestone artifact, silently invalidating `reports/M3/validate_packet.py`.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/testing.md`, `dev`, `tests/fuzz/shape/shape_fuzz.py`, `scripts/xml_fuzz_campaign.py`, `reports/M3/validate_packet.py`, `.github/workflows/fuzz.yml`, `tests/anim/README.md`.
- `./dev final` currently runs verify, asan, ubsan, shape-fuzz, mcp-fuzz, e2e, soak, editor-smoke, viewer-smoke. It does not run `xml_tokenizer_fuzz` or `animation_fuzz`.
- `./dev fuzz-xml` writes `artifacts/WO-M2-03/pr-smoke.json` (or `scheduled-campaign.json` at 600 s). `reports/M3/validate_packet.py` pins the SHA-256 of both files, so any dev invocation corrupts a milestone evidence packet.
- `tests/fuzz/shape/shape_fuzz.py` defaults `--output` to `artifacts/WO-M4-04/fuzz-report.json`; `./dev final` overrides it, but a bare `./dev shape-fuzz` also writes into a prior work order's artifact.
- `build/mac-asan/bin/animation_fuzz` takes `<cases> <seed>` and has no `./dev` wrapper.

# Existing contracts and invariants
- Pinned evidence under `artifacts/WO-*/` and the hashes in `reports/M3/validate_packet.py` are immutable; no command the project documents may rewrite them.
- `tests/e2e/test_dev_final.py` asserts that the final gate runs shape-fuzz with a seed; that assertion must keep passing and may be extended, never weakened.
- `./dev` stays POSIX `sh`, macOS-only, with no new dependency and no network access.
- Fuzz seeds stay deterministic and recorded; every gate keeps a fixed seed so failures are reproducible.

# Files allowed to touch
- `dev`
- `docs/testing.md`
- `tests/e2e/test_dev_final.py`
- `tests/fuzz/shape/shape_fuzz.py`
- `scripts/xml_fuzz_campaign.py`
- `.github/workflows/fuzz.yml`
- `docs/release.md` (only if it names the fuzz report paths)
- `artifacts/WO-QA-02/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`
- `work/WO-QA-02.md` (this order's own frozen spec)

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `src/**`, `app/**`, `CMakeLists.txt`, `tests/oracle/**`, `tests/golden/**`, `bench/**`, `reports/**` and every existing `artifacts/WO-*` path, including `artifacts/WO-M2-03/pr-smoke.json` and `artifacts/WO-M4-04/fuzz-report.json`, which must end this order byte-identical to their committed state.

# Required implementation
1. Change `./dev fuzz-xml` so its default reports go under `.cache/qa/`, and make **all** of its evidence follow the report path: `scripts/xml_fuzz_campaign.py` currently writes failure inputs and raw output to the fixed directory `artifacts/WO-M2-03/failures/**` regardless of `--output`, including the tracked `failures/synthetic-canary/**` canary files. Derive the failure directory from the requested report's parent instead, so a campaign without an explicit `--output` writes nothing under `artifacts/`. (for example `.cache/qa/xml-fuzz-pr-smoke.json` and `.cache/qa/xml-fuzz-scheduled.json`). Add an explicit `--output PATH` option so a maintainer can still deliberately write the pinned WO-M2-03 paths when refreshing that packet; without `--output`, no path under `artifacts/` may be written. Relax `scripts/xml_fuzz_campaign.py`'s output confinement from `.cache/WO-M2-03` + `artifacts/WO-M2-03` to the whole ignored `.cache/**` tree plus `artifacts/WO-M2-03/**`, so routine campaigns may write under `.cache/qa/` while the pinned packet stays the only permitted `artifacts/` destination.
2. Change `tests/fuzz/shape/shape_fuzz.py` so its default `--output` is under `.cache/qa/` rather than an `artifacts/WO-M4-04/` path.
3. Add a `./dev animation-fuzz [cases] [seed]` command with documented defaults (suggested: 200000 cases and seed 20261012), building the ASan `animation_fuzz` target and running it, with argument validation matching the other fuzz commands.
4. Add both fuzzers to the `./dev final` gate: an XML campaign step and an animation campaign step with fixed seeds, placed after the existing shape-fuzz step. Keep the whole gate bounded (suggested 30 s XML and 200000 animation cases).
5. Update the usage text, `docs/testing.md`'s final-gate description, and `.github/workflows/fuzz.yml` so every referenced report path still exists and no workflow expects a file the new defaults no longer write. The workflow keeps writing the pinned `artifacts/WO-M2-03/` packet, but must now do so with an explicit `--output`, because CI produces triage artifacts and `scripts/check_fuzz_workflow.py` confines failure logs to the task evidence directory; the local default is unchanged at `.cache/qa/`. Add the missing `push` trigger to `fuzz.yml` so the workflow satisfies `scripts/check_ci.py`'s required trigger set (`push`, `pull_request`, `workflow_dispatch`), matching `nightly.yml`.
6. Extend `tests/e2e/test_dev_final.py` so each of the four fuzzer steps is asserted against its exact bound and seed (`--seed 20261010` for shape, `--duration 30 --seed 20261010` for XML, `200000 20261010` for animation and MCP), the XML failure directory follows the report path, and no `./dev` command writes a default report under `artifacts/`. Assertions must not be vacuous: a step that drops its bound or seed must fail the test. Pass the MCP bound and seed explicitly in the final gate so they are assertable.
7. Run `python3 scripts/check_ci.py .github/workflows/fuzz.yml` and confirm it passes, and confirm `git diff --name-only` lists no `artifacts/WO-M2-03/` or `artifacts/WO-M4-04/` path.

# Acceptance criteria (machine-checkable)
- AC-1 `./dev fuzz-xml --duration 5 --seed 20261012` exits 0 and `git status --porcelain` shows no modification under `artifacts/`; the report it writes is under `.cache/`.
- AC-2 `python3 scripts/check_ci.py .github/workflows/fuzz.yml` exits 0, and `artifacts/WO-M2-03/pr-smoke.json` and `scheduled-campaign.json` are byte-identical to `HEAD` (`git diff --name-only` lists neither).
- AC-3 `./dev animation-fuzz 100000 20261012` exits 0 and reports zero crashes, invariant failures and sanitizer reports.
- AC-4 `./dev shape-fuzz --seed 20261010` (without `--output`) writes only under `.cache/` and exits 0.
- AC-5 `python3 -m unittest tests.e2e.test_dev_final` exits 0 and covers the new assertions.
- AC-6 `./dev final` exits 0 end to end on this host and its step list includes both new fuzz steps.
- AC-7 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-QA-02 --ready` and `git diff --check` exit 0.
- AC-8 A fresh read-only verifier reports pass against the exact candidate commit.

# Verification commands
Run sequentially from the repository root unless the command changes directory itself.

```text
python3 -m unittest tests.e2e.test_dev_final
./dev fuzz-xml --duration 5 --seed 20261012
./dev animation-fuzz 100000 20261012
./dev shape-fuzz --seed 20261010
python3 scripts/check_ci.py .github/workflows/fuzz.yml
./dev final
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-QA-02 --ready
git diff --check
```

# Non-goals
No change to fuzz targets, sanitizer configuration, the renderer, or any pinned evidence content. No new dependency. No change to the meaning of the existing WO-M2-03 or WO-M4-04 acceptance criteria; this order only stops routine commands from rewriting their artifacts.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 180 min; dependency/network: none.

# Failure and escalation
If `reports/M3/validate_packet.py` cannot pass without rewriting the pinned files, stop and record the mismatch rather than regenerating milestone evidence. If `./dev final` cannot stay under a reasonable wall-clock bound with the two new steps, reduce the case counts and record the chosen bounds.

# Output
One implementation commit using `test: run every fuzzer in the final gate and keep pinned evidence read-only`. Include the command evidence under `artifacts/WO-QA-02/`, update `STATE.md` and both ledgers, and provide the schema-valid executor report.
