# WO-RUST-LIMITS-03 result

Branch: worktree-agent-a3f261139dcc6ed2e (base 139f0be1; HEAD eb6adb69 adds only work/WO-RUST-LIMITS-03.md). Candidate. Changes are uncommitted: the WO forbids commit, stage and reset. Dependencies accepted in ledger.jsonl: WO-RUST-SCENE-SHAPES-01, WO-RUST-SCENE-LIMITS-01, WO-RUST-PIPELINE-PERF-01.

## Constants

| constant | file:line | old | new |
|---|---|---|---|
| MAX_SHAPES (= MAX_USE_INSTANCES) | vexel-scene/src/types.rs:98 | 262,144 | 524,288 |
| MAX_STROKE_STEPS | vexel-render/src/lib.rs:69 | 1,000,000 | 4,000,000 |
| MAX_STROKE_BYTES | vexel-render/src/lib.rs:71 | 16 MiB | 192 MiB |
| MAX_RULE_MATCHES | vexel-svg-css/src/lib.rs:25 | 4,194,304 | 8,388,608 |
| MAX_USE_TARGET_BYTES (new const) | vexel-svg-decode/src/use_expansion.rs | 64 KiB literal | 1 MiB |

Const assertions added: types.rs (MAX_SHAPES == 524,288; existing 512 MiB shape-record assert holds: 524,288 x 320 B = 160 MiB); lib.rs (steps >= 1,250,000; bytes <= 192 MiB); dash.rs (size_of Segment == SEGMENT_BYTES, size_of Join == JOIN_BYTES, 192 MiB divisible by JOIN_BYTES); css lib.rs (== 2 x 4,194,304); use_expansion.rs (<= 16 MiB). Not raised: render MAX_SHAPES (= MAX_EDGES, name collision, unchanged).

Effective clamp: `Limits::effective()` clamps to `Limits::default()`, so the raised defaults are what admit the new counts. Pinned by `validation_tests.rs` `limit03_shape_override_clamps_to_the_524288_default` (red before the raise).

## Failing-first (red before raise)

Red outputs in artifacts/WO-RUST-LIMITS-03/red/: render-lib.txt (2 failed), scene.txt (validation clamp failed), scene-integration.txt (2 failed), css.txt (failed), svg-decode.txt (3 failed; nested test passes on base by design), render-stroke-jobs.txt (2 failed). Each red run used the pre-raise constants.

Not red-run: the `validate.rs` solid-ring boundary (added after the raise), and the moved errors.rs pin (moved after the raise). The stroke-job test was redesigned once after its first red run (two edges per range plus a 1x1 tile; see Gaps). Its red record is from the first design.

## Boundary and adversarial tests (all green)

- Shapes: `vexel-scene/tests/limits03_shape_literal.rs` (2); `vexel-svg-decode/tests/limits03_boundaries.rs` flat 524,288 ok / 524,289 `Limit(Shapes)`; `<use>` 524,288 ok / 524,289 `Limit(References)`.
- Steps: `dash/tests.rs` 4,000,000 ok / +1 refused; `limit03_stroke_jobs.rs` 800,000 ranges ok / 800,001 `LimitExceeded` (5 steps per range, measured); `validate.rs` solid ring 2,000,000 (4,000,000 steps) ok / 2,000,001 refused.
- Bytes: `dash/tests.rs` 4,194,304 joins (192 MiB) ok / +1 refused; `limit03_refused_primitive_allocates_nothing` (capacity stays 0 when refused; charge precedes `reserve`).
- Rule matches: `limits03_rule_matches.rs` 8,388,608 ok / `CssError::Limit` at +1.
- Use target: `limits03_boundaries.rs` span 1 MiB ok / 1 MiB + 1 `Limit(References)`.
- 2a nested use: fan 3, depth 12 (797,161 instances, above 524,288) refused `Limit(References)`; 13.2 s in debug.
- 2b attacker stroke: over-budget job refused before allocation (test above); 4.0M-step job RSS 587 MB; 176 MB-primitive ring RSS 882 MB.
- 2c cancellation: 800,000-range job, flag raised at 50 ms, returns `Cancelled`. Release 52.7 ms. Debug 375 ms.

## Pins moved (old boundary to new boundary)

