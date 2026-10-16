# Candidate evidence: WO-RUST-TILE-CACHE-ORACLE-01

Candidate location: `.cache/WO-RUST-TILE-CACHE-ORACLE-01/worker/`.
The exact C oracle source is unchanged. All source checks below are directly mapped to Rust tests; `PASS` means the mapped test passed in the required offline crate test run. The deterministic JSON expected value uses independently asserted fixed accounting (metadata 5,600 + 100 × 4 payload bytes = 6,000), not actual stats.

## Frozen source and implementation hashes

- Frozen C source SHA-256: `e69a30865eaa2212561b9283d0cb2b6325ed72760d557981c8b56406cb8ab602` (scope expected `e69a30865eaa2212561b9283d0cb2b6325ed72760d557981c8b56406cb8ab602`)
- Candidate Rust cache `src/lib.rs` SHA-256: `85c870f4c7af8ef015f9be1a96f6439ec8091fc6fc057974a4e2bca360a44db8`
- Candidate Rust cache `tests/cache.rs` SHA-256: `77b18901fa24bbec1aced8405d1f97b90e61f6976b69e2c23acb71793f4a6b55`
- Frozen scope SHA-256: `7e0542f124c6da1e423588db2e18fac6b440f862811a48363ad49e3ad0b88ed3`
- Candidate C source copy matches original: `True`

## Assertion mapping (29/29)

