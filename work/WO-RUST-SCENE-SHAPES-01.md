---
id: WO-RUST-SCENE-SHAPES-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 87fd52864959e5f1a7aef77e454c3b67304f2b15
spec_sha256: caf4550b7025a17bc81814daf356b212ee0b833534c9e467525dabe044f0c2a5
parallel_safe: false
network: false
---
# Shape count: one named limit above 65,536, with a memory proof

## Goal
Replace the 65,536 shape clamp with one named constant `MAX_SHAPES` so
`generated-use_t2` (178,589 shapes) and other shape-limited corpus files can
decode, with peak memory proven under the corpus bound (2 GiB). Owner decision
2026-10-17 authorizes edits to protected `vexel-scene` and to the
`vexel-svg-decode`/`vexel-svg-engine` limit sites listed below. No other limit
changes.

Measured facts (WO-RUST-SCENE-LIMITS-01 result, `artifacts/WO-RUST-SCENE-LIMITS-01/result.md`
at base_sha):
- `generated-use_t2` stops at `Limit(Shapes)`, byte 3,683,732; needs 178,589 shapes.
- `Limits::default().shapes` is 65,536 (`rust/crates/vexel-scene/src/types.rs:120`).
  `Limits::effective` (`types.rs:405`) clamps every caller value to that default, so
  raising `Limits` in `vexel-app/src/compose/decode.rs` alone changes nothing.
- Still shape/element-limited in the final sweep (STATE.md 2026-10-17): generated-cad_t2,
  cnc_t2, science_t2, inkscape_t2, cambodia.

## Dependencies
- WO-RUST-SCENE-LIMITS-01 accepted on main (commit ffce1d32).
- WO-RUST-PIPELINE-PERF-01 accepted in `ledger.jsonl` before any corpus run (check with
  `python3 scripts/ledger.py summary`). If not accepted, stop. Not parallel: both touch vexel-scene.
- Read first: `BLOCKERS.md` defect 1, `rust/crates/vexel-scene/src/types.rs`,
  `shape_admission.rs`, `shape_builder.rs`, `dash_validate.rs`, `raster_job_views.rs`,
  `raster_job_scratch.rs`, `vexel-svg-decode/src/use_expansion.rs`.

## Shape-count sites (base_sha; grep `65536|65_536` in vexel-scene/src, vexel-svg-decode/src, vexel-svg-engine/src)
| site | current | role |
|---|---|---|
| `vexel-scene/src/types.rs:120` | `shapes: 65536` in `Limits::default` | source of the clamp |
| `vexel-scene/src/shape_admission.rs:99` | `used >= self.limits.shapes` | admission (`ErrorKind::Limit(Shapes)`) |
| `vexel-scene/src/shape_builder.rs:82` | `limits.shapes` in the growth loop | builder |
| `shape_admission.rs:119-137`, `mutators.rs:180,268,283` | `room!(..., limits.shapes, ...)` | capacity; read the named value, no edit expected |
| `vexel-scene/src/dash_validate.rs:8` | `self.shapes.len() > 65536` | hardcoded |
| `vexel-scene/src/raster_job_views.rs:84` | `self.shapes.len() > 65536` | hardcoded |
| `vexel-scene/src/raster_job_scratch.rs:37` | Dash `shapes` buffer capacity `65536` | raster pool capacity |
| `vexel-svg-decode/src/use_expansion.rs:71` | `use_instances >= 65_536` -> `Limit(References)` | instance count, scales with shapes |

Not shape counts (listed in `shapes.md` with reason, unchanged): `raster_job_views.rs:117`
(cumulative grid cells), `raster_job.rs:661` (per-shape grid cell kind), `validate.rs:350`
(clip edges), `clip_chain.rs:19`, `definition_links.rs:105`, `production_decode.rs:453`,
`static_pattern.rs:142`, `contract_storage.rs:70`, `media.rs:173`, `ledger_buffer.rs:4`.
`vexel-app/src/compose/decode.rs:178` (`ChargeLedger::new(..., 65536)`) is outside the
allowed paths: if it bounds shapes, stop and report instead of editing it.

