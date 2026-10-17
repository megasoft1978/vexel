# WO-RUST-SCENE-SHAPES-01: shapes.md

Constant chosen: `MAX_SHAPES = 262_144` (`vexel-scene/src/types.rs`), `MAX_USE_INSTANCES = MAX_SHAPES`.
Need: `generated-use_t2` 178,589 shapes = 1.47x headroom at 262,144.

## 1. Inventory of `65536|65_536` (base 87fd5286 == HEAD of this worktree before edits)

Source, shape-count sites (changed):

| site | meaning | decision |
|---|---|---|
| `vexel-scene/src/types.rs` `Limits::default().shapes` | scene shape cap, source of the clamp | `MAX_SHAPES` |
| `vexel-scene/src/dash_validate.rs:8` | `DashCache.shapes.len()` cap | `MAX_SHAPES` |
| `vexel-scene/src/raster_job_views.rs:84` | `JobDashCache.shapes.len()` cap | `MAX_SHAPES` |
| `vexel-scene/src/raster_job_scratch.rs:37` | Dash-pool shapes buffer cap (`job.buffer` cap, not a preallocation) | `MAX_SHAPES` |
| `vexel-svg-decode/src/use_expansion.rs:71` | decoder-wide `<use>` instance count | `MAX_USE_INSTANCES` |
| `vexel-scene/src/validation_tests.rs:103` | pin `Limits::default().shapes == 65536` | moved to `MAX_SHAPES` (section 4) |

Source, not shape counts (unchanged):

| site | meaning | reason |
|---|---|---|
| `types.rs:119,125,129,130` scenes/clips/stops/groups | other table caps | not shapes |
| `ledger_buffer.rs:4` | comment | not a cap |
| `contract_storage.rs:70,78` | slots/stops | not shapes |
| `media.rs:173` | glyphs | not shapes |
| `draft_updates.rs:207,210`, `mutators.rs:310` | filter primitives | not shapes |
| `static_pattern.rs:142` | pattern visits | not shapes |
| `cache_publication.rs:44` | doc comment | not a cap |
| `raster_job_views.rs:117` | cumulative grid cells (WO: unchanged) | grid cell cap |
| `raster_job_views.rs:118` (ref_count 1,000,000), `raster_job_views.rs:50` (build_steps 1,000,000) | grid work caps | not shape counts, unchanged |
| `raster_job.rs:661` | per-shape grid cell kind | cell kind, not shape count |
| `validate.rs:350` | clip edges | not shape count |
| `clip_chain.rs:19,31`, `definition_links.rs:105` | clip memo/seen arrays | not shape counts |
| `production_decode.rs:453` | per-definition `expansions` count, LimitKind::References | separate expansion path, not a shape count; see gaps |
| `vexel-svg-engine/src` | none | no hits |

Tests (`vexel-scene/tests`, `vexel-svg-decode/tests`, 65536 hits):
- `ChargeLedger::new(…, 65536)` allocation-lease caps (about 60 hits across both trees, e.g. decode_publication_contracts.rs, paint_channel_contracts.rs, custom_publication.rs, production_*.rs): unrelated, unchanged. The lease cap is not the render limit (section 5).
- `RasterJob::buffer(…, 65536)` grid/scratch fixtures (render_integration_contracts.rs:163, raster_job_contracts.rs:235,624,721-733, cache_publication.rs:40-41,244): unrelated capacity fixtures, unchanged. `cache_publication.rs:40` is a Dash shapes fixture for a one-shape cache, not a boundary test.
- `cache_publication.rs:285` `(65536, 1_000_000)` grid cell totals: unrelated, unchanged.
- `paint_publication.rs:387` stops: unrelated, unchanged.
- `vexel-svg-decode/tests/original_references.rs:113` use-instance boundary: MOVED (section 4).
- `vexel-svg-decode/tests/original_shapes.rs` `original_many_shapes_limit` (70,000 paths, pin uses 70000 literal): MOVED (section 4).
- `vexel-scene/tests/validation_tests` (see above): MOVED.

`vexel-app/src/compose/decode.rs:178` `ChargeLedger::new(…, 65536)`: lease cap, not a shape bound. Not edited (outside allowed paths). Excluded as the render limit by the 131,072-shape pass (section 5).

## 2. Reconciliation: which limit fires first on `generated-use_t2`