1. `vexel-render/tests/errors.rs` `over_limit_input_returns_limit_exceeded`: solid ring 600,000 edges (1.2M steps, refused at old 1M cap) to 2,000,001 edges (4,000,002 steps, refused at new 4M cap).
2. `vexel-svg-decode/tests/original_limits_math.rs` `original_css_rule_match_work_limit`: 2050 rects (4,198,400 matches, now accepted) to 4100 rects (8,396,800 matches).
3. `vexel-svg-decode/tests/original_references.rs` `original_reference_exact_bytes_and_depth_boundaries`: 64 KiB / 64 KiB+1 target to 1 MiB / 1 MiB+1.
4. `vexel-svg-decode/tests/shape_cap_contracts.rs` `nested_use_product_is_refused_at_the_instance_cap`: fan 65 (274,490 instances, now under the cap) to fan 81 (538,083 instances).
5. `vexel-svg-decode/tests/shape_cap_contracts.rs` `use_rects` fixture: target `<rect width height>` (36 B) to `<path d>` (28 B). Reason: 524,288 x 36 B = 18.9 MB exceeds the unchanged 16 MiB reference-bytes budget, so the instance cap was not what fired. The same fixture change was made in the new `limits03_boundaries.rs`.

Touches to existing test files are outside the "new files only" path rule. They follow WO scope item 1, which moves boundary pins. The owner should confirm.

## Memory (release, `/usr/bin/time -l` peak RSS, `vexel-corpus --one <svg> --size 128x128`)

Synthetic SVGs are outside the repo (/private/tmp/limits03-final). Before = base binary, largest count it accepts. After = new binary, new maximum.

| case | N | verdict | peak RSS | real s |
|---|---|---|---|---|
| before flat paths | 163,840 | ok | 553,598,976 | 0.61 |
| before flat paths (old max) | 262,144 | limit_render | 739,328,000 | 0.98 |
| before `<use>` | 163,840 | ok | 629,440,512 | 0.91 |
| before `<use>` (old max) | 262,144 | limit_render | 884,670,464 | 1.47 |
| before dashed (4 steps/shape) | 100,000 | ok | 267,059,200 | 0.85 |
| before dashed | 125,000 | limit_render | 292,732,928 (probe) | - |
| before stroke ranges (8 steps/range, 1.0M steps) | 125,000 | ok | 155,697,152 | 0.66 |
| before stroke ranges | 200,000 | limit_render | 221,134,848 | 0.15 |
| before use target 64 KiB | 65,522 pad | ok | 10,846,208 | 0.00 |
| before CSS (base quota) | 4096 x 1000 | ok | 17,055,744 | 0.24 |
| after flat paths | 524,288 | ok | 1,576,239,104 | 2.24 |
| after `<use>` | 524,288 | ok | 1,857,732,608 | 3.30 |
| after dashed | 524,288 | ok | 1,363,263,488 | 4.89 |
| after stroke ranges | 500,000 (4.0M steps) | ok | 586,776,576 | 2.47 |
| after closed ring, dash [1000,1] | 1,999,000 edges (~176 MB prims) | ok | 882,524,160 | 2.64 |
| after use target | 1 MiB | ok | 12,845,056 | 0.01 |
| after use target x16 | 16 MiB budget | ok | 12,861,440 | 0.04 |
| after CSS | 4096 x 2000 (~8.2M matches) | ok | 18,726,912 | 0.49 |

Analytic bounds: shapes 524,288 x (320 B record + 2,731 B slope, SHAPES-01) = 1.57 GB, measured 1.58 GB. Stroke primitive bytes are <= 48 B x steps, so at 4,194,304 joins (192 MiB) the byte cap equals the step cap's bytes; the step cap binds first for every constructible job. Measured `<use>` slope is about 3.4 KB per instance, so the 2 GiB gate is about 609k instances.

Note: `vexel-corpus --one` on a file of 1,999,000 edges shows `ok` with 882 MB. Peak RSS is not attributable to the primitive buffers alone (it includes decode and edge pools).

## External corpus sweep (owner-local, read-only; 128x128, 60 s, 2048 MiB)

Base: `/private/tmp/limits03-base` (binary built from the worktree before edits; sources equal 139f0be1 plus the WO file, checked by `git diff --stat`). After: `/private/tmp/limits03-after`. Full per-file rows in sweep-base.json and sweep-after.json.

Changed rows (all other rows have identical verdicts):

| file | base | after | wall ms base to after | peak RSS base to after (MB) |
|---|---|---|---|---|
| generated-cnc_t2 | limit_decode | ok | 1081 to 1737 | 597 to 954 |
| generated-inkscape_t2 | limit_decode | ok (356,820 disclosures) | 1423 to 2011 | 856 to 1422 |
| generated-map_t2 | limit_render | ok | 547 to 610 | 335 to 338 |
| generated-science_t2 | limit_render | ok | 828 to 35,189 | 588 to 657 |
| generated-use_t2 | limit_render | ok | 2973 to 3654 | 786 to 890 |
| commons-corvuscoraxiucn-svg | limit_render | ok | 931 to 2186 | 561 to 600 |
| generated-giant_path_t2 | limit_render | timeout (60 s) | 357 to 60,016 | 340 to - |
| commons-cambodia-geographic-map-en-svg | limit_decode | unsupported (<21> at byte 19,547,307) | 1100 to 1778 | 457 to 542 |