Reconcile first: LIMITS-01 records `use_t2` failing at `Shapes` (byte 3,683,732), yet
`use_expansion.rs:71` caps instances at 65,536. Reproduce with the base binary and state
which limit fires first before changing either.

## Assumptions
- Per-shape cost is `size_of::<Shape>()` (`types.rs:246`) plus per-shape pool entries
  (dash shape records, `ShapeBounds`, admissions, row events). Measured, not assumed.
- Candidates for `MAX_SHAPES`: 262,144 (1.47x the 178,589 need) or 1,048,576. Choose
  1,048,576 only if it passes the memory gate with its own measurement and the 60 s
  timeout; otherwise 262,144. If 262,144 fails the gate, stop with the measured number.
- `MAX_USE_INSTANCES` = `MAX_SHAPES`. Both checks stay (empty symbols do not add shapes).
- Edge budget stays 4,194,304 per scene; the measured edges per shape must fit it at
  `MAX_SHAPES` (record the ratio). Grid cells (`raster_job_views.rs:117`), commands,
  numbers, transforms, glyphs, text bytes stay unchanged. If a measured file trips one
  of them, record the named limit and the offset; do not raise it here.
- Cumulative `use` reference bytes (16 MiB) and reference ids stay as LIMITS-01 set them.

## Memory gate
Synthetic worst cases generated in tests (never corpus bytes in the repo): (a) N flat
rects in one group; (b) N `<use>` of one one-rect symbol; (c) N rects each with a dashed
stroke. Measure at N = chosen `MAX_SHAPES` with `/usr/bin/time -l` (macOS max RSS) on the
release `compose_svg` path, 128x128. Record bytes per shape for each case. Proof: analytic
`MAX_SHAPES x (size_of::<Shape>() + per-shape pool bytes)` <= 1 GiB, and measured peak RSS
<= 2 GiB for all three cases. A const assertion pins `MAX_SHAPES * size_of::<Shape>() <= 512 MiB`.

## Scope
1. Inventory: `shapes.md` lists every `65536|65_536` hit in the three source trees with
   file:line, meaning, shape-count yes/no, and decision.
2. Add in `vexel-scene/src/types.rs`: `pub const MAX_SHAPES: usize = <chosen>;`. Use it for
   `Limits::default().shapes` and at the hardcoded sites (dash_validate, raster_job_views,
   raster_job_scratch). Export `MAX_USE_INSTANCES` (= `MAX_SHAPES`) and use it in
   `use_expansion.rs`. No other literal changes at shape sites.
3. Const assertions: `MAX_SHAPES <= u32::MAX as usize` (admission slots are u32),
   `MAX_USE_INSTANCES == MAX_SHAPES`, and the 512 MiB shape-bytes bound above.
4. Tests that pin 65,536 shapes: `validation_tests.rs:103` and the 44 hits in
   `vexel-scene/tests` and the 20 in `vexel-svg-decode/tests` (grep at base). Each hit is
   recorded in `shapes.md` as "moved to MAX_SHAPES with a boundary test" or "unrelated,
   unchanged". Never loosen a pin without a boundary test of equal strength.
5. New adversarial tests (Acceptance 3-5) in new files under `vexel-scene/tests/` and
   `vexel-svg-decode/tests/`.

## Non-goals
Pipeline speed; edge, command, number, transform, grid-cell, glyph and text-byte caps;
`use_t3/t4` (over 32 MiB); streaming decode; new SVG features; any `vexel-app` change;
renderer or raster algorithm changes.

## Files allowed to touch
- `rust/crates/vexel-scene/src/types.rs`, `dash_validate.rs`, `raster_job_views.rs`,
  `raster_job_scratch.rs`, `validation_tests.rs`, `rust/crates/vexel-scene/tests/**`
- `rust/crates/vexel-svg-decode/src/use_expansion.rs` and `rust/crates/vexel-svg-decode/tests/**`
- `rust/crates/vexel-svg-engine/src/**` only at a limit site the inventory proves is a
  shape count (none at base); list any such change in `changed-path-audit.json`
