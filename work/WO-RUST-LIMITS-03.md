---
id: WO-RUST-LIMITS-03
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 139f0be196f31bb69c564aacf6f968deeaa8bd9f
spec_sha256: 5cc9783d18cec8dcc3ed1570297f3b266387831002f60dde4812e45c0fa8ccfa
parallel_safe: false
network: false
---
# Bounded limit raises: shapes, stroke steps/bytes, rule matches, use targets

## Goal
Raise five named limits to the measured minimum that lets the owner-selected corpus
files decode, with a memory and time proof for each raise. Owner decision 2026-10-17
authorizes these edits in otherwise protected `vexel-render`, `vexel-scene`,
`vexel-svg-css` and `vexel-svg-decode` limit sites listed below. No other limit changes.

Measured diagnosis (min passing value by bisection, owner-supplied):
- Stroke steps: needs 1.25M-2.9M; `MAX_STROKE_STEPS` 1,000,000 -> 4,000,000.
- Stroke bytes: needs up to 129.5 MB; `MAX_STROKE_BYTES` 16 MiB -> 192 MiB.
- Shapes: needs 321k-357k; `MAX_SHAPES` 262,144 -> 524,288 (320 B/shape, ~160 MiB at cap).
- Rule matches: counter only; `MAX_RULE_MATCHES` 4,194,304 -> 8,388,608.
- Use target bytes: 64 KiB -> 1 MiB, still bounded by the 16 MiB reference-bytes budget.

Expected new `ok` from the external sweep: generated-inkscape_t2 (shapes only, accepted with
~356k disclosures), generated-cnc_t2, generated-use_t2, generated-science_t2, possibly
commons-corvuscoraxiucn-svg. Expected to stay refused: generated-cad_t2; giant_path_t2
(1.8M steps takes 489 s, becomes a timeout: acceptable, recorded); cambodia (further
unsupported element at byte 19,547,307).

## Out of scope (refuse if needed to pass ACs)
- Edges: 13.7M needed, 2.09 GB peak RSS. Needs its own proof.
- TextCommands: assert caps at 512 MiB / 56 B.

## Dependencies
- WO-RUST-SCENE-SHAPES-01 accepted (`ledger.jsonl`, base chain at 139f0be1). Verify with
  `python3 scripts/ledger.py summary ledger.jsonl`. If not accepted, stop.
- WO-RUST-PIPELINE-PERF-01 accepted in `ledger.jsonl` before any corpus run. Not parallel.
- Read first: `rust/crates/vexel-scene/src/types.rs` (98-102, 126-137, 406-415),
  `rust/crates/vexel-render/src/lib.rs` (65-72), `dash.rs` (34, 47, 218), `validate.rs` (87),
  `rust/crates/vexel-svg-css/src/lib.rs` (25), `compute.rs` (164), `matching.rs` (392),
  `rust/crates/vexel-svg-decode/src/use_expansion.rs` (99),
  `artifacts/WO-RUST-SCENE-LIMITS-01/limits.md`, `artifacts/WO-RUST-SCENE-SHAPES-01/shapes.md`.

## Sites (base_sha)
| site | current | new | enforcement |
|---|---|---|---|
| `vexel-render/src/lib.rs:69` `MAX_STROKE_STEPS` | 1_000_000 | 4_000_000 | `dash.rs:34,218`, `validate.rs:87` |
| `vexel-render/src/lib.rs:71` `MAX_STROKE_BYTES` | 16 MiB | 192 MiB | `dash.rs:47` (before allocation) |
| `vexel-scene/src/types.rs:98` `MAX_SHAPES` | 262_144 | 524_288 | `Limits::default` (126-137), `effective` clamp (408-415), `dash_validate.rs:8`, `raster_job_views.rs:84`, `raster_job_scratch.rs:37` |
| `vexel-svg-css/src/lib.rs:25` `MAX_RULE_MATCHES` | 4_194_304 | 8_388_608 | `compute.rs:164`, `matching.rs:392` |
| `vexel-svg-decode/src/use_expansion.rs:99` | `64 * 1024` | `MAX_USE_TARGET_BYTES` = 1 MiB | total 16 MiB budget on same line |

