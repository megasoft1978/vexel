# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `b2092b74da9c5b3ceb51494e46c292c9fac05d49` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 1 |
| unsupported | 0 |
| limit_decode | 0 |
| limit_render | 0 |
| xml | 0 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 1 files, summed wall time 48 ms.

## Slowest five (wall ms)

- `natural-earth-world-map-blank` 48

## Largest peak RSS five (bytes)

- `natural-earth-world-map-blank` 19021824

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| natural-earth-world-map-blank | T1 | real | 241682 | ok | 48 | 19021824 | 128x128 frame, 0 disclosure(s) |