| C line | Frozen assertion | Rust test evidence | Result |
|---:|---|---|---|
| 19 | `CHECK(!raster_tile_cache_create(0, 2, 1, 1, &cache));` | `c_oracle_invalid_and_valid_cache_limits` | PASS |
| 20 | `CHECK(!raster_tile_cache_create(RASTER_TILE_CACHE_MAX_BYTES + 1U, 2, 1, 1, &cache));` | `c_oracle_invalid_and_valid_cache_limits` | PASS |
| 21 | `CHECK(!raster_tile_cache_create(8192, 500, 1, 1, &cache));` | `c_oracle_invalid_and_valid_cache_limits` | PASS |
| 22 | `CHECK(raster_tile_cache_create(1024, 2, 1, 1, &cache));` | `c_oracle_invalid_and_valid_cache_limits` | PASS |
| 24 | `CHECK(raster_tile_cache_get_stats(cache, &initial_stats));` | `c_oracle_invalid_and_valid_cache_limits` | PASS |
| 25 | `CHECK(initial_stats.accounted_bytes <= initial_stats.byte_cap);` | `c_oracle_invalid_and_valid_cache_limits` | PASS |
| 27 | `CHECK(raster_tile_create(17, 17, &bad));` | `c_oracle_oversized_tile_failure_preserves_caller_ownership` | PASS |
| 29 | `CHECK(!raster_tile_cache_put(cache, 1, &bad) && bad.rgba == owned);` | `c_oracle_oversized_tile_failure_preserves_caller_ownership` | PASS |
| 33 | `CHECK(raster_tile_cache_put(cache, 1, &a) && a.rgba == NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 34 | `CHECK(raster_tile_cache_put(cache, 2, &b) && b.rgba == NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 35 | `CHECK(raster_tile_cache_get(cache, 1) != NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 36 | `CHECK(raster_tile_cache_put(cache, 3, &c) && c.rgba == NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 37 | `CHECK(raster_tile_cache_get(cache, 1) != NULL && raster_tile_cache_get(cache, 2) == NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 38 | `CHECK(raster_tile_cache_get(cache, 3) != NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 39 | `CHECK(a_ptr != NULL);` | `c_oracle_one_pixel_insert_transfers_ownership_and_lru_is_deterministic` | PASS |
| 41 | `CHECK(raster_tile_cache_get(cache, 3) != NULL);` | `tests::c_contract_clock_rollover_order` | PASS |
| 43 | `CHECK(raster_tile_cache_put(cache, 4, &e) && e.rgba == NULL);` | `tests::c_contract_clock_rollover_order` | PASS |
| 44 | `CHECK(raster_tile_cache_get(cache, 1) == NULL && raster_tile_cache_get(cache, 2) == NULL);` | `tests::c_contract_clock_rollover_order` | PASS |
| 45 | `CHECK(raster_tile_cache_get(cache, 3) != NULL && raster_tile_cache_get(cache, 4) != NULL);` | `tests::c_contract_clock_rollover_order` | PASS |
| 46 | `CHECK(raster_tile_cache_set_epoch(cache, 2, 1));` | `c_oracle_generation_and_scale_both_invalidate_entries` | PASS |
| 47 | `CHECK(raster_tile_cache_get(cache, 1) == NULL && raster_tile_cache_get(cache, 3) == NULL && raster_tile_cache_get(cache, 4) == NULL);` | `c_oracle_generation_and_scale_both_invalidate_entries` | PASS |
| 49 | `CHECK(raster_tile_cache_put(cache, 4, &d));` | `c_oracle_generation_and_scale_both_invalidate_entries` | PASS |
| 50 | `CHECK(raster_tile_cache_set_epoch(cache, 2, 2));` | `c_oracle_generation_and_scale_both_invalidate_entries` | PASS |
| 51 | `CHECK(raster_tile_cache_get(cache, 4) == NULL);` | `c_oracle_generation_and_scale_both_invalidate_entries` | PASS |
| 55 | `CHECK(raster_tile_cache_create(cap, max_entries, 7, 3, &cache));` | `c_oracle_10000_insertions_preserve_ownership_and_bounded_stats` | PASS |
| 59 | `CHECK(tile.rgba != NULL && raster_tile_cache_put(cache, i, &tile) && tile.rgba == NULL);` | `c_oracle_10000_insertions_preserve_ownership_and_bounded_stats` | PASS |
| 61 | `CHECK(raster_tile_cache_get_stats(cache, &stats));` | `c_oracle_10000_insertions_preserve_ownership_and_bounded_stats` | PASS |
| 62 | `CHECK(stats.accounted_bytes <= cap && stats.entry_count <= max_entries);` | `c_oracle_10000_insertions_preserve_ownership_and_bounded_stats` | PASS |
| 66 | `CHECK(raster_tile_cache_get_stats(cache, &stats));` | `c_oracle_10000_insertions_preserve_ownership_and_bounded_stats` | PASS |

## JSON summary

Expected and actual deterministic JSON are byte-identical. The test independently asserts the 100-slot reserved metadata charge is 5,600 bytes and the 100 retained 1x1 RGBA payloads add 400 bytes, then asserts final and maximum accounting are 6,000. The actual summary is built from runtime stats and compared against a fixed expected JSON literal; the captured emitted line is in `json-summary.txt`.

Expected: `{"insertions":10000,"entry_count":100,"accounted_bytes":6000,"byte_cap":8192,"max_accounted_bytes":6000,"max_entries":100,"ownership_success":true,"ownership_failure":true,"deterministic_lru":true,"clock_rollover":true,"generation_invalidation":true,"scale_invalidation":true}`

Actual: `{"insertions":10000,"entry_count":100,"accounted_bytes":6000,"byte_cap":8192,"max_accounted_bytes":6000,"max_entries":100,"ownership_success":true,"ownership_failure":true,"deterministic_lru":true,"clock_rollover":true,"generation_invalidation":true,"scale_invalidation":true}`

## Verification

- `cargo +stable check -p vexel-tile-cache --locked --offline`: PASS
- `cargo +stable test -p vexel-tile-cache --locked --offline`: PASS (3 unit, 13 integration, 0 doc tests failed)
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-tile-cache -- --check`: PASS
- `python3 scripts/wo_validate.py WO-RUST-TILE-CACHE-ORACLE-01 --ready`: PASS
- `python3 scripts/ledger.py validate ledger.jsonl`: PASS (1776 entries)
- Raw outputs: `check.txt`, `test.txt`, `fmt.txt`, `wo_validate.txt`, `ledger_validate.txt`, `json-summary.txt`.

## Resolved setup issue

The first isolated ledger validation lacked the full frozen `work/` input set and reported missing `WO-BOOT-01.md`. The candidate seed was completed from the repository, the target directory was corrected to candidate-local `worker/target/`, and all required checks were rerun successfully. No contract or package check failures remain.

## Scope and path audit

Candidate `src/lib.rs` received rustfmt-only formatting (no behavior change); `tests/cache.rs` contains rustfmt cleanup plus the added parity tests. Candidate evidence files record the run. The worker did not change manifests, locks, configuration, the frozen scope, C oracle, root source, ledger, or `STATE.md`. Candidate-input copies are seed files; generated Cargo output is confined to candidate `target/`.

- Rust source lines: `lib.rs` 313; test lines: `cache.rs` 277; both below 800.
- Modified crate paths: `rust/crates/vexel-tile-cache/src/lib.rs` (format-only) and `rust/crates/vexel-tile-cache/tests/cache.rs` (parity tests plus formatting).
- Evidence paths: `artifacts/WO-RUST-TILE-CACHE-ORACLE-01/candidate/*` except frozen `scope.json`.
- No production logic changes were needed; existing cache semantics already satisfy the covered C contract.
