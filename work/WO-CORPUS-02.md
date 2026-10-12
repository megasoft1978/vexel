---
id: WO-CORPUS-02
revision: 2
status: READY
milestone: M4
owner: project-owner
executor: deepseek-flash
base_sha: f9a7fb3915d3dee0fbec7ea255614726f247dc6f
spec_sha256: d5e854a86835698d90944d17bb5f4344776cc50e8e343703619126b47ddf7ca4
parallel_safe: false
network: false
---
# Goal
Find every limit site that refuses the five generated corpus files `generated-cad_t2`, `generated-cnc_t2`, `generated-science_t2`, `generated-map_t1` and `generated-map_t2`, using a symbol-resolving probe, then lift the sites that are fixed-count budgets with a documented design that keeps every memory ceiling finite and explicit. Each file must either render or have its next blocking site named and recorded. Revision 1 failed because its premise was wrong (`artifacts/WO-CORPUS-02/probe.md`).

# Context pointers
- Revision 1 findings: `artifacts/WO-CORPUS-02/probe.md`, `BLOCKERS.md` defect 1. `cad_t2` is refused at byte 2,680,259 by `SVG_SHAPE_MAX_SHAPES` (65,536 elements, checked in `svg_shape_builder_begin_shape` in `src/svg/shape/shape_geometry.c`). The file has about 106k paths, 53k rects, 53k lines and 42k text elements. A scratch build with shapes 1,048,576 and edges 16,777,216 still failed with `limit at byte 9448701` at a site nobody has identified, with peak RSS about 375 MB. The edge array was never the refusing site.
- Current verdicts (`artifacts/WO-CORPUS-01/delta.json`): `cad_t2` limit@2680259, `cnc_t2` limit@2565705, `science_t2` limit@2709482, `map_t1` limit@0, `map_t2` limit@0.
- A `limit at byte 0` comes from the raster stage, not from decode. `viewer_load_document` (`src/app/viewer_load.c`) decodes, then calls `raster_shape_render` at the document's native size. Raster refusals set `{SVG_SHAPE_ERROR_LIMIT, 0}` directly and do not call `svg_shape_fail`; the sites are in `src/raster/shape_render.c`, `shape_render_preflight.c` (`RASTER_PREFLIGHT_MAX_WORK`), `shape_bounds.c` (`RASTER_BOUNDARY_WORK_LIMIT`), `shape_render_internal.h` (dash cache and stroke grid budgets), `shape_scanline*.c` and `tile.h`. A planner scratch run found that `map_t1` and `map_t2` are still refused at byte 0 when the root `width`/`height` are rewritten to 160x100, so their raster site does not depend on resolution.
- Decode-stage budgets: `src/svg/shape/shape.h` (`SVG_SHAPE_MAX_EDGES`, `SVG_SHAPE_MAX_SHAPES`, `SVG_SHAPE_MAX_COMMANDS`), `src/svg/shape/traverse_internal.h` (traversal visits, transform operations and nodes, use instances), and the text and glyph budgets in `src/svg/shape/text.c`. `SVG_SHAPE_MAX_EDGES` also bounds `path_subpath_count` (`shape_path.c`) and `opacity_group_count` (`traverse.c`, `src/raster/shape_render_group.c`). `src/raster/shape_render.c:165` checks `shape_count > SVG_SHAPE_MAX_EDGES`.
- Code outside `src/svg/shape` that reads `document->edges` or `SVG_SHAPE_MAX_EDGES`: `src/raster/*`, `src/svg/paint/paint.c`, `src/svg/clip/clip.c`, `src/app/viewer_editor_path_node_commands.c`, `src/app/viewer_editor_path_curves.c`, `tests/anim/anim_test.c`, `tests/editor/editor_followups_test.c`, `tests/fuzz/shape/shape_fuzz.c`, and `tests/svg/shape/*`.
- Corpus (external, read-only): `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real`. The SHA-256 of `manifest.json` is `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`, which `scripts/corpus_render_report.py` pins. Revision 1 mislabelled this value as the file digest. The per-file SHA-256 values from the manifest are `cad_t2` `2ddafd2eeb039efea9345295dbe8f56c53e16935c1699d01a5f28d52105cd934`, `cnc_t2` `f7408a2922737ba2492e0edba4491555a75c86ea684dc997d6ecf26f33f28fff`, `science_t2` `cd388b43a316494802ee7675f65a7a88bb8d26cfdd57b1b89a05494d3c6e9815`, `map_t1` `aefe1044cd8a9bb66f2e75893cdea6719abb0050fb4ca2c76f96fb63b9225a3d` and `map_t2` `53add44762c94b327f59bf8fa9e6368b42325f2e8fe15f863a6ac698eeb52ee3`.
- `HANDOFF.md` lesson 2: run a symbol-resolving probe before touching a limit. `lldb` hangs on this host, so do not use it.