Totals: base ok 12, unsupported 9, limit_decode 10, limit_render 5, xml 3, timeout 0, skipped 19. After ok 18, unsupported 10, limit_decode 7, limit_render 0, xml 3, timeout 1, skipped 19. No ok file became non-ok. rss_exceeded 0, crash 0. Every file's peak RSS is under 2 GiB (max 1.42 GB, inkscape_t2).

Expected-list check: new ok matched inkscape_t2, cnc_t2, use_t2, science_t2, corvuscoraxiucn-svg. Unexpected new ok: generated-map_t2. Expected refusals held: cad_t2 (limit_decode), giant_path_t2 (timeout), cambodia (unsupported at byte 19,547,307).

Watch item: generated-science_t2 now takes 35 s (base 0.8 s), against the 60 s timeout.

In-repo corpus (bench/corpus/manifest.json): 24 of 24 ok, `artifacts/WO-RUST-LIMITS-03/in-repo/`.

## AC1 grep (old literals at raised sites)

`grep -rn "4_194_304\|262_144\|16 \* 1024 \* 1024\|64 \* 1024"` over the four crates. Remaining hits are not raised constants: `vexel-svg-css/src/lib.rs` (const assertion), `vexel-svg-decode/src/use_expansion.rs` and `traversal.rs` (`visits >= 4_194_304`, a separate traversal counter, not in the table), `vexel-scene/src/media.rs:176` (262144 commands, MEDIA owner), `vexel-scene/src/validate.rs:387` and `raster_job.rs` 16 MiB pool budgets, and 64 KiB fixtures in `original_references.rs` and `original_m4_boundaries.rs` (below the 1 MiB cap, passing). None is at a raised site.

## Verification

- Frozen four-crate run (`cargo +stable test -p vexel-scene -p vexel-render -p vexel-svg-css -p vexel-svg-decode --locked --offline --no-fail-fast`): 285 passed, 0 failed (before the final formatting-only edits).
- Workspace (`cargo +stable test --offline --locked --no-fail-fast --workspace`): see verification.json. The expected panic line from `vexel-platform-apple` appears; the result is in the `test result` lines (298 suites).
- `python3 scripts/check_rust_fmt.py`: pass. New test files rustfmt-clean; formatted this WO's hunks in debt-listed crates.
- `python3 scripts/wo_validate.py --ready WO-RUST-LIMITS-03`: valid.
- `python3 scripts/ledger.py validate ledger.jsonl`: valid, 2069 entries.
- `git diff --check`: clean. No `unsafe` in the diff. No Cargo.toml or Cargo.lock change, so `cargo tree -p vexel-app` is unchanged by construction (not diffed).

## Gaps and deviations

1. Cancellation latency: `control` is polled only per shape and per pixel row. The stroke build of one shape cannot be interrupted, and the build is in `stroke.rs`, which is outside the WO path list. Release meets 250 ms (53 ms). Debug takes 375 ms, so the test bound is 1 s in debug and 250 ms in release. The owner should decide whether a poll inside `StrokePrims::build` is needed.
2. Named limit and offset for each refused corpus file are not exposed by the verdict tool (for example cad_t2, cnc_t2 and cambodia). The AC6 offset requirement is therefore only partly met.
3. Base test count on 139f0be1 was not re-run. The workspace count is compared against the SHAPES-01 recorded base (1965 passed); the new count is 16 tests higher.
4. The render-stage refusal of base flat paths at about 170k shapes (176,000 refused, 163,840 accepted) is not explained. The verdict collapses the render-stage limit, so it is not named.
5. `generated-science_t2` time rose to 35 s.
6. Commit not made: the WO's protected list forbids commit, stage and reset, so changes are left in the worktree.
7. `vexel-scene`, `vexel-render` and `vexel-svg-css` are rustfmt debt-listed crates. Only this WO's hunks were formatted; pre-existing drift was left untouched.


## Coordinator amendment (2026-10-17)

`MAX_USE_INSTANCES` was decoupled from `MAX_SHAPES` and kept at 262,144: the `<use>`-heavy worst case peaked at 1.86 GB at 524,288 instances, too close to the 2 GiB bound, and the corpus needs at most 178,589 instances. Shapes stay at 524,288. The two boundary tests and the equality test were updated accordingly.
