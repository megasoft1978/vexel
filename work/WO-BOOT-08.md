---
id: WO-BOOT-08
revision: 5
status: READY
milestone: M3
owner: sonnet-5.5
executor: codex-gpt-6-luna
base_sha: 97bc3d1cb89d3cddfb48a796868fdb98aada16b7
spec_sha256: 8dc70db6e0c43b44d625e675df6d417e011486058bfab2d2bda813da89005a68
parallel_safe: false
network: false
---
# Goal
Add a macOS-only local performance gate that reports current viewer/index size and performance evidence against a pinned last-green baseline, while failing only after the same metric violates its limit in two consecutive comparable runs.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/PERF.md`, `docs/research-r4/00-verdict-and-changes.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/research-r3/plan/06-benchmark-and-quality.md`, `bench/generate/README.md`, `bench/generate/index_bench.py`, `scripts/README.md`, `dev`. Revision 5 supersedes revisions 1–4 only to advance the verifier comparison base through ADR-0011. Implementation acceptance and allowlist are unchanged.
- Relevant ADRs: `docs/adr/0001-product-and-platform-locks.md`, `docs/adr/0009-boot-08-script-allowlist.md`, `docs/adr/0010-boot-08-verifier-base.md`, `docs/adr/0011-boot-08-scope-parser.md`.
- Existing last-green evidence: `bench/results/WO-M3-01.json`, `bench/results/WO-M3-02.json`, `bench/results/WO-M3-03.json`; use M3-01's same-host 10/100 MiB indexing trial data for comparable first-preview, maximum owner-step, and peak-RSS metrics.
- Frozen tests/oracles: existing `tests/test_svg_index.py`, M3-01 benchmark report and corpus/generated fixture hashes; do not edit them.
- External docs: none.

# Existing contracts and invariants
- Initial support is Apple Silicon macOS 13+ only; do not add Linux/Windows support, stubs, runners, or portability branches.
- Performance comparisons require equal machine model, architecture, compiler identity, workload case, and pinned input hash. Cold/warm labels remain run order; do not claim OS cache eviction.
- A benchmark result is not a pass when required fields are missing. Missing pan timing and app-bundle packaging must be reported with a reason, never as zero or a fabricated pass.
- Performance limits are estimates. A result within one observed violation is `warning`; only a repeated violation of that same metric on a second consecutive comparable run is `fail`. A healthy measurement resets that metric's streak. Incomparable or unavailable measurements do not count as a healthy run.
- The 10 MiB simple TTFP <1 s, 100 MiB map-like TTFP <2 s, main-owner step <100 ms, and 100 MiB RSS ≤500 MiB are measured gates. Cached pan/zoom p95 <33 ms and package-size figures (single-arch stripped binary ≤10 MiB, app bundle ≤25 MiB) are report-only stretch targets until the product can measure/build them meaningfully.
- No public speed or competitor claim is authorized by this work order.

# Files allowed to touch
- `dev`
- `scripts/perf_gate.py`
- `scripts/README.md`
- `bench/generate/index_bench.py`
- `bench/results/perf-gate-baseline.json`
- `bench/results/WO-BOOT-08.json`
- `tests/test_perf_gate.py`
- `docs/PERF.md`
- `artifacts/WO-BOOT-08/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected paths
- `work/**` except this active work order; `OWNER-DIRECTIVES.md`; `docs/adr/**`; `docs/schemas/**`; `tests/oracle/**`; `tests/golden/**`; `bench/corpus/**`; all prior benchmark results and artifacts; all other paths outside the exact allowlist. scripts/perf_gate.py and scripts/README.md are allowed explicitly; do not touch any other scripts path.
- Do not rewrite `bench/results/WO-M3-01.json`, any prior benchmark result, or any frozen fixture/oracle.