Base corpus verdict `limit_decode` at byte 3,683,732. Counting in the file: 65,536 `<use>` elements precede that offset; the 65,537th `<use>` starts at that byte. `use_expansion.rs` checks `use_instances >= 65_536` before any shape admission for that `<use>`, so the first refusal is `Limit(References)` at the `<use>` offset, not `Limit(Shapes)` as LIMITS-01 recorded. LIMITS-01's kind label is not confirmed by this run (the corpus tool does not print the kind); the count argument is the basis. Both caps were raised together (`MAX_USE_INSTANCES == MAX_SHAPES`) so the reconciliation does not change the fix.

## 3. Per-shape bytes and memory gate

- `size_of::<Shape>()` = 320 bytes (measured by `shape_record_bytes_at_the_cap_are_measured`, MEASURE line in test output). Shape records at the cap: 262,144 x 320 = 83,886,080 bytes (80 MiB). Const assertion `MAX_SHAPES * size_of::<Shape>() <= 512 MiB` holds (types.rs).
- All shape-proportional pools (edges, path segments, numbers, dash records, grid, admissions, serials, render scratch) are not inventoried separately. The measured RSS slope bounds them: base-to-new flat-path case, (740,474,880 - 203,964,416) / (262,144 - 65,536) = 2,731 bytes per shape, all pools included.
- Analytic bound: 262,144 x (320 + 2,731) = 0.82 GB (about 779 MiB) <= 1 GiB.
- Measured peak RSS, release `vexel-corpus --one <file> --size 128x128`, wrapped by `/usr/bin/time -l` (macOS "maximum resident set size", bytes). Process baseline for a one-rect SVG: 8,798,208.

| case | N | base binary | new binary |
|---|---|---|---|
| (a) flat paths, one attribute each, one group | 65,536 | ok, 203,964,416 B | - |
| (a) flat paths | 65,537 | - | ok, 257,409,024 B |
| (a) flat paths | 131,072 | - | ok, 403,521,536 B |
| (a) flat paths | 196,608 | - | limit_render, 581,386,240 B |
| (a) flat paths | 262,144 (N = MAX) | refused at Shapes | limit_render, 740,474,880 B |
| (b) N `<use>` of one one-rect symbol | 65,536 | ok, 211,156,992 B | - |
| (b) N `<use>` | 262,144 (N = MAX) | refused | limit_render, 784,875,520 B |
| (c) N dashed paths (stroke and dasharray inherited from group) | 65,536 | limit_render, 195,870,720 B | - |
| (c) N dashed paths | 262,144 (N = MAX) | refused | limit_render, 756,465,664 B |
| corpus generated-use_t2 (178,589 shapes) | - | limit_decode, 328,613,888 B | limit_render, 855,195,648 B |

Memory gate: all measured peaks are under 2 GiB (2,147,483,648). Largest peak in the external sweep after the change: 1,359,413,248 B (base largest 1,352,400,896 B). The 262,144-shape cases stop in render at 1.3-1.5 s (release). The 60 s timeout was not reached by any synthetic case.

Candidate 1,048,576 (not chosen): measured only by extrapolation, because vexel-xml caps one-element-per-shape documents at MAX_NODES = 1,000,000 (`vexel-xml/src/tokenizer.rs:6`) and MAX_TOTAL_ATTRIBUTES at 1,000,000 (`:8`), so a 1,048,576-shape document of this shape cannot be built. Linear extrapolation of the 2,731 B/shape slope gives about 2.9 GB at 1,048,576, above the 2 GiB gate. Rejected.

Timing (debug test binary): nested-use refusal 12,890 ms (under the 60 s bound). Cancellation: `cancel_latency_us=122` (raised 50 ms into a 262,144-shape decode; bound 250 ms).

## 4. Pins moved (each proven by a boundary test)

Red-first sequence:
1. Constant set to 65,536 (no behavior change) and the new tests added. `corpus_use_t2_shape_need_decodes` failed (`MAX_SHAPES > 178,589` assert), the only red. Log: `/private/tmp/wo-shapes/red2.log`.
2. Constant set to 262,144 and sites wired; all new tests green.
3. Harness fix found on the first green run: `flat_rects` with four attributes per rect hit the XML total-attribute cap (1,000,000) at about 250k elements, reported as `Limit(Attributes)` at offset 10,568,447. Test shapes changed to one-attribute paths (`flat_shapes`). This is a vexel-xml limit, not changed.

Moved pins:

