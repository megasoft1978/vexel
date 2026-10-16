# WO-RUST-V1-CORPUS-MEASURE-01 result

Verdict counts and single-run timings only; no fidelity or performance claim. Host: macOS arm64 (see report.md headers). Release build, 128x128, skip over 33554432 bytes, 60 s timeout, 2048 MiB RSS limit.

| set | ok | unsupported | limit | xml | timeout | skipped | crash | guard_failed |
|---|---|---|---|---|---|---|---|---|
| in-repo (24) | 17 | 7 | 0 | 0 | 0 | 0 | 0 | 0 |
| external Rust (58) | 1 | 7 | 28 | 2 | 1 | 19 | 0 | 0 |
| legacy WO-M4-10 (39 tested) | 3 | 13 | 13 | 8 (+2 viewport) | - | 19 | - | - |
| legacy WO-CORPUS-04 candidate | 5 | 11 | 15 | 4 (+2 viewport, 2 path) | - | 19 | - | - |

Rust external: 1/39 attempted ok (`generated-giant_path_t1`, new vs both legacy baselines).
Regressions vs 3/39: `generated-cnc_t1` (ok -> timeout at 60 s), `generated-embedded_t1` (-> unsupported), `commons-2015-louisiana-...` (-> limit). Vs 5/39 also `generated-embedded_t2`, `generated-science_t1`.
26 of 28 `limit` verdicts are sources above the 8 MiB `MAX_SVG_BYTES` decode cap (rejected in about 12 ms each, so they say nothing about the renderer). The other two are genuine renderer limits: `generated-giant_path_t2` (8,340,030 bytes, 354 ms) and `generated-use_t1` (986,323 bytes, 6.7 s).

ComposeError mapping: Unsupported, Hidden -> unsupported; LimitExceeded -> limit; Malformed, InvalidUtf8 -> xml; InvalidSize, Cancelled -> crash (internal, cannot occur here).

Disclosures: (1) legacy baselines came from untracked copies in the main checkout `.cache` (git was refused by a sandbox hook); the WO-M4-10 copy shows 13/13/8/2, not the order's 17/2/11/2/4 (that figure is WO-CORPUS-01 and had no readable per-file file). (2) Skipped files are checked by size only, never read or hashed. (3) `compare` gained `--baseline label=path` and `--append`. (4) Nothing committed; git unavailable.

Unexecuted: other hosts, repeat runs, tag-sourced baselines, `git diff --check`.
