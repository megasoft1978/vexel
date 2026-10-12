---
id: WO-CORPUS-04
revision: 1
status: READY
milestone: M4
owner: project-owner
executor: deepseek-flash
base_sha: f9a7fb3915d3dee0fbec7ea255614726f247dc6f
spec_sha256: f4ba30a0f9b15df9f25e08c8a1f257aadc649f3ba1ca7f7d2991c551af3e3b8c
parallel_safe: false
network: false
---
# Goal
Lift the resolution-bound raster refusal that stops `generated-science_t1` and `generated-embedded_t2`. Both decode, then the raster stage refuses them with `limit at byte 0` at their native 1600x1000 size. The aim is to raise the corpus `ok` count above 3 without changing a single output pixel of any document that renders today.

# Context pointers
- Planner scratch evidence at `f9a7fb3`, using the existing Release binary on copies outside the repository:
  - With the root `width`/`height` rewritten from 1600x1000 to 160x100 (same `viewBox`), `science_t1` and `embedded_t2` exit 0. Their only blocker is therefore a raster budget that scales with pixel count.
  - The same rewrite leaves `cad_t1`, `map_t1`, `map_t2`, `giant_path_t1` and `giant_path_t2` refused at byte 0, so their raster sites do not depend on resolution. They are out of scope here; `map_*` belongs to WO-CORPUS-02.
- Why not another group from `artifacts/WO-CORPUS-01/residual-causes.md`? The same scratch method showed that none of them raises `ok` alone:
  - Moving `<clipPath>` into `<defs>` turns `natural-earth-*` into a raster `limit` at byte 0, `bulgaria` into `unsupported` at byte 15068044, and `cambodia` into `limit`.
  - Normalising path numbers like `-.4.6` (rejected today as `path`; SVG allows them) turns `germany` into `limit` near the end of the file and `france` into `unsupported` at `<symbol overflow="visible">`.
- `viewer_load_document` (`src/app/viewer_load.c`) decodes the document and then calls `raster_shape_render` at native size. Raster refusals set `{SVG_SHAPE_ERROR_LIMIT, 0}` directly.
- Candidate pixel-scaled budgets: `RASTER_PREFLIGHT_MAX_WORK` (200,000,000) in `src/raster/shape_render_preflight.c` and `RASTER_BOUNDARY_WORK_LIMIT` (200,000,000) in `src/raster/shape_bounds.c`. Other candidates are in `src/raster/shape_render_internal.h` and `src/raster/shape_scanline_fill.c`. The probe decides which one applies; do not assume.
- Corpus (external, read-only): `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real`. The SHA-256 of `manifest.json` is `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`, pinned by `scripts/corpus_render_report.py`. The current histogram (`artifacts/WO-CORPUS-01/delta.json`) is ok 3, limit 17, unsupported 11, xml 4, path 2, viewport 2, skipped 19.
- `lldb` hangs on this host, so do not use it. Use the probe method of WO-CORPUS-02 revision 2, item 1.

# Existing contracts and invariants
- Pixels do not change. Every golden, oracle, reference and `render-frame` test passes unchanged and no fixture is modified. This order changes only whether a document is refused, never how an accepted one is drawn. Rendering at the output size instead of native size is out of scope.
- Every budget stays finite and named. If a budget becomes a function of pixel count, it has a finite cap. The header comment states the worst-case work and bytes.
- Ceilings: each of the two target files renders through Release `render-frame` at 128x128 in at most 60 s of wall time with at most 2 GiB maximum RSS, measured with `/usr/bin/time -l`. A document over the new budget still fails with `SVG_SHAPE_ERROR_LIMIT`.
- Probe instrumentation is temporary and is never committed. It lives only in a scratch worktree under `.cache/WO-CORPUS-04/`.
- macOS only, C17, no new dependency, files under 800 lines. No corpus byte enters the repository.