| file | old pin | new pin | proof |
|---|---|---|---|
| `vexel-scene/src/validation_tests.rs:103` | `assert_eq!(d.shapes, 65536)` | `assert_eq!(d.shapes, MAX_SHAPES)` | `default_limits_use_the_named_shape_cap` |
| `vexel-svg-decode/tests/original_references.rs:113` | `[(65536, true), (65537, false)]` (uses) | `[(MAX_USE_INSTANCES, true), (MAX_USE_INSTANCES + 1, false)]` (References) | `use_instance_boundary_accepts_max_and_refuses_next`, plus the same test body |
| `vexel-svg-decode/tests/original_shapes.rs` `original_many_shapes_limit` | 70,000 paths refused with Shapes | MAX_SHAPES paths accepted (exact count checked), MAX_SHAPES + 1 refused with Shapes | `flat_shape_boundary_accepts_max_and_refuses_next` (same boundary, rect-free) |

The old 70,000 pin would now decode (70,000 < 262,144), so it was replaced with the exact boundary, not loosened.

Shape-site literals changed: `dash_validate.rs:8`, `raster_job_views.rs:84`, `raster_job_scratch.rs:37`, `use_expansion.rs:71`.

## 5. Named limits that still bind (not raised, per WO)

- `vexel-render`/`vexel-scene` render stage (`limit_render`): `generated-use_t2`, `generated-science_t2`, and the synthetic flat/use/dash cases at 196,608+ shapes refuse at render. The corpus tool collapses this to `limit_render` (`ComposeError::LimitRender`, no kind). Excluded by evidence: ChargeLedger leases 65,536 (flat 131,072 passes; a per-shape lease would have fired), cumulative grid cells 65,536 (same argument for shapes with cells). Remaining candidates, not confirmed: `raster_job_views.rs:118` grid refs > 1,000,000, `raster_job_views.rs:50` build_steps > 1,000,000, `raster_job.rs:662` EdgeIndex 1,000,000. Kind and offset NOT established.
- `limit_decode` still for `generated-cad_t2`, `generated-cnc_t2`, `generated-inkscape_t2`, `commons-cambodia-geographic-map-en-svg`. A probe through `vexel-svg-decode` (default limits, reject operations) on cad_t2 returned `Unsupported` at byte 8,897,020, which is not the compose-path verdict; the probe is not an exact reproduction of the compose path, so the named limit for these four is NOT established. The probe file was removed before commit.
- XML: MAX_NODES 1,000,000 and MAX_TOTAL_ATTRIBUTES 1,000,000 (vexel-xml, unchanged).
- `production_decode.rs:453`: per-definition `expansions` cap 65,536 (References). Not nested-tested.

## 6. Sweep (external, owner-local manifest, release, 128x128, 60 s, 2048 MiB)

Counts (`sweep-before.json`, `sweep-after.json`):

| verdict | before | after |
|---|---|---|
| ok | 12 | 12 |
| unsupported | 9 | 9 |
| limit_decode | 12 | 10 |
| limit_render | 3 | 5 |
| xml | 3 | 3 |
| rss_exceeded | 0 | 0 |
| crash | 0 | 0 |
| timeout | 0 | 0 |
| skipped (> 32 MiB, default --max-bytes) | 19 | 19 |

ok to non-ok regressions: none. Verdict changes: `generated-use_t2` limit_decode to limit_render; `generated-science_t2` limit_decode to limit_render.

Six named files:

| file | before | after |
|---|---|---|
| generated-use_t2 | limit_decode (decode at 178,589 needed) | limit_render (decodes; render refuses, kind not named) |
| generated-cad_t2 | limit_decode | limit_decode (not named; probe inexact) |
| generated-cnc_t2 | limit_decode | limit_decode (not named) |
| generated-science_t2 | limit_decode | limit_render (decodes; render refuses) |
| generated-inkscape_t2 | limit_decode | limit_decode (not named) |
| commons-cambodia-geographic-map-en-svg | limit_decode | limit_decode (not named) |

In-repo corpus (`bench/corpus/manifest.json`): 24/24 ok before and after (`artifacts/WO-RUST-SCENE-SHAPES-01/in-repo`).

Wall time check: generated-giant_path_t1 was 55,099 ms (base) and 55,230 ms (after), ok in both; the 60 s timeout margin is unchanged for this file.

## 7. Gaps

- Render-stage limit kind for use_t2 and science_t2 is not named (verdict tool does not expose it).
- Four `limit_decode` files: named limit not established.
- `raster_job_views` shape-cap path (`JobDashCache::validate`) is not unit-tested; covered by the render sweep only.
- `production_decode.rs:453` is not nested-tested.
- The base workspace test count was not re-run. Static count: 10 new `#[test]` functions (6 svg-decode, 4 vexel-scene), 0 removed. Workspace after: 1965 passed, 5 ignored.
