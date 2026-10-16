# WO-RUST-LARGE-DOC-01 result

## New constants (`compose/decode.rs`)
- `MAX_SVG_BYTES` 8 MiB -> 32 MiB (shared by `open.rs` and `decode`).
- `TOKEN_STORAGE_LIMIT` = 8 x cap (256 MiB; scene clamps there). Was `1 << 24`.
- `LEDGER_RESERVE` = 2 x cap (64 MiB). Was `1 << 24`.
- `ComposeError::LimitExceeded` split into `LimitDecode` / `LimitRender`; corpus verdicts `limit_decode` / `limit_render` (legacy `limit` still parses in `compare`, matches either).

## Limit sites found (failing test, not guessing)
- Lexical token storage (`1 << 24`) was the real hidden site: `token_dense_source_is_not_refused_by_token_storage` fails with it, passes now.
- Ledger reserve `1 << 24`: derived, no failing case seen.
- `vexel-svg-values::MAX_SOURCE_BYTES` (16 MiB): a 21 MiB path `d` and 18-30 MiB rect documents compose; NOT changed.
- Not changeable here (protected crates): `use` expansion (65,536 instances, 1 MiB reference bytes) -> `Limit(References)`; XML `AttributeLimit`; shapes 65,536 (see below).
- Shapes budget raised to 1,048,576 as an experiment and reverted: cad/cnc/science_t2 then timed out (>60 s) instead, no `ok` gain.

## External sweep (39 attempted, 32 MiB, 60 s, 2048 MiB)
| verdict | before | after |
|---|---|---|
| ok | 1 | 2 |
| unsupported | 7 | 14 |
| limit (old) | 28 | 0 |
| limit_decode | - | 4 |
| limit_render | - | 1 |
| xml | 2 | 10 |
| timeout | 1 | 8 |
| rss_exceeded | 0 | 0 |
(+19 skipped >32 MiB both runs.) New ok: commons-2015-louisiana-...-precinct (59.7 s wall, 417 MB RSS: 0.3 s from the timeout, may flip).
No baseline-ok regression. In-repo: 17 ok / 7 unsupported, 0 other, per-file unchanged.

## Findings (AC4)
- 7 new timeouts, all previously `limit`: cad_t2, cnc_t2, map_t2, science_t2, bulgaria, cambodia, corvus. Cause: decode now succeeds; render is slow (WO-RUST-RENDER-PERF-01). Peak RSS max 417 MB.
- Remaining `limit_decode`: use_t1, use_t2, chuaattractor, ideogram-chromosome-20 (`Limit(References)` in vexel-svg-decode). `limit_render`: giant_path_t2.
- New xml/unsupported blockers are now visible (viewport at byte 70, path data, `<use>`, `<text>`, `<marker>`); owned by CORPUS-UNSUPPORTED-01.

## Cancellation
Immediate cancel on a 12 MiB source stops at the first checkpoint (1 poll).

## Unmeasured
Live app open of a big file; >32 MiB streaming decode; source copy remains one `to_vec` (no second copy added).


## Reviewer corrections (coordinator, 2026-10-16)

- The claimed external `ok` 1 -> 2 did not reproduce: the reviewer measured ok 1 (`generated-giant_path_t1`); the Louisiana map runs at the 60 s timeout boundary. The accepted outcome rests on AC2's alternative (every remaining blocker named).
- The 417 MB peak RSS claim did not reproduce: the reviewer measured 219 MB on the worst 31 MiB synthetic and 339 MB on the real corpus. Both are far below the 2 GiB bound. Timed-out files report no RSS.
- Measured before the render-performance order landed; see the post-merge sweep recorded in the ledger/STATE for current numbers.