- `artifacts/WO-RUST-SCENE-SHAPES-01/**`

## Protected paths
Everything else, including `vexel-app` (src and tests; `pipeline_hashes.rs` is read only),
`vexel-render`, `vexel-raster`, `vexel-svg-media`, `vexel-svg-reference`,
`vexel-scene-query`, workspace and lock manifests, `rust/tools/**`, goldens, oracles,
`docs/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders,
git HEAD and index, the external corpus. No unsafe, no new deps, no network, no commit,
stage, reset or deletion. New files under 800 lines.

## Acceptance criteria
1. `shapes.md`: inventory table (every hit, decision), per-shape bytes, analytic bound,
   measured peak RSS for the three cases at N, the chosen value, and the rejected
   candidate's result if any.
2. One definition: `MAX_SHAPES`. `grep -rn "65536\|65_536"` shows no remaining literal at
   the shape sites in the inventory. `MAX_USE_INSTANCES == MAX_SHAPES`.
3. Boundary tests: flat rects, N accepted and N+1 refused with `ErrorKind::Limit(LimitKind::Shapes)`;
   `<use>` instances, N accepted and N+1 refused with the existing `Limit(References)`.
   Both through `compose_svg`-level decode and at the `ShapeBuilder`/`DashCache` unit level.
4. Expansion bound: nested `<use>` (symbol A holds two uses of B, B holds two uses of C, ...)
   whose expanded product exceeds `MAX_SHAPES` refuses at the cap, admitting no more than
   `MAX_SHAPES` shapes, in bounded time (recorded).
5. Cancellation: a cancel signal raised during an N-shape decode returns the existing
   cancelled kind within 250 ms (measured with `Instant`, recorded).
6. Memory gate as stated above, recorded in `shapes.md`.
7. Sweep (release, 128x128, 60 s, 2048 MiB, external manifest). Base and after both run
   from this base_sha (`git archive 87fd5286` extracted to /private/tmp for the base run).
   No file that was `ok` becomes non-ok; no `rss_exceeded` or `crash`; verdicts recorded
   for generated-use_t2, cad_t2, cnc_t2, science_t2, inkscape_t2, cambodia (before and
   after, with the named limit and offset if still refused). Output outside the repo;
   only verdict counts go into artifacts.
8. In-repo corpus (`bench/corpus/manifest.json`) 24/24 ok. `pipeline_hashes.rs` passes with
   no hash edited (byte-identical renders).
9. `cargo test --offline --locked --workspace` passes; test count >= base count; each
   65,536 pin is recorded in `shapes.md`.
10. `scripts/check_rust_fmt.py` no worse than base (base failures recorded, new files clean).
11. `cargo tree -p vexel-app` unchanged; no `unsafe` in the diff.
12. `result.md` <= 300 words: before/after verdict table for the six files, each constant
    changed with its proving test, remaining ceiling per file.

If an AC cannot be met, stop with the exact blocker. No test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable test -p vexel-scene -p vexel-svg-decode -p vexel-svg-engine --locked --offline`
- `cargo +stable test --offline --locked --workspace`
- `python3 scripts/check_rust_fmt.py`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-SCENE-SHAPES-01/in-repo`
- same `run` with `--manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --pins rust/tools/vexel-corpus/pins.json --out /private/tmp/<dir>` (owner-local; if absent, stop and report)
- `python3 scripts/wo_validate.py --ready WO-RUST-SCENE-SHAPES-01`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-SCENE-SHAPES-01/`: `result.md`, `shapes.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `sweep-before.json`, `sweep-after.json`.

## Worker and review
Worker: gpt-6-luna High, one writer, parallel_safe false, existing allowance only; stop on
quota errors. Candidate only until a fresh read-only reviewer (separate session, diff and
this order only) reruns the frozen commands and checks each AC. The coordinator integrates
after WO-RUST-PIPELINE-PERF-01, reruns main-tree checks, then updates STATE.md, LEDGER.md
and ledger.jsonl.