Name collision (do not edit): `vexel-render/src/lib.rs:65` `MAX_SHAPES = MAX_EDGES` (4 Mi) is a
different render-side constant. Keep it unchanged; tests must name the scene constant by path.
`rust/fixtures/legacy/**` and `vexel-svg-css` test files use the symbol, so they need no edit.
Any literal `65536` or `262144` hit not in this table stays unchanged.

## Constants and assertions
- `types.rs`: keep `MAX_USE_INSTANCES == MAX_SHAPES`; keep
  `MAX_SHAPES <= u32::MAX as usize`; keep the 512 MiB shape-bytes assert (160 MiB holds).
  Add `const _: () = assert!(MAX_SHAPES == 524_288);` only as a pin next to the constant.
- `vexel-render/lib.rs`: `const _: () = assert!(MAX_STROKE_BYTES <= 192 * 1024 * 1024);`
  and `const _: () = assert!(MAX_STROKE_STEPS >= 1_250_000);` (documents the measured floor).
- `use_expansion.rs`: `const MAX_USE_TARGET_BYTES: usize = 1024 * 1024;` with
  `const _: () = assert!(MAX_USE_TARGET_BYTES <= 16 * 1_048_576);`.
- `vexel-svg-css/lib.rs`: `const _: () = assert!(MAX_RULE_MATCHES == 2 * 4_194_304);`.

## Assumptions
- Shape cost: `size_of::<Shape>()` (`types.rs:246`) = 320 B plus pool entries. Measured
  in the memory gate, not assumed.
- Stroke bytes are charged before allocation in `dash.rs:47`. Confirm by reading; if the
  charge occurs after allocation, fix the order within `dash.rs` (allowed path).
- Edge budget (4,194,304 per scene, render MAX_EDGES) is unchanged; record measured
  edges per shape at 524,288 shapes if any file reaches it.

## Scope
1. Failing-first boundary tests per raised constant, written and run red against the old
   value, then the constant raised to green. N accepted, N+1 refused with the existing
   typed error unchanged: `ErrorKind::Limit(LimitKind::Shapes)` (scene),
   `RenderError` stroke kind for steps and bytes (render), `CssError::Limit` (css),
   `Limit(References)` (use targets). Boundary pins that name old values move deliberately
   and each is recorded in `limits.md`.
2. Adversarial tests:
   - nested `<use>` (A holds two B, B holds two C, ...) whose product exceeds 524,288:
     refuses at the cap, admits at most 524,288 shapes, bounded time recorded;
   - attacker stroke file cannot allocate beyond 192 MiB (bytes charged before allocation,
     measured via peak RSS on the synthetic case);
   - cancellation during the largest accepted stroke job returns the existing cancelled
     kind within 250 ms (measured with `Instant`, recorded).
3. Memory and time: peak RSS (`/usr/bin/time -l`, release, 128x128) for the largest accepted
   synthetic scene of each raise: 524,288 flat rects; 524,288 `<use>` instances; 524,288
   dashed strokes; 4M stroke steps at 192 MiB; 8M rule matches; 1 MiB use target.
   Analytic bound per raise recorded beside the measurement.
4. Full external sweep (release, 128x128, 60 s, 2048 MiB) from base (`git archive
   139f0be1` into /private/tmp) and after. Record verdict counts only.

## Non-goals
Pipeline speed; Edges; TextCommands; `use_t3/t4`; streaming decode; new SVG features;
any `vexel-app` change; renderer or raster algorithm change; any corpus bytes in the repo;
any golden, oracle or hash edit.