# Required implementation
1. Add a reproducible local `./dev perf-gate` command. It builds the macOS Release index benchmark, runs the frozen 5 cold-order + 10 warm-order trials for 10 MiB simple and 100 MiB map-like inputs, and writes raw observations plus summaries to a new report under ignored `.cache/` by default. Do not use hyperfine or add dependencies.
2. Correct the index benchmark's hardware metadata to query the current macOS model, CPU brand, physical memory, architecture, OS, and compiler rather than hard-coding this machine's values. Keep the existing seed, workload content, and trial protocol unchanged.
3. Add a strict report/gate utility that records build SHA, input hashes, toolchain/hardware, trial samples, median/p95/max-step/peak-RSS summaries, Release executable bytes, linked dynamic-library identities from `otool -L`, and app-bundle size or an explicit unavailable reason. Build size and bundle stretch limits are visible but do not fail this gate.
4. Add a hash-pinned baseline record sourced without modification from the last-green M3-01 report and commit. Compare only matching workload hashes and matching host/toolchain identity; otherwise emit `incomparable` and do not mutate regression streaks. Preserve baseline provenance in the result.
5. Track per-metric consecutive violations in an explicit history JSON path under `.cache/`. First comparable breach produces `warning`/exit 0; the same metric breaching again on the next comparable observation produces `fail`/exit 1; a non-breaching observation resets the streak. Missing metrics do not count as healthy observations. Reject malformed, non-finite, negative, or unknown report/history fields with a clear error.
6. Report pan/zoom event-to-present p95 as unavailable with its reason because the current navigation test is a correctness workload and does not time presentation. Do not relabel spatial-query timing as pan latency. Preserve the 100 MiB first-preview/step/RSS gate while making this gap visible to M3-05.
7. Add focused tests for baseline comparison, first/second violation, noisy single-run reset, independent per-metric streaks, incomparable host/workload, unavailable metrics, malformed inputs, and deterministic output. Record the run evidence and update `docs/PERF.md`, `scripts/README.md`, `STATE.md`, and ledgers.

# Acceptance criteria (machine-checkable)
- AC-1: `./dev perf-gate --help` documents the command; `./dev perf-gate --output .cache/WO-BOOT-08/result.json --history .cache/WO-BOOT-08/history.json` completes on this Apple Silicon host and records exactly 5 + 10 trials per supported benchmark case with source/build hashes and hardware/toolchain metadata. Evidence: `artifacts/WO-BOOT-08/perf-gate.json`.
- AC-2: the result records Release executable byte size and `otool -L` dependencies, records app bundle bytes or an explicit unavailable reason, and marks pan/zoom p95 unavailable with an explicit reason. No unavailable metric is encoded as zero. Evidence: `artifacts/WO-BOOT-08/perf-gate.json`.
- AC-3: `bench/results/perf-gate-baseline.json` identifies the M3-01 commit, exact report hash, host identity, workload source hashes, and measured baseline values; the live run either compares to that exact baseline or says `incomparable` without altering streaks. Evidence: baseline and result JSON.
- AC-4: injected metric sequences prove one breach warns, two consecutive breaches of the same metric fail, a healthy observation resets the streak, another metric's streak is independent, and unavailable/incomparable observations do not advance streaks. `python3 -m unittest tests.test_perf_gate` exits 0. Evidence: `artifacts/WO-BOOT-08/gate-tests.json`.
- AC-5: the gate exposes stretch targets without failing on them; malformed inputs fail closed; report serialization is deterministic apart from explicitly declared run timestamp/raw timing observations. Evidence: `artifacts/WO-BOOT-08/gate-contract.json`.
- AC-6: `./dev verify`, `./dev asan`, `./dev ubsan`, `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-BOOT-08 --ready`, JSON validation of committed result/baseline/evidence, and `git diff --check` pass. Evidence: `artifacts/WO-BOOT-08/full-checks.txt`.

# Verification commands

```text
./dev perf-gate --help
python3 -m unittest tests.test_perf_gate
./dev perf-gate --output .cache/WO-BOOT-08/result.json --history .cache/WO-BOOT-08/history.json
python3 -m json.tool .cache/WO-BOOT-08/result.json
./dev verify
./dev asan
./dev ubsan
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-BOOT-08 --ready
python3 -m json.tool bench/results/perf-gate-baseline.json
python3 -m json.tool bench/results/WO-BOOT-08.json
python3 -m json.tool artifacts/WO-BOOT-08/perf-gate.json
python3 -m json.tool artifacts/WO-BOOT-08/gate-tests.json
git diff --check
```

# Non-goals
Changing renderer/index algorithms; adding a macOS app bundle or packaging/signing flow; producing pan timing without an end-to-end presented-frame signal; competitor benchmarks; hosted performance CI without a matching dedicated Mac; adding dependencies; changing prior benchmark baselines; Linux/Windows support; public performance claims.

# Budget
Executor max runs: 3; verifier max rounds: 2 (standing planner authorization permits another round after fixing genuine findings); elapsed hard limit: 180 min; dependency/network: none.

# Failure and escalation
If the current host differs from the pinned M3-01 baseline, retain the complete measurements, label comparison `incomparable`, and report why; do not relax the identity check. If any required metric cannot be collected, report its explicit reason and continue with the remaining metrics. After three genuine repair attempts, retain evidence in `artifacts/WO-BOOT-08/BLOCKER.md` and stop.

# Output
Commit with `perf: add macOS size and performance gates`; retain raw trials, summaries, injected-gate tests, and exact verifier evidence under `artifacts/WO-BOOT-08/` and `artifacts/runs/WO-BOOT-08/`. Report the next work order as M3-05, whose owner review is a human gate.
