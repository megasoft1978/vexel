---
id: WO-CORPUS-02
revision: 1
status: READY
milestone: M4
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: 92b207ae24d74f61bf0cc99c27201315201aff8d1f423a92c563b9c0bd2aa09c
parallel_safe: false
network: false
---
# Goal
Replace the single document-wide edge array bounded by `SVG_SHAPE_MAX_EDGES` (4,194,304 edges, 32 bytes each) with per-element edge storage or a streaming traversal, so that `generated/cad_t2` (10,592,198 bytes, refused today with `limit` at byte 9,365,917) renders. This is a storage design change, not a budget raise (`BLOCKERS.md` defect 1).

# Context pointers
- `BLOCKERS.md` defect 1, `HANDOFF.md` lesson 2 (symbol-resolving probe before touching a limit).
- `src/svg/shape/shape.h` (`SVG_SHAPE_MAX_EDGES`, `edge_count`, `edges`, `shape_edge_count`), `src/svg/shape/shape_geometry.c` (`add_edge`), `src/svg/shape/traverse.c` (opacity groups share the bound).
- Edge consumers: `src/svg/paint/paint.c`, `src/svg/clip/clip.c`, `src/svg/shape/{selection,mask,marker,pattern}.c`, `src/raster/shape_render_animation_clips.c`, `src/app/viewer_editor_path_node_commands.c`, `src/app/viewer_editor_path_curves.c`.
- Corpus (external, read-only): `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real/files/generated/cad_t2.svg`, manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.

# Existing contracts and invariants
- Rendered output is unchanged: every golden, oracle and reference test passes with no fixture modified.
- One element keeps a ceiling of 4,194,304 edges under a new name `SVG_SHAPE_MAX_ELEMENT_EDGES`; the opacity-group bound that reused `SVG_SHAPE_MAX_EDGES` gets its own constant with the same value; all other budgets stay as they are.
- Memory ceiling (stated, enforced, documented in `shape.h`): total resident edge storage per document at most 1 GiB (33,554,432 edges), no single edge allocation larger than 128 MiB, and peak RSS of the Release `render-frame` on `cad_t2` at 128x128 at most 2 GiB, measured with `/usr/bin/time -l`.
- Exceeding a ceiling fails cleanly with `SVG_SHAPE_ERROR_LIMIT` and an offset; never abort or exhaust memory.
- macOS-only, C17, no new dependency, files under 800 lines; no corpus byte enters the repository.

# Files allowed to touch
- `src/svg/shape/**`, `src/svg/paint/**`, `src/svg/clip/**`, `src/raster/**`, `src/app/viewer_editor_path_node_commands.c`, `src/app/viewer_editor_path_curves.c`, `tests/svg/**`, `tests/raster/**` (not fixtures), `CMakeLists.txt`, `src/svg/shape/README.md`, `artifacts/WO-CORPUS-02/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `scripts/**`, `tests/oracle/**`, `tests/golden/**`, every golden or reference fixture, `src/svg/xml/**`, `bench/**`, `reports/**`, prior artifacts, every path outside the allowlist.

# Required implementation
1. Before designing, run a symbol-resolving probe on `cad_t2` at the refusing site and record the call chain and the edge count reached in `artifacts/WO-CORPUS-02/probe.md`.
2. Record the chosen design (per-element chunks or streaming) and why in `artifacts/WO-CORPUS-02/design.md`, including the memory arithmetic for the ceilings above.
3. Implement it and adapt every consumer listed in Context; remove the document-wide array.
4. Focused tests: a generated document whose total edges exceed 4,194,304 while each element stays under the per-element cap renders; one element over the per-element cap fails with `LIMIT`; a document over the 1 GiB total fails with `LIMIT` without allocating it; allocation stats balance after destroy.
5. Measure `cad_t2`: `/usr/bin/time -l build/mac-release/bin/svgworkbench --render-frame <corpus>/files/generated/cad_t2.svg --size 128x128 --out .cache/WO-CORPUS-02/cad_t2.ppm`; record exit code, wall time and maximum resident set size in `artifacts/WO-CORPUS-02/cad_t2.json`. The `.ppm` stays in `.cache/`.
6. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-CORPUS-02/check-NN.txt`.

# Acceptance criteria
- AC-1 `./dev verify`, `./dev asan`, `./dev ubsan` and `./dev t --all` exit 0, with output recorded.
- AC-2 `./dev shape-fuzz` exits 0.
- AC-3 The item 5 command exits 0 and `cad_t2.json` records maximum resident set size at most 2 GiB.
- AC-4 `grep -rnw SVG_SHAPE_MAX_EDGES src` prints nothing; the tests of item 4 run under CTest and pass.
- AC-5 `git show --name-only --format= HEAD` (this order's single commit) lists only allowlisted paths and no fixture.
- AC-6 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-CORPUS-02 --ready` and `git diff --check` exit 0.
- AC-7 A fresh read-only verifier reports pass in `artifacts/WO-CORPUS-02/verdict.txt`.

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
grep -rnw SVG_SHAPE_MAX_EDGES src
git show --name-only --format= HEAD
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-CORPUS-02 --ready
git diff --check
```

# Non-goals
No change to other budgets, the tokenizer, attribute policy or the report script; no rendering-quality change; no tiling or progressive rendering.

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 360 min; dependency/network: none.

# Failure and escalation
If `cad_t2` then fails at a different site, record that site with the probe and stop; do not raise any other budget. If the design cannot meet the memory ceiling, record the measured numbers and stop for planner review rather than changing the ceiling.

# Output
One commit `perf(shape): store edges per element instead of one document array`, evidence under `artifacts/WO-CORPUS-02/`, updated `BLOCKERS.md`, `STATE.md` and both ledgers, and the schema-valid executor report.
