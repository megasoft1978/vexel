# WO-RUST-PIPELINE-PERF-01 result

Cause: `SceneBuilder::commit_request` ran `validate::scene` over the whole scene on every
request commit. Fix: `validate::scene_from` checks only records appended since the request
began (edges, shapes, path segments/numbers, transforms, geometry sources, images, clips,
patterns, masks, paints, filters, groups, dashes, stops) plus the one record before the
boundary (seam). Seal, draft commit and freeze still run the unchanged whole-scene
`validate::scene`. The per-shape `SceneView` is hoisted. No decode (`vexel-svg-decode`) edit
was needed: with O(1) commits the whitespace flush costs nothing (R8000 44 ms), so the
`Malformed(offset 0)` flush-order dependency was left untouched and uninvestigated.

## Before / after (release, 128x128, best of 3, same host)
| scene | base | after |
|---|---|---|
| R8000 | 1334 ms | 44.5 ms |
| R16000 | 5454 ms | 86.3 ms (1.94x R8000) |
| P8000 | 6219 ms | 628 ms |
| S8000 | 1427 ms | 43.2 ms |
| G8000 | 1354 ms | 44.4 ms |

External corpus (12 timeouts at base, 60 s): after, 0 timeouts. cnc_t1 60+ s -> 0.18 s,
inkscape_t1 60+ s -> 0.18 s, louisiana 60+ s -> 0.67 s, cad_t1 57.9 s -> 0.35 s,
embedded_t2 timeout -> ok 24.4 s; cad_t2, cnc_t2, inkscape_t2, map_t2, science_t2,
bulgaria, cambodia, corvus now end in `limit_decode`/`xml`/`unsupported` (fast refusals,
same classes the corpus already uses). No ok file regressed; giant_path_t1 stays ok at
55.4 s (render: `StrokePrims::{join_hit_at,segment_hit_at}`, vexel-render, out of scope; it
is 5 s from the timeout). In-repo sweep: 24/24 ok, verdicts identical to
WO-RUST-CORPUS-UNSUPPORTED-01.

## Byte identity
`before.json` (base) vs `after.json`: `cmp` identical for 136 fixture SVGs x {128, 512} plus
7 synthetic scenes (verdict, disclosures, SHA-256 of frame).

## Protected-crate edits and proving tests
- `vexel-scene/src/validate.rs`: `scene_from`, `group_view`. Proof: benchmark
  `pipeline_perf.rs` (failed at base: R8000 1334 > 300 ms, P8000 6219 > 1200, R16000
  quadratic) and 11 differential tests in `request_tests.rs`.
- `vexel-scene/src/request.rs`: commit passes the request-begin lengths; registers the test module.
- `vexel-scene/src/request_tests.rs` (new, `#[path]` from request.rs): 9 corruptions + unresolved
  admission compare commit result with the whole-scene validator and constants (kind, offset);
  multi-request scene equals one-shot scene; prefix corruption still rejected at draft commit/freeze.

## Notes / gaps
- Not detected at commit: corruption of records older than the seam (unreachable through
  the public API: appends and finalizers validate; still rejected at draft commit/freeze).
- `render_integration_contracts` expects non-zero commit work on an empty request; the seam
  record keeps that (no test changed).
- Frozen fmt check fails at base on unrelated files (tests_open.rs, pattern_publication.rs,
  production_decode.rs, many vexel-scene tests); not touched.
- Not run: live app, text documents (CoreText provider), git commit (git unavailable in this
  worker, left uncommitted).
- Cancellation: new test cancels R8000 at poll 200; 0 polls occur after the flag (stops at the
  first poll), latency about 1 microsecond.
- perf-baseline.json: compose_2000_rects_512 235.16 ms -> max 1882; added compose_4000_rects_512
  495.82 ms -> max 3967 (debug).