# Files allowed to touch
- `src/raster/**`, `tests/raster/**` and `tests/svg/**` (no fixtures), `CMakeLists.txt`, `artifacts/WO-CORPUS-04/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `scripts/**`, `tests/oracle/**`, `tests/golden/**`, every golden, reference or fixture file, `src/app/**`, `src/svg/**`, `bench/**`, `reports/**`, every prior artifact, and every path outside the allowlist.

# Required implementation
1. **Probe.** Create a scratch worktree with `git worktree add --detach .cache/WO-CORPUS-04/probe HEAD`. Instrument every `SVG_SHAPE_ERROR_LIMIT` site in `src/raster/**`; each site prints `__FILE__:__LINE__`, `__func__`, the budget and the value reached, plus `backtrace()` with `dladdr` names. Build Release there and run `science_t1` and `embedded_t2` at 128x128. Record the site, the value reached and the budget in `artifacts/WO-CORPUS-04/probe.md`.
2. **Design.** Write `artifacts/WO-CORPUS-04/design.md` with the new value or formula of the probed budget and its finite cap. Include the worst-case arithmetic, and the measured wall time and RSS for the two files in the probe worktree. If the probed site is not a work or time budget (for example a memory ceiling or an allocation-size check), do not change it: record it and stop (see Failure and escalation).
3. **Implement** the budget change in the main tree. Remove the scratch worktree with `git worktree remove --force .cache/WO-CORPUS-04/probe`.
4. **Tests under CTest.** Add one test where a generated document whose work is just inside the new budget renders, and one where a document just over it fails with `LIMIT` at offset 0.
5. **Measure.** Run the two commands below and record the exit code, last stderr line, wall seconds and maximum RSS in `artifacts/WO-CORPUS-04/measure.json`. Run the sweep into `artifacts/WO-CORPUS-04/render-diagnostics.{json,md}`. Write `artifacts/WO-CORPUS-04/delta.json` with the histogram before (`artifacts/WO-CORPUS-01/delta.json` `histogram_after`) and after, plus per-file verdict changes.
6. **Next blockers.** For each target file still not `ok`, re-run the item 1 probe on the final tree. Record its next site (`file:line`, function, budget) under `## Remaining blockers` in `probe.md`.
7. Record each verification command, its exit code and the last 40 lines of output in `artifacts/WO-CORPUS-04/check-NN.txt`.

# Acceptance criteria
- AC-1 `./dev verify`, `./dev asan`, `./dev ubsan` and `./dev t --all` exit 0, with output recorded.
- AC-2 `./dev shape-fuzz` exits 0.
- AC-3 The committed sweep keeps `manifest_sha256` `9fbf9a2f…bc30`, `max_bytes` 33554432 and `render_tested` 39. Either `histogram.ok` is greater than 3, or `probe.md` `## Remaining blockers` names the next site for each target file that is not `ok`.
- AC-4 No file that is `ok` in `artifacts/WO-M4-10/render-diagnostics.json` is not `ok` after.
- AC-5 Every target file that exits 0 in `measure.json` does so in at most 60 s with at most 2 GiB maximum RSS.
- AC-6 The tests of item 4 run under CTest and pass. `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths and no fixture. `git worktree list` shows no `.cache/WO-CORPUS-04/probe` entry.
- AC-7 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-CORPUS-04 --ready` and `git diff --check` exit 0.
- AC-8 A fresh read-only verifier reports pass in `artifacts/WO-CORPUS-04/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
cmake --preset mac-release
cmake --build --preset mac-release
./dev verify
./dev asan
./dev ubsan
./dev t --all
./dev shape-fuzz
mkdir -p .cache/WO-CORPUS-04
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/science_t1.svg --size 128x128 --out .cache/WO-CORPUS-04/science_t1.ppm
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/embedded_t2.svg --size 128x128 --out .cache/WO-CORPUS-04/embedded_t2.ppm
python3 scripts/corpus_render_report.py --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real --binary build/mac-release/bin/svgworkbench --output-json .cache/WO-CORPUS-04/render-diagnostics.json --output-markdown .cache/WO-CORPUS-04/render-diagnostics.md
git worktree list
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-CORPUS-04 --ready
git diff --check
```
The two `render-frame` commands may exit 1 when AC-3's second condition holds; their exit codes are recorded. The verifier's sweep writes to `.cache/` and is compared with the committed `artifacts/WO-CORPUS-04/render-diagnostics.json` by histogram and per-file verdict.

# Non-goals
- No render-at-output-size path, no change to `viewer_load.c` or `render_frame_cli.c`, and no tiling.
- No change to decode-stage budgets, `clipPath` placement, path number syntax, `<symbol>`, `<use>` or `stroke="url(#…)"`.
- No work on the resolution-independent byte-0 refusals of `cad_t1`, `map_*` and `giant_path_*`.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 180 min; dependency/network: none.

# Failure and escalation
- If the probed site is a memory ceiling or allocation-size check rather than a work budget, or if meeting the 60 s / 2 GiB ceiling needs a budget the probe shows is unsafe, change no code. Commit `probe.md` and `design.md` with the numbers and stop for planner review.
- Never raise a ceiling stated in the invariants.

# Output
One commit `perf(raster): scale the native-resolution work budget for large canvases`, evidence under `artifacts/WO-CORPUS-04/`, updated `BLOCKERS.md` defect 2, `STATE.md` and both ledgers, and the schema-valid executor report.