## Files allowed to touch
- `rust/crates/vexel-render/src/lib.rs`, `dash.rs`, `validate.rs`, `dash/tests.rs`,
  `rust/crates/vexel-render/tests/**` (new files only)
- `rust/crates/vexel-scene/src/types.rs`, `request.rs`, `raster_job_*.rs`, `validate.rs`,
  `validation_tests.rs`, `rust/crates/vexel-scene/tests/**` (new files only)
- `rust/crates/vexel-svg-css/src/lib.rs`, `rust/crates/vexel-svg-css/tests/**` (new files only)
- `rust/crates/vexel-svg-decode/src/use_expansion.rs`, `rust/crates/vexel-svg-decode/tests/**` (new files only)
- `artifacts/WO-RUST-LIMITS-03/**`

## Protected paths
Everything else: `vexel-app` (src and tests, including `pipeline_hashes.rs` read only),
`vexel-raster`, `vexel-svg-media`, `vexel-svg-reference`, `vexel-scene-query`, workspace and
lock manifests, `rust/tools/**`, `rust/fixtures/**`, goldens, oracles, `docs/**`,
`BLOCKERS.md`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD and
index, the external corpus. No unsafe, no new deps, no network, no commit, stage, reset or
deletion. New files under 800 lines.

## Acceptance criteria
1. Each raised constant has one named definition, the const assertions above, and no
   remaining old literal at its site (`grep -rn "4_194_304\|262_144\|16 \* 1024 \* 1024\|64 \* 1024" <paths>` recorded).
2. Failing-first commit-free evidence: red run output recorded before each raise, green after.
3. Boundary tests N accepted / N+1 refused, typed error unchanged, for all five raises.
4. Adversarial tests 2a-2c pass with recorded numbers.
5. Peak RSS recorded for each item in Scope 3; each < 2 GiB; no `rss_exceeded` or `crash`.
6. Sweep: no file that is `ok` in base becomes non-ok; expected new `ok` list reported
   against the actual list; final `ok` count >= 12 of 39 (base 12, expected up to 17).
   Refusals recorded with named limit and offset.
7. In-repo corpus `bench/corpus/manifest.json` 24/24 ok. `pipeline_hashes.rs` passes with
   no hash edited.
8. `cargo test --offline --locked --workspace` passes; record the final `cargo test:` line.
   Test count >= base count.
9. `scripts/check_rust_fmt.py` no worse than base; new files clean.
10. `cargo tree -p vexel-app` unchanged; no `unsafe` in the diff; `git diff --check` clean.
11. `artifacts/WO-RUST-LIMITS-03/limits.md` (<= 300 words): per-constant old/new value,
    proving test, measured RSS and time, sweep before/after counts, remaining ceiling per file.

If an AC cannot be met, stop with the exact blocker. No test weakening; no threshold moved
without the boundary test of equal strength.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable test -p vexel-scene -p vexel-render -p vexel-svg-css -p vexel-svg-decode --locked --offline`
- `cargo +stable test --offline --locked --workspace`
- `python3 scripts/check_rust_fmt.py`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-LIMITS-03/in-repo`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --pins rust/tools/vexel-corpus/pins.json --out /private/tmp/limits03-after`
  (owner-local; if absent, stop and report). Base run from `git archive 139f0be1` at `/private/tmp/limits03-base`.
- `python3 scripts/wo_validate.py --ready WO-RUST-LIMITS-03`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-LIMITS-03/`: `limits.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`, `sweep-before.json`, `sweep-after.json`, `in-repo/`.
Only verdict counts and measurements go into artifacts; corpus bytes never.

## Worker and review
Worker: gpt-6-luna High, one writer, parallel_safe false, existing allowance only; stop on
quota errors. Candidate only until a fresh read-only reviewer (separate session, diff and
this order only) reruns the frozen commands and checks each AC. The coordinator integrates
after the dependencies above, reruns main-tree checks, then updates STATE.md, LEDGER.md
and ledger.jsonl.