# Existing contracts and invariants
- Rendered output for every input that renders today is unchanged. Every golden, oracle and reference test passes and no fixture is modified.
- Every budget stays finite. A changed budget is either a named constant or a named formula with a finite cap, defined in a header. A comment states the worst-case bytes, and `design.md` repeats them.
- Memory ceilings stay explicit and enforced. The worst-case decoded document model (all per-element, per-edge and per-subpath arrays at their caps) is at most 1 GiB. No single allocation is larger than 256 MiB. A Release `render-frame` at 128x128 on each of the five files peaks at no more than 2 GiB of resident memory and finishes within 120 s of wall time, measured with `/usr/bin/time -l`.
- Going over any budget or ceiling fails cleanly with `SVG_SHAPE_ERROR_LIMIT`. It never aborts or exhausts memory.
- Probe instrumentation is temporary and is never committed. It lives only in a scratch worktree under `.cache/WO-CORPUS-02/`.
- macOS only, C17, no new dependency, files under 800 lines. No corpus byte enters the repository.

# Files allowed to touch
- `src/svg/shape/**`, `src/svg/paint/**`, `src/svg/clip/**`, `src/raster/**`, `src/app/viewer_editor_path_node_commands.c`, `src/app/viewer_editor_path_curves.c`.
- `tests/svg/**`, `tests/raster/**`, `tests/anim/anim_test.c`, `tests/editor/editor_followups_test.c`, `tests/fuzz/shape/shape_fuzz.c`. No fixture under any of these paths.
- `CMakeLists.txt`, `artifacts/WO-CORPUS-02/r2/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `scripts/**`, `tests/oracle/**`, `tests/golden/**`, and every golden, reference or fixture file.
- `src/svg/xml/**`, `src/app/**` other than the two files allowlisted above, `bench/**`, `reports/**`, `artifacts/WO-M4-10/**`, the revision 1 files directly under `artifacts/WO-CORPUS-02/`, every other prior artifact, and every path outside the allowlist.

# Required implementation
1. **Probe (before any design).** Create a scratch worktree with `git worktree add --detach .cache/WO-CORPUS-02/probe HEAD`. In that worktree only, add a probe helper and call it at `svg_shape_fail` for `SVG_SHAPE_ERROR_LIMIT` and at every assignment of `SVG_SHAPE_ERROR_LIMIT` or `return false` that leads to one in `src/raster/**`. The helper prints `__FILE__:__LINE__`, `__func__`, the limit constant and the value reached when one is in scope. It also prints `backtrace()` addresses, `dladdr` symbol names and the image slide from `_dyld_get_image_vmaddr_slide(0)`. Build Release with `-g -fno-omit-frame-pointer` into the worktree's own build directory. Resolve the static frames with `atos -o <binary> -l <load address> <addresses>`, and never use `lldb`.
2. **Iterate per file.** For each of the five files, run the probe binary with `--render-frame <file> --size 128x128` and record the site. Lift that single site in the probe worktree only (raise its bound by 16x, or bypass it) and run again. Stop for a file when it exits 0, when the next run exceeds 4 GiB of RSS or 300 s, or after 8 sites. Write `artifacts/WO-CORPUS-02/r2/probe.md` with a table of file, iteration, stage (decode or raster), byte offset, `file:line`, function chain, constant and value reached. Add the probe commit `git -C .cache/WO-CORPUS-02/probe rev-parse HEAD` and the final RSS and wall time of each file.
3. **Design.** Write `artifacts/WO-CORPUS-02/r2/design.md`. For each probed site, say whether it is lifted and how, or deferred and why. The design may raise budgets or make them proportional to the source byte length with a finite cap, and it may store per-element data. Include the worst-case byte arithmetic that shows the ceilings in the invariants are met. If a raster site needs a change to `src/app/viewer_load.c` or to `render_frame_cli.c`, defer it and name it; this order does not touch them.
4. **Implement** the lifted sites in the main tree within the allowlist. Update every consumer and test in the allowlist that the change affects, including `shape_fuzz.c`, which must keep a bounded edge check. Remove the scratch worktree with `git worktree remove --force .cache/WO-CORPUS-02/probe`.
5. **Focused tests under CTest.** For each lifted budget, add one test where a generated document just inside the new bound decodes or renders, and one where a document just over it fails with `LIMIT` and a stable offset without allocating past the ceiling. Allocation stats must balance after destroy.
6. **Measure.** Run the Release binary on each of the five files with the commands below. Record the exit code, final stderr line, wall seconds and maximum RSS in `artifacts/WO-CORPUS-02/r2/measure.json`. Output images stay under `.cache/WO-CORPUS-02/`. Then run the corpus sweep into `artifacts/WO-CORPUS-02/r2/render-diagnostics.{json,md}` and record its histogram next to the WO-CORPUS-01 histogram in `artifacts/WO-CORPUS-02/r2/delta.json`.
7. **Next blockers.** For each of the five files that still exits non-zero, run the probe procedure of item 1 once more on the final tree and record its next blocking site (`file:line`, function, constant) in `artifacts/WO-CORPUS-02/r2/probe.md` under the heading `## Remaining blockers`.
8. Record each verification command, its exit code and the last 40 lines of output in `artifacts/WO-CORPUS-02/r2/check-NN.txt`.

# Acceptance criteria
- AC-1 `./dev verify`, `./dev asan`, `./dev ubsan` and `./dev t --all` exit 0, with output recorded.
- AC-2 `./dev shape-fuzz` exits 0.
- AC-3 `probe.md` has the item 2 table with at least one resolved site for each of the five files, and every site has a `file:line` and a function name.
- AC-4 Each of the five files satisfies one of two conditions. Either its measurement command exits 0, or `probe.md` `## Remaining blockers` names its next site with `file:line`, function and constant, and `design.md` says why that site was deferred.
- AC-5 Every run in `measure.json` has maximum RSS of at most 2 GiB and wall time of at most 120 s. `design.md` shows a worst-case document model of at most 1 GiB and no single allocation over 256 MiB.
- AC-6 The sweep exits 0. Its JSON keeps `manifest_sha256` `9fbf9a2f…bc30`, `max_bytes` 33554432 and `render_tested` 39. `histogram.ok` is at least 3. Every file that was `ok` in `artifacts/WO-M4-10/render-diagnostics.json` is still `ok`, and so is every file that was `ok` in `artifacts/WO-CORPUS-04/render-diagnostics.json` if WO-CORPUS-04 has merged by then.
- AC-7 The tests of item 5 run under CTest and pass. `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths and no fixture. `git worktree list` shows no `.cache/WO-CORPUS-02/probe` entry.
- AC-8 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-CORPUS-02 --ready` and `git diff --check` exit 0.
- AC-9 A fresh read-only verifier reports pass in `artifacts/WO-CORPUS-02/r2/verdict.txt`.

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
mkdir -p .cache/WO-CORPUS-02
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/cad_t2.svg --size 128x128 --out .cache/WO-CORPUS-02/cad_t2.ppm
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/cnc_t2.svg --size 128x128 --out .cache/WO-CORPUS-02/cnc_t2.ppm
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/science_t2.svg --size 128x128 --out .cache/WO-CORPUS-02/science_t2.ppm
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/map_t1.svg --size 128x128 --out .cache/WO-CORPUS-02/map_t1.ppm
/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame /Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/map_t2.svg --size 128x128 --out .cache/WO-CORPUS-02/map_t2.ppm
python3 scripts/corpus_render_report.py --corpus /Users/megasoft78/Desktop/Freelance/vexel-corpus-real --binary build/mac-release/bin/svgworkbench --output-json .cache/WO-CORPUS-02/render-diagnostics.json --output-markdown .cache/WO-CORPUS-02/render-diagnostics.md
git worktree list
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-CORPUS-02 --ready
git diff --check
```
The five `render-frame` commands may exit 1 when AC-4's second condition holds. Each one's exit code is recorded, not required to be 0. The verifier's sweep writes to `.cache/` and is compared with the committed `artifacts/WO-CORPUS-02/r2/render-diagnostics.json` by histogram and per-file verdict.

# Non-goals
- No change to the XML tokenizer, the attribute policy, the report script, `viewer_load.c` or `render_frame_cli.c`.
- No change to rendering quality, no tiling or progressive rendering, and no render-at-output-size path (see WO-CORPUS-04).
- No work on corpus files other than the five, beyond the regression check of AC-6.

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 480 min; dependency/network: none.

# Failure and escalation
- If a site can be lifted only by breaking a ceiling in the invariants, defer it, record the measured numbers in `design.md` and continue with the others. Never raise a ceiling.
- If a site needs a path outside the allowlist, defer it and name it.
- If no site can be lifted within the ceilings, commit the probe and design evidence only and stop for planner review.

# Output
One commit `perf(shape): lift probed element and raster budgets for large generated drawings`, evidence under `artifacts/WO-CORPUS-02/r2/`, updated `BLOCKERS.md` defect 1, `STATE.md` and both ledgers, and the schema-valid executor report.
