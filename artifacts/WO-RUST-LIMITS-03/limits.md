# WO-RUST-LIMITS-03: limits

| constant | file | old | new | proof (boundary N ok / N+1 refused) | peak RSS at N (release, 128x128) |
|---|---|---|---|---|---|
| `MAX_SHAPES` (=`MAX_USE_INSTANCES`) | vexel-scene `types.rs` | 262,144 | 524,288 | scene `limits03_shape_literal.rs`; svg-decode `limits03_boundaries.rs` (flat and `<use>`) | flat 1.58 GB; `<use>` 1.86 GB |
| `MAX_STROKE_STEPS` | vexel-render `lib.rs` | 1,000,000 | 4,000,000 | `dash/tests.rs` budget; `limit03_stroke_jobs.rs` 800,000 / 800,001 ranges; `validate.rs` solid ring 2,000,000 / 2,000,001 | 500,000 ranges (4.0M steps) 587 MB |
| `MAX_STROKE_BYTES` | vexel-render `lib.rs` | 16 MiB | 192 MiB | `dash/tests.rs` 4,194,304 joins / +1; refused push allocates nothing | 1.99M-edge ring (176 MB primitives) 882 MB |
| `MAX_RULE_MATCHES` | vexel-svg-css `lib.rs` | 4,194,304 | 8,388,608 | `limits03_rule_matches.rs` (`CssError::Limit` at +1) | 4096 rules x 2000 elements 19 MB |
| `MAX_USE_TARGET_BYTES` | vexel-svg-decode `use_expansion.rs` | 64 KiB | 1 MiB | `limits03_boundaries.rs` (`Limit(References)` at +1) | 1 MiB x16 targets 13 MB |

Base RSS at the largest base-accepted count: flat 554 MB; `<use>` 629 MB; dashed 267 MB; stroke 156 MB.

Moved pins: `errors.rs`, `original_limits_math.rs`, `original_references.rs`, `shape_cap_contracts.rs` (see result.md).

External sweep (39 files, 60 s, 2048 MiB), base to after: ok 12 to 18; limit_decode 10 to 7; limit_render 5 to 0; unsupported 9 to 10; xml 3 to 3; timeout 0 to 1 (giant_path_t2). No ok file regressed; no rss_exceeded or crash. Largest peak RSS 1.42 GB (inkscape_t2).

Remaining ceiling: 2 GiB is reached near 609k `<use>` instances (about 3.4 KB each, extrapolated) or about 725k flat shapes (about 2.8 KB each). The shape and instance caps keep about 14 percent headroom.

Cancellation under the largest accepted stroke job (800,000 ranges): 53 ms release; 375 ms debug (test bound 1 s in debug, 250 ms in release).
