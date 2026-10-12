---
id: WO-CORPUS-03
revision: 2
status: READY
milestone: M4
owner: project-owner
executor: deepseek-flash
base_sha: f9a7fb3915d3dee0fbec7ea255614726f247dc6f
spec_sha256: fa07c0b5cbfe985f13d0ab805bfb45bafa6a95efa5ffa4b88a4040cc52815bbb
parallel_safe: false
network: false
---
# Goal
Read-only measurement: render the 19 corpus files that the 32 MiB sweep limit skipped, and time the ~100 MB files against the "100 MB map-like first visible pixel under 2 s" target. Record results only; change no code, budget or threshold.

# Context pointers
- Dependencies: none. This order is read-only measurement and depends on no code order (WO-CORPUS-02, WO-CORPUS-04 or any other). It measures whatever `main` is at run time and records that commit and the binary SHA-256; a later code order does not invalidate it, it only dates it. It may run before, between or after the code orders, but not concurrently with one (`parallel_safe: false`; a concurrent rebuild would change the binary under measurement).
- `artifacts/WO-M4-10/render-diagnostics.json` (`max_bytes` 33554432; 19 `skipped`, 90.5 MB to 1,005,597,835 bytes), `scripts/corpus_render_report.py` (`--max-bytes`, `--timeout`, `--binary`), `src/app/render_frame_cli.c`.
- Target source: `docs/research-r3/plan/01-product-scope.md` line 33 and `docs/research-r3/plan/06-benchmark-and-quality.md` line 14. `docs/adr/0016-product-target-and-gates.md` does not state this number; it makes full-document renders the fidelity metric. Report the target with its true source.
- Corpus (external, read-only): `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real`, manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.

# Existing contracts and invariants
- No source, test, script, threshold or budget change. No corpus byte enters the repository; rendered images stay under `.cache/WO-CORPUS-03/`.
- The render CLI draws the whole document; its wall time is an upper bound on time to first pixel, not the progressive viewer's first pixel. Every timing is labelled that way and no first-pixel claim is made.

# Files allowed to touch
- `artifacts/WO-CORPUS-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- Every other path, including `src/**`, `tests/**`, `scripts/**`, `bench/**`, `docs/**`, `work/**`, `reports/**` and all prior artifacts.

# Required implementation
1. Build Release and run the sweep command below with `--output-json artifacts/WO-CORPUS-03/sweep.json --output-markdown artifacts/WO-CORPUS-03/sweep.md` (the committed copy); the verification list writes the same command's output under `.cache/WO-CORPUS-03/` so a re-run never dirties tracked files.
2. Write a stdlib-only helper `artifacts/WO-CORPUS-03/time_render.py` that first checks the manifest digest, then, for every manifest file of 90,000,000 to 160,000,000 bytes (12 files at this manifest), runs `/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame <file> --size 128x128 --out .cache/WO-CORPUS-03/<id>.ppm` three times with a 60 s timeout each, recording exit code, wall seconds, maximum resident set size and the last stderr line per run.
3. Write `artifacts/WO-CORPUS-03/timing.json`: host (`sysctl -n hw.model`, `sysctl -n machdep.cpu.brand_string`, `sysctl -n hw.memsize`, `sw_vers -productVersion`), commit, binary SHA-256, metric label, target with its source, and per file the runs, the median wall time of exit-0 runs, and `meets_2s` (false when no run exits 0).
4. Write `artifacts/WO-CORPUS-03/summary.md`: for each of the 19 formerly skipped files, its new verdict, category and offset; the histogram of those 19; and the timing table with pass/fail against 2 s.
5. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-CORPUS-03/check-NN.txt`.

# Acceptance criteria
- AC-1 The sweep command exits 0; `sweep.json` has `manifest_sha256` `9fbf9a2f…bc30`, `max_bytes` 1073741824 and `coverage.not_render_tested` 0; each of the 19 files skipped in `artifacts/WO-M4-10/render-diagnostics.json` has a verdict other than `skipped`.
- AC-2 `timing.json` has three runs for each of the 12 files and every field of item 3.
- AC-3 `summary.md` covers all 19 files and all 12 timings and uses the metric label of the invariants.
- AC-4 `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths; `git status --porcelain` is empty afterwards.
- AC-5 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-CORPUS-03 --ready` and `git diff --check` exit 0.
- AC-6 A fresh read-only verifier reports pass in `artifacts/WO-CORPUS-03/verdict.txt`.

# Verification commands
```text
cmake --preset mac-release
cmake --build --preset mac-release
mkdir -p .cache/WO-CORPUS-03
python3 scripts/corpus_render_report.py --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real --binary build/mac-release/bin/svgworkbench --max-bytes 1073741824 --timeout 300 --output-json .cache/WO-CORPUS-03/sweep.json --output-markdown .cache/WO-CORPUS-03/sweep.md
python3 -I artifacts/WO-CORPUS-03/time_render.py --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real --binary build/mac-release/bin/svgworkbench --output .cache/WO-CORPUS-03/timing.json
git show --name-only --format= HEAD
git status --porcelain
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-CORPUS-03 --ready
git diff --check
```
The verifier compares its `.cache/WO-CORPUS-03/` outputs with the committed `artifacts/WO-CORPUS-03/{sweep,timing}.json` by verdict, category and offset per file (timings are compared only for presence of three runs per file, not equality).

# Non-goals
No fix for any failure found, no budget or timeout change in the product, no change to `artifacts/WO-M4-10/` or `reports/`, no progressive first-pixel instrumentation.

# Budget
Executor max runs: 2; verifier max rounds: 1; elapsed hard limit: 240 min; dependency/network: none.

# Failure and escalation
A render that times out, crashes or is killed is a result: record it, do not retry with a larger timeout. If the host runs out of memory or disk, record the file and the condition and stop.

# Output
One commit `docs(corpus): measure the large corpus files and 100 MB render time`, evidence under `artifacts/WO-CORPUS-03/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
